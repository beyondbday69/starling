//! Easing curves for Punchy Moves' animations, ported from the Fabric mod's
//! `punchymoves.client.animation.Easing` (30 variants, `byName`, `apply`).
//!
//! The port keeps the Java behaviour exactly: every curve is evaluated in
//! `f64` and narrowed to `f32` once, at the end, like the bytecode's
//! `d2f`; `bounce_out` mirrors the class's private helper, branch for branch.

/// One easing curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
}

/// The back overshoot of the `back` curves (`BACK` in the Java class).
const BACK: f64 = 1.70158;
/// The leading coefficient of the `in` and `out` `back` curves (`BACK + 1`).
const BACK_PLUS: f64 = 2.70158;
/// The back overshoot of the `in-out` `back` curve (`BACK_IN_OUT`).
const BACK_IN_OUT: f64 = BACK * 1.525;
/// One elastic period (`ELASTIC`): `2 * pi / 3`.
const ELASTIC: f64 = 2.0943951023931953;
/// One elastic period for the `in-out` curve (`ELASTIC_IN_OUT`): `4 * pi / 9`.
const ELASTIC_IN_OUT: f64 = 1.3962634015954636;

impl Easing {
    /// Every variant, in the Java enum's declaration order.
    pub const ALL: [Easing; 30] = [
        Easing::EaseInSine,
        Easing::EaseOutSine,
        Easing::EaseInOutSine,
        Easing::EaseInQuad,
        Easing::EaseOutQuad,
        Easing::EaseInOutQuad,
        Easing::EaseInCubic,
        Easing::EaseOutCubic,
        Easing::EaseInOutCubic,
        Easing::EaseInQuart,
        Easing::EaseOutQuart,
        Easing::EaseInOutQuart,
        Easing::EaseInQuint,
        Easing::EaseOutQuint,
        Easing::EaseInOutQuint,
        Easing::EaseInExpo,
        Easing::EaseOutExpo,
        Easing::EaseInOutExpo,
        Easing::EaseInCirc,
        Easing::EaseOutCirc,
        Easing::EaseInOutCirc,
        Easing::EaseInBack,
        Easing::EaseOutBack,
        Easing::EaseInOutBack,
        Easing::EaseInElastic,
        Easing::EaseOutElastic,
        Easing::EaseInOutElastic,
        Easing::EaseInBounce,
        Easing::EaseOutBounce,
        Easing::EaseInOutBounce,
    ];

    /// The curve's name in the mod's animation files, e.g. `easeInOutSine`.
    pub fn id(self) -> &'static str {
        match self {
            Easing::EaseInSine => "easeInSine",
            Easing::EaseOutSine => "easeOutSine",
            Easing::EaseInOutSine => "easeInOutSine",
            Easing::EaseInQuad => "easeInQuad",
            Easing::EaseOutQuad => "easeOutQuad",
            Easing::EaseInOutQuad => "easeInOutQuad",
            Easing::EaseInCubic => "easeInCubic",
            Easing::EaseOutCubic => "easeOutCubic",
            Easing::EaseInOutCubic => "easeInOutCubic",
            Easing::EaseInQuart => "easeInQuart",
            Easing::EaseOutQuart => "easeOutQuart",
            Easing::EaseInOutQuart => "easeInOutQuart",
            Easing::EaseInQuint => "easeInQuint",
            Easing::EaseOutQuint => "easeOutQuint",
            Easing::EaseInOutQuint => "easeInOutQuint",
            Easing::EaseInExpo => "easeInExpo",
            Easing::EaseOutExpo => "easeOutExpo",
            Easing::EaseInOutExpo => "easeInOutExpo",
            Easing::EaseInCirc => "easeInCirc",
            Easing::EaseOutCirc => "easeOutCirc",
            Easing::EaseInOutCirc => "easeInOutCirc",
            Easing::EaseInBack => "easeInBack",
            Easing::EaseOutBack => "easeOutBack",
            Easing::EaseInOutBack => "easeInOutBack",
            Easing::EaseInElastic => "easeInElastic",
            Easing::EaseOutElastic => "easeOutElastic",
            Easing::EaseInOutElastic => "easeInOutElastic",
            Easing::EaseInBounce => "easeInBounce",
            Easing::EaseOutBounce => "easeOutBounce",
            Easing::EaseInOutBounce => "easeInOutBounce",
        }
    }

    /// Looks a curve up by name, the way the Java `byName` does: `_`, `-`
    /// and spaces are dropped, the rest is lowercased, and the name is
    /// compared against every variant's `id`. `None` when nothing matches.
    pub fn by_name(name: &str) -> Option<Easing> {
        let wanted = name.replace(['_', '-', ' '], "").to_ascii_lowercase();
        Easing::ALL
            .into_iter()
            .find(|curve| curve.id().eq_ignore_ascii_case(&wanted))
    }

    /// Draws the curve at `t` (0.0 to 1.0), like the Java `apply`.
    pub fn apply(self, t: f32) -> f32 {
        let t = f64::from(t);
        let value = match self {
            Easing::EaseInSine => 1.0 - (t * core::f64::consts::PI / 2.0).cos(),
            Easing::EaseOutSine => (t * core::f64::consts::PI / 2.0).sin(),
            Easing::EaseInOutSine => (1.0 - (core::f64::consts::PI * t).cos()) / 2.0,
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::EaseInQuart => t * t * t * t,
            Easing::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            Easing::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            Easing::EaseInQuint => t * t * t * t * t,
            Easing::EaseOutQuint => 1.0 - (1.0 - t).powi(5),
            Easing::EaseInOutQuint => {
                if t < 0.5 {
                    16.0 * t * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            }
            Easing::EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0f64.powf(10.0 * t - 10.0)
                }
            }
            Easing::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0f64.powf(-10.0 * t)
                }
            }
            Easing::EaseInOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2.0f64.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0f64.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            Easing::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            Easing::EaseOutCirc => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
            Easing::EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t) * (2.0 * t)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0) * (-2.0 * t + 2.0)).sqrt() + 1.0) / 2.0
                }
            }
            Easing::EaseInBack => BACK_PLUS * t * t * t - BACK * t * t,
            Easing::EaseOutBack => 1.0 + BACK_PLUS * (t - 1.0).powi(3) + BACK * (t - 1.0).powi(2),
            Easing::EaseInOutBack => {
                if t < 0.5 {
                    (2.0 * t).powi(2) * ((BACK_IN_OUT + 1.0) * 2.0 * t - BACK_IN_OUT) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2)
                        * ((BACK_IN_OUT + 1.0) * (t * 2.0 - 2.0) + BACK_IN_OUT)
                        + 2.0)
                        / 2.0
                }
            }
            Easing::EaseInElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    -2.0f64.powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * ELASTIC).sin()
                }
            }
            Easing::EaseOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    2.0f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * ELASTIC).sin() + 1.0
                }
            }
            Easing::EaseInOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    -(2.0f64.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * ELASTIC_IN_OUT).sin())
                        / 2.0
                } else {
                    2.0f64.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * ELASTIC_IN_OUT).sin()
                        / 2.0
                        + 1.0
                }
            }
            Easing::EaseInBounce => 1.0 - bounce_out(1.0 - t),
            Easing::EaseOutBounce => bounce_out(t),
            Easing::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (bounce_out(2.0 * t - 1.0) + 1.0) / 2.0
                }
            }
        };
        value as f32
    }
}

/// The Java class's private `bounceOut` helper, branch for branch.
fn bounce_out(x: f64) -> f64 {
    let c = 7.5625;
    let n = 2.75;
    if x < 1.0 / n {
        c * x * x
    } else if x < 2.0 / n {
        let x = x - 1.5 / n;
        c * x * x + 0.75
    } else if x < 2.5 / n {
        let x = x - 2.25 / n;
        c * x * x + 0.9375
    } else {
        let x = x - 2.625 / n;
        c * x * x + 0.984375
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every curve drawn at 0.0 lands on 0.0 and at 1.0 on 1.0. The back
    /// curves round to a few units in the last place there (the Java class's
    /// own f64 math does the same), so the check allows that much.
    #[test]
    fn every_curve_reaches_zero_and_one() {
        for curve in Easing::ALL {
            let at_zero = curve.apply(0.0);
            let at_one = curve.apply(1.0);
            assert!(at_zero.abs() <= 1.0e-6, "{} at 0.0: {at_zero}", curve.id());
            assert!(
                (at_one - 1.0).abs() <= 1.0e-6,
                "{} at 1.0: {at_one}",
                curve.id()
            );
        }
    }

    /// Sample values checked against the Java class's own math (f64 inside,
    /// one f32 narrowing at the end), computed independently of this port.
    #[test]
    fn sample_values_match_the_java_math() {
        let oracle: &[(Easing, f32, f32)] = &[
            (Easing::EaseInSine, 0.25, 7.612_046_6e-2),
            (Easing::EaseOutSine, 0.25, 3.826_834_3e-1),
            (Easing::EaseInOutSine, 0.25, 1.464_466_2e-1),
            (Easing::EaseInQuad, 0.25, 6.25e-2),
            (Easing::EaseOutQuad, 0.25, 4.375e-1),
            (Easing::EaseInOutQuad, 0.25, 1.25e-1),
            (Easing::EaseInCubic, 0.25, 1.562_5e-2),
            (Easing::EaseOutCubic, 0.25, 5.781_25e-1),
            (Easing::EaseInOutCubic, 0.25, 6.25e-2),
            (Easing::EaseInQuart, 0.25, 3.906_25e-3),
            (Easing::EaseOutQuart, 0.25, 6.835_937_5e-1),
            (Easing::EaseInOutQuart, 0.25, 3.125e-2),
            (Easing::EaseInQuint, 0.25, 9.765_625e-4),
            (Easing::EaseOutQuint, 0.25, 7.626_953e-1),
            (Easing::EaseInOutQuint, 0.25, 1.562_5e-2),
            (Easing::EaseInExpo, 0.25, 5.524_271_6e-3),
            (Easing::EaseOutExpo, 0.25, 8.232_233e-1),
            (Easing::EaseInOutExpo, 0.25, 1.562_5e-2),
            (Easing::EaseInCirc, 0.25, 3.175_416_2e-2),
            (Easing::EaseOutCirc, 0.25, 6.614_378e-1),
            (Easing::EaseInOutCirc, 0.25, 6.698_73e-2),
            (Easing::EaseInBack, 0.25, -6.413_656_5e-2),
            (Easing::EaseOutBack, 0.25, 8.174_097e-1),
            (Easing::EaseInOutBack, 0.25, -9.968_185e-2),
            (Easing::EaseInElastic, 0.25, -5.524_271_6e-3),
            (Easing::EaseOutElastic, 0.25, 9.116_117e-1),
            (Easing::EaseInOutElastic, 0.25, 1.196_944_4e-2),
            (Easing::EaseInBounce, 0.25, 2.734_375e-2),
            (Easing::EaseOutBounce, 0.25, 4.726_562_5e-1),
            (Easing::EaseInOutBounce, 0.25, 1.171_875e-1),
            (Easing::EaseInSine, 0.4, 1.909_830_1e-1),
            (Easing::EaseOutSine, 0.4, 5.877_852_4e-1),
            (Easing::EaseInOutSine, 0.4, 3.454_915e-1),
            (Easing::EaseInQuad, 0.4, 1.6e-1),
            (Easing::EaseOutQuad, 0.4, 6.4e-1),
            (Easing::EaseInOutQuad, 0.4, 3.2e-1),
            (Easing::EaseInCubic, 0.4, 6.4e-2),
            (Easing::EaseOutCubic, 0.4, 7.84e-1),
            (Easing::EaseInOutCubic, 0.4, 2.56e-1),
            (Easing::EaseInQuart, 0.4, 2.56e-2),
            (Easing::EaseOutQuart, 0.4, 8.704e-1),
            (Easing::EaseInOutQuart, 0.4, 2.048e-1),
            (Easing::EaseInQuint, 0.4, 1.024e-2),
            (Easing::EaseOutQuint, 0.4, 9.222_4e-1),
            (Easing::EaseInOutQuint, 0.4, 1.638_4e-1),
            (Easing::EaseInExpo, 0.4, 1.562_5e-2),
            (Easing::EaseOutExpo, 0.4, 9.375e-1),
            (Easing::EaseInOutExpo, 0.4, 1.25e-1),
            (Easing::EaseInCirc, 0.4, 8.348_486e-2),
            (Easing::EaseOutCirc, 0.4, 8e-1),
            (Easing::EaseInOutCirc, 0.4, 2e-1),
            (Easing::EaseInBack, 0.4, -9.935_168e-2),
            (Easing::EaseOutBack, 0.4, 1.029_027_5),
            (Easing::EaseInOutBack, 0.4, 8.992_579e-2),
            (Easing::EaseInElastic, 0.4, 1.562_5e-2),
            (Easing::EaseOutElastic, 0.4, 1.031_25),
            (Easing::EaseInOutElastic, 0.4, -1.174_615_8e-1),
            (Easing::EaseInBounce, 0.4, 2.275e-1),
            (Easing::EaseOutBounce, 0.4, 9.1e-1),
            (Easing::EaseInOutBounce, 0.4, 3.487_5e-1),
            (Easing::EaseInSine, 0.6, 4.122_147_6e-1),
            (Easing::EaseOutSine, 0.6, 8.090_17e-1),
            (Easing::EaseInOutSine, 0.6, 6.545_085e-1),
            (Easing::EaseInQuad, 0.6, 3.6e-1),
            (Easing::EaseOutQuad, 0.6, 8.4e-1),
            (Easing::EaseInOutQuad, 0.6, 6.8e-1),
            (Easing::EaseInCubic, 0.6, 2.16e-1),
            (Easing::EaseOutCubic, 0.6, 9.36e-1),
            (Easing::EaseInOutCubic, 0.6, 7.44e-1),
            (Easing::EaseInQuart, 0.6, 1.296e-1),
            (Easing::EaseOutQuart, 0.6, 9.744e-1),
            (Easing::EaseInOutQuart, 0.6, 7.952e-1),
            (Easing::EaseInQuint, 0.6, 7.776e-2),
            (Easing::EaseOutQuint, 0.6, 9.897_6e-1),
            (Easing::EaseInOutQuint, 0.6, 8.361_6e-1),
            (Easing::EaseInExpo, 0.6, 6.25e-2),
            (Easing::EaseOutExpo, 0.6, 9.843_75e-1),
            (Easing::EaseInOutExpo, 0.6, 8.75e-1),
            (Easing::EaseInCirc, 0.6, 2e-1),
            (Easing::EaseOutCirc, 0.6, 9.165_151e-1),
            (Easing::EaseInOutCirc, 0.6, 8e-1),
            (Easing::EaseInBack, 0.6, -2.902_752e-2),
            (Easing::EaseOutBack, 0.6, 1.099_351_6),
            (Easing::EaseInOutBack, 0.6, 9.100_742_3e-1),
            (Easing::EaseInElastic, 0.6, -3.125e-2),
            (Easing::EaseOutElastic, 0.6, 9.843_75e-1),
            (Easing::EaseInOutElastic, 0.6, 1.117_461_6),
            (Easing::EaseInBounce, 0.6, 9e-2),
            (Easing::EaseOutBounce, 0.6, 7.725e-1),
            (Easing::EaseInOutBounce, 0.6, 6.512_5e-1),
            (Easing::EaseInSine, 0.9, 8.435_655e-1),
            (Easing::EaseOutSine, 0.9, 9.876_883_6e-1),
            (Easing::EaseInOutSine, 0.9, 9.755_282_4e-1),
            (Easing::EaseInQuad, 0.9, 8.1e-1),
            (Easing::EaseOutQuad, 0.9, 9.9e-1),
            (Easing::EaseInOutQuad, 0.9, 9.8e-1),
            (Easing::EaseInCubic, 0.9, 7.29e-1),
            (Easing::EaseOutCubic, 0.9, 9.99e-1),
            (Easing::EaseInOutCubic, 0.9, 9.96e-1),
            (Easing::EaseInQuart, 0.9, 6.561e-1),
            (Easing::EaseOutQuart, 0.9, 9.999e-1),
            (Easing::EaseInOutQuart, 0.9, 9.992e-1),
            (Easing::EaseInQuint, 0.9, 5.904_9e-1),
            (Easing::EaseOutQuint, 0.9, 9.999_9e-1),
            (Easing::EaseInOutQuint, 0.9, 9.998_4e-1),
            (Easing::EaseInExpo, 0.9, 5e-1),
            (Easing::EaseOutExpo, 0.9, 9.980_469e-1),
            (Easing::EaseInOutExpo, 0.9, 9.980_469e-1),
            (Easing::EaseInCirc, 0.9, 5.641_101e-1),
            (Easing::EaseOutCirc, 0.9, 9.949_874e-1),
            (Easing::EaseInOutCirc, 0.9, 9.898_979_7e-1),
            (Easing::EaseInBack, 0.9, 5.911_720_4e-1),
            (Easing::EaseOutBack, 0.9, 1.014_314_2),
            (Easing::EaseInOutBack, 0.9, 1.037_518_5),
            (Easing::EaseInElastic, 0.9, -2.5e-1),
            (Easing::EaseOutElastic, 0.9, 9.980_469e-1),
            (Easing::EaseInOutElastic, 0.9, 9.996_608_5e-1),
            (Easing::EaseInBounce, 0.9, 9.243_75e-1),
            (Easing::EaseOutBounce, 0.9, 9.881_25e-1),
            (Easing::EaseInOutBounce, 0.9, 9.7e-1),
        ];
        for (curve, t, expected) in oracle {
            let got = curve.apply(*t);
            assert!(
                (got - expected).abs() <= 1.0e-6 * expected.abs().max(1.0),
                "{} at {t}: got {got}, expected {expected}",
                curve.id()
            );
        }
    }

    /// `by_name` accepts the id itself and the separators the Java lookup
    /// strips: underscores, hyphens and spaces, in any case.
    #[test]
    fn by_name_matches_the_java_normalization() {
        assert_eq!(
            Easing::by_name("easeInOutSine"),
            Some(Easing::EaseInOutSine)
        );
        assert_eq!(
            Easing::by_name("ease_in_out_sine"),
            Some(Easing::EaseInOutSine)
        );
        assert_eq!(
            Easing::by_name("EASE-IN-OUT-SINE"),
            Some(Easing::EaseInOutSine)
        );
        assert_eq!(
            Easing::by_name("ease in out sine"),
            Some(Easing::EaseInOutSine)
        );
        assert_eq!(
            Easing::by_name("easeinoutsine"),
            Some(Easing::EaseInOutSine)
        );
        assert_eq!(
            Easing::by_name("Ease_In-Out Sine"),
            Some(Easing::EaseInOutSine)
        );
        assert_eq!(
            Easing::by_name("easeOutBounce"),
            Some(Easing::EaseOutBounce)
        );
        assert_eq!(Easing::by_name(""), None);
        assert_eq!(Easing::by_name("nope"), None);
        assert_eq!(Easing::by_name("easeInOutSineX"), None);
    }

    /// The ids, in order, are the Java enum's: every id resolves back to its
    /// variant and none is empty.
    #[test]
    fn ids_are_the_fabric_mods() {
        let ids = [
            "easeInSine",
            "easeOutSine",
            "easeInOutSine",
            "easeInQuad",
            "easeOutQuad",
            "easeInOutQuad",
            "easeInCubic",
            "easeOutCubic",
            "easeInOutCubic",
            "easeInQuart",
            "easeOutQuart",
            "easeInOutQuart",
            "easeInQuint",
            "easeOutQuint",
            "easeInOutQuint",
            "easeInExpo",
            "easeOutExpo",
            "easeInOutExpo",
            "easeInCirc",
            "easeOutCirc",
            "easeInOutCirc",
            "easeInBack",
            "easeOutBack",
            "easeInOutBack",
            "easeInElastic",
            "easeOutElastic",
            "easeInOutElastic",
            "easeInBounce",
            "easeOutBounce",
            "easeInOutBounce",
        ];
        for (curve, id) in Easing::ALL.into_iter().zip(ids) {
            assert_eq!(curve.id(), id);
            assert_eq!(Easing::by_name(id), Some(curve));
        }
    }

    /// The bounce helper crosses its three inner branches: each plateau value
    /// the Java helper returns is pinned here.
    #[test]
    fn bounce_out_hits_every_branch() {
        assert_eq!(bounce_out(0.0), 0.0);
        assert_eq!(bounce_out(1.0), 1.0);
        // First branch: x < 1/2.75.
        let first = bounce_out(0.2);
        assert!((first - (7.5625 * 0.2 * 0.2)).abs() < 1.0e-12);
        // Second branch: x < 2/2.75, x -= 1.5/2.75.
        let second = bounce_out(0.6);
        let x = 0.6 - 1.5 / 2.75;
        assert!((second - (7.5625 * x * x + 0.75)).abs() < 1.0e-12);
        // Third branch: x < 2.5/2.75, x -= 2.25/2.75.
        let third = bounce_out(0.85);
        let x = 0.85 - 2.25 / 2.75;
        assert!((third - (7.5625 * x * x + 0.9375)).abs() < 1.0e-12);
        // Last branch: x -= 2.625/2.75.
        let last = bounce_out(0.95);
        let x = 0.95 - 2.625 / 2.75;
        assert!((last - (7.5625 * x * x + 0.984375)).abs() < 1.0e-12);
    }
}
