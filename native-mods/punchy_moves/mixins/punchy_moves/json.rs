//! A minimal, dependency-free JSON parser for Punchy Moves' animation and
//! binding files.
//!
//! The Fabric mod used Google's Gson; this port is std-only. The `Value`
//! tree keeps object members in document order (a `Vec` of pairs with a
//! linear-scan lookup) so the library's loading code sees the file exactly
//! as it was written — the same property Gson's `JsonObject` has.

/// One JSON value in document order.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    /// Members in document order; `get` scans linearly.
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|n| n as f32)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(members) => Some(members),
            _ => None,
        }
    }

    /// The first member with this key (document order).
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Whether the value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    /// Whether the value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Removes and returns the first member with this key, if the value is
    /// an object.
    pub fn remove_member(&mut self, name: &str) -> Option<Value> {
        if let Value::Object(members) = self
            && let Some(pos) = members.iter().position(|(k, _)| k == name)
        {
            return Some(members.remove(pos).1);
        }
        None
    }

    /// The member as a `f32`, or `fallback` when absent or not a number.
    pub fn number_or(&self, name: &str, fallback: f32) -> f32 {
        self.get(name).and_then(Value::as_f32).unwrap_or(fallback)
    }

    /// The member as a string, or `fallback` when absent or not a string.
    pub fn string_or(&self, name: &str, fallback: &str) -> String {
        self.get(name)
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_string()
    }

    /// The member as a string, or `None` when absent, not a string, or blank
    /// (Gson's `getAsString` semantics the library relies on).
    pub fn non_empty_string(&self, name: &str) -> Option<String> {
        self.get(name)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }
}

/// Parses one JSON document. Trailing content after the top-level value is
/// an error, like every standard JSON parser.
pub fn parse(text: &str) -> Result<Value, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut parser = Parser { chars, index: 0 };
    parser.skip_whitespace();
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.index != parser.chars.len() {
        return Err(parser.error("unexpected trailing content"));
    }
    Ok(value)
}

/// Maximum nesting depth: guards the recursion against pathological input.
const MAX_DEPTH: usize = 128;

struct Parser {
    chars: Vec<char>,
    index: usize,
}

impl Parser {
    fn error(&self, message: &str) -> String {
        format!("JSON error at offset {}: {}", self.index, message)
    }

    fn error_at(&self, index: usize, message: &str) -> String {
        format!("JSON error at offset {}: {}", index, message)
    }

    fn skip_whitespace(&mut self) {
        while matches!(
            self.chars.get(self.index),
            Some(' ') | Some('\t') | Some('\n') | Some('\r')
        ) {
            self.index += 1;
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err(self.error("nesting too deep"));
        }
        self.skip_whitespace();
        match self.peek() {
            None => Err(self.error("unexpected end of input")),
            Some('{') => self.parse_object(depth),
            Some('[') => self.parse_array(depth),
            Some('"') => Ok(Value::String(self.parse_string()?)),
            Some('t') => self.parse_keyword("true", Value::Bool(true)),
            Some('f') => self.parse_keyword("false", Value::Bool(false)),
            Some('n') => self.parse_keyword("null", Value::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(self.error_at(self.index, &format!("unexpected character {:?}", c))),
        }
    }

    fn parse_keyword(&mut self, keyword: &str, value: Value) -> Result<Value, String> {
        for expected in keyword.chars() {
            match self.peek() {
                Some(c) if c == expected => self.index += 1,
                _ => return Err(self.error("invalid literal")),
            }
        }
        Ok(value)
    }

    fn parse_object(&mut self, depth: usize) -> Result<Value, String> {
        self.index += 1; // '{'
        let mut members = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.index += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(self.error("expected ':'"));
            }
            self.index += 1;
            let value = self.parse_value(depth + 1)?;
            members.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(',') => self.index += 1,
                Some('}') => {
                    self.index += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<Value, String> {
        self.index += 1; // '['
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.index += 1;
            return Ok(Value::Array(items));
        }
        loop {
            let value = self.parse_value(depth + 1)?;
            items.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => self.index += 1,
                Some(']') => {
                    self.index += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.index;
        if self.peek() == Some('-') {
            self.index += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.index += 1;
        }
        if self.peek() == Some('.') {
            self.index += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.index += 1;
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.index += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.index += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.index += 1;
            }
        }
        let text: String = self.chars[start..self.index].iter().collect();
        text.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| self.error_at(start, &format!("invalid number {:?}", text)))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        let open = self.index;
        self.index += 1; // opening quote
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(self.error_at(open, "unterminated string"));
            };
            self.index += 1;
            match c {
                '"' => return Ok(out),
                '\\' => out.push(self.parse_escape()?),
                c if (c as u32) < 0x20 => {
                    return Err(self.error("unescaped control character in string"));
                }
                c => out.push(c),
            }
        }
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        let Some(c) = self.peek() else {
            return Err(self.error("unterminated escape"));
        };
        self.index += 1;
        Ok(match c {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'b' => '\u{0008}',
            'f' => '\u{000C}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => self.parse_unicode_escape()?,
            other => {
                return Err(self.error(&format!("invalid escape \\{}", other)));
            }
        })
    }

    /// `\uXXXX`, including surrogate pairs.
    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let first = self.parse_hex4()?;
        if !(0xD800..0xDC00).contains(&first) {
            return Ok(char::from_u32(first as u32).unwrap_or('\u{FFFD}'));
        }
        // High surrogate: a low surrogate must follow.
        if self.peek() == Some('\\') {
            self.index += 1;
            if self.peek() == Some('u') {
                self.index += 1;
                let second = self.parse_hex4()?;
                if (0xDC00..0xE000).contains(&second) {
                    let combined =
                        0x10000 + (((first - 0xD800) as u32) << 10) + (second - 0xDC00) as u32;
                    return Ok(char::from_u32(combined).unwrap_or('\u{FFFD}'));
                }
                return Err(self.error("invalid low surrogate"));
            }
        }
        Err(self.error("unpaired surrogate"))
    }

    fn parse_hex4(&mut self) -> Result<u16, String> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let Some(c) = self.peek() else {
                return Err(self.error("unterminated unicode escape"));
            };
            let digit = c
                .to_digit(16)
                .ok_or_else(|| self.error("invalid \\u escape"))?;
            value = value * 16 + digit as u16;
            self.index += 1;
        }
        Ok(value)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(text: &str) -> Value {
        parse(text).unwrap_or_else(|e| panic!("parse failed: {}", e))
    }

    #[test]
    fn parses_scalars() {
        assert_eq!(parse_ok("null"), Value::Null);
        assert_eq!(parse_ok("true"), Value::Bool(true));
        assert_eq!(parse_ok("false"), Value::Bool(false));
        assert_eq!(parse_ok("0"), Value::Number(0.0));
        assert_eq!(parse_ok("-12.5"), Value::Number(-12.5));
        assert_eq!(parse_ok("1e3"), Value::Number(1000.0));
        assert_eq!(parse_ok("-1.25e-2"), Value::Number(-0.0125));
        assert_eq!(parse_ok("\"hi\""), Value::String("hi".into()));
    }

    #[test]
    fn parses_containers_in_document_order() {
        let v = parse_ok("{\"b\":1,\"a\":[true,null,{\"k\":\"v\"}]}");
        let members = v.as_object().unwrap();
        // Document order is preserved (b before a).
        assert_eq!(members[0].0, "b");
        assert_eq!(members[1].0, "a");
        let inner = members[1].1.as_array().unwrap();
        assert_eq!(inner.len(), 3);
        assert_eq!(
            inner[2],
            Value::Object(vec![("k".into(), Value::String("v".into()))])
        );
        assert_eq!(v.get("a").unwrap().as_array().unwrap().len(), 3);
        assert!(v.get("missing").is_none());
    }

    #[test]
    fn parses_string_escapes() {
        assert_eq!(
            parse_ok("\"a\\nb\\t\\\\c\\\"d\""),
            Value::String("a\nb\t\\c\"d".into())
        );
        assert_eq!(parse_ok("\"\\u0041\""), Value::String("A".into()));
        // Surrogate pair for U+1F600.
        assert_eq!(
            parse_ok("\"\\uD83D\\uDE00\""),
            Value::String("\u{1F600}".into())
        );
        // Unpaired surrogate is an error.
        assert!(parse("\"\\uD83D\"").is_err());
        assert!(parse("\"\\u00ZZ\"").is_err());
    }

    #[test]
    fn rejects_malformed_documents() {
        for text in [
            "",
            "{",
            "[",
            "{\"a\"}",
            "{\"a\":}",
            "[1,]",
            "[1 2]",
            "{\"a\":1,}",
            "tru",
            "nul",
            "01a",
            "+1",
            "01x",
            "\"unterminated",
            "1 2",
            "{\"a\":1,}",
        ] {
            assert!(parse(text).is_err(), "parsed {:?}", text);
        }
        // Trailing content after the top-level value.
        assert!(parse("{} {}").is_err());
        // Deep nesting beyond the guard.
        let deep = "[".repeat(200) + &"]".repeat(200);
        assert!(parse(&deep).is_err());
    }

    #[test]
    fn accessors_behave() {
        let v = parse_ok("{\"n\":-1.25,\"b\":true,\"s\":\"hi\",\"arr\":[1],\"nul\":null}");
        assert_eq!(v.number_or("n", 9.0), -1.25);
        assert_eq!(v.number_or("missing", 9.0), 9.0);
        assert_eq!(v.number_or("s", 9.0), 9.0);
        assert_eq!(v.string_or("s", "fb"), "hi");
        assert_eq!(v.string_or("missing", "fb"), "fb");
        assert_eq!(v.string_or("n", "fb"), "fb");
        assert_eq!(v.non_empty_string("s"), Some("hi".into()));
        assert_eq!(v.non_empty_string("missing"), None);
        assert!(v.has("nul"));
        assert!(v.get("nul").unwrap().is_null());
        assert_eq!(v.get("arr").unwrap().as_array().unwrap().len(), 1);
        assert_eq!(v.get("b").unwrap().as_bool(), Some(true));
        assert_eq!(v.get("n").unwrap().as_f64(), Some(-1.25));
        assert_eq!(v.get("n").unwrap().as_f32(), Some(-1.25));
    }
}
