//! Animation expressions for Punchy Moves, ported from the Fabric mod's
//! `punchymoves.client.animation.AnimationExpression` and its nested
//! `Context`, `Value`, `Node` and `Parser`.
//!
//! The grammar, lowest binding first: `||`, `&&`, `==`/`!=`, `>=`/`<=`/`>`/`<`,
//! `+`/`-`, `*`/`/`/`%`, unary `!`/`-`/`+`, then primaries (numbers, the
//! `true`/`false`/`pi` literals, variables and the built-in functions `sin`,
//! `cos`, `abs`, `sqrt`, `round`, `random`, `min`, `max`, `smoothstep`,
//! `clamp` and `lerp`). Everything the Java class computes in `double` stays
//! `double` here; only `eval_number` narrows to `f32`, like the bytecode's
//! `d2f`.
//!
//! One deliberate difference: the Java parser accepts a two- or three-arg
//! function call with missing arguments and throws when the node is later
//! evaluated, which the `number`/`bool` helpers turn into their default.
//! Here the parser rejects the call outright, so `compile` returns `None` and
//! the same helpers return the same default.

use std::collections::HashMap;

/// The values an expression can read while it is evaluated: variables by
/// name, plus the seed its `random()` draws mix in.
pub trait Context {
    /// A numeric variable, or `None` when the name is unknown.
    fn number(&self, name: &str) -> Option<f64>;
    /// A boolean variable, or `None` when the name is unknown.
    fn bool(&self, name: &str) -> Option<bool>;
    /// The seed `random()` mixes into its draws. Zero unless overridden.
    fn random_seed(&self) -> i64 {
        0
    }
    /// One pseudo-random draw in `[0, 1)`, mixed from the seed and `bound`:
    /// the Java interface's splitmix64-based default, bit for bit.
    fn random(&self, bound: f64) -> f64 {
        let bits = java_double_to_long_bits(if bound.is_finite() { bound } else { 0.0 });
        let mut z = self.random_seed() ^ bits ^ -7046029254386353131i64;
        z ^= ((z as u64) >> 30) as i64;
        z = z.wrapping_mul(-4658895280553007687i64);
        z ^= ((z as u64) >> 27) as i64;
        z = z.wrapping_mul(-7723592293110705685i64);
        z ^= ((z as u64) >> 31) as i64;
        ((z as u64) >> 11) as f64 * 1.1102230246251565e-16
    }
}

/// `Double.doubleToLongBits`: NaN collapses to the canonical quiet NaN.
fn java_double_to_long_bits(value: f64) -> i64 {
    if value.is_nan() {
        0x7ff8000000000000i64
    } else {
        value.to_bits() as i64
    }
}

/// The context with no variables: the Java class's `EMPTY`.
pub struct EmptyContext;

impl Context for EmptyContext {
    fn number(&self, _name: &str) -> Option<f64> {
        None
    }
    fn bool(&self, _name: &str) -> Option<bool> {
        None
    }
}

/// The empty context the helpers fall back to, like the Java `EMPTY`.
pub const EMPTY: EmptyContext = EmptyContext;

/// A context over a name-to-number map: the Java `mapContext`. Its `bool`
/// never answers, exactly like the Java wrapper's.
pub struct MapContext<'a> {
    variables: &'a HashMap<String, f64>,
}

impl<'a> MapContext<'a> {
    pub fn new(variables: &'a HashMap<String, f64>) -> Self {
        Self { variables }
    }
}

impl Context for MapContext<'_> {
    fn number(&self, name: &str) -> Option<f64> {
        self.variables.get(name).copied()
    }
    fn bool(&self, _name: &str) -> Option<bool> {
        None
    }
}

/// One evaluated value: the Java record `Value`, a `double` in disguise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Value(f64);

impl Value {
    /// Wraps a number.
    pub fn new(number: f64) -> Self {
        Self(number)
    }
    /// Wraps a boolean as its 0.0/1.0 number, like the Java `Value.bool`.
    pub fn from_bool(value: bool) -> Self {
        Self(if value { 1.0 } else { 0.0 })
    }
    /// The number itself.
    pub fn number(self) -> f64 {
        self.0
    }
    /// The number as a boolean: anything but 0.0 is true, NaN included.
    pub fn bool(self) -> bool {
        self.0 != 0.0
    }
}

/// One parsed expression tree node: the Java interface `Node` and the shapes
/// its lambdas take. Function arguments are stored in call order.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A number literal.
    Number(f64),
    /// The `true` or `false` literal.
    Bool(bool),
    /// The `pi` literal.
    Pi,
    /// A variable, looked up in the context when evaluated.
    Variable(String),
    Not(Box<Node>),
    Neg(Box<Node>),
    Or(Box<Node>, Box<Node>),
    And(Box<Node>, Box<Node>),
    Equal(Box<Node>, Box<Node>),
    NotEqual(Box<Node>, Box<Node>),
    GreaterOrEqual(Box<Node>, Box<Node>),
    LessOrEqual(Box<Node>, Box<Node>),
    Greater(Box<Node>, Box<Node>),
    Less(Box<Node>, Box<Node>),
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    /// Division; a zero divisor yields 0.0, like the Java lambda.
    Divide(Box<Node>, Box<Node>),
    /// Remainder; a zero divisor yields 0.0, like the Java lambda.
    Remainder(Box<Node>, Box<Node>),
    Sin(Box<Node>),
    Cos(Box<Node>),
    Abs(Box<Node>),
    /// `sqrt`, of the argument clamped at zero from below.
    Sqrt(Box<Node>),
    /// `round`, Java's `Math.round`: half up, towards positive infinity.
    Round(Box<Node>),
    /// `random(x)`: one context draw mixed from `x`.
    Random(Box<Node>),
    Min(Box<Node>, Box<Node>),
    Max(Box<Node>, Box<Node>),
    /// `smoothstep(edge0, edge1, x)`.
    Smoothstep(Box<Node>, Box<Node>, Box<Node>),
    /// `clamp(value, low, high)`.
    Clamp(Box<Node>, Box<Node>, Box<Node>),
    /// `lerp(t, from, to)`: `from + (to - from) * t`.
    Lerp(Box<Node>, Box<Node>, Box<Node>),
}

impl Node {
    /// Evaluates the node. `None` stands for the Java evaluator's
    /// `RuntimeException`, e.g. an unknown variable, which the helpers turn
    /// into their default.
    pub fn eval(&self, ctx: &dyn Context) -> Option<Value> {
        let number = |node: &Node| Some(node.eval(ctx)?.number());
        match self {
            Node::Number(value) => Some(Value(*value)),
            Node::Bool(value) => Some(Value::from_bool(*value)),
            Node::Pi => Some(Value(core::f64::consts::PI)),
            Node::Variable(name) => match ctx.bool(name) {
                Some(value) => Some(Value::from_bool(value)),
                None => ctx.number(name).map(Value),
            },
            Node::Not(inner) => Some(Value::from_bool(!inner.eval(ctx)?.bool())),
            Node::Neg(inner) => Some(Value(-inner.eval(ctx)?.number())),
            Node::Or(left, right) => {
                if left.eval(ctx)?.bool() {
                    Some(Value::from_bool(true))
                } else {
                    Some(Value::from_bool(right.eval(ctx)?.bool()))
                }
            }
            Node::And(left, right) => {
                if left.eval(ctx)?.bool() {
                    Some(Value::from_bool(right.eval(ctx)?.bool()))
                } else {
                    Some(Value::from_bool(false))
                }
            }
            Node::Equal(left, right) => Some(Value::from_bool(number(left)? == number(right)?)),
            Node::NotEqual(left, right) => Some(Value::from_bool(number(left)? != number(right)?)),
            Node::GreaterOrEqual(left, right) => {
                Some(Value::from_bool(number(left)? >= number(right)?))
            }
            Node::LessOrEqual(left, right) => {
                Some(Value::from_bool(number(left)? <= number(right)?))
            }
            Node::Greater(left, right) => Some(Value::from_bool(number(left)? > number(right)?)),
            Node::Less(left, right) => Some(Value::from_bool(number(left)? < number(right)?)),
            Node::Add(left, right) => Some(Value(number(left)? + number(right)?)),
            Node::Subtract(left, right) => Some(Value(number(left)? - number(right)?)),
            Node::Multiply(left, right) => Some(Value(number(left)? * number(right)?)),
            Node::Divide(left, right) => {
                let divisor = number(right)?;
                if divisor == 0.0 {
                    Some(Value(0.0))
                } else {
                    Some(Value(number(left)? / divisor))
                }
            }
            Node::Remainder(left, right) => {
                let divisor = number(right)?;
                if divisor == 0.0 {
                    Some(Value(0.0))
                } else {
                    Some(Value(number(left)? % divisor))
                }
            }
            Node::Sin(inner) => Some(Value(number(inner)?.sin())),
            Node::Cos(inner) => Some(Value(number(inner)?.cos())),
            Node::Abs(inner) => Some(Value(number(inner)?.abs())),
            Node::Sqrt(inner) => Some(Value(java_max(0.0, number(inner)?).sqrt())),
            Node::Round(inner) => Some(Value(java_round(number(inner)?))),
            Node::Random(inner) => Some(Value(ctx.random(number(inner)?))),
            Node::Min(left, right) => Some(Value(java_min(number(left)?, number(right)?))),
            Node::Max(left, right) => Some(Value(java_max(number(left)?, number(right)?))),
            Node::Smoothstep(edge0, edge1, x) => {
                let (edge0, edge1, x) = (number(edge0)?, number(edge1)?, number(x)?);
                let t = if edge0 == edge1 {
                    0.0
                } else {
                    java_max(0.0, java_min(1.0, (x - edge0) / (edge1 - edge0)))
                };
                Some(Value(t * t * (3.0 - 2.0 * t)))
            }
            Node::Clamp(value, low, high) => {
                let (value, low, high) = (number(value)?, number(low)?, number(high)?);
                Some(Value(java_max(low, java_min(high, value))))
            }
            Node::Lerp(t, from, to) => {
                let (t, from, to) = (number(t)?, number(from)?, number(to)?);
                Some(Value(from + (to - from) * t))
            }
        }
    }
}

/// `Math.min`: NaN poisons, otherwise the smaller.
fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if b.is_nan() {
        return b;
    }
    if a <= b { a } else { b }
}

/// `Math.max`: NaN poisons, otherwise the larger.
fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if b.is_nan() {
        return b;
    }
    if a >= b { a } else { b }
}

/// `Math.round`: `floor(x + 0.5)` as a `long`, saturated, NaN as 0.
fn java_round(x: f64) -> f64 {
    if x.is_nan() {
        return 0.0;
    }
    let rounded = (x + 0.5).floor();
    if rounded >= 9_223_372_036_854_776.0 {
        i64::MAX as f64
    } else if rounded <= -9_223_372_036_854_776.0 {
        i64::MIN as f64
    } else {
        rounded as i64 as f64
    }
}

/// One compiled expression: the Java `Expression`, its root plus its source.
#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    root: Node,
}

impl Expression {
    /// The parsed tree.
    pub fn root(&self) -> &Node {
        &self.root
    }
}

/// Parses `source` into an expression, or `None` when it is blank, malformed
/// or has trailing input: the Java `compile`, whose `RuntimeException` paths
/// all return null.
pub fn compile(source: &str) -> Option<Expression> {
    if source.trim().is_empty() {
        return None;
    }
    let mut parser = Parser::new(source);
    let root = parser.expression()?;
    parser.space();
    if parser.end() {
        Some(Expression { root })
    } else {
        None
    }
}

/// Evaluates a compiled expression as a number, `default` when the
/// expression is missing, fails or lands outside the finite numbers.
pub fn eval_number(expr: Option<&Expression>, ctx: Option<&dyn Context>, default: f32) -> f32 {
    let Some(expr) = expr else {
        return default;
    };
    match expr.root.eval(ctx.unwrap_or(&EMPTY)) {
        Some(value) if value.number().is_finite() => value.number() as f32,
        _ => default,
    }
}

/// Evaluates a compiled expression as a boolean, `default` when the
/// expression is missing or fails.
pub fn eval_bool(expr: Option<&Expression>, ctx: Option<&dyn Context>, default: bool) -> bool {
    let Some(expr) = expr else {
        return default;
    };
    expr.root
        .eval(ctx.unwrap_or(&EMPTY))
        .map_or(default, |value| value.bool())
}

/// Compiles `source` and evaluates it as a boolean over `variables`, `true`
/// when the expression is missing or fails: the Java `condition`.
pub fn condition(source: &str, variables: &HashMap<String, f64>) -> bool {
    let ctx = MapContext::new(variables);
    eval_bool(compile(source).as_ref(), Some(&ctx), true)
}

/// The recursive-descent parser: the Java `Parser`, char for char.
struct Parser {
    chars: Vec<char>,
    index: usize,
}

impl Parser {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            index: 0,
        }
    }

    fn end(&self) -> bool {
        self.index >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    /// Skips whitespace.
    fn space(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.index += 1;
            } else {
                break;
            }
        }
    }

    /// Skips whitespace, then consumes `token` when it is next.
    fn take(&mut self, token: &str) -> bool {
        self.space();
        let end = self.index + token.chars().count();
        if self
            .chars
            .get(self.index..end)
            .is_some_and(|slice| slice.iter().copied().eq(token.chars()))
        {
            self.index = end;
            true
        } else {
            false
        }
    }

    fn expression(&mut self) -> Option<Node> {
        self.or()
    }

    /// `||`, left-associative, short-circuiting.
    fn or(&mut self) -> Option<Node> {
        let mut left = self.and()?;
        while self.take("||") {
            let right = self.and()?;
            left = Node::Or(Box::new(left), Box::new(right));
        }
        Some(left)
    }

    /// `&&`, left-associative, short-circuiting.
    fn and(&mut self) -> Option<Node> {
        let mut left = self.equality()?;
        while self.take("&&") {
            let right = self.equality()?;
            left = Node::And(Box::new(left), Box::new(right));
        }
        Some(left)
    }

    /// `==` and `!=`, left-associative.
    fn equality(&mut self) -> Option<Node> {
        let mut left = self.comparison()?;
        loop {
            if self.take("==") {
                let right = self.comparison()?;
                left = Node::Equal(Box::new(left), Box::new(right));
            } else if self.take("!=") {
                let right = self.comparison()?;
                left = Node::NotEqual(Box::new(left), Box::new(right));
            } else {
                return Some(left);
            }
        }
    }

    /// `>=`, `<=`, `>` and `<`, left-associative.
    fn comparison(&mut self) -> Option<Node> {
        let mut left = self.sum()?;
        loop {
            if self.take(">=") {
                let right = self.sum()?;
                left = Node::GreaterOrEqual(Box::new(left), Box::new(right));
            } else if self.take("<=") {
                let right = self.sum()?;
                left = Node::LessOrEqual(Box::new(left), Box::new(right));
            } else if self.take(">") {
                let right = self.sum()?;
                left = Node::Greater(Box::new(left), Box::new(right));
            } else if self.take("<") {
                let right = self.sum()?;
                left = Node::Less(Box::new(left), Box::new(right));
            } else {
                return Some(left);
            }
        }
    }

    /// `+` and `-`, left-associative.
    fn sum(&mut self) -> Option<Node> {
        let mut left = self.product()?;
        loop {
            if self.take("+") {
                let right = self.product()?;
                left = Node::Add(Box::new(left), Box::new(right));
            } else if self.take("-") {
                let right = self.product()?;
                left = Node::Subtract(Box::new(left), Box::new(right));
            } else {
                return Some(left);
            }
        }
    }

    /// `*`, `/` and `%`, left-associative.
    fn product(&mut self) -> Option<Node> {
        let mut left = self.unary()?;
        loop {
            if self.take("*") {
                let right = self.unary()?;
                left = Node::Multiply(Box::new(left), Box::new(right));
            } else if self.take("/") {
                let right = self.unary()?;
                left = Node::Divide(Box::new(left), Box::new(right));
            } else if self.take("%") {
                let right = self.unary()?;
                left = Node::Remainder(Box::new(left), Box::new(right));
            } else {
                return Some(left);
            }
        }
    }

    /// Unary `!` and `-`, transparent unary `+`.
    fn unary(&mut self) -> Option<Node> {
        if self.take("!") {
            return Some(Node::Not(Box::new(self.unary()?)));
        }
        if self.take("-") {
            return Some(Node::Neg(Box::new(self.unary()?)));
        }
        if self.take("+") {
            return self.unary();
        }
        self.primary()
    }

    /// A parenthesised expression, a number literal, or an identifier: a
    /// function call, a literal (`true`, `false`, `pi`) or a variable.
    fn primary(&mut self) -> Option<Node> {
        if self.take("(") {
            let inner = self.expression()?;
            if !self.take(")") {
                return None;
            }
            return Some(inner);
        }
        self.space();
        if matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            let start = self.index;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
                self.index += 1;
            }
            let text: String = self.chars[start..self.index].iter().collect();
            return text.parse::<f64>().ok().map(Node::Number);
        }
        let start = self.index;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_' || c == '.') {
            self.index += 1;
        }
        let name: String = self.chars[start..self.index].iter().collect();
        let name = name.to_lowercase();
        if name.is_empty() {
            return None;
        }
        if self.take("(") {
            return self.function(&name);
        }
        match name.as_str() {
            "true" => Some(Node::Bool(true)),
            "false" => Some(Node::Bool(false)),
            "pi" => Some(Node::Pi),
            _ => Some(Node::Variable(name)),
        }
    }

    /// A call to one of the eleven built-in functions. `min` and `max` need
    /// two arguments; `smoothstep`, `clamp` and `lerp` need three; the rest
    /// take one.
    fn function(&mut self, name: &str) -> Option<Node> {
        let first = self.expression()?;
        let mut second = None;
        let mut third = None;
        if self.take(",") {
            second = Some(self.expression()?);
        }
        if self.take(",") {
            third = Some(self.expression()?);
        }
        if !self.take(")") {
            return None;
        }
        match name {
            "sin" => Some(Node::Sin(Box::new(first))),
            "cos" => Some(Node::Cos(Box::new(first))),
            "abs" => Some(Node::Abs(Box::new(first))),
            "sqrt" => Some(Node::Sqrt(Box::new(first))),
            "round" => Some(Node::Round(Box::new(first))),
            "random" => Some(Node::Random(Box::new(first))),
            "min" => Some(Node::Min(Box::new(first), Box::new(second?))),
            "max" => Some(Node::Max(Box::new(first), Box::new(second?))),
            "smoothstep" => Some(Node::Smoothstep(
                Box::new(first),
                Box::new(second?),
                Box::new(third?),
            )),
            "clamp" => Some(Node::Clamp(
                Box::new(first),
                Box::new(second?),
                Box::new(third?),
            )),
            "lerp" => Some(Node::Lerp(
                Box::new(first),
                Box::new(second?),
                Box::new(third?),
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evaluates `source` as a number over `variables`.
    fn number_of(source: &str, variables: &HashMap<String, f64>) -> f32 {
        eval_number(
            compile(source).as_ref(),
            Some(&MapContext::new(variables)),
            f32::NAN,
        )
    }

    fn variables(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    /// A context with a fixed random seed, for the `random` oracle.
    struct Seeded {
        seed: i64,
        bools: HashMap<String, bool>,
    }

    impl Context for Seeded {
        fn number(&self, name: &str) -> Option<f64> {
            match name {
                "one" => Some(1.0),
                "two" => Some(2.0),
                "half" => Some(0.5),
                _ => None,
            }
        }
        fn bool(&self, name: &str) -> Option<bool> {
            self.bools.get(name).copied()
        }
        fn random_seed(&self) -> i64 {
            self.seed
        }
    }

    #[test]
    fn parses_numbers_literals_and_parentheses() {
        assert_eq!(number_of("1.5", &variables(&[])), 1.5);
        assert_eq!(number_of(".5", &variables(&[])), 0.5);
        assert_eq!(number_of("-2", &variables(&[])), -2.0);
        assert_eq!(number_of("+2", &variables(&[])), 2.0);
        assert_eq!(number_of("--2", &variables(&[])), 2.0);
        assert_eq!(number_of("(1+2)*3", &variables(&[])), 9.0);
        assert_eq!(number_of("pi", &variables(&[])), core::f32::consts::PI);
        assert_eq!(number_of("PI", &variables(&[])), core::f32::consts::PI);
        assert!(eval_bool(compile("true").as_ref(), None, false));
        assert!(!eval_bool(compile("false").as_ref(), None, true));
        assert!(eval_bool(compile("TRUE").as_ref(), None, true));
    }

    /// The precedence ladder, each level tighter than the one above it.
    #[test]
    fn precedence_follows_the_java_grammar() {
        let none = variables(&[]);
        assert_eq!(number_of("1+2*3", &none), 7.0);
        assert_eq!(number_of("2*3+1", &none), 7.0);
        assert_eq!(number_of("8-3-2", &none), 3.0);
        assert_eq!(number_of("8/2/2", &none), 2.0);
        assert_eq!(number_of("7%4", &none), 3.0);
        assert_eq!(number_of("-2*3", &none), -6.0);
        assert_eq!(number_of("2*-3", &none), -6.0);
        assert!(eval_bool(compile("1+1==2").as_ref(), None, false));
        assert!(!eval_bool(compile("1+1!=2").as_ref(), None, true));
        assert!(eval_bool(compile("1 < 2 && 2 < 3").as_ref(), None, false));
        assert!(eval_bool(compile("1 > 2 || 2 > 1").as_ref(), None, false));
        assert!(eval_bool(compile("!false && true").as_ref(), None, false));
        assert!(eval_bool(compile("2>=2").as_ref(), None, false));
        assert!(!eval_bool(compile("2<=1").as_ref(), None, true));
        // Comparisons bind tighter than equality, which binds tighter than
        // the logical operators.
        assert!(eval_bool(compile("1+1==2==true").as_ref(), None, false));
        assert!(eval_bool(
            compile("true||false&&false").as_ref(),
            None,
            false
        ));
    }

    /// `&&` and `||` short-circuit, so an unknown variable on the dead side
    /// never fails the expression.
    #[test]
    fn logic_short_circuits() {
        let ctx = Seeded {
            seed: 0,
            bools: HashMap::new(),
        };
        assert!(!eval_bool(
            compile("false && unknown").as_ref(),
            Some(&ctx),
            true
        ));
        assert!(eval_bool(
            compile("true || unknown").as_ref(),
            Some(&ctx),
            false
        ));
        assert!(eval_bool(
            compile("true && unknown").as_ref(),
            Some(&ctx),
            true
        ));
        assert!(!eval_bool(
            compile("false || unknown").as_ref(),
            Some(&ctx),
            false
        ));
    }

    /// Every built-in function, with values pinned by hand.
    #[test]
    fn functions_match_the_java_builtins() {
        let none = variables(&[]);
        assert_eq!(number_of("sin(0)", &none), 0.0);
        assert!((number_of("sin(pi/2)", &none) - 1.0).abs() < 1.0e-6);
        assert_eq!(number_of("cos(0)", &none), 1.0);
        assert_eq!(number_of("abs(-3)", &none), 3.0);
        assert_eq!(number_of("sqrt(9)", &none), 3.0);
        assert_eq!(number_of("sqrt(-4)", &none), 0.0);
        assert_eq!(number_of("round(2.5)", &none), 3.0);
        assert_eq!(number_of("round(-1.5)", &none), -1.0);
        assert_eq!(number_of("round(2.4)", &none), 2.0);
        assert_eq!(number_of("min(3, 2)", &none), 2.0);
        assert_eq!(number_of("max(3, 2)", &none), 3.0);
        assert_eq!(number_of("clamp(5, 0, 1)", &none), 1.0);
        assert_eq!(number_of("clamp(-1, 0, 1)", &none), 0.0);
        assert_eq!(number_of("clamp(0.5, 0, 1)", &none), 0.5);
        assert_eq!(number_of("smoothstep(0, 1, 0.5)", &none), 0.5);
        assert_eq!(number_of("smoothstep(0, 1, 0.25)", &none), 0.15625);
        assert_eq!(number_of("smoothstep(0, 1, 0)", &none), 0.0);
        assert_eq!(number_of("smoothstep(0, 1, 1)", &none), 1.0);
        assert_eq!(number_of("smoothstep(1, 1, 0.5)", &none), 0.0);
        assert_eq!(number_of("lerp(0.25, 10, 20)", &none), 12.5);
        assert_eq!(number_of("lerp(2, 0, 10)", &none), 20.0);
        assert_eq!(number_of("lerp(0, 3, 9)", &none), 3.0);
        // Function names are matched after lowercasing, like identifiers.
        assert_eq!(number_of("SQRT(16)", &none), 4.0);
        assert_eq!(number_of("Min(1, 2)", &none), 1.0);
        // Calls tolerate space before the parenthesis and between arguments.
        assert_eq!(number_of("max ( 1 , 2 )", &none), 2.0);
    }

    /// A zero divisor yields 0.0, not infinity, in both `/` and `%`.
    #[test]
    fn division_by_zero_yields_zero() {
        let none = variables(&[]);
        assert_eq!(number_of("1/0", &none), 0.0);
        assert_eq!(number_of("1%0", &none), 0.0);
        assert_eq!(number_of("0/5", &none), 0.0);
    }

    /// Variables resolve through the context: numbers from the map, booleans
    /// from `bool`, and identifiers keep dots and underscores.
    #[test]
    fn variables_read_from_the_context() {
        let values = variables(&[("speed", 5.0), ("a.b_1", 2.0)]);
        assert_eq!(number_of("speed * 2", &values), 10.0);
        assert_eq!(number_of("a.b_1 + 1", &values), 3.0);
        assert_eq!(number_of("SPEED", &values), 5.0);
        let ctx = Seeded {
            seed: 0,
            bools: [("sneaking".to_string(), true)].into_iter().collect(),
        };
        assert!(eval_bool(compile("sneaking").as_ref(), Some(&ctx), false));
        // A boolean variable wins over a number of the same name.
        let both = Seeded {
            seed: 0,
            bools: [("flag".to_string(), true)].into_iter().collect(),
        };
        assert!(eval_bool(compile("flag").as_ref(), Some(&both), false));
    }

    /// A missing expression, an unknown variable, or a non-finite result all
    /// fall back to the default, like the Java helpers catching
    /// `RuntimeException`.
    #[test]
    fn failures_fall_back_to_the_default() {
        let none = variables(&[]);
        assert_eq!(eval_number(None, None, 0.5), 0.5);
        assert!(eval_bool(None, None, true));
        assert_eq!(eval_number(compile("unknown").as_ref(), None, 0.25), 0.25);
        assert!(eval_bool(compile("unknown").as_ref(), None, true));
        assert_eq!(
            eval_number(compile("unknown").as_ref(), Some(&EMPTY), 0.25),
            0.25
        );
        // The empty context answers nothing, so every variable fails.
        assert!(number_of("1 + unknown", &none).is_nan());
        // An infinite result falls back too: a 400-digit literal parses to
        // infinity in both Java and Rust.
        let big = "9".repeat(400);
        assert_eq!(eval_number(compile(&big).as_ref(), None, 7.0), 7.0);
    }

    /// The Java `condition`: missing or failing expressions count as true.
    #[test]
    fn condition_defaults_to_true() {
        assert!(condition("speed > 4", &variables(&[("speed", 5.0)])));
        assert!(!condition("speed > 6", &variables(&[("speed", 5.0)])));
        assert!(condition("", &variables(&[])));
        assert!(condition("1 +", &variables(&[])));
        // An unknown variable on the live side fails the whole expression,
        // so true; on the dead side of a false `&&` it is never read.
        assert!(condition(
            "unknown && speed > 4",
            &variables(&[("speed", 1.0)])
        ));
        assert!(!condition(
            "speed > 4 && unknown",
            &variables(&[("speed", 1.0)])
        ));
    }

    /// What the Java parser rejects: blank input, trailing input, unbalanced
    /// parentheses, unknown functions and malformed numbers.
    #[test]
    fn malformed_sources_do_not_compile() {
        for source in [
            "",
            "   ",
            "1+",
            "(1",
            "1)",
            "1 2",
            "foo(",
            "nope(1)",
            "1.2.3",
            ".",
            "@",
            "min(1)",
            "clamp(1, 2)",
            "lerp(1, 2)",
            "smoothstep(1, 2)",
            "sin()",
            "max(1,2,3",
        ] {
            assert!(compile(source).is_none(), "compiled {source:?}");
        }
    }

    /// Well-formed sources do compile, whitespace and all.
    #[test]
    fn well_formed_sources_compile() {
        for source in ["0", "1 + 2", "(1)", "sin(0)", "a || b", "!x", "-1.5", "1<2"] {
            assert!(compile(source).is_some(), "rejected {source:?}");
        }
    }

    /// The value's boolean read: anything but 0.0 is true, NaN included.
    #[test]
    fn value_bool_is_number_not_zero() {
        assert!(!Value(0.0).bool());
        assert!(Value(1.0).bool());
        assert!(Value(-0.5).bool());
        assert!(Value(f64::NAN).bool());
        assert_eq!(Value::from_bool(true).number(), 1.0);
        assert_eq!(Value::from_bool(false).number(), 0.0);
    }

    /// The default `random`: the Java interface's splitmix64 finalizer over
    /// the seed and the bound's bit pattern, checked against values computed
    /// independently of this port.
    #[test]
    fn random_matches_the_java_splitmix() {
        let oracle: &[(i64, f64, f64)] = &[
            (0, 0.5, 0.7216154935425203),
            (0, 1.0, 0.7369963625803222),
            (0, 100.0, 0.5590520836369351),
            (42, 0.5, 0.5046243355678185),
            (42, 1.0, 0.14826850179997497),
            (42, 100.0, 0.15748469477422922),
            (-1, 0.5, 0.37779812434422433),
            (-1, 1.0, 0.41997707258606787),
            (-1, 100.0, 0.6860217405702748),
        ];
        for &(seed, bound, expected) in oracle {
            let ctx = Seeded {
                seed,
                bools: HashMap::new(),
            };
            let got = ctx.random(bound);
            assert_eq!(got, expected, "seed {seed}, bound {bound}");
            assert!((0.0..1.0).contains(&got));
        }
        // A non-finite bound collapses to zero's bit pattern first.
        let ctx = Seeded {
            seed: 0,
            bools: HashMap::new(),
        };
        assert_eq!(ctx.random(f64::NAN), ctx.random(0.0));
        assert_eq!(ctx.random(f64::INFINITY), ctx.random(0.0));
        // The plain context seeds with zero.
        assert_eq!(EMPTY.random_seed(), 0);
        assert_eq!(EmptyContext.random(0.5), 0.7216154935425203);
    }

    /// `random()` in an expression draws from the context.
    #[test]
    fn random_function_draws_from_the_context() {
        let ctx = Seeded {
            seed: 42,
            bools: HashMap::new(),
        };
        let expr = compile("random(1)").unwrap();
        assert_eq!(
            expr.root().eval(&ctx).unwrap().number(),
            0.14826850179997497
        );
    }

    /// `Math.min`/`Math.max` poison on NaN, unlike Rust's own.
    #[test]
    fn min_and_max_poison_on_nan() {
        assert!(java_min(5.0, f64::NAN).is_nan());
        assert!(java_min(f64::NAN, 5.0).is_nan());
        assert!(java_max(5.0, f64::NAN).is_nan());
        assert!(java_max(f64::NAN, 5.0).is_nan());
        assert_eq!(java_min(1.0, 2.0), 1.0);
        assert_eq!(java_max(1.0, 2.0), 2.0);
    }

    /// `Math.round`: half away towards positive infinity, NaN as 0, huge
    /// values saturate like a Java `(long)` cast.
    #[test]
    fn round_is_java_math_round() {
        assert_eq!(java_round(2.5), 3.0);
        assert_eq!(java_round(2.4), 2.0);
        assert_eq!(java_round(-1.5), -1.0);
        assert_eq!(java_round(-1.6), -2.0);
        assert_eq!(java_round(f64::NAN), 0.0);
        assert_eq!(java_round(1.0e300), i64::MAX as f64);
        assert_eq!(java_round(-1.0e300), i64::MIN as f64);
    }

    /// The expression tree records what was parsed, so a caller can walk it.
    #[test]
    fn the_tree_exposes_its_shape() {
        let expr = compile("1 + 2 * 3").unwrap();
        assert_eq!(
            expr.root(),
            &Node::Add(
                Box::new(Node::Number(1.0)),
                Box::new(Node::Multiply(
                    Box::new(Node::Number(2.0)),
                    Box::new(Node::Number(3.0))
                ))
            )
        );
    }
}
