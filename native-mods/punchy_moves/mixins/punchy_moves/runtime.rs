// Punchy Moves for Horizon (native mixin runtime): third-person character
// animation, a shoulder camera, expressions and emotes.
//
// This file is both the mod's Cargo library (its tests) and the single
// self-contained module the bundle adds to the game as `game::punchy_moves`.

/// The mod's id: its settings folder and its row in Options > Mods.
pub const ID: &str = "punchy_moves";
pub const VERSION: &str = "1.0.9";
pub const DESCRIPTION: &str =
    "Third-person character animation, shoulder camera, expressions and emotes. Horizon's own.";

pub mod easing {
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
                Easing::EaseOutBack => {
                    1.0 + BACK_PLUS * (t - 1.0).powi(3) + BACK * (t - 1.0).powi(2)
                }
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
                        -(2.0f64.powf(20.0 * t - 10.0)
                            * ((20.0 * t - 11.125) * ELASTIC_IN_OUT).sin())
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
}

pub mod expressions {
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
                Node::NotEqual(left, right) => {
                    Some(Value::from_bool(number(left)? != number(right)?))
                }
                Node::GreaterOrEqual(left, right) => {
                    Some(Value::from_bool(number(left)? >= number(right)?))
                }
                Node::LessOrEqual(left, right) => {
                    Some(Value::from_bool(number(left)? <= number(right)?))
                }
                Node::Greater(left, right) => {
                    Some(Value::from_bool(number(left)? > number(right)?))
                }
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
}

pub mod json {
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
}

pub mod library {
    //! Punchy Moves' animation library, ported from the Fabric mod's
    //! `punchymoves.client.animation.AnimationLibrary` and its nested records,
    //! plus `CompatFormat` (legacy pack normalization) and
    //! `ItemAnimationBindings` (per-item binding resolution).
    //!
    //! Pure Rust and std-only: vectors and quaternions are local f32
    //! implementations whose formulas match JOML 1.10.8 exactly
    //! (`rotationZYX`, `mul` = `this * other`, `transform`, `getEulerAnglesZYX`);
    //! JSON loading runs on the mod's own `json` module; curves come from
    //! `easing` and variable expressions from `expressions`.

    use std::collections::HashMap;

    use super::signals::BasePoseRetention;

    /// One 3-component f32 vector: the JOML `Vector3f` surface the library uses.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl Vec3 {
        pub const ZERO: Vec3 = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        pub const ONE: Vec3 = Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };

        pub fn new(x: f32, y: f32, z: f32) -> Self {
            Self { x, y, z }
        }

        /// `lengthSquared`.
        pub fn length_squared(self) -> f32 {
            self.x * self.x + self.y * self.y + self.z * self.z
        }

        /// `Vector3f.lerp(other, t)`: `this + (other - this) * t`.
        pub fn lerp(self, other: Vec3, t: f32) -> Vec3 {
            Vec3 {
                x: self.x + (other.x - self.x) * t,
                y: self.y + (other.y - self.y) * t,
                z: self.z + (other.z - self.z) * t,
            }
        }
    }

    /// f32 quaternion: the JOML `Quaternionf` surface the library uses.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Quat {
        pub x: f32,
        pub y: f32,
        pub z: f32,
        pub w: f32,
    }

    impl Quat {
        /// `Quaternionf.rotationZYX(angleZ, angleY, angleX)`: rotation about Z,
        /// then Y, then X — the quaternion product `qz * qy * qx`.
        pub fn from_euler_zyx(angle_z_deg: f32, angle_y_deg: f32, angle_x_deg: f32) -> Self {
            let sx = (angle_x_deg.to_radians() * 0.5).sin();
            let cx = (angle_x_deg.to_radians() * 0.5).cos();
            let sy = (angle_y_deg.to_radians() * 0.5).sin();
            let cy = (angle_y_deg.to_radians() * 0.5).cos();
            let sz = (angle_z_deg.to_radians() * 0.5).sin();
            let cz = (angle_z_deg.to_radians() * 0.5).cos();
            let cycz = cy * cz;
            let sysz = sy * sz;
            let sycz = sy * cz;
            let cysz = cy * sz;
            Self {
                x: sx * cycz - cx * sysz,
                y: cx * sycz + sx * cysz,
                z: cx * cysz - sx * sycz,
                w: cx * cycz + sx * sysz,
            }
        }

        /// `Quaternionf.transform(Vector3f)`: rotates a vector by this
        /// quaternion (JOML's normalized formula).
        pub fn transform_by(self, v: Vec3) -> Vec3 {
            let xx = self.x * self.x;
            let yy = self.y * self.y;
            let zz = self.z * self.z;
            let ww = self.w * self.w;
            let xy = self.x * self.y;
            let xz = self.x * self.z;
            let yz = self.y * self.z;
            let xw = self.x * self.w;
            let zw = self.z * self.w;
            let yw = self.y * self.w;
            let k = 1.0 / (xx + yy + zz + ww);
            Vec3 {
                x: (xx - yy - zz + ww) * k * v.x
                    + 2.0 * (xy - zw) * k * v.y
                    + (2.0 * (xz + yw) * k) * v.z,
                y: 2.0 * (xy + zw) * k * v.x
                    + (yy - xx - zz + ww) * k * v.y
                    + 2.0 * (yz - xw) * k * v.z,
                z: 2.0 * (xz - yw) * k * v.x
                    + 2.0 * (yz + xw) * k * v.y
                    + ((zz - xx - yy + ww) * k) * v.z,
            }
        }

        /// `Quaternionf.getEulerAnglesZYX`, radians, with JOML's `safeAsin`
        /// (asin of a clamped argument).
        pub fn get_euler_zyx(self) -> Vec3 {
            let xx = self.x * self.x;
            let yy = self.y * self.y;
            let zz = self.z * self.z;
            Vec3 {
                x: (self.y * self.z + self.w * self.x).atan2(0.5 - xx - yy),
                y: (-2.0 * (self.x * self.z - self.w * self.y))
                    .clamp(-1.0, 1.0)
                    .asin(),
                z: (self.x * self.y + self.w * self.z).atan2(0.5 - yy - zz),
            }
        }
    }

    impl std::ops::Add for Vec3 {
        type Output = Vec3;
        fn add(self, other: Vec3) -> Vec3 {
            Vec3 {
                x: self.x + other.x,
                y: self.y + other.y,
                z: self.z + other.z,
            }
        }
    }

    impl std::ops::Sub for Vec3 {
        type Output = Vec3;
        fn sub(self, other: Vec3) -> Vec3 {
            Vec3 {
                x: self.x - other.x,
                y: self.y - other.y,
                z: self.z - other.z,
            }
        }
    }

    /// Componentwise product, like `Vector3f* Vector3fc`.
    impl std::ops::Mul for Vec3 {
        type Output = Vec3;
        fn mul(self, other: Vec3) -> Vec3 {
            Vec3 {
                x: self.x * other.x,
                y: self.y * other.y,
                z: self.z * other.z,
            }
        }
    }

    /// `Vector3f.mul(float)`: every component times a scalar.
    impl std::ops::Mul<f32> for Vec3 {
        type Output = Vec3;
        fn mul(self, k: f32) -> Vec3 {
            Vec3 {
                x: self.x * k,
                y: self.y * k,
                z: self.z * k,
            }
        }
    }

    impl std::ops::Mul for Quat {
        type Output = Quat;
        /// Post-multiplication, `R = self * other`: `other` applies first.
        fn mul(self, other: Quat) -> Quat {
            Self {
                x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
                y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
                z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
                w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            }
        }
    }

    /// One sampled bone pose: the JOML `Pose` record (position, rotation in
    /// degrees, scale).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Pose {
        pub position: Vec3,
        pub rotation: Vec3,
        pub scale: Vec3,
    }

    impl Pose {
        pub fn new(position: Vec3, rotation: Vec3, scale: Vec3) -> Self {
            Self {
                position,
                rotation,
                scale,
            }
        }

        pub fn identity() -> Self {
            Self {
                position: Vec3::ZERO,
                rotation: Vec3::ZERO,
                scale: Vec3::ONE,
            }
        }
    }

    /// One keyframe: the `Key` record (pre/post vectors, interpolation name,
    /// optional easing curve).
    #[derive(Debug, Clone, PartialEq)]
    pub struct Key {
        pub pre: Vec3,
        pub post: Vec3,
        pub interpolation: String,
        pub easing: Option<super::easing::Easing>,
    }

    /// One animated channel: the `Channel` record — a time-ordered list of keys
    /// plus the vector returned when the channel has no keys at all. The keys
    /// are kept sorted by time, like the Java `TreeMap`.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Channel {
        keys: Vec<(f32, Key)>,
        fallback: Vec3,
    }

    impl Channel {
        /// Builds a channel from `(time, key)` pairs; they are sorted by time.
        pub fn new(mut keys: Vec<(f32, Key)>, fallback: Vec3) -> Self {
            keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            Self { keys, fallback }
        }

        /// Draws the channel at `t` with no look-behind: the Java `sample(float)`.
        pub fn sample(&self, t: f32) -> Vec3 {
            self.sample_before(t, 0.0)
        }

        /// Draws the channel at `t`. `before` is the clip's total length: when a
        /// Catmull-Rom segment at either end has no neighbouring key, the lookup
        /// may reach that far for the missing control point.
        pub fn sample_before(&self, t: f32, before: f32) -> Vec3 {
            if self.keys.is_empty() {
                return self.fallback;
            }
            let floor = self.floor_idx(t);
            let ceiling = self.ceiling_idx(t);
            let Some(ceil_idx) = ceiling else {
                // After the last key: its `post` value (the floor key is there).
                return match floor {
                    Some(i) => self.keys[i].1.post,
                    None => self.fallback,
                };
            };
            let Some(floor_idx) = floor else {
                // Before the first key: its `pre` value.
                return self.keys[ceil_idx].1.pre;
            };
            let floor_time = self.keys[floor_idx].0;
            let ceil_time = self.keys[ceil_idx].0;
            if floor_time == ceil_time {
                return self.keys[floor_idx].1.post;
            }
            let mut alpha = (t - floor_time) / (ceil_time - floor_time);
            if let Some(easing) = self.keys[ceil_idx].1.easing {
                alpha = easing.apply(alpha);
            }
            let a = self.keys[floor_idx].1.post;
            let b = self.keys[ceil_idx].1.pre;
            if is_catmull(&self.keys[floor_idx].1) || is_catmull(&self.keys[ceil_idx].1) {
                let mut lower = self.lower_idx(floor_time);
                let mut higher = self.higher_idx(ceil_time);
                if before > 0.0 && lower.is_none() {
                    lower = self.lower_idx(before);
                }
                if before > 0.0 && higher.is_none() && self.keys.len() > 2 {
                    higher = self.higher_idx(self.keys[0].0);
                }
                let p0 = match lower {
                    Some(i) => self.keys[i].1.post,
                    None => a,
                };
                let p3 = match higher {
                    Some(i) => self.keys[i].1.pre,
                    None => b,
                };
                return catmull(p0, a, b, p3, alpha);
            }
            a.lerp(b, alpha)
        }

        /// The last key's time, or 0.0 when the channel has no keys.
        pub fn last_key_time(&self) -> f32 {
            self.keys.last().map_or(0.0, |(t, _)| *t)
        }

        /// Index of the last key at or before `t` (`floorEntry`).
        fn floor_idx(&self, t: f32) -> Option<usize> {
            let n = self.keys.partition_point(|(k, _)| *k <= t);
            if n == 0 { None } else { Some(n - 1) }
        }

        /// Index of the first key at or after `t` (`ceilingEntry`).
        fn ceiling_idx(&self, t: f32) -> Option<usize> {
            let n = self.keys.partition_point(|(k, _)| *k < t);
            if n == self.keys.len() { None } else { Some(n) }
        }

        /// Index of the last key strictly before `t` (`lowerEntry`).
        fn lower_idx(&self, t: f32) -> Option<usize> {
            let n = self.keys.partition_point(|(k, _)| *k < t);
            if n == 0 { None } else { Some(n - 1) }
        }

        /// Index of the first key strictly after `t` (`higherEntry`).
        fn higher_idx(&self, t: f32) -> Option<usize> {
            let n = self.keys.partition_point(|(k, _)| *k <= t);
            if n == self.keys.len() { None } else { Some(n) }
        }
    }

    /// `catmullrom` interpolation, matched case-insensitively.
    fn is_catmull(key: &Key) -> bool {
        key.interpolation.eq_ignore_ascii_case("catmullrom")
    }

    /// Catmull-Rom spline through `p1 -> p2` with control points `p0`/`p3`.
    fn catmull(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
        let t2 = t * t;
        let t3 = t2 * t;
        Vec3 {
            x: curve(p0.x, p1.x, p2.x, p3.x, t, t2, t3),
            y: curve(p0.y, p1.y, p2.y, p3.y, t, t2, t3),
            z: curve(p0.z, p1.z, p2.z, p3.z, t, t2, t3),
        }
    }

    /// The Catmull-Rom basis for one component.
    fn curve(p0: f32, p1: f32, p2: f32, p3: f32, t: f32, t2: f32, t3: f32) -> f32 {
        0.5 * ((2.0 * p1)
            + (p2 - p0) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }

    /// One bone's animated channels: the `BoneTrack` record.
    #[derive(Debug, Clone, PartialEq)]
    pub struct BoneTrack {
        pub position: Channel,
        pub rotation: Channel,
        pub scale: Channel,
    }

    impl BoneTrack {
        pub fn new(position: Channel, rotation: Channel, scale: Channel) -> Self {
            Self {
                position,
                rotation,
                scale,
            }
        }

        /// Samples all three channels at `t`.
        pub fn sample(&self, t: f32, before: f32) -> Pose {
            Pose {
                position: self.position.sample_before(t, before),
                rotation: self.rotation.sample_before(t, before),
                scale: self.scale.sample_before(t, before),
            }
        }

        /// The last key's time across all three channels.
        pub fn last_key_time(&self) -> f32 {
            self.position
                .last_key_time()
                .max(self.rotation.last_key_time())
                .max(self.scale.last_key_time())
        }
    }

    /// One animated clip: the `Clip` record — a named, timed set of bone tracks.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Clip {
        pub name: String,
        pub length: f32,
        pub loop_: bool,
        pub bones: std::collections::HashMap<String, BoneTrack>,
    }

    impl Clip {
        pub fn new(
            name: String,
            length: f32,
            loop_: bool,
            bones: std::collections::HashMap<String, BoneTrack>,
        ) -> Self {
            Self {
                name,
                length,
                loop_,
                bones,
            }
        }

        /// A copy without the local arm tracks (`left_arm`, `right_arm`).
        pub fn without_local_arms(&self) -> Clip {
            let mut bones = self.bones.clone();
            bones.remove("left_arm");
            bones.remove("right_arm");
            Clip::new(self.name.clone(), self.length, self.loop_, bones)
        }

        /// Samples one bone at `t`, in model space: the Java `sample`.
        pub fn sample(&self, bone: &str, t: f32) -> Pose {
            let bone = bone.to_lowercase();
            let track = self.bones.get(&bone);
            let time = self.clamp_time(t);
            let loop_length = if self.loop_ { self.length } else { 0.0 };
            let pose = match track {
                None => Pose::identity(),
                Some(track) => track.sample(time, loop_length),
            };
            self.to_model_space(&bone, pose, time, loop_length)
        }

        /// Samples the carry pose of a bone: identity transformed only by the
        /// virtual waist, like the Java `sampleCarry`.
        pub fn sample_carry(&self, bone: &str, t: f32) -> Pose {
            let time = self.clamp_time(t);
            let loop_length = if self.loop_ { self.length } else { 0.0 };
            self.inherit_virtual_waist(&bone.to_lowercase(), Pose::identity(), time, loop_length)
        }

        /// Samples one bone once (no looping, no look-behind): `sampleOnce`.
        pub fn sample_once(&self, bone: &str, t: f32) -> Pose {
            let bone = bone.to_lowercase();
            let track = self.bones.get(&bone);
            let time = 0.0f32.max(t.min(self.length));
            let pose = match track {
                None => Pose::identity(),
                Some(track) => track.sample(time, 0.0),
            };
            self.to_model_space(&bone, pose, time, 0.0)
        }

        fn clamp_time(&self, t: f32) -> f32 {
            if self.loop_ {
                positive_modulo(t, self.length)
            } else {
                0.0f32.max(t.min(self.length))
            }
        }

        /// Model-space adjustment: the idle root pivot, then the virtual waist.
        fn to_model_space(&self, bone: &str, pose: Pose, time: f32, loop_length: f32) -> Pose {
            if bone == "root" && self.name == "idle" {
                let pivot = Vec3::new(0.0, 12.0, 0.0);
                let q = quaternion(pose.rotation);
                let offset =
                    q.transform_by(Vec3::new(pivot.x, pivot.y, pivot.z) * pose.scale) - pivot;
                let position = Vec3::new(
                    pose.position.x + offset.x,
                    pose.position.y - offset.y,
                    pose.position.z + offset.z,
                );
                return Pose::new(position, pose.rotation, pose.scale);
            }
            self.inherit_virtual_waist(bone, pose, time, loop_length)
        }

        fn inherit_virtual_waist(
            &self,
            bone: &str,
            pose: Pose,
            time: f32,
            loop_length: f32,
        ) -> Pose {
            if !is_virtual_waist_child(bone) {
                return pose;
            }
            let Some(waist) = self.bones.get("waist") else {
                return pose;
            };
            let waist_pose = waist.sample(time, loop_length);
            let q = quaternion(waist_pose.rotation);
            let combined = q * quaternion(pose.rotation);
            let pivot = virtual_waist_child_pivot(bone);
            let waist_scale = waist_pose.scale;
            let rotated_pivot = q.transform_by(Vec3::new(pivot.x, pivot.y, pivot.z) * waist_scale);
            let rotated_offset = q.transform_by(
                Vec3::new(pose.position.x, -pose.position.y, pose.position.z) * waist_scale,
            );
            let mut position = rotated_pivot - pivot
                + rotated_offset
                + Vec3::new(
                    waist_pose.position.x,
                    -waist_pose.position.y,
                    waist_pose.position.z,
                );
            position.y = -position.y;
            Pose::new(
                position,
                // JOML's getEulerAnglesZYX is radians; the pose carries degrees.
                combined.get_euler_zyx()
                    * Vec3::new(
                        (180.0 / std::f64::consts::PI) as f32,
                        (180.0 / std::f64::consts::PI) as f32,
                        (180.0 / std::f64::consts::PI) as f32,
                    ),
                waist_pose.scale * pose.scale,
            )
        }
    }

    /// `MovementTuning.jumpAirHold`: how far into the jump clip the air pose
    /// holds before the landing clip's first frame takes over — the time whose
    /// pose is closest to the landing's, searched at `AIR_HOLD_STEPS` + 1 points
    /// across the jump (or across `hold` seconds of it when the pack set one).
    /// A landing with no bones holds forever (`INFINITY`).
    pub fn jump_air_hold(jump: &Clip, landing: &Clip, hold: f32) -> f32 {
        if landing.bones.is_empty() {
            return f32::INFINITY;
        }
        let span = if hold.is_finite() && hold > 0.0 {
            jump.length.min(hold)
        } else {
            jump.length
        };
        let mut best = span;
        let mut best_cost = f32::MAX;
        for i in 0..=96u32 {
            let t = span * i as f32 / 96.0;
            let mut cost = 0.0_f32;
            for bone in landing.bones.keys() {
                let air = jump.sample_once(bone, t);
                let landed = landing.sample_once(bone, 0.0);
                cost += distance_squared(air.rotation, landed.rotation)
                    + 16.0 * distance_squared(air.position, landed.position);
            }
            if cost < best_cost - 1.0e-4 {
                best_cost = cost;
                best = t;
            }
        }
        best
    }

    /// `Vector3f.distanceSquared`: the squared straight distance.
    fn distance_squared(a: Vec3, b: Vec3) -> f32 {
        let d = a - b;
        d.x * d.x + d.y * d.y + d.z * d.z
    }

    /// The pivot a virtual-waist child rotates around, in the clip's own space.
    fn virtual_waist_child_pivot(bone: &str) -> Vec3 {
        match bone {
            "right_arm" => Vec3::new(-5.0, 2.0, 0.0),
            "left_arm" => Vec3::new(5.0, 2.0, 0.0),
            _ => Vec3::ZERO,
        }
    }

    /// Builds the quaternion from an Euler vector: JOML `rotationZYX`.
    fn quaternion(rotation: Vec3) -> Quat {
        Quat::from_euler_zyx(rotation.z, rotation.y, rotation.x)
    }

    /// Bones that inherit the virtual waist's motion.
    fn is_virtual_waist_child(bone: &str) -> bool {
        matches!(bone, "body" | "head" | "right_arm" | "left_arm")
    }

    /// Java `Float.remed`-style positive modulo: the remainder with the
    /// divisor's sign made positive.
    fn positive_modulo(value: f32, divisor: f32) -> f32 {
        let r = value % divisor;
        if r < 0.0 { r + divisor } else { r }
    }

    /// Camera profile: the `CameraProfile` record (shoulder camera offsets and
    /// field of view).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CameraProfile {
        pub horizontal: f32,
        pub vertical: f32,
        pub depth: f32,
        pub fov: f32,
        pub response: f32,
    }

    impl CameraProfile {
        pub fn is_complete(&self) -> bool {
            self.horizontal.is_finite()
                && self.vertical.is_finite()
                && self.depth.is_finite()
                && self.fov.is_finite()
                && self.response.is_finite()
        }
    }

    /// One locomotion clip with its weight and overlays: the `LocomotionClip`
    /// record.
    #[derive(Debug, Clone, PartialEq)]
    pub struct LocomotionClip {
        pub clip: Clip,
        pub multiplier: f32,
        pub overlays: Vec<ResolvedOverlay>,
        pub reverse: bool,
        pub add: Option<Clip>,
    }

    impl LocomotionClip {
        pub fn new(clip: Clip, multiplier: f32, overlays: Vec<ResolvedOverlay>) -> Self {
            Self {
                clip,
                multiplier,
                overlays,
                reverse: false,
                add: None,
            }
        }
    }

    /// One locomotion influence: the `LocomotionInfluence` record — the weights
    /// per locomotion state and the transition share.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct LocomotionInfluence {
        pub walk: f32,
        pub sprint: f32,
        pub jump: f32,
        pub transition: f32,
        pub hand_only: bool,
    }

    impl LocomotionInfluence {
        pub fn new(walk: f32, sprint: f32, jump: f32) -> Self {
            Self {
                walk,
                sprint,
                jump,
                transition: 0.0,
                hand_only: false,
            }
        }

        /// Picks the weight for the current locomotion state: jumping wins over
        /// sprinting, sprinting over walking.
        pub fn select(&self, sprinting: bool, jumping: bool) -> f32 {
            if jumping {
                self.jump
            } else if sprinting {
                self.sprint
            } else {
                self.walk
            }
        }
    }

    /// One emote: the `Emote` record.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Emote {
        pub id: String,
        pub display_name: String,
        pub animation: String,
        pub default_key: String,
        pub face: Option<String>,
    }

    /// The emote menu: the `EmoteCatalog` record.
    #[derive(Debug, Clone, PartialEq)]
    pub struct EmoteCatalog {
        pub menu_key: String,
        pub entries: Vec<Emote>,
    }

    /// One handed animation: the `HandedAnimation` record.
    #[derive(Debug, Clone, PartialEq)]
    pub struct HandedAnimation {
        pub clip: Clip,
        pub mirror: bool,
    }

    /// One resolved overlay: the `ResolvedOverlay` record — a clip plus the
    /// bones it applies to.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ResolvedOverlay {
        pub clip: Clip,
        pub mode: OverlayMode,
        pub bones: Vec<String>,
    }

    impl ResolvedOverlay {
        /// Whether the overlay applies to `bone`: no bone list means every bone.
        pub fn applies_to(&self, bone: &str) -> bool {
            self.bones.is_empty() || self.bones.iter().any(|b| b == bone)
        }
    }

    /// How an overlay combines with the base pose.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OverlayMode {
        Additive,
        Override,
        Exclusive,
    }

    impl OverlayMode {
        /// The Java `fromString`: null or anything unknown is `ADDITIVE`.
        pub fn from_name(name: Option<&str>) -> Self {
            match name {
                Some("override") => OverlayMode::Override,
                Some("exclusive") => OverlayMode::Exclusive,
                _ => OverlayMode::Additive,
            }
        }
    }

    /// The bone-name prefix the mod's face shorthand keys use (`FACE_PREFIX`).
    pub const FACE_PREFIX: &str = "face_";
    /// The mix bone the face shorthand installs (`face_mix`).
    pub const FACE_MIX_BONE: &str = "face_mix";

    /// Looks a member up in an object's member slice.
    fn member<'a>(
        obj: &'a [(String, super::json::Value)],
        key: &str,
    ) -> Option<&'a super::json::Value> {
        obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// `safeFloat`: the element as an f32, or `fallback` on null or any parse
    /// failure (the Java helper catches every `RuntimeException`).
    fn safe_float(elem: Option<&super::json::Value>, fallback: f32) -> f32 {
        match elem {
            Some(super::json::Value::Number(n)) => *n as f32,
            Some(super::json::Value::String(s)) => s.trim().parse::<f32>().unwrap_or(fallback),
            _ => fallback,
        }
    }

    /// `vector`: a scalar becomes a uniform vector, an array of three or more
    /// numbers becomes a vector (missing components fall back per component),
    /// anything else is the fallback.
    fn vector(elem: Option<&super::json::Value>, fallback: Vec3) -> Vec3 {
        match elem {
            Some(super::json::Value::Number(n)) => {
                let f = *n as f32;
                if f.is_finite() {
                    Vec3::new(f, f, f)
                } else {
                    fallback
                }
            }
            Some(super::json::Value::String(s)) => match s.trim().parse::<f32>() {
                Ok(f) if f.is_finite() => Vec3::new(f, f, f),
                _ => fallback,
            },
            Some(super::json::Value::Array(items)) if items.len() >= 3 => Vec3::new(
                safe_float(items.first(), fallback.x),
                safe_float(items.get(1), fallback.y),
                safe_float(items.get(2), fallback.z),
            ),
            _ => fallback,
        }
    }

    /// `parseChannel`: a channel from a JSON element. An array is one key at
    /// time 0.0; an object maps time strings to keys (a bare array value is a
    /// symmetric key, an object carries `pre`/`post`/`vector`, `lerp_mode` and
    /// `easing`); anything else is an empty channel on the fallback.
    pub fn parse_channel(elem: Option<&super::json::Value>, fallback: Vec3) -> Channel {
        let mut keys: Vec<(f32, Key)> = Vec::new();
        let Some(elem) = elem.filter(|e| !e.is_null()) else {
            return Channel::new(keys, fallback);
        };
        match elem {
            super::json::Value::Array(_) => {
                let v = vector(Some(elem), fallback);
                keys.push((
                    0.0,
                    Key {
                        pre: v,
                        post: v,
                        interpolation: "linear".to_string(),
                        easing: None,
                    },
                ));
            }
            super::json::Value::Object(members) => {
                for (key, value) in members {
                    // A time that does not parse skips the entry, like the Java
                    // catch around the whole entry.
                    let Ok(time) = key.parse::<f32>() else {
                        continue;
                    };
                    match value {
                        super::json::Value::Array(_) => {
                            let v = vector(Some(value), fallback);
                            keys.push((
                                time,
                                Key {
                                    pre: v,
                                    post: v,
                                    interpolation: "linear".to_string(),
                                    easing: None,
                                },
                            ));
                        }
                        super::json::Value::Object(obj) => {
                            let post_elem = member(obj, "post")
                                .or_else(|| member(obj, "vector"))
                                .or_else(|| member(obj, "pre"));
                            let pre_elem = member(obj, "pre").or(post_elem);
                            let pre = vector(pre_elem, fallback);
                            let post = vector(post_elem, pre);
                            let lerp_mode = member(obj, "lerp_mode")
                                .and_then(super::json::Value::as_str)
                                .unwrap_or("linear")
                                .to_string();
                            let easing = member(obj, "easing")
                                .filter(|e| matches!(e, super::json::Value::String(_)))
                                .and_then(|e| e.as_str())
                                .and_then(super::easing::Easing::by_name);
                            keys.push((
                                time,
                                Key {
                                    pre,
                                    post,
                                    interpolation: lerp_mode,
                                    easing,
                                },
                            ));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Channel::new(keys, fallback)
    }

    /// `heldY`: a one-key position track that holds the bone at `y`, with an
    /// empty rotation and a unit scale.
    pub fn held_y(y: f32) -> BoneTrack {
        let v = Vec3::new(0.0, y, 0.0);
        let key = Key {
            pre: v,
            post: v,
            interpolation: "linear".to_string(),
            easing: None,
        };
        BoneTrack {
            position: Channel::new(vec![(0.0, key)], Vec3::ZERO),
            rotation: Channel::new(Vec::new(), Vec3::ZERO),
            scale: Channel::new(Vec::new(), Vec3::ONE),
        }
    }

    /// The bone the face shorthand installs for the mix weight (`face_mix`).
    fn face_shorthand(
        obj: &super::json::Value,
        bones: &mut std::collections::HashMap<String, BoneTrack>,
    ) {
        let face_elem = obj.get("face");
        let mut name: Option<String> = None;
        let mut mix = 1.0f32;
        match face_elem {
            Some(super::json::Value::String(s)) => name = Some(s.clone()),
            Some(super::json::Value::Object(obj2)) => {
                name = member(obj2, "look")
                    .and_then(super::json::Value::as_str)
                    .map(str::to_string);
                mix = member(obj2, "mix")
                    .map(|e| safe_float(Some(e), 1.0))
                    .unwrap_or(1.0);
            }
            _ => {}
        }
        let Some(name) = name.filter(|n| !n.trim().is_empty()) else {
            return;
        };
        bones
            .entry(FACE_MIX_BONE.to_string())
            .or_insert_with(|| held_y(mix));
        let normalized = name.trim().to_lowercase().replace(' ', "_");
        bones
            .entry(format!("{}{}", FACE_PREFIX, normalized))
            .or_insert_with(|| held_y(1.0));
    }

    /// `readClip`: parses one clip from its JSON object (the Java public entry).
    pub fn read_clip(name: &str, obj: &super::json::Value) -> Clip {
        parse_clip(name, obj)
    }

    /// `parseClip`: a clip from its JSON object. The length comes from
    /// `animation_length`; a non-looping clip stretches to the last key's time;
    /// the length never drops below 1e-4.
    pub fn parse_clip(name: &str, obj: &super::json::Value) -> Clip {
        let mut length = match obj.get("animation_length") {
            Some(elem) => safe_float(Some(elem), 0.0),
            None => 0.0,
        };
        let looping = obj
            .get("loop")
            .map(|e| matches!(e, super::json::Value::Bool(true)))
            .unwrap_or(false);
        let mut bones: std::collections::HashMap<String, BoneTrack> =
            std::collections::HashMap::new();
        if let Some(super::json::Value::Object(members)) = obj.get("bones") {
            for (bone_name, value) in members {
                let super::json::Value::Object(track_obj) = value else {
                    continue;
                };
                let track = BoneTrack {
                    position: parse_channel(member(track_obj, "position"), Vec3::ZERO),
                    rotation: parse_channel(member(track_obj, "rotation"), Vec3::ZERO),
                    scale: parse_channel(member(track_obj, "scale"), Vec3::ONE),
                };
                bones.insert(bone_name.to_lowercase(), track.clone());
                if !looping {
                    length = length.max(track.last_key_time());
                }
            }
        }
        face_shorthand(obj, &mut bones);
        Clip::new(name.to_string(), 1.0e-4f32.max(length), looping, bones)
    }

    /// The mod's animation library: the Java `AnimationLibrary`'s storage and
    /// lookup, without the engine-coupled resolution (item stacks, biomes and
    /// passengers stay with the game layer; `groupFor` is supplied by the game
    /// layer too, since it needs the item registry).
    pub struct AnimationLibrary {
        clips: std::collections::HashMap<String, Clip>,
        bindings: ItemAnimationBindings,
        face_bones: std::collections::HashSet<String>,
        hands: Option<Hands>,
    }

    impl AnimationLibrary {
        /// Builds a library; the face bones are derived from the clips, like the
        /// Java `faceBones(clips)` helper.
        pub fn new(
            clips: std::collections::HashMap<String, Clip>,
            bindings: ItemAnimationBindings,
        ) -> Self {
            Self::with_hands(clips, bindings, None)
        }

        /// Builds a library with the hands state set.
        pub fn with_hands(
            clips: std::collections::HashMap<String, Clip>,
            bindings: ItemAnimationBindings,
            hands: Option<Hands>,
        ) -> Self {
            let mut face_bones = std::collections::HashSet::new();
            for clip in clips.values() {
                for bone in clip.bones.keys() {
                    if bone.starts_with(FACE_PREFIX) {
                        face_bones.insert(bone.clone());
                    }
                }
            }
            // A face bone with a side gains its opposite, so a blink of one eye
            // can find the other.
            for name in std::mem::take(&mut face_bones) {
                face_bones.insert(name.clone());
                if let Some(rest) = name.strip_suffix("_right") {
                    face_bones.insert(format!("{}_left", rest));
                } else if let Some(rest) = name.strip_suffix("_left") {
                    face_bones.insert(format!("{}_right", rest));
                }
            }
            Self {
                clips,
                bindings,
                face_bones,
                hands,
            }
        }

        /// Whether any clip loaded.
        pub fn has_animations(&self) -> bool {
            !self.clips.is_empty()
        }

        /// A clip by its exact name.
        pub fn clip(&self, name: &str) -> Option<&Clip> {
            self.clips.get(name)
        }

        /// Every clip whose name starts with `prefix`, sorted by name.
        pub fn clips_with_prefix(&self, prefix: &str) -> Vec<&Clip> {
            let mut found: Vec<&Clip> = self
                .clips
                .iter()
                .filter(|(name, _)| name.starts_with(prefix))
                .map(|(_, clip)| clip)
                .collect();
            found.sort_by(|a, b| a.name.cmp(&b.name));
            found
        }

        /// The bones the face shorthand installed across every loaded clip.
        pub fn face_bones(&self) -> &std::collections::HashSet<String> {
            &self.face_bones
        }

        /// The bindings the library loads.
        pub fn bindings(&self) -> &ItemAnimationBindings {
            &self.bindings
        }
    }

    // ---------------------------------------------------------------------------
    // The lookups the animation controllers read (the Java `AnimationLibrary`
    // handed-clip and binding chains)
    // ---------------------------------------------------------------------------

    impl AnimationLibrary {
        /// The hands state the per-frame glue sets, the Java `hands` field: which
        /// arm is the main one and whether each hand carries something.
        pub fn set_hands(&mut self, main_left: bool, main_occupied: bool, off_occupied: bool) {
            self.hands = Some(Hands {
                main_left,
                main_occupied,
                off_occupied,
            });
        }

        /// `otherHandOccupied`.
        pub fn other_hand_occupied(&self, left: bool) -> bool {
            match &self.hands {
                Some(hands) => {
                    if left == hands.main_left {
                        hands.off_occupied
                    } else {
                        hands.main_occupied
                    }
                }
                None => false,
            }
        }

        /// `namedClip`: the clip under `name_left`/`name_right`, then the
        /// `_alt` forms, then the bare name.
        pub fn named_clip(&self, name: Option<&str>, left: Option<bool>) -> Option<Clip> {
            let name = name.filter(|n| !n.trim().is_empty())?;
            if let Some(left) = left {
                if let Some(clip) =
                    self.clips
                        .get(&format!("{}_{}", name, if left { "left" } else { "right" }))
                {
                    return Some(clip.clone());
                }
                if let Some(base) = name.strip_suffix("_alt")
                    && let Some(clip) = self.clips.get(&format!(
                        "{}_{}_alt",
                        base,
                        if left { "left" } else { "right" }
                    ))
                {
                    return Some(clip.clone());
                }
            }
            self.clips.get(name).cloned()
        }

        /// `oneHandedName`: the first action prefix carrying a `one_handed`
        /// setting, walking up the `_` separators.
        pub fn one_handed_name(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Option<String> {
            let mut action = action.to_string();
            loop {
                let name = self.bindings.string_setting(
                    item,
                    group,
                    &format!("{}_one_handed", action),
                    "",
                );
                if !name.is_empty() {
                    return Some(name);
                }
                let cut = action.rfind('_')?;
                action.truncate(cut);
            }
        }

        /// `resolve`: the clip name an action resolves to, layered over the
        /// item, its group, the defaults and the empty hand.
        pub fn resolve_clip_name(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Option<String> {
            let action = normalize_id(action);
            let bindings = &self.bindings;
            if let Some(id) = item {
                if let Some(name) = bindings
                    .specific_items
                    .get(id)
                    .and_then(|map| map.get(&action))
                {
                    return Some(name.clone());
                }
                if let Some(group) = group
                    && let Some(name) = bindings.groups.get(group).and_then(|map| map.get(&action))
                {
                    return Some(name.clone());
                }
                return bindings.defaults.get(&action).cloned();
            }
            bindings
                .empty_hand
                .get(&action)
                .or_else(|| bindings.defaults.get(&action))
                .cloned()
        }

        /// `resolveClip`: the one-handed clip wins when the other hand is full.
        pub fn resolve_clip(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
            left: Option<bool>,
            other_occupied: bool,
        ) -> Option<Clip> {
            if other_occupied {
                let one_handed = self.one_handed_name(item, group, action);
                if let Some(clip) = self.named_clip(
                    one_handed
                        .as_deref()
                        .map(|_| {
                            self.one_handed_name(item, group, action)
                                .unwrap_or_default()
                        })
                        .as_deref()
                        .or(one_handed.as_deref()),
                    left,
                ) {
                    return Some(clip);
                }
            }
            let name = self.resolve_clip_name(item, group, action);
            self.named_clip(name.as_deref(), left)
        }

        /// `handedClipFor`: the clip for one action on one hand.
        pub fn handed_clip_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
            left: bool,
            other_occupied: bool,
        ) -> Option<Clip> {
            let action = action.to_string();
            if let Some(base) = action.strip_suffix("_one_handed") {
                let base = base.to_string();
                let name =
                    self.bindings
                        .string_setting(item, group, &format!("{}_one_handed", base), "");
                let name = if name.is_empty() {
                    self.resolve_clip_name(item, group, &action)
                } else {
                    Some(name)
                };
                return self.named_clip(name.as_deref(), Some(left));
            }
            // The bindings' own resolution may name a different clip.
            let resolved = self
                .resolve_clip_name(item, group, &action)
                .unwrap_or(action.clone());
            self.resolve_clip(item, group, &action, Some(left), other_occupied)
                .or_else(|| self.named_clip(Some(&resolved), Some(left)))
        }

        /// `handedClipsFor`: the sequence's clips, `action_1`, `action_2`, ...
        pub fn handed_clips_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
            left: bool,
            other_occupied: bool,
        ) -> Vec<Clip> {
            if other_occupied {
                let one_handed = self.one_handed_name(item, group, action);
                if let Some(clip) = self.named_clip(one_handed.as_deref(), Some(left)) {
                    return vec![clip];
                }
            }
            let mut out = Vec::new();
            for (index, name) in self
                .bindings
                .sequences_for(item, group, action)
                .iter()
                .enumerate()
            {
                let named = format!("{}_{}", action, index + 1);
                if let Some(clip) = self
                    .resolve_clip(item, group, &named, Some(left), other_occupied)
                    .or_else(|| self.named_clip(Some(name), Some(left)))
                {
                    out.push(clip);
                }
            }
            out
        }

        /// `overlaysFor`: the overlays for an action, resolved to clips, with
        /// the handed name preferred and the one-handed variant honoured.
        pub fn overlays_resolved(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
            left: bool,
            other_occupied: bool,
        ) -> Vec<ResolvedOverlay> {
            let mut out = Vec::new();
            for binding in self.bindings().overlays_for(item, group, action) {
                let clip = self
                    .named_clip(
                        Some(&format!(
                            "{}_{}",
                            binding.name,
                            if left { "left" } else { "right" }
                        )),
                        Some(left),
                    )
                    .or_else(|| self.named_clip(Some(&binding.name), Some(left)));
                if let Some(clip) = clip {
                    out.push(ResolvedOverlay {
                        clip,
                        mode: binding.mode,
                        bones: binding.bones,
                    });
                }
            }
            let _ = other_occupied;
            out
        }

        /// `hasItemBinding`.
        pub fn has_item_binding(&self, item: Option<&str>, action: &str) -> bool {
            let action = normalize_id(action);
            let bindings = &self.bindings;
            match item {
                Some(id) => {
                    bindings
                        .specific_items
                        .get(id)
                        .is_some_and(|map| map.contains_key(&action))
                        || bindings.defaults.contains_key(&action)
                }
                None => bindings.empty_hand.contains_key(&action),
            }
        }

        /// `retriggerProgress`: the stored percent over 100, clamped into a
        /// fraction.
        pub fn retrigger_progress(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
            default: f32,
        ) -> f32 {
            self.bindings.retrigger_for(item, group, action, default)
        }

        /// `basePoseRetention`: the specific over the group over the default
        /// (or the empty hand over the default).
        pub fn base_pose_retention_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> BasePoseRetention {
            let action = normalize_id(action);
            let bindings = &self.bindings;
            let base = bindings
                .default_retentions
                .get(&action)
                .cloned()
                .unwrap_or_default();
            match item {
                Some(id) => {
                    let specific = bindings
                        .specific_retentions
                        .get(id)
                        .and_then(|map| map.get(&action))
                        .cloned()
                        .unwrap_or_default();
                    let group = group
                        .and_then(|g| bindings.group_retentions.get(g))
                        .and_then(|map| map.get(&action))
                        .cloned()
                        .unwrap_or_default();
                    specific.over(&group.over(&base))
                }
                None => bindings
                    .empty_hand_retentions
                    .get(&action)
                    .cloned()
                    .unwrap_or_default()
                    .over(&base),
            }
        }

        /// `configuredLocomotionInfluence`.
        pub fn configured_locomotion_influence(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Option<LocomotionInfluence> {
            self.bindings
                .influence_for(item, group, action)
                .map(|influence| LocomotionInfluence {
                    walk: influence.walk,
                    sprint: influence.sprint,
                    jump: influence.jump,
                    transition: influence.transition,
                    hand_only: influence.hand_only,
                })
        }
    }

    /// The per-frame hands state.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Hands {
        /// The main hand is the left one.
        pub main_left: bool,
        /// The main hand carries something.
        pub main_occupied: bool,
        /// The off hand carries something.
        pub off_occupied: bool,
    }

    /// One overlay binding: the `OverlayBinding` record — a clip plus the bones
    /// it applies to.
    #[derive(Debug, Clone, PartialEq)]
    pub struct OverlayBinding {
        pub name: String,
        pub mode: OverlayMode,
        pub bones: Vec<String>,
    }

    /// One blend binding: the `BlendBinding` record — a from/to progress window
    /// with an optional boolean condition and a numeric modifier, each side
    /// optionally driven by a compiled expression.
    #[derive(Debug, Clone, PartialEq)]
    pub struct BlendBinding {
        pub from: f32,
        pub to: f32,
        pub duration_seconds: f32,
        pub condition: bool,
        pub condition_expression: Option<super::expressions::Expression>,
        pub modifier: f32,
        pub modifier_expression: Option<super::expressions::Expression>,
    }

    /// One timer binding: the `TimerBinding` record — a numeric modifier and an
    /// optional boolean condition, each side optionally expression-driven.
    #[derive(Debug, Clone, PartialEq)]
    pub struct TimerBinding {
        pub modifier: f32,
        pub condition: bool,
        pub modifier_expression: Option<super::expressions::Expression>,
        pub condition_expression: Option<super::expressions::Expression>,
    }

    /// One variable binding: the `VariableBinding` record — a named variable an
    /// expression can read, boolean or numeric, with the names it references.
    #[derive(Debug, Clone, PartialEq)]
    pub struct VariableBinding {
        pub name: String,
        pub is_bool: bool,
        pub expression: Option<super::expressions::Expression>,
        pub references: Vec<String>,
    }

    /// One locomotion binding: the `LocomotionBinding` record — a clip with an
    /// optional condition string, an effect, overlays, a reverse flag and an
    /// optional add clip.
    #[derive(Debug, Clone, PartialEq)]
    pub struct LocomotionBinding {
        pub clip: String,
        pub condition: Option<String>,
        pub effect: Option<String>,
        pub overlays: Vec<OverlayBinding>,
        pub reverse: bool,
        pub add: Option<String>,
    }

    /// One locomotion influence: the bindings' `LocomotionInfluence` record.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct BindLocomotionInfluence {
        pub walk: f32,
        pub sprint: f32,
        pub jump: f32,
        pub transition: f32,
        pub hand_only: bool,
    }

    /// One emote binding: the `EmoteBinding` record.
    #[derive(Debug, Clone, PartialEq)]
    pub struct EmoteBinding {
        pub id: String,
        pub display_name: String,
        pub animation: String,
        pub key: Option<String>,
        pub face: Option<String>,
    }

    /// The item bindings: the Java `ItemAnimationBindings`' storage and lookup.
    /// The layering is specific item → group → default, with the empty-hand
    /// layer standing in for a missing item. `group_for` is supplied by the
    /// caller (the game layer owns the item registry), so resolution takes the
    /// group explicitly.
    pub struct ItemAnimationBindings {
        defaults: HashMap<String, String>,
        empty_hand: HashMap<String, String>,
        groups: HashMap<String, HashMap<String, String>>,
        specific_items: HashMap<String, HashMap<String, String>>,
        default_sequences: HashMap<String, Vec<String>>,
        empty_hand_sequences: HashMap<String, Vec<String>>,
        group_sequences: HashMap<String, HashMap<String, Vec<String>>>,
        specific_sequences: HashMap<String, HashMap<String, Vec<String>>>,
        pose_idles: HashMap<String, String>,
        timer_effects: HashMap<String, String>,
        timer_scales: HashMap<String, f64>,
        numeric_settings: HashMap<String, f64>,
        string_settings: HashMap<String, String>,
        empty_hand_string_settings: HashMap<String, String>,
        variants: HashMap<String, Vec<String>>,
        locomotion: HashMap<String, Vec<LocomotionBinding>>,
        default_retrigger: HashMap<String, f64>,
        empty_hand_retrigger: HashMap<String, f64>,
        group_retrigger: HashMap<String, HashMap<String, f64>>,
        specific_retrigger: HashMap<String, HashMap<String, f64>>,
        default_influences: HashMap<String, BindLocomotionInfluence>,
        empty_hand_influences: HashMap<String, BindLocomotionInfluence>,
        group_influences: HashMap<String, HashMap<String, BindLocomotionInfluence>>,
        specific_influences: HashMap<String, HashMap<String, BindLocomotionInfluence>>,
        default_overlays: HashMap<String, Vec<OverlayBinding>>,
        group_overlays: HashMap<String, HashMap<String, Vec<OverlayBinding>>>,
        specific_overlays: HashMap<String, HashMap<String, Vec<OverlayBinding>>>,
        default_blends: HashMap<String, BlendBinding>,
        empty_hand_blends: HashMap<String, BlendBinding>,
        group_blends: HashMap<String, HashMap<String, BlendBinding>>,
        specific_blends: HashMap<String, HashMap<String, BlendBinding>>,
        default_timers: HashMap<String, TimerBinding>,
        empty_hand_timers: HashMap<String, TimerBinding>,
        group_timers: HashMap<String, HashMap<String, TimerBinding>>,
        specific_timers: HashMap<String, HashMap<String, TimerBinding>>,
        group_numeric_settings: HashMap<String, HashMap<String, f64>>,
        specific_numeric_settings: HashMap<String, HashMap<String, f64>>,
        group_string_settings: HashMap<String, HashMap<String, String>>,
        specific_string_settings: HashMap<String, HashMap<String, String>>,
        default_static_when_using: Option<bool>,
        empty_hand_static_when_using: Option<bool>,
        group_static_when_using: HashMap<String, bool>,
        specific_static_when_using: HashMap<String, bool>,
        variables: Vec<VariableBinding>,
        emote_menu_key: Option<String>,
        emote_bindings: Vec<EmoteBinding>,
        default_one_handed: HashMap<String, String>,
        default_animation_types: std::collections::HashSet<String>,
        default_retentions: HashMap<String, BasePoseRetention>,
        empty_hand_retentions: HashMap<String, BasePoseRetention>,
        group_retentions: HashMap<String, HashMap<String, BasePoseRetention>>,
        specific_retentions: HashMap<String, HashMap<String, BasePoseRetention>>,
    }

    impl Default for ItemAnimationBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ItemAnimationBindings {
        /// `numericSetting(stack, action, key, default)`: the composite
        /// `action + field` key, layered, falling back to the bare field.
        pub fn numeric_action_setting(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
            key: &str,
            default: f32,
        ) -> f32 {
            let composite = format!("{}{}", normalize_id(action), normalize_id(key));
            let flat = normalize_id(key);
            let pick = |map: &HashMap<String, f64>| {
                map.get(&composite).or_else(|| map.get(&flat)).copied()
            };
            let value = match item {
                Some(id) => self
                    .specific_numeric_settings
                    .get(id)
                    .and_then(pick)
                    .or_else(|| {
                        group
                            .and_then(|g| self.group_numeric_settings.get(g))
                            .and_then(pick)
                    })
                    .or_else(|| pick(&self.numeric_settings)),
                None => pick(&self.numeric_settings),
            };
            value.map(|v| v as f32).unwrap_or(default)
        }

        /// `stringSetting(stack, action, key, default)`: the composite key,
        /// falling back to the bare field.
        pub fn string_action_setting(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
            key: &str,
            default: &str,
        ) -> String {
            let composite = format!("{}{}", normalize_id(action), normalize_id(key));
            let flat = normalize_id(key);
            let pick = |map: &HashMap<String, String>| {
                map.get(&composite).or_else(|| map.get(&flat)).cloned()
            };
            match item {
                Some(id) => self
                    .specific_string_settings
                    .get(id)
                    .and_then(pick)
                    .or_else(|| {
                        group
                            .and_then(|g| self.group_string_settings.get(g))
                            .and_then(pick)
                    })
                    .or_else(|| pick(&self.string_settings))
                    .unwrap_or_else(|| default.to_string()),
                None => pick(&self.empty_hand_string_settings)
                    .or_else(|| pick(&self.string_settings))
                    .unwrap_or_else(|| default.to_string()),
            }
        }

        pub fn new() -> Self {
            Self {
                defaults: HashMap::new(),
                empty_hand: HashMap::new(),
                groups: HashMap::new(),
                specific_items: HashMap::new(),
                default_sequences: HashMap::new(),
                empty_hand_sequences: HashMap::new(),
                group_sequences: HashMap::new(),
                specific_sequences: HashMap::new(),
                pose_idles: HashMap::new(),
                timer_effects: HashMap::new(),
                timer_scales: HashMap::new(),
                numeric_settings: HashMap::new(),
                string_settings: HashMap::new(),
                empty_hand_string_settings: HashMap::new(),
                variants: HashMap::new(),
                locomotion: HashMap::new(),
                default_retrigger: HashMap::new(),
                empty_hand_retrigger: HashMap::new(),
                group_retrigger: HashMap::new(),
                specific_retrigger: HashMap::new(),
                default_influences: HashMap::new(),
                empty_hand_influences: HashMap::new(),
                group_influences: HashMap::new(),
                specific_influences: HashMap::new(),
                default_overlays: HashMap::new(),
                group_overlays: HashMap::new(),
                specific_overlays: HashMap::new(),
                default_blends: HashMap::new(),
                empty_hand_blends: HashMap::new(),
                group_blends: HashMap::new(),
                specific_blends: HashMap::new(),
                default_timers: HashMap::new(),
                empty_hand_timers: HashMap::new(),
                group_timers: HashMap::new(),
                specific_timers: HashMap::new(),
                group_numeric_settings: HashMap::new(),
                specific_numeric_settings: HashMap::new(),
                group_string_settings: HashMap::new(),
                specific_string_settings: HashMap::new(),
                default_static_when_using: None,
                empty_hand_static_when_using: None,
                group_static_when_using: HashMap::new(),
                specific_static_when_using: HashMap::new(),
                variables: Vec::new(),
                emote_menu_key: None,
                emote_bindings: Vec::new(),
                default_one_handed: HashMap::new(),
                default_animation_types: std::collections::HashSet::new(),
                default_retentions: HashMap::new(),
                empty_hand_retentions: HashMap::new(),
                group_retentions: HashMap::new(),
                specific_retentions: HashMap::new(),
            }
        }

        /// Ingests one normalized (format 2) bindings JSON object: the Java
        /// `add`. The four layers are `default_animations`, `emptyHand`,
        /// `item` (groups) and `itemSpecific`.
        pub fn add(&mut self, root: &super::json::Value) {
            if !root.is_object() {
                return;
            }
            if let Some(v) = root.get("static_when_using") {
                self.default_static_when_using = optional_boolean(v);
            }
            if let Some(v) = root.get("variables") {
                self.add_variables(v);
            }
            if let Some(elem) = root.get("default_animations") {
                self.add_entries(elem, Layer::Default, EntryKinds::all());
                self.add_sequences(elem, Layer::Default);
                self.add_retrigger(elem, Layer::Default);
                self.add_influences(elem, Layer::Default);
                self.add_overlays(elem, Layer::Default);
                self.add_blends(elem, Layer::Default);
                self.add_retentions(elem, Layer::Default);
                self.add_timers(elem, Layer::Default);
                self.add_emote_entries(elem);
                self.index_one_handed_clips(elem);
            }
            if let Some(empty_obj) = root.get("emptyHand").and_then(|e| e.as_object()) {
                if let Some(v) = member(empty_obj, "static_when_using") {
                    self.empty_hand_static_when_using = optional_boolean(v);
                }
                if let Some(custom) = member(empty_obj, "customAnimation") {
                    self.add_entries(
                        custom,
                        Layer::EmptyHand,
                        EntryKinds {
                            strings_to_empty_hand: true,
                            ..EntryKinds::default()
                        },
                    );
                    self.add_sequences(custom, Layer::EmptyHand);
                    self.add_retrigger(custom, Layer::EmptyHand);
                    self.add_influences(custom, Layer::EmptyHand);
                    self.add_blends(custom, Layer::EmptyHand);
                    self.add_retentions(custom, Layer::EmptyHand);
                    self.add_timers(custom, Layer::EmptyHand);
                }
            }
            if let Some(groups) = root.get("item").and_then(|e| e.as_object()) {
                for (group_name, group_obj) in groups {
                    let group = normalize_id(group_name);
                    if let Some(v) = group_obj.get("static_when_using") {
                        self.group_static_when_using
                            .insert(group.clone(), optional_boolean(v).unwrap_or(false));
                    }
                    if let Some(camera) = group_obj.get("throwable_camera") {
                        let map = self
                            .group_numeric_settings
                            .entry(group.clone())
                            .or_default();
                        Self::add_camera_settings(camera, map);
                    }
                    if let Some(custom) = group_obj.get("customAnimation") {
                        self.add_entries(
                            custom,
                            Layer::Group(group.clone()),
                            EntryKinds {
                                numeric: true,
                                ..EntryKinds::default()
                            },
                        );
                        self.add_sequences(custom, Layer::Group(group.clone()));
                        self.add_retrigger(custom, Layer::Group(group.clone()));
                        self.add_influences(custom, Layer::Group(group.clone()));
                        self.add_overlays(custom, Layer::Group(group.clone()));
                        self.add_blends(custom, Layer::Group(group.clone()));
                        self.add_retentions(custom, Layer::Group(group.clone()));
                        self.add_timers(custom, Layer::Group(group.clone()));
                    }
                }
            }
            if let Some(specific) = root.get("itemSpecific").and_then(|e| e.as_object()) {
                for (item_name, item_obj) in specific {
                    let item = normalize_id(item_name);
                    if let Some(v) = item_obj.get("static_when_using") {
                        self.specific_static_when_using
                            .insert(item.clone(), optional_boolean(v).unwrap_or(false));
                    }
                    if let Some(camera) = item_obj.get("throwable_camera") {
                        let map = self
                            .specific_numeric_settings
                            .entry(item.clone())
                            .or_default();
                        Self::add_camera_settings(camera, map);
                    }
                    if let Some(custom) = item_obj.get("customAnimation") {
                        self.add_entries(
                            custom,
                            Layer::Specific(item.clone()),
                            EntryKinds {
                                numeric: true,
                                ..EntryKinds::default()
                            },
                        );
                        self.add_sequences(custom, Layer::Specific(item.clone()));
                        self.add_retrigger(custom, Layer::Specific(item.clone()));
                        self.add_influences(custom, Layer::Specific(item.clone()));
                        self.add_overlays(custom, Layer::Specific(item.clone()));
                        self.add_blends(custom, Layer::Specific(item.clone()));
                        self.add_retentions(custom, Layer::Specific(item.clone()));
                        self.add_timers(custom, Layer::Specific(item.clone()));
                    }
                }
            }
        }

        /// The clip an action resolves to: specific item → group → default,
        /// then the empty-hand layer for a missing item.
        pub fn resolve(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Option<&String> {
            let action = normalize_id(action);
            if let Some(id) = item {
                if let Some(map) = self.specific_items.get(id)
                    && let Some(clip) = map.get(&action)
                {
                    return Some(clip);
                }
                if let Some(group) = group
                    && let Some(map) = self.groups.get(group)
                    && let Some(clip) = map.get(&action)
                {
                    return Some(clip);
                }
                return self.defaults.get(&action);
            }
            if let Some(map) = self.empty_hand.get(&action) {
                return Some(map);
            }
            self.defaults.get(&action)
        }

        /// `first`: the layered string lookup — specific map, then group map,
        /// then the default map.
        fn first<'a>(
            specific: Option<&'a HashMap<String, String>>,
            group: Option<&'a HashMap<String, String>>,
            defaults: &'a HashMap<String, String>,
            action: &str,
        ) -> Option<&'a String> {
            if let Some(map) = specific
                && let Some(v) = map.get(action)
            {
                return Some(v);
            }
            if let Some(map) = group
                && let Some(v) = map.get(action)
            {
                return Some(v);
            }
            defaults.get(action)
        }
    }

    impl EntryKinds {
        /// Every sub-target filled: the Java defaults call passes all nine maps.
        pub fn all() -> Self {
            Self {
                pose_idles: true,
                timer_effects: true,
                timer_scales: true,
                numeric: true,
                strings_to_empty_hand: false,
                variants: true,
                locomotion: true,
            }
        }
    }

    impl ItemAnimationBindings {
        /// Ingests one action-to-descriptor pass into the layer's maps. The
        /// per-action descriptor array carries the clip (`name` or `clip`),
        /// optional `variants`, and an optional `one_handed` clip.
        fn add_entries(&mut self, elem: &super::json::Value, layer: Layer, kinds: EntryKinds) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(desc) = descriptor.as_object() else {
                    continue;
                };
                // The entry's `type` names the action it poses.
                let Some(action) = member(desc, "type")
                    .and_then(super::json::Value::as_str)
                    .map(normalize_id)
                else {
                    continue;
                };
                let clip_name = member(desc, "name")
                    .and_then(super::json::Value::as_str)
                    .or_else(|| member(desc, "clip").and_then(super::json::Value::as_str));
                if let Some(clip_name) = clip_name
                    && let Some(entries) = self.entries_map(&layer)
                {
                    entries.insert(action.clone(), clip_name.to_string());
                    self.default_animation_types.insert(action.clone());
                }
                if kinds.variants
                    && let Some(vs) = member(desc, "variants").and_then(|v| v.as_array())
                {
                    // A variant is a clip name, or an object with `name`/`clip`.
                    let names: Vec<String> = vs
                        .iter()
                        .filter_map(|v| {
                            v.as_str()
                                .map(str::to_string)
                                .or_else(|| value_of(v.as_object()?, &["name", "clip"]))
                        })
                        .collect();
                    if !names.is_empty() {
                        self.variants.insert(action.clone(), names);
                    }
                }
                if let Some(one_handed) = member(desc, "one_handed") {
                    let clip = match one_handed {
                        super::json::Value::String(s) => Some(s.clone()),
                        super::json::Value::Object(o) => value_of(o, &["clip", "name"]),
                        _ => None,
                    };
                    if let Some(clip) = clip {
                        self.default_one_handed.insert(action.clone(), clip);
                    }
                }
                if kinds.pose_idles
                    && let Some(idle) = member(desc, "pose_idle").and_then(|v| v.as_str())
                {
                    self.pose_idles.insert(action.clone(), idle.to_string());
                }
                if kinds.timer_effects
                    && let Some(effect) = member(desc, "timer_effect").and_then(|v| v.as_str())
                {
                    self.timer_effects
                        .insert(action.clone(), effect.to_string());
                }
                if kinds.timer_scales
                    && let Some(scale) = member(desc, "timer_scale")
                    && let Some(f) = scale.as_f64()
                {
                    self.timer_scales.insert(action.clone(), f);
                }
                // The Java chain reads its setting fields straight off the
                // descriptor and stores them under `action + field`.
                for (field, value) in desc {
                    if let Some(f) = value.as_f64()
                        && NUMERIC_SETTING_FIELDS.contains(&field.as_str())
                    {
                        let target = self.numeric_map(&layer);
                        target.insert(format!("{}{}", action, normalize_id(field)), f);
                    }
                    if let Some(text) = value.as_str()
                        && STRING_SETTING_FIELDS.contains(&field.as_str())
                        && let Some(target) = self.string_map(&layer)
                    {
                        target.insert(
                            format!("{}{}", action, normalize_id(field)),
                            text.to_string(),
                        );
                    }
                }
                if kinds.numeric
                    && let Some(numeric) = member(desc, "settings").and_then(|v| v.as_object())
                {
                    let target = self.numeric_map(&layer);
                    for (k, v) in numeric {
                        if let Some(f) = v.as_f64() {
                            target.insert(normalize_id(k), f);
                        }
                    }
                }
                if kinds.strings_to_empty_hand {
                    if let Some(strings) = member(desc, "settings").and_then(|v| v.as_object()) {
                        for (k, v) in strings {
                            if let Some(s) = v.as_str() {
                                self.empty_hand_string_settings
                                    .insert(normalize_id(k), s.to_string());
                            }
                        }
                    }
                } else if let Some(strings) = member(desc, "settings").and_then(|v| v.as_object())
                    && let Some(target) = self.string_map(&layer)
                {
                    for (k, v) in strings {
                        if let Some(s) = v.as_str() {
                            target.insert(normalize_id(k), s.to_string());
                        }
                    }
                }
                if kinds.locomotion
                    && let Some(loc) = member(desc, "locomotion")
                {
                    let binding = parse_locomotion_binding(loc);
                    self.locomotion
                        .entry(action.clone())
                        .or_default()
                        .push(binding);
                }
            }
        }

        fn entries_map(&mut self, layer: &Layer) -> Option<&mut HashMap<String, String>> {
            match layer {
                Layer::Default => Some(&mut self.defaults),
                Layer::EmptyHand => Some(&mut self.empty_hand),
                Layer::Group(g) => Some(self.groups.entry(g.clone()).or_default()),
                Layer::Specific(i) => Some(self.specific_items.entry(i.clone()).or_default()),
            }
        }

        fn numeric_map(&mut self, layer: &Layer) -> &mut HashMap<String, f64> {
            match layer {
                Layer::Default => &mut self.numeric_settings,
                Layer::EmptyHand => &mut self.numeric_settings,
                Layer::Group(g) => self.group_numeric_settings.entry(g.clone()).or_default(),
                Layer::Specific(i) => self.specific_numeric_settings.entry(i.clone()).or_default(),
            }
        }

        fn string_map(&mut self, layer: &Layer) -> Option<&mut HashMap<String, String>> {
            match layer {
                Layer::Default => Some(&mut self.string_settings),
                Layer::EmptyHand => Some(&mut self.empty_hand_string_settings),
                Layer::Group(g) => Some(self.group_string_settings.entry(g.clone()).or_default()),
                Layer::Specific(i) => {
                    Some(self.specific_string_settings.entry(i.clone()).or_default())
                }
            }
        }

        fn add_sequences(&mut self, elem: &super::json::Value, layer: Layer) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(obj) = descriptor.as_object() else {
                    continue;
                };
                let Some(action) = member(obj, "type")
                    .and_then(super::json::Value::as_str)
                    .map(normalize_id)
                else {
                    continue;
                };
                // The sequence is the entry's `var_1`, `var_2`, ... clips.
                let mut names = Vec::new();
                let mut index = 1;
                while let Some(name) = value_of(obj, &[&format!("var_{}", index)]) {
                    names.push(name);
                    index += 1;
                }
                if names.is_empty() {
                    continue;
                }
                match &layer {
                    Layer::Default => {
                        self.default_sequences.insert(action, names);
                    }
                    Layer::EmptyHand => {
                        self.empty_hand_sequences.insert(action, names);
                    }
                    Layer::Group(g) => {
                        self.group_sequences
                            .entry(g.clone())
                            .or_default()
                            .insert(action, names);
                    }
                    Layer::Specific(i) => {
                        self.specific_sequences
                            .entry(i.clone())
                            .or_default()
                            .insert(action, names);
                    }
                }
            }
        }

        fn add_retrigger(&mut self, elem: &super::json::Value, layer: Layer) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(obj) = descriptor.as_object() else {
                    continue;
                };
                let Some(action) = member(obj, "type")
                    .and_then(super::json::Value::as_str)
                    .map(normalize_id)
                else {
                    continue;
                };
                let Some(f) = member(obj, "retrigger_progress").and_then(|v| v.as_f64()) else {
                    continue;
                };
                match &layer {
                    Layer::Default => {
                        self.default_retrigger.insert(action, f);
                    }
                    Layer::EmptyHand => {
                        self.empty_hand_retrigger.insert(action, f);
                    }
                    Layer::Group(g) => {
                        self.group_retrigger
                            .entry(g.clone())
                            .or_default()
                            .insert(action, f);
                    }
                    Layer::Specific(i) => {
                        self.specific_retrigger
                            .entry(i.clone())
                            .or_default()
                            .insert(action, f);
                    }
                }
            }
        }

        fn add_influences(&mut self, elem: &super::json::Value, layer: Layer) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(obj) = descriptor.as_object() else {
                    continue;
                };
                let Some(action) = member(obj, "type")
                    .and_then(super::json::Value::as_str)
                    .map(normalize_id)
                else {
                    continue;
                };
                let Some(influence) = locomotion_influence(member(obj, "locomotion_influence"))
                else {
                    continue;
                };
                match &layer {
                    Layer::Default => {
                        self.default_influences.insert(action, influence);
                    }
                    Layer::EmptyHand => {
                        self.empty_hand_influences.insert(action, influence);
                    }
                    Layer::Group(g) => {
                        self.group_influences
                            .entry(g.clone())
                            .or_default()
                            .insert(action, influence);
                    }
                    Layer::Specific(i) => {
                        self.specific_influences
                            .entry(i.clone())
                            .or_default()
                            .insert(action, influence);
                    }
                }
            }
        }

        fn add_overlays(&mut self, elem: &super::json::Value, layer: Layer) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(obj) = descriptor.as_object() else {
                    continue;
                };
                let Some(ty) = member(obj, "type").and_then(super::json::Value::as_str) else {
                    continue;
                };
                let action = normalize_id(ty);
                for (field, suffix) in [("overlays", ""), ("one_handed_overlays", "onehanded")] {
                    let Some(list) = member(obj, field).and_then(|v| v.as_array()) else {
                        continue;
                    };
                    let bindings: Vec<OverlayBinding> =
                        list.iter().filter_map(parse_overlay_binding).collect();
                    if bindings.is_empty() {
                        continue;
                    }
                    let key = format!("{}{}", action, suffix);
                    match &layer {
                        Layer::Default => {
                            self.default_overlays.insert(key, bindings);
                        }
                        Layer::EmptyHand => {}
                        Layer::Group(g) => {
                            self.group_overlays
                                .entry(g.clone())
                                .or_default()
                                .insert(key, bindings);
                        }
                        Layer::Specific(i) => {
                            self.specific_overlays
                                .entry(i.clone())
                                .or_default()
                                .insert(key, bindings);
                        }
                    }
                }
            }
        }

        fn add_blends(&mut self, elem: &super::json::Value, layer: Layer) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(obj) = descriptor.as_object() else {
                    continue;
                };
                let Some(action) = member(obj, "type")
                    .and_then(super::json::Value::as_str)
                    .map(normalize_id)
                else {
                    continue;
                };
                let Some(blending) = member(obj, "blending").and_then(|v| v.as_object()) else {
                    continue;
                };
                let binding = BlendBinding {
                    from: bounded_float(blending, "from", 0.0),
                    to: bounded_float(blending, "to", 1.0),
                    duration_seconds: non_negative_float(obj, "blending_progression", 0.0),
                    condition: literal_blend_condition(member(obj, "blending_condition")),
                    condition_expression: expression(obj, "blending_condition", "true"),
                    modifier: non_negative_float(obj, "blending_modifier", 1.0),
                    modifier_expression: expression(obj, "blending_modifier", "1"),
                };
                match &layer {
                    Layer::Default => {
                        self.default_blends.insert(action, binding);
                    }
                    Layer::EmptyHand => {
                        self.empty_hand_blends.insert(action, binding);
                    }
                    Layer::Group(g) => {
                        self.group_blends
                            .entry(g.clone())
                            .or_default()
                            .insert(action, binding);
                    }
                    Layer::Specific(i) => {
                        self.specific_blends
                            .entry(i.clone())
                            .or_default()
                            .insert(action, binding);
                    }
                }
            }
        }

        fn add_timers(&mut self, elem: &super::json::Value, layer: Layer) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(obj) = descriptor.as_object() else {
                    continue;
                };
                let Some(action) = member(obj, "type")
                    .and_then(super::json::Value::as_str)
                    .map(normalize_id)
                else {
                    continue;
                };
                if member(obj, "timer_modifier").is_none()
                    && member(obj, "timer_condition").is_none()
                {
                    continue;
                }
                let binding = TimerBinding {
                    modifier: non_negative_float(obj, "timer_modifier", 1.0),
                    condition: literal_blend_condition(member(obj, "timer_condition")),
                    modifier_expression: expression(obj, "timer_modifier", "1"),
                    condition_expression: expression(obj, "timer_condition", "true"),
                };
                match &layer {
                    Layer::Default => {
                        self.default_timers.insert(action, binding);
                    }
                    Layer::EmptyHand => {
                        self.empty_hand_timers.insert(action, binding);
                    }
                    Layer::Group(g) => {
                        self.group_timers
                            .entry(g.clone())
                            .or_default()
                            .insert(action, binding);
                    }
                    Layer::Specific(i) => {
                        self.specific_timers
                            .entry(i.clone())
                            .or_default()
                            .insert(action, binding);
                    }
                }
            }
        }

        /// `addRetentions`: the entry's `base_pose_retain`, parsed with the
        /// entry's type as the error context.
        fn add_retentions(&mut self, elem: &super::json::Value, layer: Layer) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(obj) = descriptor.as_object() else {
                    continue;
                };
                let Some(ty) = member(obj, "type").and_then(super::json::Value::as_str) else {
                    continue;
                };
                let Some(retain) = member(obj, "base_pose_retain") else {
                    continue;
                };
                let Ok(retention) =
                    BasePoseRetention::parse(Some(retain), &format!("{}.base_pose_retain", ty))
                else {
                    continue;
                };
                let action = normalize_id(ty);
                match &layer {
                    Layer::Default => {
                        self.default_retentions.insert(action, retention);
                    }
                    Layer::EmptyHand => {
                        self.empty_hand_retentions.insert(action, retention);
                    }
                    Layer::Group(g) => {
                        self.group_retentions
                            .entry(g.clone())
                            .or_default()
                            .insert(action, retention);
                    }
                    Layer::Specific(i) => {
                        self.specific_retentions
                            .entry(i.clone())
                            .or_default()
                            .insert(action, retention);
                    }
                }
            }
        }

        /// `addEmoteEntries`: the `emotes` entry's menu key and its emote list.
        fn add_emote_entries(&mut self, elem: &super::json::Value) {
            let Some(descriptors) = elem.as_array() else {
                return;
            };
            for descriptor in descriptors {
                let Some(obj) = descriptor.as_object() else {
                    continue;
                };
                let is_emotes = member(obj, "type")
                    .and_then(super::json::Value::as_str)
                    .is_some_and(|ty| normalize_id(ty) == "emotes");
                if !is_emotes {
                    continue;
                }
                if let Some(key) = value_of(obj, &["menu_key", "keybind", "key"]) {
                    self.emote_menu_key = Some(key);
                }
                let entries = member(obj, "entries")
                    .or_else(|| member(obj, "emotes"))
                    .and_then(|v| v.as_array());
                let Some(entries) = entries else {
                    continue;
                };
                for entry in entries {
                    let Some(entry) = entry.as_object() else {
                        continue;
                    };
                    let Some(animation) = value_of(entry, &["animation", "name", "clip"]) else {
                        continue;
                    };
                    let id = value_of(entry, &["id"]).unwrap_or_else(|| animation.clone());
                    let display_name = value_of(entry, &["display_name", "label", "title"])
                        .unwrap_or_else(|| id.clone());
                    let key =
                        value_of(entry, &["keybind", "key"]).unwrap_or_else(|| "UNKNOWN".into());
                    let face = value_of(entry, &["face", "expression"]);
                    self.emote_bindings.push(EmoteBinding {
                        id,
                        display_name,
                        animation,
                        key: Some(key),
                        face,
                    });
                }
            }
        }

        fn add_camera_settings(camera: &super::json::Value, map: &mut HashMap<String, f64>) {
            let Some(obj) = camera.as_object() else {
                return;
            };
            for (k, v) in obj {
                if let Some(f) = v.as_f64() {
                    map.insert(normalize_id(k), f);
                }
            }
        }

        fn add_variables(&mut self, elem: &super::json::Value) {
            let Some(items) = elem.as_array() else {
                return;
            };
            for item in items {
                let Some(obj) = item.as_object() else {
                    continue;
                };
                let Some(name) = member(obj, "name").and_then(super::json::Value::as_str) else {
                    continue;
                };
                let is_bool = member(obj, "bool")
                    .map(|v| matches!(v, super::json::Value::Bool(true)))
                    .unwrap_or(false);
                let expression = member(obj, "expression")
                    .and_then(super::json::Value::as_str)
                    .and_then(super::expressions::compile);
                let references = expression
                    .as_ref()
                    .map(|_| vec![name.to_string()])
                    .unwrap_or_default();
                self.variables.push(VariableBinding {
                    name: name.to_string(),
                    is_bool,
                    expression,
                    references,
                });
            }
        }

        /// `indexOneHandedClips`: the entry's `one_handed` clip, or the first
        /// one found on a nested entry under `subanimations`.
        fn index_one_handed_clips(&mut self, elem: &super::json::Value) {
            self.index_one_handed(elem, None);
        }

        fn index_one_handed(&mut self, elem: &super::json::Value, action: Option<&str>) {
            if let Some(items) = elem.as_array() {
                for item in items {
                    self.index_one_handed(item, action);
                }
                return;
            }
            let Some(obj) = elem.as_object() else {
                return;
            };
            let action = member(obj, "type")
                .and_then(super::json::Value::as_str)
                .map(normalize_id)
                .or_else(|| action.map(str::to_string));
            if let Some(one_handed) = member(obj, "one_handed") {
                let clip = match one_handed {
                    super::json::Value::String(s) => Some(s.clone()),
                    super::json::Value::Object(o) => value_of(o, &["clip", "name"]),
                    _ => None,
                };
                if let (Some(clip), Some(action)) = (clip, action.as_deref()) {
                    self.default_one_handed
                        .entry(action.to_string())
                        .or_insert(clip);
                }
            }
            if let Some(children) = member(obj, "subanimations") {
                self.index_one_handed(children, action.as_deref());
            }
        }

        /// `firstRetention`: the specific, then group, then empty-hand, then
        /// default retention for an action.
        #[allow(dead_code)] // the game layer calls this once retention lookups land
        fn first_retention(
            &self,
            item: &str,
            group: &str,
            action: &str,
        ) -> Option<BasePoseRetention> {
            self.specific_retentions
                .get(item)
                .and_then(|map| map.get(action))
                .or_else(|| {
                    self.group_retentions
                        .get(group)
                        .and_then(|map| map.get(action))
                })
                .or_else(|| self.empty_hand_retentions.get(action))
                .or_else(|| self.default_retentions.get(action))
                .cloned()
        }
    }

    /// `optional_boolean`: a JSON `true`/`false`, else `None`.
    fn optional_boolean(v: &super::json::Value) -> Option<bool> {
        match v {
            super::json::Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// `value`: the first member of `keys` holding a non-blank string.
    fn value_of(obj: &[(String, super::json::Value)], keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(value) = member(obj, key)
                && let Some(text) = value.as_str()
                && !text.trim().is_empty()
            {
                return Some(text.to_string());
            }
        }
        None
    }

    /// `boundedFloat`: a number clamped into `0.0..=1.0`, `fallback` when the
    /// member is missing or not a number.
    fn bounded_float(obj: &[(String, super::json::Value)], key: &str, fallback: f32) -> f32 {
        match member(obj, key).and_then(|v| v.as_f64()) {
            Some(f) => (f as f32).clamp(0.0, 1.0),
            None => fallback,
        }
    }

    /// `nonNegativeFloat` (and `positiveFloat`, the same body): a number floored
    /// at `0.0`, `fallback` when the member is missing or not a number.
    fn non_negative_float(obj: &[(String, super::json::Value)], key: &str, fallback: f32) -> f32 {
        match member(obj, key).and_then(|v| v.as_f64()) {
            Some(f) => (f as f32).max(0.0),
            None => fallback,
        }
    }

    /// `literalBlendCondition`: the condition as a plain value — absent is true,
    /// a boolean is itself, a string is anything but `false`, `never` and `0`
    /// (normalized), and anything else is true.
    fn literal_blend_condition(value: Option<&super::json::Value>) -> bool {
        let Some(value) = value else {
            return true;
        };
        if value.is_null() {
            return true;
        }
        match value {
            super::json::Value::Bool(b) => *b,
            super::json::Value::String(s) => {
                let normalized = normalize_id(s);
                normalized != "false" && normalized != "never" && normalized != "0"
            }
            _ => true,
        }
    }

    /// `expression`: the field compiled, the fallback compiled when the field is
    /// missing or does not compile.
    fn expression(
        obj: &[(String, super::json::Value)],
        field: &str,
        fallback: &str,
    ) -> Option<super::expressions::Expression> {
        let source = value_of(obj, &[field]).unwrap_or_else(|| fallback.to_string());
        super::expressions::compile(&source).or_else(|| super::expressions::compile(fallback))
    }

    /// `locomotionInfluence`: a number is all three weights at once; an object
    /// carries `default`, `walk`/`walking`, `sprint`/`run`, `jump`,
    /// `transition` and `hand_only`.
    fn locomotion_influence(value: Option<&super::json::Value>) -> Option<BindLocomotionInfluence> {
        let value = value.filter(|v| !v.is_null())?;
        if let Some(f) = value.as_f64() {
            let f = normalize_locomotion_influence(f as f32);
            return Some(BindLocomotionInfluence {
                walk: f,
                sprint: f,
                jump: f,
                transition: 1.0,
                hand_only: false,
            });
        }
        let obj = value.as_object()?;
        let default = normalize_locomotion_influence(number(obj, "default", 1.0));
        let walk =
            normalize_locomotion_influence(number(obj, "walk", number(obj, "walking", default)));
        let sprint =
            normalize_locomotion_influence(number(obj, "sprint", number(obj, "run", default)));
        let jump = normalize_locomotion_influence(number(obj, "jump", default));
        let transition = normalize_locomotion_influence(number(obj, "transition", 1.0));
        let hand_only = member(obj, "hand_only")
            .map(|v| matches!(v, super::json::Value::Bool(true)))
            .unwrap_or(false);
        Some(BindLocomotionInfluence {
            walk,
            sprint,
            jump,
            transition,
            hand_only,
        })
    }

    /// `number`: the member as an f32, `fallback` when missing or not a number.
    fn number(obj: &[(String, super::json::Value)], key: &str, fallback: f32) -> f32 {
        member(obj, key)
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .unwrap_or(fallback)
    }

    /// The Java `normalize`: lowercase, then strip `_`, `-` and spaces.
    fn normalize_id(name: &str) -> String {
        name.to_lowercase().replace(['_', '-', ' '], "")
    }

    /// `parse_overlay_binding`: an overlay binding from one JSON object — a clip
    /// name, a mode and an optional bone list.
    fn parse_overlay_binding(value: &super::json::Value) -> Option<OverlayBinding> {
        let obj = value.as_object()?;
        let name = member(obj, "name")
            .or_else(|| member(obj, "clip"))
            .and_then(super::json::Value::as_str)?;
        let mode = OverlayMode::from_name(member(obj, "mode").and_then(super::json::Value::as_str));
        let bones = member(obj, "bones")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        Some(OverlayBinding {
            name: name.to_string(),
            mode,
            bones,
        })
    }

    /// `parse_locomotion_binding`: a locomotion binding from one JSON object.
    fn parse_locomotion_binding(value: &super::json::Value) -> LocomotionBinding {
        let Some(obj) = value.as_object() else {
            return LocomotionBinding {
                clip: String::new(),
                condition: None,
                effect: None,
                overlays: Vec::new(),
                reverse: false,
                add: None,
            };
        };
        LocomotionBinding {
            clip: member(obj, "clip")
                .and_then(super::json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            condition: member(obj, "condition")
                .and_then(super::json::Value::as_str)
                .map(str::to_string),
            effect: member(obj, "effect")
                .and_then(super::json::Value::as_str)
                .map(str::to_string),
            overlays: member(obj, "overlays")
                .and_then(|v| v.as_array())
                .map(|items| items.iter().filter_map(parse_overlay_binding).collect())
                .unwrap_or_default(),
            reverse: member(obj, "reverse")
                .map(|v| matches!(v, super::json::Value::Bool(true)))
                .unwrap_or(false),
            add: member(obj, "add")
                .and_then(super::json::Value::as_str)
                .map(str::to_string),
        }
    }

    /// `normalizeLocomotionInfluence`: a weight over `1.0` reads as a percentage
    /// and comes down by a hundred; everything clamps into `0.0..=1.0`, and a
    /// broken weight reads as `1.0`.
    fn normalize_locomotion_influence(value: f32) -> f32 {
        if !value.is_finite() {
            return 1.0;
        }
        let value = if value > 1.0 { value / 100.0 } else { value };
        value.clamp(0.0, 1.0)
    }

    impl ItemAnimationBindings {
        /// `resolveSequence`: the sequence's clip names, layered.
        pub fn sequences_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Vec<String> {
            let action = normalize_id(action);
            let at = |sequences: Option<&HashMap<String, Vec<String>>>,
                      entries: Option<&HashMap<String, String>>|
             -> Option<Vec<String>> {
                if let Some(map) = sequences
                    && let Some(list) = map.get(&action)
                {
                    return Some(list.clone());
                }
                entries
                    .and_then(|map| map.get(&action))
                    .map(|n| vec![n.clone()])
            };
            match item {
                Some(id) => at(self.specific_sequences.get(id), self.specific_items.get(id))
                    .or_else(|| {
                        group.and_then(|g| at(self.group_sequences.get(g), self.groups.get(g)))
                    })
                    .or_else(|| at(Some(&self.default_sequences), Some(&self.defaults)))
                    .unwrap_or_default(),
                None => at(Some(&self.empty_hand_sequences), Some(&self.empty_hand))
                    .or_else(|| at(Some(&self.default_sequences), Some(&self.defaults)))
                    .unwrap_or_default(),
            }
        }
    }

    /// The numeric setting fields the Java per-action chain reads off a
    /// descriptor.
    pub(crate) const NUMERIC_SETTING_FIELDS: &[&str] = &[
        "transition_seconds",
        "locomotion_retain",
        "start_at_end",
        "moving_legs_influence",
        "legs_locomotion_retain",
        "over_idle",
        "fence_balance_influence",
        "charge_ticks",
        "minimum_charge_ticks",
        "charge_height",
        "smash_distance",
        "additive",
        "disable_locomotion_arm_swing",
        "stride_blocks_per_cycle",
        "sprint_stride_blocks_per_cycle",
        "crouch_stride_blocks_per_cycle",
        "max_cycles_per_second",
        "sprint_max_cycles_per_second",
        "crouch_max_cycles_per_second",
        "minimum_displacement",
        "maximum_displacement",
        "reverse_backward",
        "amplitude_reference_speed",
        "max_amplitude",
        "horizontal",
        "vertical",
        "depth",
        "fov",
        "response",
        "timer_scale",
    ];

    /// The string setting fields the Java chain reads off a descriptor.
    pub(crate) const STRING_SETTING_FIELDS: &[&str] =
        &["behavior", "one_handed", "sprint_overlay", "timer_effect"];

    /// Which bindings layer an ingest pass writes to.
    #[derive(Clone)]
    enum Layer {
        Default,
        EmptyHand,
        Group(String),
        Specific(String),
    }

    /// Which sub-targets one `addEntries` pass fills; unset kinds are skipped,
    /// like the Java nulls in the nine-argument call.
    #[derive(Clone, Copy, Default)]
    struct EntryKinds {
        pose_idles: bool,
        timer_effects: bool,
        timer_scales: bool,
        numeric: bool,
        strings_to_empty_hand: bool,
        variants: bool,
        locomotion: bool,
    }

    /// The bindings' lookup side: every layer follows specific item → group →
    /// default, with the empty-hand layer standing in for a missing item.
    impl ItemAnimationBindings {
        /// `firstFloat`: the layered numeric lookup.
        fn first_f64_layered(
            specific: Option<&HashMap<String, HashMap<String, f64>>>,
            group: Option<&HashMap<String, HashMap<String, f64>>>,
            defaults: &HashMap<String, f64>,
            item: Option<&str>,
            group_key: Option<&str>,
            action: &str,
        ) -> Option<f64> {
            if let (Some(map), Some(id)) = (specific, item)
                && let Some(inner) = map.get(id)
                && let Some(v) = inner.get(action)
            {
                return Some(*v);
            }
            if let (Some(map), Some(g)) = (group, group_key)
                && let Some(inner) = map.get(g)
                && let Some(v) = inner.get(action)
            {
                return Some(*v);
            }
            defaults.get(action).copied()
        }

        /// `resolveSequence`: the layered sequence lookup — specific sequences
        /// and items, then group sequences and groups, then the defaults.
        pub fn resolve_sequence(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Vec<String> {
            let action = normalize_id(action);
            if let Some(id) = item {
                if let Some(found) = sequence_at(
                    self.specific_sequences.get(id),
                    self.specific_items.get(id),
                    &action,
                ) {
                    return found.clone();
                }
                if let Some(group) = group
                    && let Some(found) = sequence_at(
                        self.group_sequences.get(group),
                        self.groups.get(group),
                        &action,
                    )
                {
                    return found.clone();
                }
                return sequence_at(Some(&self.default_sequences), Some(&self.defaults), &action)
                    .unwrap_or_default();
            }
            if let Some(found) = sequence_at(
                Some(&self.empty_hand_sequences),
                Some(&self.empty_hand),
                &action,
            ) {
                return found;
            }
            sequence_at(Some(&self.default_sequences), Some(&self.defaults), &action)
                .unwrap_or_default()
        }

        /// The layered overlay lookup for one item.
        pub fn overlays_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Vec<OverlayBinding> {
            let action = normalize_id(action);
            if let Some(id) = item {
                if let Some(map) = self.specific_overlays.get(id)
                    && let Some(bindings) = map.get(&action)
                {
                    return bindings.clone();
                }
                if let Some(group) = group
                    && let Some(map) = self.group_overlays.get(group)
                    && let Some(bindings) = map.get(&action)
                {
                    return bindings.clone();
                }
                return self
                    .default_overlays
                    .get(&action)
                    .cloned()
                    .unwrap_or_default();
            }
            self.default_overlays
                .get(&action)
                .cloned()
                .unwrap_or_default()
        }

        /// The layered blend lookup.
        pub fn blending_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Option<BlendBinding> {
            let action = normalize_id(action);
            if let Some(id) = item {
                if let Some(map) = self.specific_blends.get(id)
                    && let Some(binding) = map.get(&action)
                {
                    return Some(binding.clone());
                }
                if let Some(group) = group
                    && let Some(map) = self.group_blends.get(group)
                    && let Some(binding) = map.get(&action)
                {
                    return Some(binding.clone());
                }
                return self.default_blends.get(&action).cloned();
            }
            self.empty_hand_blends.get(&action).cloned()
        }

        /// The layered timer lookup.
        pub fn timer_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Option<TimerBinding> {
            let action = normalize_id(action);
            if let Some(id) = item {
                if let Some(map) = self.specific_timers.get(id)
                    && let Some(binding) = map.get(&action)
                {
                    return Some(binding.clone());
                }
                if let Some(group) = group
                    && let Some(map) = self.group_timers.get(group)
                    && let Some(binding) = map.get(&action)
                {
                    return Some(binding.clone());
                }
                return self.default_timers.get(&action).cloned();
            }
            self.empty_hand_timers.get(&action).cloned()
        }

        /// `retriggerProgress`: the layered percentage divided by 100 and
        /// clamped to `[0, 1]`.
        pub fn retrigger_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
            default: f32,
        ) -> f32 {
            let action = normalize_id(action);
            let value = if let Some(id) = item {
                Self::first_f64_layered(
                    Some(&self.specific_retrigger),
                    group.map(|_| &self.group_retrigger),
                    &self.default_retrigger,
                    Some(id),
                    group,
                    &action,
                )
            } else {
                self.empty_hand_retrigger
                    .get(&action)
                    .copied()
                    .or_else(|| self.default_retrigger.get(&action).copied())
            }
            .unwrap_or(f64::from(default));
            (value / 100.0) as f32
        }

        /// The layered influence lookup.
        pub fn influence_for(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            action: &str,
        ) -> Option<BindLocomotionInfluence> {
            let action = normalize_id(action);
            if let Some(id) = item {
                if let Some(map) = self.specific_influences.get(id)
                    && let Some(influence) = map.get(&action)
                {
                    return Some(*influence);
                }
                if let Some(group) = group
                    && let Some(map) = self.group_influences.get(group)
                    && let Some(influence) = map.get(&action)
                {
                    return Some(*influence);
                }
                return self.default_influences.get(&action).copied();
            }
            self.empty_hand_influences
                .get(&action)
                .copied()
                .or_else(|| self.default_influences.get(&action).copied())
        }

        /// The layered numeric setting.
        pub fn numeric_setting(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            key: &str,
            default: f32,
        ) -> f32 {
            let key = normalize_id(key);
            if let Some(id) = item {
                return Self::first_f64_layered(
                    Some(&self.specific_numeric_settings),
                    group.map(|_| &self.group_numeric_settings),
                    &self.numeric_settings,
                    Some(id),
                    group,
                    &key,
                )
                .unwrap_or(f64::from(default)) as f32;
            }
            self.numeric_settings
                .get(&key)
                .copied()
                .unwrap_or(f64::from(default)) as f32
        }

        /// The layered string setting.
        pub fn string_setting(
            &self,
            item: Option<&str>,
            group: Option<&str>,
            key: &str,
            default: &str,
        ) -> String {
            let key = normalize_id(key);
            if let Some(id) = item {
                return Self::first(
                    self.specific_string_settings.get(id),
                    group.and_then(|g| self.group_string_settings.get(g)),
                    &self.string_settings,
                    &key,
                )
                .cloned()
                .unwrap_or_else(|| default.to_string());
            }
            self.empty_hand_string_settings
                .get(&key)
                .cloned()
                .unwrap_or_else(|| default.to_string())
        }

        /// The variant clip names an action carries.
        pub fn variants_for(&self, action: &str) -> Vec<String> {
            self.variants
                .get(&normalize_id(action))
                .cloned()
                .unwrap_or_default()
        }

        /// The locomotion bindings an action carries.
        pub fn locomotion_for(&self, action: &str) -> Vec<LocomotionBinding> {
            self.locomotion
                .get(&normalize_id(action))
                .cloned()
                .unwrap_or_default()
        }

        /// The one-handed clip an action carries.
        pub fn one_handed_clip(&self, action: &str) -> Option<&String> {
            self.default_one_handed.get(&normalize_id(action))
        }

        /// Whether the action has any default binding at all.
        pub fn has_default_type(&self, action: &str) -> bool {
            self.default_animation_types.contains(&normalize_id(action))
        }

        /// The registered variable bindings.
        pub fn variables(&self) -> &[VariableBinding] {
            &self.variables
        }

        /// The emote menu the bindings carry, if any.
        pub fn emote_catalog(&self) -> Option<EmoteCatalog> {
            if self.emote_bindings.is_empty() {
                return None;
            }
            Some(EmoteCatalog {
                menu_key: self.emote_menu_key.clone().unwrap_or_default(),
                entries: self
                    .emote_bindings
                    .iter()
                    .map(|b| Emote {
                        id: b.id.clone(),
                        display_name: b.display_name.clone(),
                        animation: b.animation.clone(),
                        default_key: b.key.clone().unwrap_or_default(),
                        face: b.face.clone(),
                    })
                    .collect(),
            })
        }

        /// The emote menu key, if any.
        pub fn emote_menu_key(&self) -> Option<&str> {
            self.emote_menu_key.as_deref()
        }

        /// The emote bindings.
        pub fn emote_bindings(&self) -> &[EmoteBinding] {
            &self.emote_bindings
        }

        /// Whether the action has a static-when-using override at any layer.
        pub fn static_when_using(&self, item: Option<&str>, group: Option<&str>) -> bool {
            if let Some(id) = item {
                if let Some(v) = self.specific_static_when_using.get(id) {
                    return *v;
                }
                if let Some(group) = group
                    && let Some(v) = self.group_static_when_using.get(group)
                {
                    return *v;
                }
                return self.default_static_when_using.unwrap_or(false);
            }
            self.empty_hand_static_when_using
                .or(self.default_static_when_using)
                .unwrap_or(false)
        }
    }

    /// `sequenceAt`: the sequence list for an action, or the single clip an
    /// entries map carries for it.
    fn sequence_at(
        sequences: Option<&HashMap<String, Vec<String>>>,
        entries: Option<&HashMap<String, String>>,
        action: &str,
    ) -> Option<Vec<String>> {
        if let Some(map) = sequences
            && let Some(list) = map.get(action)
        {
            return Some(list.clone());
        }
        if let Some(map) = entries
            && let Some(clip) = map.get(action)
        {
            return Some(vec![clip.clone()]);
        }
        None
    }

    /// `load_clips_from_str`: parses one animation JSON document (a root object
    /// with an `animations` map of name → clip object) into `clips`, like the
    /// Java `loadClips`. Malformed entries are skipped, like the Java catch.
    pub fn load_clips_from_str(
        text: &str,
        clips: &mut HashMap<String, Clip>,
    ) -> Result<(), String> {
        let root = super::json::parse(text)?;
        let Some(members) = root
            .get("animations")
            .and_then(super::json::Value::as_object)
        else {
            return Ok(());
        };
        for (name, value) in members {
            if let super::json::Value::Object(_) = value {
                clips.insert(name.clone(), read_clip(name, value));
            }
        }
        Ok(())
    }

    /// Builds an animation library from one animation JSON document and one
    /// bindings JSON document.
    pub fn load_library_from_str(
        clips_text: &str,
        bindings_json: Option<&super::json::Value>,
    ) -> Result<AnimationLibrary, String> {
        let mut clips = std::collections::HashMap::new();
        load_clips_from_str(clips_text, &mut clips)?;
        let mut bindings = ItemAnimationBindings::new();
        if let Some(bindings_json) = bindings_json {
            bindings.add(bindings_json);
        }
        Ok(AnimationLibrary::new(clips, bindings))
    }

    /// Normalizes a legacy compat pack into format 2: the Java `CompatFormat`.
    /// `Err` carries the validation message (the Java `IllegalArgumentException`).
    pub mod compat_format {
        /// The sequence phases the format 2 accepts (`PHASES`).
        pub const PHASES: [&str; 11] = [
            "equip", "idle", "charge", "hold", "release", "slam", "attack", "use", "cast",
            "fishing", "reel",
        ];
        /// The sequence behaviors the format 2 accepts (`BEHAVIORS`).
        pub const BEHAVIORS: [&str; 8] = [
            "mace",
            "bow",
            "trident",
            "flint",
            "shears",
            "fishing",
            "bucket",
            "throwable",
        ];
        /// The per-phase settings the format 2 accepts (`PHASE_SETTINGS`).
        pub const PHASE_SETTINGS: [&str; 8] = [
            "transition_seconds",
            "locomotion_retain",
            "locomotion_influence",
            "retrigger_progress",
            "moving_legs_influence",
            "legs_locomotion_retain",
            "base_pose_retain",
            "over_idle",
        ];

        /// Normalizes a legacy compat pack into format 2. `Err` carries the
        /// validation message (the Java `IllegalArgumentException`).
        pub fn normalize(
            root: &super::super::json::Value,
        ) -> Result<super::super::json::Value, String> {
            if !root.is_object() {
                return Err("compat: expected an object".to_string());
            }
            let Some(format_version) = root.get("format_version") else {
                for key in [
                    "defaults",
                    "item_groups",
                    "items",
                    "empty_hand",
                    "cameras",
                    "emotes",
                ] {
                    if root.has(key) {
                        return Err(error(key, "o formato novo precisa de format_version: 2"));
                    }
                }
                return Ok(deep_copy(root));
            };
            let version = format_version.as_str().unwrap_or("");
            if version != "2" {
                return Err(error(
                    "format_version",
                    "versão suportada: 2 (ou omita para packs antigos)",
                ));
            }
            keys(
                root,
                &[
                    "$schema",
                    "format_version",
                    "defaults",
                    "item_groups",
                    "items",
                    "empty_hand",
                    "variables",
                    "cameras",
                    "emotes",
                ],
                "compat",
            )?;
            let mut out_members: Vec<(String, super::super::json::Value)> = Vec::new();
            if let Some(variables) = root.get("variables") {
                out_members.push(("variables".to_string(), deep_copy(variables)));
            }
            // defaults → default_animations.
            let defaults = object_or_empty(root, "defaults", "defaults")?;
            keys(
                defaults,
                &["animations", "static_when_using", "better_combat"],
                "defaults",
            )?;
            let mut defaults_out = super::super::json::Value::Object(Vec::new());
            copy_better_combat(defaults, &mut defaults_out, "defaults.better_combat")?;
            if let Some(static_when_using) = defaults.get("static_when_using")
                && let super::super::json::Value::Object(out) = &mut defaults_out
            {
                out.push((
                    "static_when_using".to_string(),
                    deep_copy(static_when_using),
                ));
            }
            let mut default_animations: Vec<super::super::json::Value> = Vec::new();
            // defaults.animations → the default entries.
            let defaults_animations = animations(
                defaults.get("animations"),
                &super::super::json::Value::Object(Vec::new()),
                false,
                "defaults.animations",
            )?;
            if let super::super::json::Value::Array(items) = defaults_animations {
                default_animations.extend(items);
            }
            // cameras → camera entries appended to the default animations.
            let cameras = object_or_empty(root, "cameras", "cameras")?;
            keys(cameras, &["proximity", "aiming"], "cameras")?;
            let Some(camera_members) = cameras.as_object() else {
                return Ok(super::super::json::Value::Object(Vec::new()));
            };
            for (camera_name, camera_elem) in camera_members {
                let label = format!("cameras.{}", camera_name);
                let mut camera = deep_copy(object(Some(camera_elem), &label)?);
                let camera_type = if camera_name == "aiming" {
                    "bow_and_crossbow_camera_fix"
                } else {
                    "proximity_camera"
                };
                set_string(&mut camera, "type", camera_type);
                default_animations.push(camera);
            }
            if let Some(emotes_elem) = root.get("emotes") {
                let label = "emotes";
                let mut emotes = deep_copy(object(Some(emotes_elem), label)?);
                set_string(&mut emotes, "type", "emotes");
                default_animations.push(emotes);
            }
            // empty_hand → the emptyHand layer.
            if let Some(empty_hand) = root.get("empty_hand") {
                let empty_item = item(empty_hand, "empty_hand", false)?;
                if let super::super::json::Value::Object(out) = &mut defaults_out {
                    // The empty-hand layer rides beside the defaults.
                    out.push(("emptyHand".to_string(), empty_item));
                }
            }
            // item_groups and items → per-group and per-item layers.
            for (section, out_key) in [("item_groups", "item"), ("items", "itemSpecific")] {
                let mut items_out: Vec<(String, super::super::json::Value)> = Vec::new();
                let section_obj = object_or_empty(root, section, section)?;
                let Some(section_members) = section_obj.as_object() else {
                    continue;
                };
                for (entry_name, entry_elem) in section_members {
                    let label = format!("{}.{}", section, entry_name);
                    let item = item(entry_elem, &label, true)?;
                    items_out.push((entry_name.clone(), item));
                }
                out_members.push((
                    out_key.to_string(),
                    super::super::json::Value::Object(items_out),
                ));
            }
            if let super::super::json::Value::Object(out) = &mut defaults_out {
                out.push((
                    "default_animations".to_string(),
                    super::super::json::Value::Array(default_animations),
                ));
            }
            let mut out = defaults_out;
            if let super::super::json::Value::Object(members) = &mut out {
                let ordered: Vec<(String, super::super::json::Value)> = std::mem::take(members);
                let mut reordered: Vec<(String, super::super::json::Value)> = Vec::new();
                for key in [
                    "variables",
                    "static_when_using",
                    "default_animations",
                    "emptyHand",
                ] {
                    if let Some(pos) = ordered.iter().position(|(k, _)| k == key) {
                        reordered.push(ordered[pos].clone());
                    }
                }
                for entry in ordered {
                    if !reordered.iter().any(|(k, _)| k == &entry.0) {
                        reordered.push(entry);
                    }
                }
                *members = reordered;
            }
            for (key, value) in out_members {
                if let super::super::json::Value::Object(members) = &mut out {
                    members.push((key, value));
                }
            }
            legacy_clip_names(&mut out);
            Ok(out)
        }

        /// `legacyClipNames`: every `clip` key becomes `name`, recursively.
        fn legacy_clip_names(elem: &mut super::super::json::Value) {
            match elem {
                super::super::json::Value::Array(items) => {
                    for item in items {
                        legacy_clip_names(item);
                    }
                }
                super::super::json::Value::Object(members) => {
                    if let Some(pos) = members.iter().position(|(k, _)| k == "clip") {
                        let (_, value) = members.remove(pos);
                        members.insert(0, ("name".to_string(), value));
                    }
                    for (_, value) in members.iter_mut() {
                        legacy_clip_names(value);
                    }
                }
                _ => {}
            }
        }

        /// `item`: normalizes one item/group/empty-hand entry into the
        /// `customAnimation` shape the bindings ingest.
        fn item(
            elem: &super::super::json::Value,
            label: &str,
            is_item: bool,
        ) -> Result<super::super::json::Value, String> {
            let source = object(Some(elem), label)?;
            keys(
                source,
                &[
                    "static_when_using",
                    "camera",
                    "sequence",
                    "animations",
                    "better_combat",
                ],
                label,
            )?;
            let mut out = super::super::json::Value::Object(Vec::new());
            copy_better_combat(source, &mut out, &format!("{}.better_combat", label))?;
            for key in ["static_when_using", "camera"] {
                if let Some(v) = source.get(key)
                    && let super::super::json::Value::Object(out_obj) = &mut out
                {
                    out_obj.push((key.to_string(), deep_copy(v)));
                }
            }
            let sequence = object_or_empty(source, "sequence", &format!("{}.sequence", label))?;
            let has_sequence = source.has("sequence");
            if has_sequence && !is_item {
                return Err(error(label, "sequence só pode ser usado em itens"));
            }
            keys(
                sequence,
                &[
                    "behavior",
                    "defaults",
                    "charge_ticks",
                    "minimum_charge_ticks",
                    "charge_height",
                    "smash_distance",
                    "legs_locomotion_retain",
                ],
                &format!("{}.sequence", label),
            )?;
            let sequence_defaults = object_or_empty(
                sequence,
                "defaults",
                &format!("{}.sequence.defaults", label),
            )?;
            keys(
                sequence_defaults,
                &PHASE_SETTINGS,
                &format!("{}.sequence.defaults", label),
            )?;
            let mut custom: Vec<super::super::json::Value> = Vec::new();
            if has_sequence {
                let behavior = string(
                    sequence.get("behavior"),
                    &format!("{}.sequence.behavior", label),
                )?;
                if !BEHAVIORS.contains(&behavior.as_str()) {
                    return Err(error(
                        &format!("{}.sequence.behavior", label),
                        &format!("comportamento desconhecido: {}", behavior),
                    ));
                }
                let mut merged = merge(sequence_defaults, sequence);
                if let super::super::json::Value::Object(m) = &mut merged {
                    m.retain(|(k, _)| k != "defaults");
                }
                if let Some(retain) = sequence_defaults.get("base_pose_retain") {
                    // Java validates the retention here, at normalize time.
                    super::super::signals::BasePoseRetention::parse(
                        Some(retain),
                        &format!("{}.sequence.defaults.base_pose_retain", label),
                    )?;
                }
                set_string(&mut merged, "type", "item_sequence");
                custom.push(merged);
            }
            let source_animations = source.get("animations");
            let parsed = animations(
                source_animations,
                sequence_defaults,
                has_sequence,
                &format!("{}.animations", label),
            )?;
            if let super::super::json::Value::Array(items) = &parsed {
                custom.extend(items.iter().cloned());
            }
            if let super::super::json::Value::Object(out_obj) = &mut out {
                out_obj.push((
                    "customAnimation".to_string(),
                    super::super::json::Value::Array(custom),
                ));
            }
            Ok(out)
        }

        /// `animations`: normalizes one action-to-clip map into an array of clip
        /// descriptors.
        fn animations(
            elem: Option<&super::super::json::Value>,
            parent: &super::super::json::Value,
            is_phase: bool,
            label: &str,
        ) -> Result<super::super::json::Value, String> {
            let mut out: Vec<super::super::json::Value> = Vec::new();
            let Some(elem) = elem else {
                return Ok(super::super::json::Value::Array(out));
            };
            let source = object(Some(elem), label)?;
            let Some(source_members) = source.as_object() else {
                return Ok(super::super::json::Value::Array(out));
            };
            for (action_name, value) in source_members {
                let mut action = action_name.clone();
                if action == "useItem" || action == "useitem" {
                    action = "use_item".to_string();
                }
                let full_label = format!("{}.{}", label, action);
                if action
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase())
                    && action
                        .chars()
                        .skip(1)
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                    && !action.starts_with("sequence_")
                    && action != "item_sequence"
                {
                    // A valid action name.
                } else {
                    return Err(error(
                        &full_label,
                        "use o nome da ação, sem sequence_ ou type",
                    ));
                }
                if is_phase && (action.starts_with("attack_") || action.ends_with("_one_handed")) {
                    return Err(error(
                        &full_label,
                        "mantenha variants e one_handed dentro da ação",
                    ));
                }
                let is_phase_name = is_phase && PHASES.contains(&action.as_str());
                let final_name = if is_phase_name {
                    format!("sequence_{}", action)
                } else {
                    action.clone()
                };
                if value.is_array() {
                    if !["biome", "passenger_state", "sitting_state"].contains(&action.as_str()) {
                        return Err(error(
                            &full_label,
                            "use um objeto com variants para variações",
                        ));
                    }
                    let Some(items) = value.as_array() else {
                        continue;
                    };
                    for item in items {
                        emit(
                            &mut out,
                            &final_name,
                            item,
                            &super::super::json::Value::Object(Vec::new()),
                            false,
                            &full_label,
                        )?;
                    }
                } else {
                    let parent_for_merge = if is_phase_name {
                        parent
                    } else {
                        &super::super::json::Value::Object(Vec::new())
                    };
                    emit(
                        &mut out,
                        &final_name,
                        value,
                        parent_for_merge,
                        is_phase_name,
                        &full_label,
                    )?;
                }
            }
            Ok(super::super::json::Value::Array(out))
        }

        /// `emit`: normalizes one animation descriptor into the format 2 shape.
        fn emit(
            out: &mut Vec<super::super::json::Value>,
            name: &str,
            elem: &super::super::json::Value,
            parent: &super::super::json::Value,
            is_phase: bool,
            label: &str,
        ) -> Result<(), String> {
            let mut desc = descriptor(elem, label)?;
            if name == "turn_in_place" {
                if desc.has("left_overlay")
                    && !desc.has("left_add")
                    && let Some(v) = desc.get("left_overlay").cloned()
                {
                    set_member(&mut desc, "left_add", v);
                }
                if desc.has("right_overlay")
                    && !desc.has("right_add")
                    && let Some(v) = desc.get("right_overlay").cloned()
                {
                    set_member(&mut desc, "right_add", v);
                }
                desc.remove_member("left_overlay");
                desc.remove_member("right_overlay");
                keys(
                    &desc,
                    &[
                        "enabled",
                        "mode",
                        "clip",
                        "reference_degrees",
                        "double_turn_degrees",
                        "minimum_turn_degrees",
                        "turn_delay_seconds",
                        "look_idle_seconds",
                        "fade_out_seconds",
                        "left",
                        "right",
                        "left_add",
                        "right_add",
                    ],
                    label,
                )?;
                if let Some(mode) = desc.get("mode") {
                    let valid = matches!(mode, super::super::json::Value::String(s)
                        if matches!(s.as_str(), "scaled" | "steps" | "camera" | "dynamic" | "procedural"));
                    if !valid {
                        return Err(error(label, "use dynamic, camera, scaled ou steps"));
                    }
                }
                let scaled = desc
                    .get("mode")
                    .and_then(super::super::json::Value::as_str)
                    .map(|s| s == "scaled")
                    .unwrap_or(false);
                if scaled
                    && (desc.has("left")
                        || desc.has("right")
                        || desc.has("left_add")
                        || desc.has("right_add"))
                {
                    return Err(error(
                        label,
                        "scaled usa um clip; left e right pertencem ao modo steps",
                    ));
                }
                if !scaled && desc.has("clip") {
                    return Err(error(label, "declare mode: scaled para usar um único clip"));
                }
                if let Some(enabled) = desc.get("enabled")
                    && !matches!(enabled, super::super::json::Value::Bool(_))
                {
                    return Err(error(label, "use true/false"));
                }
                if desc.has("left") != desc.has("right") {
                    return Err(error(label, "declare left e right juntos"));
                }
                for key in ["left", "right", "left_add", "right_add", "clip"] {
                    if let Some(v) = desc.get(key) {
                        let blank = match v {
                            super::super::json::Value::String(s) => s.trim().is_empty(),
                            _ => false,
                        };
                        if blank {
                            return Err(error(
                                &format!("{}.{}", label, key),
                                "use o nome de um clip",
                            ));
                        }
                    }
                }
                if let Some(degrees) = desc.get("reference_degrees") {
                    let valid = matches!(degrees, super::super::json::Value::Number(n)
                        if n.is_finite() && *n as f32 > 0.0);
                    if !valid {
                        return Err(error(label, "use um número positivo"));
                    }
                }
                if let Some(degrees) = desc.get("double_turn_degrees") {
                    let valid = matches!(degrees, super::super::json::Value::Number(n)
                        if n.is_finite() && (*n as f32) >= 30.0 && (*n as f32) <= 180.0);
                    if !valid {
                        return Err(error(label, "use graus entre 30 e 180"));
                    }
                }
                if let Some(seconds) = desc.get("turn_delay_seconds") {
                    let valid = matches!(seconds, super::super::json::Value::Number(n)
                        if n.is_finite() && (*n as f32) >= 0.0 && (*n as f32) <= 0.3);
                    if !valid {
                        return Err(error(label, "use segundos entre 0 e 0.3"));
                    }
                }
            }
            if is_phase {
                let mut allowed: Vec<&str> = PHASE_SETTINGS.to_vec();
                allowed.extend([
                    "clip",
                    "variants",
                    "one_handed",
                    "hold_last_frame",
                    "overlays",
                    "one_handed_overlays",
                ]);
                keys(&desc, &allowed, label)?;
            }
            if desc.has("type") || desc.has("name") || desc.has("start_at_end") {
                return Err(error(
                    label,
                    "use a chave da ação, clip e hold_last_frame no formato 2",
                ));
            }
            if let Some(members) = desc.as_object() {
                for (key, _) in members {
                    if key.starts_with("var_")
                        && key[4..].chars().all(|c| c.is_ascii_digit())
                        && key.len() > 4
                    {
                        return Err(error(label, "use variants em vez de var_N"));
                    }
                }
            }
            let mut merged = merge(parent, &desc);
            if let Some(retain) = merged.get("base_pose_retain") {
                let allowed = ["attack", "mining", "pose", "use_item"]
                    .iter()
                    .any(|k| name == *k || name.starts_with("sequence_"));
                if !is_phase && !allowed {
                    return Err(error(
                        label,
                        "disponível em attack, mining, pose, use_item e fases de sequence",
                    ));
                }
                let _ = retain; // BasePoseRetention.parse runs in the game layer.
            }
            set_string(&mut merged, "type", name);
            if merged.has("clip") {
                if let Some(v) = merged.get("clip").cloned() {
                    set_member(&mut merged, "name", v);
                }
                merged.remove_member("clip");
            }
            if let Some(hold) = merged.get("hold_last_frame").cloned() {
                merged.remove_member("hold_last_frame");
                if !is_phase || !matches!(hold, super::super::json::Value::Bool(_)) {
                    return Err(error(label, "use true/false em uma fase de sequence"));
                }
                let start = matches!(hold, super::super::json::Value::Bool(true));
                set_number(&mut merged, "start_at_end", if start { 1.0 } else { 0.0 });
            }
            let variants = merged.get("variants").cloned();
            if let Some(variants) = variants {
                merged.remove_member("variants");
                if merged.has("name") {
                    return Err(error(label, "escolha clip ou variants"));
                }
                let Some(items) = variants.as_array().filter(|a| !a.is_empty()) else {
                    return Err(error(label, "informe uma lista não vazia"));
                };
                if name == "greeting" || name == "idle_events" {
                    set_member(&mut merged, "variants", variants);
                } else {
                    for (index, item) in items.iter().enumerate() {
                        let index = index + 1;
                        let variant_label = format!("{}.variants[{}]", label, index - 1);
                        let desc = descriptor(item, &variant_label)?;
                        let clip = string(desc.get("clip"), &variant_label)?;
                        set_string(&mut merged, &format!("var_{}", index), &clip);
                        if name == "sequence_attack" {
                            return Err(error(
                                label,
                                "variants de sequência pertence à ação attack",
                            ));
                        }
                        let mut copy = deep_copy(&desc);
                        copy.remove_member("variants");
                        let named = format!("{}_{}", name, index);
                        let merged_variant = merge(&copy, &desc);
                        emit_entry(&mut *out, &named, merged_variant)?;
                    }
                }
            } else if name == "sequence_attack" && merged.has("name") {
                let named = format!("{}_1", name);
                let mut copy = deep_copy(&desc);
                copy.remove_member("variants");
                let mut merged_variant = merge(&copy, &desc);
                if let Some(v) = merged.get("name").cloned() {
                    set_member(&mut merged_variant, "var_1", v);
                }
                emit_entry(out, &named, merged_variant)?;
            }
            if name == "climbing" {
                if merged.has("up") {
                    if !merged.has("name")
                        && let Some(v) = merged.get("up").cloned()
                    {
                        set_member(&mut merged, "name", v);
                    }
                    let mut copy = deep_copy(&merged);
                    copy.remove_member("up");
                    let named = "climb_up".to_string();
                    emit_entry(out, &named, copy)?;
                }
                if merged.has("down") {
                    let mut copy = deep_copy(&merged);
                    copy.remove_member("down");
                    emit_entry(out, "climb_down", copy)?;
                }
            }
            let one_handed = merged.get("one_handed");
            if let Some(one_handed) = one_handed {
                if one_handed.as_object().is_some() {
                    if !is_phase {
                        return Err(error(
                            label,
                            "fora de sequence, informe apenas o nome do clip",
                        ));
                    }
                    let desc = descriptor(one_handed, &format!("{}.one_handed", label))?;
                    let clip = string(desc.get("clip"), &format!("{}.one_handed.clip", label))?;
                    set_string(&mut merged, "one_handed", &clip);
                    let mut copy = deep_copy(&merged);
                    copy.remove_member("one_handed");
                    copy.remove_member("variants");
                    let named = format!("{}_one_handed", name);
                    let merged_one_handed = merge(&copy, &desc);
                    emit_entry(out, &named, merged_one_handed)?;
                } else if is_phase && !one_handed.is_null() {
                    let clip = string(Some(one_handed), &format!("{}.one_handed", label))?;
                    let mut copy = deep_copy(&merged);
                    copy.remove_member("one_handed");
                    copy.remove_member("variants");
                    set_string(&mut copy, "clip", &clip);
                    let named = format!("{}_one_handed", name);
                    emit_entry(out, &named, copy)?;
                }
            }
            for key in ["overlays", "one_handed_overlays"] {
                if let Some(overlays) = merged.get(key) {
                    let Some(items) = overlays.as_array() else {
                        return Err(error(&format!("{}.{}", label, key), "esperada uma lista"));
                    };
                    for item in items {
                        let overlay_label = format!("{}.{}", label, key);
                        let mut obj = deep_copy(object(Some(item), &overlay_label)?);
                        if obj.has("clip") {
                            if let Some(v) = obj.get("clip").cloned() {
                                set_member(&mut obj, "name", v);
                            }
                            obj.remove_member("clip");
                        }
                    }
                }
            }
            out.push(merged);
            Ok(())
        }

        /// `descriptor`: a bare string becomes `{clip: s}`, an object is copied.
        fn descriptor(
            elem: &super::super::json::Value,
            label: &str,
        ) -> Result<super::super::json::Value, String> {
            if let super::super::json::Value::String(s) = elem
                && !s.trim().is_empty()
            {
                let mut obj = super::super::json::Value::Object(Vec::new());
                set_string(&mut obj, "clip", s);
                return Ok(obj);
            }
            Ok(deep_copy(object(Some(elem), label)?))
        }

        /// `copyBetterCombat`: the better_combat block is copied verbatim (its
        /// validation runs in the game layer).
        fn copy_better_combat(
            source: &super::super::json::Value,
            out: &mut super::super::json::Value,
            _label: &str,
        ) -> Result<(), String> {
            if let Some(v) = source.get("better_combat")
                && let super::super::json::Value::Object(out_obj) = out
            {
                out_obj.push(("better_combat".to_string(), deep_copy(v)));
            }
            Ok(())
        }

        /// `merge`: a recursive object merge; `base_pose_retain` merges
        /// recursively, everything else overwrites.
        fn merge(
            base: &super::super::json::Value,
            overlay: &super::super::json::Value,
        ) -> super::super::json::Value {
            let Some(base_obj) = base.as_object() else {
                return deep_copy(overlay);
            };
            let mut out: Vec<(String, super::super::json::Value)> = base_obj.to_vec();
            if let Some(overlay_obj) = overlay.as_object() {
                for (key, value) in overlay_obj {
                    if key == "base_pose_retain" {
                        let existing = out.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
                        if let (
                            Some(super::super::json::Value::Object(_)),
                            Some(super::super::json::Value::Object(_)),
                        ) = (existing.as_ref(), Some(value))
                        {
                            let base_member = existing.as_ref().unwrap();
                            let merged = merge(base_member, value);
                            if let Some(pos) = out.iter().position(|(k, _)| k == key) {
                                out[pos] = (key.clone(), merged);
                            }
                            continue;
                        }
                    }
                    if let Some(pos) = out.iter().position(|(k, _)| k == key) {
                        out[pos] = (key.clone(), deep_copy(value));
                    } else {
                        out.push((key.clone(), deep_copy(value)));
                    }
                }
            }
            super::super::json::Value::Object(out)
        }

        /// `objectOrEmpty`: the member as an object, or an empty object.
        fn object_or_empty<'a>(
            root: &'a super::super::json::Value,
            key: &str,
            label: &str,
        ) -> Result<&'a super::super::json::Value, String> {
            if let Some(v) = root.get(key) {
                object(Some(v), label)?;
                return Ok(v);
            }
            static EMPTY_OBJECT: super::super::json::Value =
                super::super::json::Value::Object(Vec::new());
            Ok(&EMPTY_OBJECT)
        }

        /// `object`: the element as an object, or a validation error.
        fn object<'a>(
            elem: Option<&'a super::super::json::Value>,
            label: &str,
        ) -> Result<&'a super::super::json::Value, String> {
            match elem {
                Some(v @ super::super::json::Value::Object(_)) => Ok(v),
                _ => Err(error(label, "esperado um objeto")),
            }
        }

        /// `string`: a non-empty string, or a validation error.
        fn string(elem: Option<&super::super::json::Value>, label: &str) -> Result<String, String> {
            match elem {
                Some(super::super::json::Value::String(s)) if !s.trim().is_empty() => Ok(s.clone()),
                _ => Err(error(label, "esperado um nome não vazio")),
            }
        }

        /// `keys`: every object key must be in the allowed set.
        fn keys(
            obj: &super::super::json::Value,
            allowed: &[&str],
            label: &str,
        ) -> Result<(), String> {
            let Some(members) = obj.as_object() else {
                return Ok(());
            };
            for (key, _) in members {
                if !allowed.contains(&key.as_str()) {
                    return Err(error(&format!("{}.{}", label, key), "campo desconhecido"));
                }
            }
            Ok(())
        }

        /// `error`: the Java `IllegalArgumentException` message shape.
        fn error(label: &str, message: &str) -> String {
            format!("Compat {}: {}", label, message)
        }

        /// A deep copy of a JSON value.
        fn deep_copy(value: &super::super::json::Value) -> super::super::json::Value {
            value.clone()
        }

        fn set_string(value: &mut super::super::json::Value, key: &str, s: &str) {
            if let super::super::json::Value::Object(members) = value {
                if let Some(pos) = members.iter().position(|(k, _)| k == key) {
                    members[pos] = (
                        key.to_string(),
                        super::super::json::Value::String(s.to_string()),
                    );
                } else {
                    members.push((
                        key.to_string(),
                        super::super::json::Value::String(s.to_string()),
                    ));
                }
            }
        }

        fn set_number(value: &mut super::super::json::Value, key: &str, n: f64) {
            if let super::super::json::Value::Object(members) = value {
                if let Some(pos) = members.iter().position(|(k, _)| k == key) {
                    members[pos] = (key.to_string(), super::super::json::Value::Number(n));
                } else {
                    members.push((key.to_string(), super::super::json::Value::Number(n)));
                }
            }
        }

        fn set_member(
            value: &mut super::super::json::Value,
            key: &str,
            member: super::super::json::Value,
        ) {
            if let super::super::json::Value::Object(members) = value {
                if let Some(pos) = members.iter().position(|(k, _)| k == key) {
                    members[pos] = (key.to_string(), member);
                } else {
                    members.push((key.to_string(), member));
                }
            }
        }

        /// Emits one finished descriptor into the output array, applying the
        /// same tail `emit` applies: `type` = the name, `clip` becomes `name`.
        fn emit_entry(
            out: &mut Vec<super::super::json::Value>,
            name: &str,
            mut merged: super::super::json::Value,
        ) -> Result<(), String> {
            set_string(&mut merged, "type", name);
            if merged.has("clip") {
                if let Some(v) = merged.get("clip").cloned() {
                    set_member(&mut merged, "name", v);
                }
                merged.remove_member("clip");
            }
            out.push(merged);
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::easing::Easing;
        use super::*;

        fn v(x: f32, y: f32, z: f32) -> Vec3 {
            Vec3::new(x, y, z)
        }

        fn channel_from(keys: Vec<(f32, Vec3)>) -> Channel {
            Channel::new(
                keys.into_iter()
                    .map(|(t, v)| {
                        (
                            t,
                            Key {
                                pre: v,
                                post: v,
                                interpolation: "linear".into(),
                                easing: None,
                            },
                        )
                    })
                    .collect(),
                Vec3::ZERO,
            )
        }

        #[test]
        fn vec3_lerp_and_componentwise_mul() {
            assert_eq!(
                v(1.0, 2.0, 3.0).lerp(v(5.0, 6.0, 7.0), 0.25),
                v(2.0, 3.0, 4.0)
            );
            assert_eq!(v(1.0, 2.0, 3.0) * v(2.0, 3.0, 4.0), v(2.0, 6.0, 12.0));
            assert_eq!(v(1.0, 2.0, 3.0) + v(1.0, 1.0, 1.0), v(2.0, 3.0, 4.0));
            assert_eq!(v(3.0, 2.0, 1.0) - v(1.0, 1.0, 1.0), v(2.0, 1.0, 0.0));
        }

        /// The quaternion math matches JOML 1.10.8 exactly (values probed from
        /// joml-1.10.8 on the JVM).
        #[test]
        fn quaternion_math_matches_joml() {
            let q = Quat::from_euler_zyx(60.0, 45.0, 30.0);
            assert!((q.x - 0.02226001).abs() < 1.0e-5, "x {}", q.x);
            assert!((q.y - 0.43967977).abs() < 1.0e-5, "y {}", q.y);
            assert!((q.z - 0.3604234).abs() < 1.0e-5, "z {}", q.z);
            assert!((q.w - 0.82236314).abs() < 1.0e-5, "w {}", q.w);
            // A pure X rotation round-trips through getEulerAnglesZYX.
            let q = Quat::from_euler_zyx(0.0, 0.0, 90.0);
            let e = q.get_euler_zyx();
            assert!((e.x - core::f32::consts::FRAC_PI_2).abs() < 1.0e-5);
            // qz(90) * qx(90) sends (1,0,0) to (0,1,0): `other` applies first.
            let qz = Quat::from_euler_zyx(90.0, 0.0, 0.0);
            let qx = Quat::from_euler_zyx(0.0, 0.0, 90.0);
            let combined = qz * qx;
            let out = combined.transform_by(v(1.0, 0.0, 0.0));
            assert!((out.x - 0.0).abs() < 1.0e-5);
            assert!((out.y - 1.0).abs() < 1.0e-5);
            assert!((out.z - 0.0).abs() < 1.0e-5);
            // JOML's transform normalizes by the quaternion's squared length.
            let q = Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 2.0,
            };
            let out = q.transform_by(v(3.0, 4.0, 5.0));
            assert!((out.x - 3.0).abs() < 1.0e-5, "unnormalized: {}", out.x);
        }

        /// Linear interpolation between two keys, with the ceiling key's easing
        /// applied to the alpha.
        #[test]
        fn channel_samples_linear_and_eased() {
            let keys = vec![
                (
                    0.0,
                    Key {
                        pre: v(0.0, 0.0, 0.0),
                        post: v(0.0, 0.0, 0.0),
                        interpolation: "linear".into(),
                        easing: None,
                    },
                ),
                (
                    1.0,
                    Key {
                        pre: v(10.0, 0.0, 0.0),
                        post: v(10.0, 0.0, 0.0),
                        interpolation: "linear".into(),
                        easing: None,
                    },
                ),
            ];
            let channel = Channel::new(keys, Vec3::ZERO);
            assert_eq!(channel.sample(0.5), v(5.0, 0.0, 0.0));
            // Before the first key: its pre value; after the last: its post.
            assert_eq!(channel.sample(-1.0), v(0.0, 0.0, 0.0));
            assert_eq!(channel.sample(2.0), v(10.0, 0.0, 0.0));
            // The ceiling key's easing shapes the alpha.
            let keys = vec![
                (
                    0.0,
                    Key {
                        pre: v(0.0, 0.0, 0.0),
                        post: v(0.0, 0.0, 0.0),
                        interpolation: "linear".into(),
                        easing: None,
                    },
                ),
                (
                    1.0,
                    Key {
                        pre: v(10.0, 0.0, 0.0),
                        post: v(10.0, 0.0, 0.0),
                        interpolation: "linear".into(),
                        easing: Some(Easing::EaseInQuad),
                    },
                ),
            ];
            let channel = Channel::new(keys, Vec3::ZERO);
            assert_eq!(channel.sample(0.5), v(2.5, 0.0, 0.0));
        }

        /// Catmull-Rom through the midpoints of a straight line stays straight.
        #[test]
        fn catmullrom_through_a_straight_line_stays_straight() {
            let keys: Vec<(f32, Key)> = [(0.0, 0.0f32), (1.0, 10.0), (2.0, 20.0), (3.0, 30.0)]
                .into_iter()
                .map(|(t, x)| {
                    (
                        t,
                        Key {
                            pre: v(x, 0.0, 0.0),
                            post: v(x, 0.0, 0.0),
                            interpolation: "catmullrom".into(),
                            easing: None,
                        },
                    )
                })
                .collect();
            let channel = Channel::new(keys, Vec3::ZERO);
            let mid = channel.sample(1.5);
            assert!((mid.x - 15.0).abs() < 1.0e-4, "mid {}", mid.x);
            assert!(mid.y.abs() < 1.0e-4 && mid.z.abs() < 1.0e-4);
        }

        #[test]
        fn channel_endpoints_and_last_key_time() {
            let channel = channel_from(vec![(0.0, v(1.0, 0.0, 0.0)), (2.0, v(3.0, 0.0, 0.0))]);
            assert_eq!(channel.sample(-1.0), v(1.0, 0.0, 0.0));
            assert_eq!(channel.sample(3.0), v(3.0, 0.0, 0.0));
            assert_eq!(channel.last_key_time(), 2.0);
            let empty = Channel::new(Vec::new(), v(4.0, 5.0, 6.0));
            assert_eq!(empty.sample(1.0), v(4.0, 5.0, 6.0));
            assert_eq!(empty.last_key_time(), 0.0);
        }

        #[test]
        fn clip_samples_and_loops() {
            let mut bones = std::collections::HashMap::new();
            bones.insert(
                "body".to_string(),
                BoneTrack {
                    position: channel_from(vec![(0.0, v(0.0, 0.0, 0.0)), (1.0, v(10.0, 0.0, 0.0))]),
                    rotation: Channel::new(Vec::new(), Vec3::ZERO),
                    scale: Channel::new(Vec::new(), Vec3::ONE),
                },
            );
            let looping = Clip::new("walk".into(), 1.0, true, bones.clone());
            let pose = looping.sample("BODY", 1.5);
            assert!(
                (pose.position.x - 5.0).abs() < 1.0e-5,
                "{}",
                pose.position.x
            );
            // A non-looping clip clamps at its length.
            let once = Clip::new("hit".into(), 1.0, false, bones);
            let pose = once.sample("body", 5.0);
            assert!((pose.position.x - 10.0).abs() < 1.0e-5);
            // An unknown bone samples to identity.
            let pose = once.sample("nope", 0.5);
            assert_eq!(pose, Pose::identity());
        }

        #[test]
        fn positive_modulo_wraps_negatives() {
            assert_eq!(positive_modulo(1.5, 1.0), 0.5);
            assert_eq!(positive_modulo(-0.5, 1.0), 0.5);
            assert_eq!(positive_modulo(2.5, 1.0), 0.5);
        }

        #[test]
        fn parse_channel_reads_all_key_shapes() {
            // A bare array is one symmetric key at time 0.
            let channel = parse_channel(
                Some(&super::super::json::parse("[1,2,3]").unwrap()),
                Vec3::ZERO,
            );
            assert_eq!(channel.sample(0.0), v(1.0, 2.0, 3.0));
            assert_eq!(channel.last_key_time(), 0.0);
            // An object maps time strings to keys; a bare array value is a
            // symmetric key, an object carries pre/post/easing.
            let obj = super::super::json::parse(
                r#"{"0.0": [0,0,0], "1.0": {"pre": [10,0,0], "post": [10,0,0], "easing": "easeInQuad"}}"#,
            )
            .unwrap();
            let channel = parse_channel(Some(&obj), Vec3::ZERO);
            assert_eq!(channel.last_key_time(), 1.0);
            let mid = channel.sample(0.5);
            // The ceiling key's easing (easeInQuad at alpha 0.5 → 0.25).
            assert!((mid.x - 2.5).abs() < 1.0e-5, "{}", mid.x);
            // A time that does not parse skips the entry.
            let bad = super::super::json::parse(r#"{"abc": [1,2,3]}"#).unwrap();
            let channel = parse_channel(Some(&bad), Vec3::ZERO);
            assert!(channel.keys.is_empty());
            // A null element is an empty channel on the fallback.
            let channel = parse_channel(None, v(4.0, 5.0, 6.0));
            assert!(channel.keys.is_empty());
            assert_eq!(channel.sample(1.0), v(4.0, 5.0, 6.0));
        }

        #[test]
        fn vector_reads_scalars_and_arrays() {
            let scalar = super::super::json::parse("2").unwrap();
            assert_eq!(vector(Some(&scalar), Vec3::ZERO), v(2.0, 2.0, 2.0));
            let arr = super::super::json::parse("[1, 2, 3]").unwrap();
            assert_eq!(vector(Some(&arr), Vec3::ZERO), v(1.0, 2.0, 3.0));
            let short = super::super::json::parse("[1, 2]").unwrap();
            assert_eq!(vector(Some(&short), v(7.0, 8.0, 9.0)), v(7.0, 8.0, 9.0));
            let with_default = super::super::json::parse("[1]").unwrap();
            assert_eq!(
                vector(Some(&with_default), v(7.0, 8.0, 9.0)),
                v(7.0, 8.0, 9.0)
            );
            assert_eq!(vector(None, v(7.0, 8.0, 9.0)), v(7.0, 8.0, 9.0));
        }

        #[test]
        fn parse_clip_reads_the_animation_shape() {
            let obj = super::super::json::parse(
                r#"{
                    "animation_length": 2.0,
                    "loop": true,
                    "bones": {
                        "Body": {
                            "position": {"0.0": [0,0,0], "1.0": [10,0,0]},
                            "rotation": [0,0,0],
                            "scale": [1,1,1]
                        }
                    }
                }"#,
            )
            .unwrap();
            let clip = parse_clip("walk", &obj);
            assert_eq!(clip.name, "walk");
            assert_eq!(clip.length, 2.0);
            assert!(clip.loop_);
            assert!(clip.bones.contains_key("body"));
            let pose = clip.sample("BODY", 0.5);
            assert!((pose.position.x - 5.0).abs() < 1.0e-5);
            // A non-looping clip stretches to the last key's time.
            let obj = super::super::json::parse(
                r#"{"bones": {"body": {"position": {"0.0": [0,0,0], "2.5": [5,0,0]}}}}"#,
            )
            .unwrap();
            let clip = parse_clip("hit", &obj);
            assert!((clip.length - 2.5).abs() < 1.0e-5);
            // The length never drops below 1e-4.
            let obj = super::super::json::parse("{}").unwrap();
            let clip = parse_clip("empty", &obj);
            assert!((clip.length - 1.0e-4).abs() < 1.0e-9);
        }

        #[test]
        fn face_shorthand_installs_the_face_bones() {
            let obj =
                super::super::json::parse(r#"{"face": {"look": "Happy", "mix": 0.5}}"#).unwrap();
            let mut bones = std::collections::HashMap::new();
            face_shorthand(&obj, &mut bones);
            assert!(bones.contains_key("face_mix"));
            assert!(bones.contains_key("face_happy"));
            // The mix bone carries the mix weight; the named bone is a marker.
            assert!((bones["face_mix"].position.sample(0.0).y - 0.5).abs() < 1.0e-6);
            assert!((bones["face_happy"].position.sample(0.0).y - 1.0).abs() < 1.0e-6);
            // A bare string is the look with the default mix.
            let obj = super::super::json::parse(r#"{"face": "sad face"}"#).unwrap();
            let mut bones = std::collections::HashMap::new();
            face_shorthand(&obj, &mut bones);
            assert!(bones.contains_key("face_sad_face"));
            assert!((bones["face_mix"].position.sample(0.0).y - 1.0).abs() < 1.0e-6);
            // Blank or missing faces install nothing.
            let obj = super::super::json::parse(r#"{"face": "  "}"#).unwrap();
            let mut bones = std::collections::HashMap::new();
            face_shorthand(&obj, &mut bones);
            assert!(bones.is_empty());
        }

        #[test]
        fn load_clips_reads_the_animation_document() {
            let mut clips = HashMap::new();
            load_clips_from_str(
                r#"{"animations": {"walk": {"animation_length": 1.0, "bones": {"body": {"position": {"0.0": [0,0,0], "1.0": [10,0,0]}}}}}}"#,
                &mut clips,
            )
            .unwrap();
            assert_eq!(clips.len(), 1);
            let clip = clips.get("walk").unwrap();
            assert!((clip.length - 1.0).abs() < 1.0e-5);
            let pose = clip.sample("body", 0.5);
            assert!((pose.position.x - 5.0).abs() < 1.0e-5);
            // A document without an animations map loads nothing.
            let mut clips = HashMap::new();
            load_clips_from_str("{}", &mut clips).unwrap();
            assert!(clips.is_empty());
        }

        #[test]
        fn resolved_overlay_applies_to_bones() {
            let clip = Clip::new("ov".into(), 1.0, false, HashMap::new());
            let overlay = ResolvedOverlay {
                clip,
                mode: OverlayMode::Additive,
                bones: vec!["left_arm".to_string()],
            };
            assert!(overlay.applies_to("left_arm"));
            assert!(!overlay.applies_to("right_arm"));
            let overlay = ResolvedOverlay {
                clip: Clip::new("ov".into(), 1.0, false, HashMap::new()),
                mode: OverlayMode::Override,
                bones: Vec::new(),
            };
            assert!(overlay.applies_to("anything"));
        }

        #[test]
        fn overlay_mode_parses_the_java_names() {
            assert_eq!(OverlayMode::from_name(None), OverlayMode::Additive);
            assert_eq!(
                OverlayMode::from_name(Some("additive")),
                OverlayMode::Additive
            );
            assert_eq!(
                OverlayMode::from_name(Some("override")),
                OverlayMode::Override
            );
            assert_eq!(
                OverlayMode::from_name(Some("exclusive")),
                OverlayMode::Exclusive
            );
            assert_eq!(OverlayMode::from_name(Some("nope")), OverlayMode::Additive);
        }

        #[test]
        fn locomotion_influence_selects_by_state() {
            let influence = LocomotionInfluence::new(1.0, 2.0, 3.0);
            assert_eq!(influence.select(false, false), 1.0);
            assert_eq!(influence.select(true, false), 2.0);
            assert_eq!(influence.select(false, true), 3.0);
            assert_eq!(influence.select(true, true), 3.0);
        }

        #[test]
        fn normalize_id_strips_separators() {
            assert_eq!(normalize_id("ease_In-Out Sine"), "easeinoutsine");
            assert_eq!(normalize_id("Walk-Forward"), "walkforward");
        }

        #[test]
        fn turn_in_place_copies_overlays_to_the_add_layers() {
            let root = super::super::json::parse(
                r#"{
                    "format_version": "2",
                    "items": {
                        "minecraft:stick": {
                            "animations": {
                                "turn_in_place": {
                                    "left_overlay": {"clip": "ov_left"},
                                    "right_overlay": {"clip": "ov_right"}
                                }
                            }
                        }
                    }
                }"#,
            )
            .unwrap();
            let out = compat_format::normalize(&root).unwrap();
            let entry = out
                .get("itemSpecific")
                .and_then(|v| v.get("minecraft:stick"))
                .and_then(|v| v.get("customAnimation"))
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .expect("turn_in_place entry");
            // The overlays became the add layers; the overlay keys are gone.
            assert_eq!(
                entry.get("left_add").and_then(|v| v.get("name")),
                Some(&super::super::json::Value::String("ov_left".to_string()))
            );
            assert_eq!(
                entry.get("right_add").and_then(|v| v.get("name")),
                Some(&super::super::json::Value::String("ov_right".to_string()))
            );
            assert!(entry.get("left_overlay").is_none());
            assert!(entry.get("right_overlay").is_none());
        }
        #[test]
        fn entries_ingest_from_the_java_array_shape() {
            let root = super::super::json::parse(
                r#"{
                    "format_version": "2",
                    "defaults": {
                        "animations": {
                            "attack": {"clip": "swing"},
                            "use_item": {"clip": "drink", "base_pose_retain": {"right_arm": 0.5}},
                            "idle_events": {"variants": ["wave", {"clip": "clap", "name": "clap"}]},
                            "emotes": {
                                "menu_key": "key.keyboard.b",
                                "entries": [
                                    {"animation": "wave", "display_name": "Wave", "keybind": "key.keyboard.h", "face": "cheerful"}
                                ]
                            }
                        }
                    }
                }"#,
            )
            .unwrap();
            let normalized = compat_format::normalize(&root).unwrap();
            // default_animations is an array of typed entries now.
            let list = normalized
                .get("default_animations")
                .and_then(|v| v.as_array())
                .expect("array default_animations");
            let attack = list
                .iter()
                .find(|e| {
                    e.get("type").and_then(super::super::json::Value::as_str) == Some("attack")
                })
                .unwrap();
            assert_eq!(
                attack
                    .get("name")
                    .and_then(super::super::json::Value::as_str),
                Some("swing")
            );

            let mut bindings = ItemAnimationBindings::new();
            bindings.add(&normalized);
            assert_eq!(
                bindings.defaults.get("attack").map(String::as_str),
                Some("swing")
            );
            // idle_events keeps its variants list; the ingest reads name or clip.
            assert_eq!(bindings.variants.get("idleevents").map(Vec::len), Some(2));
            // The retention came through the typed entry.
            let retention = bindings.first_retention("", "", "useitem").unwrap();
            assert_eq!(
                retention.value("right_arm", false),
                0.5,
                "retention from the entry"
            );
            // Emote entries: menu key, id/display defaults, keybind fallback.
            assert_eq!(bindings.emote_menu_key.as_deref(), Some("key.keyboard.b"));
            assert_eq!(bindings.emote_bindings.len(), 1);
            let emote = &bindings.emote_bindings[0];
            assert_eq!(emote.id, "wave");
            assert_eq!(emote.display_name, "Wave");
            assert_eq!(emote.animation, "wave");
            assert_eq!(emote.key.as_deref(), Some("key.keyboard.h"));
            assert_eq!(emote.face.as_deref(), Some("cheerful"));
        }

        #[test]
        fn normalize_orders_the_java_top_level_keys() {
            let root = super::super::json::parse(
                r#"{
                    "format_version": "2",
                    "defaults": {"static_when_using": {"attack": {"clip": "s"}}},
                    "empty_hand": {"animations": {"attack": {"clip": "e"}}},
                    "items": {"minecraft:stick": {"animations": {"attack": {"clip": "i"}}}},
                    "item_groups": {"swords": {"animations": {"attack": {"clip": "g"}}}}
                }"#,
            )
            .unwrap();
            let out = compat_format::normalize(&root).unwrap();
            let keys: Vec<&str> = out
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, _)| k.as_str())
                .collect();
            assert_eq!(
                keys,
                vec![
                    "static_when_using",
                    "default_animations",
                    "emptyHand",
                    "item",
                    "itemSpecific"
                ]
            );
            // The layers reach the bindings through the Java key names.
            let mut bindings = ItemAnimationBindings::new();
            bindings.add(&out);
            assert_eq!(
                bindings.empty_hand.get("attack").map(String::as_str),
                Some("e")
            );
            assert_eq!(
                bindings
                    .groups
                    .get("swords")
                    .and_then(|m| m.get("attack"))
                    .map(String::as_str),
                Some("g")
            );
            assert_eq!(
                bindings
                    .specific_items
                    .get("minecraft:stick")
                    .and_then(|m| m.get("attack"))
                    .map(String::as_str),
                Some("i")
            );
        }

        #[test]
        fn sequences_retrigger_influences_and_blends_ingest_from_entries() {
            // The normalized bindings document: typed entries under
            // default_animations, with the settings the compat format does not
            // carry.
            let normalized = super::super::json::parse(
                r#"{
                    "default_animations": [
                        {
                            "type": "sequence_attack",
                            "var_1": "s1",
                            "var_2": "s2",
                            "retrigger_progress": 0.4,
                            "locomotion_influence": {"walk": 0.5, "sprint": 150, "jump": 2},
                            "blending": {"from": 0.2, "to": 0.8},
                            "blending_progression": 0.3,
                            "timer_modifier": 0.5
                        }
                    ]
                }"#,
            )
            .unwrap();
            let mut bindings = ItemAnimationBindings::new();
            bindings.add(&normalized);
            assert_eq!(
                bindings
                    .default_sequences
                    .get("sequenceattack")
                    .map(|v| v.as_slice()),
                Some(&["s1".to_string(), "s2".to_string()][..])
            );
            assert_eq!(bindings.default_retrigger.get("sequenceattack"), Some(&0.4));
            let influence = bindings.default_influences.get("sequenceattack").unwrap();
            assert_eq!(influence.walk, 0.5);
            assert_eq!(influence.sprint, 1.0, "150 reads as 150% and clamps");
            assert_eq!(influence.jump, 0.02, "2 reads as 200%: 2/100");
            assert_eq!(influence.transition, 1.0);
            let blend = bindings.default_blends.get("sequenceattack").unwrap();
            assert_eq!(blend.from, 0.2);
            assert_eq!(blend.to, 0.8);
            assert_eq!(blend.duration_seconds, 0.3);
            assert!(blend.condition, "absent condition is true");
            let timer = bindings.default_timers.get("sequenceattack").unwrap();
            assert_eq!(timer.modifier, 0.5);
        }

        #[test]
        fn one_handed_index_walks_subanimations() {
            let root = super::super::json::parse(
                r#"{
                    "format_version": "2",
                    "defaults": {
                        "animations": {
                            "attack": {
                                "clip": "swing",
                                "one_handed": "swing_one",
                                "subanimations": [
                                    {"type": "use_item", "one_handed": {"clip": "drink_one"}}
                                ]
                            }
                        }
                    }
                }"#,
            )
            .unwrap();
            let normalized = compat_format::normalize(&root).unwrap();
            let mut bindings = ItemAnimationBindings::new();
            bindings.add(&normalized);
            assert_eq!(
                bindings
                    .default_one_handed
                    .get("attack")
                    .map(String::as_str),
                Some("swing_one")
            );
            assert_eq!(
                bindings
                    .default_one_handed
                    .get("useitem")
                    .map(String::as_str),
                Some("drink_one")
            );
        }
    }
}

pub mod signals {
    //! Motion signals, gait phase matching, base-pose retention and movement
    //! tuning, ported 1:1 from the Fabric mod's `punchymoves.client.animation`
    //! (`MotionSignals`, `GaitPhaseMatcher`, `BasePoseRetention`,
    //! `MovementTuning`).
    //!
    //! Every constant, field, formula and error message here is the Java class's,
    //! including the Portuguese validation strings the compat loader raises.
    //! Rust shapes differ only where the language forces it: the Java statics are
    //! free functions, the records are structs, and the JSON tree is the mod's own
    //! `json` module instead of Gson.

    use std::collections::HashMap;

    use super::json::Value;
    use super::library::Vec3;

    // ---------------------------------------------------------------------------
    // MotionSignals
    // ---------------------------------------------------------------------------

    /// `MotionSignals.NEVER`: no event has happened.
    pub const NEVER: i32 = i32::MIN;
    /// `MotionSignals.LIGHT`: the lightest fall or landing weight.
    pub const LIGHT: i32 = 1;
    /// `MotionSignals.MEDIUM`: the middle fall or landing weight.
    pub const MEDIUM: i32 = 2;
    /// `MotionSignals.HEAVY`: the heaviest fall or landing weight.
    pub const HEAVY: i32 = 3;
    /// `MotionSignals.FALL`: an ordinary fall.
    pub const FALL: i32 = 1;
    /// `MotionSignals.FREE_FALL`: a fall with no ground in sight.
    pub const FREE_FALL: i32 = 2;
    /// `MotionSignals.HARD_FALL`: a fall fast enough to hurt.
    pub const HARD_FALL: i32 = 3;
    /// `MotionSignals.TOSS`: a light throw.
    pub const TOSS: i32 = 1;
    /// `MotionSignals.PEARL`: an ender pearl throw.
    pub const PEARL: i32 = 2;
    /// `MotionSignals.BREW`: a potion throw.
    pub const BREW: i32 = 3;

    /// One tick's worth of the player's movement state: what they touch, how they
    /// move, which discrete action they are in, and the tick each one-shot event
    /// last happened on.
    ///
    /// The event ticks (`jumped`, `landed`, `hurt`, ...) hold [`NEVER`] until the
    /// event happens, then the tick it happened on; [`MotionSignals::since`]
    /// turns one into an age in ticks.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct MotionSignals {
        /// The tick this snapshot was taken on, [`NEVER`] before the first.
        pub tick: i32,
        /// Standing on something solid.
        pub grounded: bool,
        /// The fall's weight: [`LIGHT`], [`MEDIUM`] or [`HEAVY`].
        pub fall_weight: i32,
        /// Blocks dropped since the player last touched the ground.
        pub fall_distance: f32,
        /// Water (or flowing water) is directly below.
        pub water_below: bool,
        /// Lava is directly below.
        pub lava_below: bool,
        /// Swimming with the head above the surface.
        pub surface_swimming: bool,
        /// Swimming with the whole body under the surface.
        pub submerged_swimming: bool,
        /// Sprint-swimming: the sprint key is held in water.
        pub swim_sprint: bool,
        /// Vertical swim input: `-1` down, `1` up.
        pub swim_vertical: i32,
        /// Standing in water shallow enough to wade through.
        pub shallow_water: bool,
        /// Holding a ladder, vine or scaffolding.
        pub climbing: bool,
        /// Vertical climb input: `-1` down, `1` up.
        pub climb_direction: i32,
        /// Crawling under a one-block gap.
        pub crawling: bool,
        /// Gliding with an elytra.
        pub elytra: bool,
        /// Air speed while gliding, in blocks per tick.
        pub elytra_speed: f32,
        /// Roll while gliding, in radians: positive banks right.
        pub elytra_bank: f32,
        /// Flying in creative mode.
        pub creative_flight: bool,
        /// Flying faster than creative flight (the sprint key in the air).
        pub fast_flight: bool,
        /// Standing on a fence, wall or similar narrow-topped block.
        pub on_fence: bool,
        /// The sprint key is held and the player is moving.
        pub running: bool,
        /// Seconds the player has been running.
        pub run_seconds: f32,
        /// The sneak key is held.
        pub crouching: bool,
        /// The player's position changed since the previous tick.
        pub moving: bool,
        /// Forward movement input, `-1.0` (back) .. `1.0` (forward).
        pub walk_forward: f32,
        /// Left movement input, `-1.0` (right) .. `1.0` (left).
        pub walk_left: f32,
        /// Breaking a block.
        pub mining: bool,
        /// Drawing a crossbow.
        pub charging_crossbow: bool,
        /// Winding up a spear throw.
        pub charging_spear: bool,
        /// Holding a trident ready to throw.
        pub charging_trident: bool,
        /// Looking at a held map.
        pub map_view: bool,
        /// Music is playing within earshot.
        pub music_nearby: bool,
        /// Seconds since the player last moved.
        pub still_seconds: f32,
        /// The player is taking freezing damage.
        pub freezing: bool,
        /// A fishing line is out.
        pub fishing: bool,
        /// Something is pulling on the fishing line.
        pub biting: bool,
        /// The tick the player last jumped.
        pub jumped: i32,
        /// Whether that jump began while moving.
        pub jump_moving: bool,
        /// The tick the player last landed.
        pub landed: i32,
        /// The landing's weight: [`LIGHT`], [`MEDIUM`] or [`HEAVY`].
        pub landing_weight: i32,
        /// The tick the run began.
        pub run_started: i32,
        /// The tick the run ended.
        pub run_stopped: i32,
        /// The tick the player last crouched.
        pub crouched: i32,
        /// The tick the player last stood up.
        pub stood_up: i32,
        /// The tick the player last splashed.
        pub splashed: i32,
        /// The tick the player was last boosted (a firework, a riptide trident).
        pub boosted: i32,
        /// The tick the player last swapped hands.
        pub swapped: i32,
        /// The tick the player last equipped armour.
        pub equipped: i32,
        /// The tick the player last interacted with something.
        pub interacted: i32,
        /// The tick the player last threw an item.
        pub threw: i32,
        /// The weight of that throw: [`TOSS`], [`PEARL`] or [`BREW`].
        pub thrown: i32,
        /// The tick the player last ignited something.
        pub ignited: i32,
        /// The tick the player last sheared something.
        pub sheared: i32,
        /// The tick the player last released a draw.
        pub released: i32,
        /// The tick the player last hurled an item.
        pub hurled: i32,
        /// The tick the player last lunged.
        pub lunged: i32,
        /// The tick the player last cast a fishing line.
        pub cast: i32,
        /// The tick the player last reeled the line in.
        pub reeled: i32,
        /// The tick the player was last hurt.
        pub hurt: i32,
        /// Whether that hurt came from the front.
        pub hurt_from_front: bool,
        /// The tick the last idle event played.
        pub idle_event: i32,
    }

    impl Default for MotionSignals {
        fn default() -> Self {
            Self {
                tick: NEVER,
                grounded: true,
                fall_weight: 0,
                fall_distance: 0.0,
                water_below: false,
                lava_below: false,
                surface_swimming: false,
                submerged_swimming: false,
                swim_sprint: false,
                swim_vertical: 0,
                shallow_water: false,
                climbing: false,
                climb_direction: 0,
                crawling: false,
                elytra: false,
                elytra_speed: 0.0,
                elytra_bank: 0.0,
                creative_flight: false,
                fast_flight: false,
                on_fence: false,
                running: false,
                run_seconds: 0.0,
                crouching: false,
                moving: false,
                walk_forward: 0.0,
                walk_left: 0.0,
                mining: false,
                charging_crossbow: false,
                charging_spear: false,
                charging_trident: false,
                map_view: false,
                music_nearby: false,
                still_seconds: 0.0,
                freezing: false,
                fishing: false,
                biting: false,
                jumped: NEVER,
                jump_moving: false,
                landed: NEVER,
                landing_weight: 0,
                run_started: NEVER,
                run_stopped: NEVER,
                crouched: NEVER,
                stood_up: NEVER,
                splashed: NEVER,
                boosted: NEVER,
                swapped: NEVER,
                equipped: NEVER,
                interacted: NEVER,
                threw: NEVER,
                thrown: 0,
                ignited: NEVER,
                sheared: NEVER,
                released: NEVER,
                hurled: NEVER,
                lunged: NEVER,
                cast: NEVER,
                reeled: NEVER,
                hurt: NEVER,
                hurt_from_front: true,
                idle_event: NEVER,
            }
        }
    }

    impl MotionSignals {
        /// A snapshot as a fresh tick begins: every per-tick field cleared, the
        /// event ticks left as they were.
        pub fn new() -> Self {
            Self::default()
        }

        /// `MotionSignals.since`: the age of `event` at `tick`, in ticks, or
        /// [`i32::MAX`] when the event never happened or is in the future.
        pub fn since(event: i32, tick: i32) -> i32 {
            if event != NEVER && tick >= event {
                tick - event
            } else {
                i32::MAX
            }
        }

        /// `MotionSignals.fresh`: whether `tick` is this snapshot's tick or the
        /// one right after it.
        pub fn fresh(&self, tick: i32) -> bool {
            self.tick != NEVER && tick >= self.tick && tick - self.tick <= 1
        }

        /// `MotionSignals.begin`: clears every field a new tick rebuilds, keeping
        /// the event ticks.
        pub fn begin(&mut self, tick: i32) {
            self.tick = tick;
            self.elytra = false;
            self.elytra_speed = 0.0;
            self.elytra_bank = 0.0;
            self.creative_flight = false;
            self.fast_flight = false;
            self.climbing = false;
            self.crawling = false;
            self.on_fence = false;
            self.running = false;
            self.run_seconds = 0.0;
            self.moving = false;
            self.walk_forward = 0.0;
            self.walk_left = 0.0;
            self.music_nearby = false;
            self.still_seconds = 0.0;
            self.freezing = false;
            self.fall_weight = 0;
            self.fall_distance = 0.0;
            self.water_below = false;
            self.lava_below = false;
            self.surface_swimming = false;
            self.submerged_swimming = false;
            self.swim_sprint = false;
            self.swim_vertical = 0;
            self.shallow_water = false;
            self.climb_direction = 0;
            self.mining = false;
            self.charging_crossbow = false;
            self.charging_spear = false;
            self.charging_trident = false;
            self.map_view = false;
        }

        /// `MotionSignals.landingWeight`: the weight a landing gets from two
        /// measures (a drop and a speed, say), each with its own medium and hard
        /// threshold: hard when either measure reaches its hard threshold, medium
        /// when either reaches its medium one, light otherwise.
        pub fn landing_weight(
            first: f32,
            second: f32,
            medium_first: f32,
            medium_second: f32,
            hard_first: f32,
            hard_second: f32,
        ) -> i32 {
            if first >= hard_first || second >= hard_second {
                HEAVY
            } else if first >= medium_first || second >= medium_second {
                MEDIUM
            } else {
                LIGHT
            }
        }

        /// `MotionSignals.fallWeight`: the same three-way measure, for a fall in
        /// progress.
        pub fn fall_weight(
            first: f32,
            second: f32,
            medium_first: f32,
            medium_second: f32,
            hard_first: f32,
            hard_second: f32,
        ) -> i32 {
            if first >= hard_first || second >= hard_second {
                HEAVY
            } else if first >= medium_first || second >= medium_second {
                MEDIUM
            } else {
                LIGHT
            }
        }
    }

    // ---------------------------------------------------------------------------
    // GaitPhaseMatcher
    // ---------------------------------------------------------------------------

    /// `GaitPhaseMatcher`: the stride phase the gait settles on, matched by
    /// walking a cost function round the cycle.
    ///
    /// The phase is a fraction of the walk cycle, `0.0..1.0`. [`match_phase`]
    /// scans the whole cycle in 64 steps, keeps the cheapest, then refines
    /// ±8/512 of a cycle either side of that; a step is only kept when its cost
    /// drops by more than `1e-4`, so the match never wanders on a flat cost.
    pub mod gait_phase_matcher {
        /// The cost difference a step must beat to be kept.
        const IMPROVEMENT: f64 = 1.0e-4;
        /// The coarse pass's steps round the whole cycle.
        const COARSE_STEPS: i32 = 64;
        /// The fine pass's steps either side of the coarse best.
        const FINE_STEPS: i32 = 8;
        /// The fine pass's divisor.
        const FINE_DIVISOR: f32 = 512.0;

        /// `GaitPhaseMatcher.wrap`: the phase wrapped into `0.0..1.0`
        /// (`x - floor(x)`).
        pub fn wrap(phase: f32) -> f32 {
            phase - phase.floor()
        }

        /// `GaitPhaseMatcher.match`: the phase in `0.0..1.0` whose `cost` is
        /// cheapest, starting from `phase`. A cost that is not finite anywhere
        /// returns the phase unchanged.
        pub fn match_phase<F>(phase: f32, cost: F) -> f32
        where
            F: Fn(f32) -> f64,
        {
            let mut best = phase;
            let Some(mut best_cost) = finite(cost(phase)) else {
                return phase;
            };
            for i in 1..COARSE_STEPS {
                let candidate = wrap(phase + i as f32 / COARSE_STEPS as f32);
                if let Some(value) = finite(cost(candidate))
                    && value < best_cost - IMPROVEMENT
                {
                    best = candidate;
                    best_cost = value;
                }
            }
            let coarse = best;
            for j in -FINE_STEPS..=FINE_STEPS {
                let candidate = wrap(coarse + j as f32 / FINE_DIVISOR);
                if let Some(value) = finite(cost(candidate))
                    && value < best_cost - IMPROVEMENT
                {
                    best = candidate;
                    best_cost = value;
                }
            }
            best
        }

        /// The cost when it is finite.
        fn finite(cost: f64) -> Option<f64> {
            cost.is_finite().then_some(cost)
        }
    }

    pub use gait_phase_matcher::match_phase;

    // ---------------------------------------------------------------------------
    // BasePoseRetention
    // ---------------------------------------------------------------------------

    /// `BasePoseRetention`: how much of the base (vanilla) pose each bone keeps,
    /// `0.0..=1.0` per bone.
    ///
    /// A bone the set does not mention keeps nothing ([`BasePoseRetention::value`]
    /// answers `NaN`, which the animation layer reads as "the animation owns the
    /// bone"), except that an upper-body bone falls back to the `waist` value when
    /// there is one.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct BasePoseRetention {
        bones: HashMap<String, f32>,
    }

    impl BasePoseRetention {
        /// `BasePoseRetention.EMPTY`.
        pub fn empty() -> Self {
            Self::default()
        }

        /// Sets one bone's weight (the tests and the additive retention build
        /// their sets this way).
        pub fn set(&mut self, bone: &str, weight: f32) {
            self.bones.insert(bone.to_string(), weight);
        }

        /// `BasePoseRetention.BONES`: every bone the mod can retain.
        pub const BONES: [&str; 8] = [
            "root",
            "waist",
            "head",
            "body",
            "left_arm",
            "right_arm",
            "left_leg",
            "right_leg",
        ];

        /// `BasePoseRetention.UPPER_BODY`: the bones a waist value stands in for.
        pub const UPPER_BODY: [&str; 4] = ["body", "head", "left_arm", "right_arm"];

        /// Whether the set mentions no bone at all.
        pub fn is_empty(&self) -> bool {
            self.bones.is_empty()
        }

        /// `BasePoseRetention.parse`: the weights from a JSON object of bone
        /// names to numbers in `0.0..=1.0`. `None` is the empty set; anything
        /// else that is not an object, an unknown bone or a weight out of range
        /// is an error carrying the Java message (`Compat <context>...`).
        pub fn parse(value: Option<&Value>, context: &str) -> Result<Self, String> {
            let Some(value) = value else {
                return Ok(Self::empty());
            };
            let Some(members) = value.as_object() else {
                return Err(invalid(context, "esperado um objeto por osso"));
            };
            let mut bones = HashMap::new();
            for (bone, weight) in members {
                let label = format!("{}.{}", context, bone);
                if !Self::BONES.contains(&bone.as_str()) {
                    return Err(invalid(&label, "osso desconhecido"));
                }
                let Some(number) = weight.as_f32() else {
                    return Err(invalid(&label, "esperado um número entre 0 e 1"));
                };
                if !number.is_finite() || number < 0.0 || number > 1.0 {
                    return Err(invalid(&label, "esperado um número entre 0 e 1"));
                }
                bones.insert(bone.clone(), number);
            }
            Ok(Self { bones })
        }

        /// `BasePoseRetention.over`: this set with `other` laid under it — a bone
        /// both mention keeps this set's weight. An empty set on either side
        /// returns the other one.
        pub fn over(&self, other: &BasePoseRetention) -> BasePoseRetention {
            if self.bones.is_empty() {
                return other.clone();
            }
            if other.bones.is_empty() {
                return self.clone();
            }
            let mut merged = other.bones.clone();
            for (bone, weight) in &self.bones {
                merged.insert(bone.clone(), *weight);
            }
            BasePoseRetention { bones: merged }
        }

        /// `BasePoseRetention.value`: the weight `bone` keeps, or `NaN` when the
        /// set does not mention it. With `mirror`, `left_` and `right_` prefixes
        /// swap before the lookup; an upper-body bone with no value of its own
        /// falls back to the `waist` value.
        pub fn value(&self, bone: &str, mirror: bool) -> f32 {
            let mut bone = bone;
            let owned;
            if mirror {
                if let Some(rest) = bone.strip_prefix("left_") {
                    owned = format!("right_{}", rest);
                    bone = &owned;
                } else if let Some(rest) = bone.strip_prefix("right_") {
                    owned = format!("left_{}", rest);
                    bone = &owned;
                }
            }
            if let Some(weight) = self.bones.get(bone) {
                return *weight;
            }
            if Self::UPPER_BODY.contains(&bone)
                && let Some(waist) = self.bones.get("waist")
            {
                return *waist;
            }
            f32::NAN
        }
    }

    /// `BasePoseRetention.invalid`: the Java error message.
    fn invalid(label: &str, message: &str) -> String {
        format!("Compat {}: {}", label, message)
    }

    // ---------------------------------------------------------------------------
    // MovementTuning
    // ---------------------------------------------------------------------------

    /// `MovementTuning.Directions`: the sprint weights per direction.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Directions {
        pub forward: f32,
        pub backward: f32,
        pub left: f32,
        pub right: f32,
    }

    /// `MovementTuning`: the numbers the animation layer is tuned with, each a
    /// small pure function of the pack's settings.
    pub mod movement_tuning {
        use super::super::library::Quat;
        use super::{Directions, Vec3};

        /// `MovementTuning.AIR_HOLD_STEPS`: how finely the jump's air hold is
        /// searched.
        pub const AIR_HOLD_STEPS: u32 = 96;
        /// The stride distance a pack may ask for at most.
        const STRIDE_MAX: f32 = 8.0;
        /// The gait amplitude a pack may ask for at most, when it sets no cap.
        const AMPLITUDE_CAP: f32 = 1.2;
        /// The fallback distance a full landing weighs in at.
        const LANDING_FULL: f64 = 5.0;
        /// The per-step growth of the landing's blend distance (`0.08`), damped
        /// by `0.98F` each step.
        const LANDING_STEP: f64 = 0.08;
        const LANDING_DAMP: f64 = 0.98_f32 as f64;
        /// The stride scale's floor.
        const USE_STRIDE_MIN: f32 = 0.1;
        /// The walk weight's stride divisor at scale 1.
        const WALK_DIVISOR: f32 = 0.11;
        /// The fade rate's bounds, in seconds.
        const FADE_MIN: f32 = 0.05;
        const FADE_MAX: f32 = 5.0;

        /// `MovementTuning.strideDistance`: the stride distance a pack asked
        /// for, clamped into `min..=8.0` and capped by `max`; `0.0` when the
        /// value is broken or out of range.
        pub fn stride_distance(value: f32, min: f32, max: f32) -> f32 {
            if value.is_finite() && value >= min && value <= STRIDE_MAX {
                value.min(max)
            } else {
                0.0
            }
        }

        /// `MovementTuning.sprintDirections`: the sprint direction weights with
        /// `blend` (`0.0..=1.0`) of the sideways and backward weights folded
        /// into the forward one.
        pub fn sprint_directions(
            forward: f32,
            backward: f32,
            left: f32,
            right: f32,
            blend: f32,
        ) -> Directions {
            let blend = if blend.is_finite() {
                blend.clamp(0.0, 1.0)
            } else {
                0.0
            };
            Directions {
                forward: forward + (backward + left + right) * blend,
                backward: backward * (1.0 - blend),
                left: left * (1.0 - blend),
                right: right * (1.0 - blend),
            }
        }

        /// `MovementTuning.gaitAmplitude`: how far the legs swing at `speed`,
        /// relative to a full walk at `stride`, capped by `cap` (`1.2` when the
        /// pack sets none). A broken or non-positive stride leaves the swing at
        /// `1.0`; a broken speed flattens it.
        pub fn gait_amplitude(speed: f32, stride: f32, cap: f32) -> f32 {
            if !stride.is_finite() || stride <= 0.0 {
                return 1.0;
            }
            if !speed.is_finite() {
                return 0.0;
            }
            let cap = if cap.is_finite() {
                cap.max(0.0)
            } else {
                AMPLITUDE_CAP
            };
            (speed.max(0.0) / stride).clamp(0.0, cap)
        }

        /// `MovementTuning.earlyJumpLanding`: whether a landing `t` seconds into
        /// a jump counts as early (before either of the two windows closes).
        pub fn early_jump_landing(t: f32, a: f32, b: f32) -> bool {
            t.is_finite() && a.is_finite() && b.is_finite() && t >= 0.0 && t < a && t < b
        }

        /// `MovementTuning.landingAmplitude`: how much of the landing animation
        /// shows for a fall of `distance` blocks, against a full weight at
        /// `full` blocks (`5.0` when the pack sets none). The full distance is
        /// reached by summing a damped growth (`(d + 0.08) * 0.98F` a step) for
        /// at most 1024 steps, exactly as the Java does, so a pack's `full`
        /// lands on the same weight it always did.
        pub fn landing_amplitude(distance: f32, full: f32) -> f32 {
            if !distance.is_finite() || distance <= 0.0 {
                return 0.0;
            }
            let full = if full.is_finite() && full > 0.0 {
                full as f64
            } else {
                LANDING_FULL
            };
            let mut step = 0.0_f64;
            let mut total = 0.0_f64;
            let mut i = 0;
            while i < 1024 && total < full {
                step = (step + LANDING_STEP) * LANDING_DAMP;
                total += step;
                i += 1;
            }
            (distance as f64 / step).clamp(0.0, 1.0) as f32
        }

        /// `MovementTuning.jumpFamilies`: the clip-name prefixes a jump matches,
        /// by whether the player was running and whether they were moving.
        pub fn jump_families(running: bool, moving: bool) -> Vec<&'static str> {
            if !running {
                vec!["jump_idle_"]
            } else if moving {
                vec!["jump_running_", "jump_moving_"]
            } else {
                vec!["jump_moving_"]
            }
        }

        /// `MovementTuning.jumpContact`: the landing clip of a jump family, the
        /// family's own `landing` when it has one, the shared `jump_landing`
        /// otherwise.
        pub fn jump_contact<T>(
            lookup: impl Fn(&str) -> Option<T>,
            prefix: Option<&str>,
        ) -> Option<T> {
            let family = prefix.filter(|p| !p.is_empty());
            let own = family.and_then(|prefix| lookup(&format!("{}landing", prefix)));
            own.or_else(|| lookup("jump_landing"))
        }

        /// `MovementTuning.gaitPhase`: the cycle phase, reversed (`1 - phase`)
        /// when the clip runs backwards, and `0.0` when the phase is broken.
        pub fn gait_phase(phase: f32, reverse: bool) -> f32 {
            if !reverse {
                return phase;
            }
            if !phase.is_finite() {
                return 0.0;
            }
            let fraction = phase - phase.floor();
            if fraction == 0.0 { 0.0 } else { 1.0 - fraction }
        }

        /// `MovementTuning.useStrideScale`: the stride scale a use animation
        /// plays at, clamped into `0.1..=1.0`; `1.0` when broken.
        pub fn use_stride_scale(value: f32) -> f32 {
            if value.is_finite() {
                value.clamp(USE_STRIDE_MIN, 1.0)
            } else {
                1.0
            }
        }

        /// `MovementTuning.walkWeight`: how much of the walk cycle shows at
        /// `speed`, against a full walk at `0.11 * scale` blocks a tick.
        pub fn walk_weight(speed: f32, scale: f32) -> f32 {
            let scale = if scale.is_finite() {
                scale.clamp(USE_STRIDE_MIN, 1.0)
            } else {
                1.0
            };
            if speed.is_finite() {
                (speed / (WALK_DIVISOR * scale)).clamp(0.0, 1.0)
            } else {
                0.0
            }
        }

        /// `MovementTuning.leanUnder`: the Euler angles (degrees) of `base`
        /// turned by `under` through `weight` (`0.0..=1.0`).
        pub fn lean_under(base: Vec3, under: Vec3, weight: f32) -> Vec3 {
            if weight <= 0.0 {
                return base;
            }
            let turned = quaternion(scaled(under, weight.min(1.0))) * quaternion(base);
            scaled(
                turned.get_euler_zyx(),
                (180.0 / std::f64::consts::PI) as f32,
            )
        }

        /// `MovementTuning.shoulderOnLean`: how the shoulder offset `a` moves
        /// when the model leans by `lean` about `lean_pivot` and shifts by
        /// `shift`, through `weight`.
        pub fn shoulder_on_lean(a: Vec3, b: Vec3, c: Vec3, d: Vec3, weight: f32) -> Vec3 {
            if weight <= 0.0 {
                return Vec3::ZERO;
            }
            let f = weight.min(1.0);
            let base = Vec3::new(a.x + b.x, a.y - b.y, a.z + b.z);
            let turned = quaternion(scaled(c, f)).transform_by(base);
            let moved = Vec3::new(turned.x + d.x * f, turned.y - d.y * f, turned.z + d.z * f);
            let delta = moved - base;
            Vec3::new(delta.x, -delta.y, delta.z)
        }

        /// `MovementTuning.armPivot`: the arms' rotation pivot, in model pixels,
        /// mirrored for the left arm.
        pub fn arm_pivot(right: bool) -> Vec3 {
            Vec3::new(if right { 5.0 } else { -5.0 }, 2.0, 0.0)
        }

        /// `MovementTuning.fadeRate`: how fast a weight moves per second,
        /// `1 / seconds` clamped into `0.05..5.0` seconds; `fallback` when the
        /// pack's value is broken or not positive.
        pub fn fade_rate(seconds: f32, fallback: f32) -> f32 {
            let seconds = if seconds.is_finite() && seconds > 0.0 {
                seconds
            } else {
                fallback
            };
            1.0 / seconds.clamp(FADE_MIN, FADE_MAX)
        }

        /// `MovementTuning.shortWay`: every angle wrapped into `-180..180`.
        pub fn short_way(angles: Vec3) -> Vec3 {
            Vec3::new(
                wrap_degrees(angles.x),
                wrap_degrees(angles.y),
                wrap_degrees(angles.z),
            )
        }

        /// `MovementTuning.wrapDegrees`: an angle wrapped into `-180..180`
        /// (`NaN` stays `NaN`).
        pub fn wrap_degrees(degrees: f32) -> f32 {
            let wrapped = degrees % 360.0;
            if wrapped >= 180.0 {
                wrapped - 360.0
            } else if wrapped < -180.0 {
                wrapped + 360.0
            } else {
                wrapped
            }
        }

        /// A vector scaled componentwise.
        fn scaled(v: Vec3, k: f32) -> Vec3 {
            Vec3::new(v.x * k, v.y * k, v.z * k)
        }

        /// A quaternion from Euler degrees: JOML's `rotationZYX`.
        fn quaternion(euler: Vec3) -> Quat {
            Quat::from_euler_zyx(euler.z, euler.y, euler.x)
        }
    }

    pub use movement_tuning as tuning;

    #[cfg(test)]
    mod tests {
        use super::super::json::parse;
        use super::*;

        fn value(text: &str) -> Value {
            parse(text).unwrap_or_else(|e| panic!("parse failed: {}", e))
        }

        #[test]
        fn the_constants_hold_their_java_values() {
            assert_eq!(NEVER, i32::MIN);
            assert_eq!((LIGHT, MEDIUM, HEAVY), (1, 2, 3));
            assert_eq!((FALL, FREE_FALL, HARD_FALL), (1, 2, 3));
            assert_eq!((TOSS, PEARL, BREW), (1, 2, 3));
        }

        #[test]
        fn a_fresh_snapshot_starts_as_the_java_fields_do() {
            let signals = MotionSignals::new();
            assert_eq!(signals.tick, NEVER);
            assert!(signals.grounded);
            assert_eq!(signals.fall_weight, 0);
            assert_eq!(signals.fall_distance, 0.0);
            assert!(!signals.elytra);
            assert_eq!(signals.swim_vertical, 0);
            assert_eq!(signals.climb_direction, 0);
            assert_eq!(signals.walk_forward, 0.0);
            assert_eq!(signals.walk_left, 0.0);
            assert_eq!(signals.still_seconds, 0.0);
            // The event ticks have never happened, and the last hurt came from
            // the front.
            for event in [
                signals.jumped,
                signals.landed,
                signals.run_started,
                signals.run_stopped,
                signals.crouched,
                signals.stood_up,
                signals.splashed,
                signals.boosted,
                signals.swapped,
                signals.equipped,
                signals.interacted,
                signals.threw,
                signals.ignited,
                signals.sheared,
                signals.released,
                signals.hurled,
                signals.lunged,
                signals.cast,
                signals.reeled,
                signals.hurt,
                signals.idle_event,
            ] {
                assert_eq!(event, NEVER);
            }
            assert!(!signals.jump_moving);
            assert_eq!(signals.thrown, 0);
            assert_eq!(signals.landing_weight, 0);
            assert!(signals.hurt_from_front);
        }

        #[test]
        fn since_ages_an_event_and_fresh_spots_the_next_tick() {
            assert_eq!(MotionSignals::since(NEVER, 10), i32::MAX);
            assert_eq!(MotionSignals::since(7, 10), 3);
            assert_eq!(MotionSignals::since(7, 7), 0);
            // An event in the future is no age at all.
            assert_eq!(MotionSignals::since(11, 10), i32::MAX);

            let mut signals = MotionSignals::new();
            assert!(!signals.fresh(10));
            signals.tick = 10;
            assert!(signals.fresh(10));
            assert!(signals.fresh(11));
            assert!(!signals.fresh(12));
            assert!(!signals.fresh(9));
        }

        #[test]
        fn begin_clears_the_per_tick_fields_and_keeps_the_events() {
            let mut signals = MotionSignals {
                tick: 5,
                grounded: false,
                elytra: true,
                elytra_speed: 2.0,
                elytra_bank: 0.5,
                creative_flight: true,
                fast_flight: true,
                climbing: true,
                crawling: true,
                on_fence: true,
                running: true,
                run_seconds: 3.0,
                moving: true,
                walk_forward: 1.0,
                walk_left: -1.0,
                music_nearby: true,
                still_seconds: 4.0,
                freezing: true,
                fall_weight: HEAVY,
                fall_distance: 9.0,
                water_below: true,
                lava_below: true,
                surface_swimming: true,
                submerged_swimming: true,
                swim_sprint: true,
                swim_vertical: 1,
                shallow_water: true,
                climb_direction: -1,
                mining: true,
                charging_crossbow: true,
                charging_spear: true,
                charging_trident: true,
                map_view: true,
                jumped: 3,
                landed: 4,
                hurt: 2,
                ..MotionSignals::new()
            };
            signals.begin(6);
            assert_eq!(signals.tick, 6);
            // The events survive the tick, and so does the ground state: the
            // tick's collector sets it, `begin` only clears what it rebuilds.
            assert_eq!(signals.jumped, 3);
            assert_eq!(signals.landed, 4);
            assert_eq!(signals.hurt, 2);
            assert!(!signals.grounded);
            assert!(!signals.elytra);
            assert_eq!(signals.elytra_speed, 0.0);
            assert_eq!(signals.elytra_bank, 0.0);
            assert!(!signals.creative_flight);
            assert!(!signals.fast_flight);
            assert!(!signals.climbing);
            assert!(!signals.crawling);
            assert!(!signals.on_fence);
            assert!(!signals.running);
            assert_eq!(signals.run_seconds, 0.0);
            assert!(!signals.moving);
            assert_eq!(signals.walk_forward, 0.0);
            assert_eq!(signals.walk_left, 0.0);
            assert!(!signals.music_nearby);
            assert_eq!(signals.still_seconds, 0.0);
            assert!(!signals.freezing);
            assert_eq!(signals.fall_weight, 0);
            assert_eq!(signals.fall_distance, 0.0);
            assert!(!signals.water_below);
            assert!(!signals.lava_below);
            assert!(!signals.surface_swimming);
            assert!(!signals.submerged_swimming);
            assert!(!signals.swim_sprint);
            assert_eq!(signals.swim_vertical, 0);
            assert!(!signals.shallow_water);
            assert_eq!(signals.climb_direction, 0);
            assert!(!signals.mining);
            assert!(!signals.charging_crossbow);
            assert!(!signals.charging_spear);
            assert!(!signals.charging_trident);
            assert!(!signals.map_view);
        }

        #[test]
        fn the_landing_and_fall_weights_measure_the_same_way() {
            // Hard when either measure reaches its hard threshold.
            assert_eq!(
                MotionSignals::landing_weight(9.0, 0.0, 2.0, 3.0, 4.0, 5.0),
                HEAVY
            );
            assert_eq!(
                MotionSignals::landing_weight(0.0, 6.0, 2.0, 3.0, 4.0, 5.0),
                HEAVY
            );
            // Medium when only a medium threshold is reached.
            assert_eq!(
                MotionSignals::landing_weight(3.0, 0.0, 2.0, 3.0, 4.0, 5.0),
                MEDIUM
            );
            assert_eq!(
                MotionSignals::landing_weight(0.0, 3.5, 2.0, 3.0, 4.0, 5.0),
                MEDIUM
            );
            // Light otherwise, and exactly on a threshold is reached.
            assert_eq!(
                MotionSignals::landing_weight(1.0, 1.0, 2.0, 3.0, 4.0, 5.0),
                LIGHT
            );
            assert_eq!(
                MotionSignals::landing_weight(4.0, 0.0, 2.0, 3.0, 4.0, 5.0),
                HEAVY
            );
            assert_eq!(
                MotionSignals::landing_weight(2.0, 0.0, 2.0, 3.0, 4.0, 5.0),
                MEDIUM
            );
            // The fall weight is the same measure.
            for (a, b) in [(9.0, 0.0), (0.0, 6.0), (3.0, 0.0), (1.0, 1.0), (4.0, 0.0)] {
                assert_eq!(
                    MotionSignals::fall_weight(a, b, 2.0, 3.0, 4.0, 5.0),
                    MotionSignals::landing_weight(a, b, 2.0, 3.0, 4.0, 5.0)
                );
            }
        }

        #[test]
        fn the_phase_match_walks_the_cycle_for_the_cheapest_cost() {
            use gait_phase_matcher::match_phase;

            // A cost with one dip finds it from anywhere, to within the fine
            // pass's 1/512 of a cycle.
            let cost = |phase: f32| {
                let d = (phase - 0.75).abs();
                d.min(1.0 - d) as f64 // shortest way round the cycle
            };
            let matched = match_phase(0.1, cost);
            assert!((matched - 0.75).abs() < 1.0 / 512.0, "matched {}", matched);

            // A cost that is never finite leaves the phase alone.
            assert_eq!(match_phase(0.3, |_| f64::NAN), 0.3);
            // A flat cost keeps the starting phase (no step beats it).
            assert_eq!(match_phase(0.42, |_| 1.0), 0.42);
            // The result is always wrapped into 0.0..1.0.
            assert!((0.0..1.0).contains(&matched));

            // The wrap is `x - floor(x)`.
            assert_eq!(gait_phase_matcher::wrap(1.25), 0.25);
            assert_eq!(gait_phase_matcher::wrap(-0.25), 0.75);
            assert_eq!(gait_phase_matcher::wrap(3.0), 0.0);
        }

        #[test]
        fn retention_parses_bones_and_names_the_bad_ones() {
            // Nothing to parse is the empty set.
            assert!(BasePoseRetention::parse(None, "test").unwrap().is_empty());

            let parsed = BasePoseRetention::parse(
                Some(&value("{\"head\":1.0,\"body\":0.5,\"left_arm\":0.0}")),
                "test",
            )
            .unwrap();
            assert!((parsed.value("head", false) - 1.0).abs() < 1e-6);
            assert!((parsed.value("body", false) - 0.5).abs() < 1e-6);
            assert!((parsed.value("left_arm", false) - 0.0).abs() < 1e-6);
            // A bone the set does not mention is NaN.
            assert!(parsed.value("right_leg", false).is_nan());

            // The errors carry the Java messages.
            assert_eq!(
                BasePoseRetention::parse(Some(&value("[]")), "ctx").unwrap_err(),
                "Compat ctx: esperado um objeto por osso"
            );
            assert_eq!(
                BasePoseRetention::parse(Some(&value("{\"tail\":1.0}")), "ctx").unwrap_err(),
                "Compat ctx.tail: osso desconhecido"
            );
            assert_eq!(
                BasePoseRetention::parse(Some(&value("{\"head\":2.0}")), "ctx").unwrap_err(),
                "Compat ctx.head: esperado um número entre 0 e 1"
            );
            assert_eq!(
                BasePoseRetention::parse(Some(&value("{\"head\":-0.5}")), "ctx").unwrap_err(),
                "Compat ctx.head: esperado um número entre 0 e 1"
            );
            assert_eq!(
                BasePoseRetention::parse(Some(&value("{\"head\":\"lots\"}")), "ctx").unwrap_err(),
                "Compat ctx.head: esperado um número entre 0 e 1"
            );
            // A JSON null is not an object either.
            assert_eq!(
                BasePoseRetention::parse(Some(&value("null")), "ctx").unwrap_err(),
                "Compat ctx: esperado um objeto por osso"
            );
        }

        #[test]
        fn retention_mirrors_and_falls_back_to_the_waist() {
            let parsed = BasePoseRetention::parse(
                Some(&value("{\"right_arm\":0.25,\"waist\":0.75}")),
                "test",
            )
            .unwrap();

            // Mirroring swaps the left and right prefixes: the stored right arm
            // answers for the left, and the mirrored right arm finds nothing, so
            // it takes the waist's value.
            assert!((parsed.value("left_arm", true) - 0.25).abs() < 1e-6);
            assert!((parsed.value("right_arm", true) - 0.75).abs() < 1e-6);
            // Without mirroring, only the stored side answers.
            assert!((parsed.value("right_arm", false) - 0.25).abs() < 1e-6);

            // An upper-body bone with nothing of its own takes the waist's value.
            assert!((parsed.value("head", false) - 0.75).abs() < 1e-6);
            assert!((parsed.value("body", false) - 0.75).abs() < 1e-6);
            // The other arm is upper body too, so it also takes the waist.
            assert!((parsed.value("left_arm", false) - 0.75).abs() < 1e-6);
            // A leg is not upper body, so it stays NaN.
            assert!(parsed.value("left_leg", false).is_nan());
        }

        #[test]
        fn retention_over_keeps_the_base_where_both_speak() {
            let base =
                BasePoseRetention::parse(Some(&value("{\"head\":1.0,\"body\":0.5}")), "a").unwrap();
            let overlay =
                BasePoseRetention::parse(Some(&value("{\"body\":0.25,\"left_leg\":1.0}")), "b")
                    .unwrap();

            // `base.over(overlay)`: the base's weights win, the overlay's new
            // bones join.
            let merged = base.over(&overlay);
            assert!((merged.value("head", false) - 1.0).abs() < 1e-6);
            assert!((merged.value("body", false) - 0.5).abs() < 1e-6);
            assert!((merged.value("left_leg", false) - 1.0).abs() < 1e-6);

            // An empty side returns the other whole.
            let empty = BasePoseRetention::empty();
            assert_eq!(empty.over(&base), base);
            assert_eq!(base.over(&empty), base);
            assert_eq!(BasePoseRetention::BONES.len(), 8);
            assert_eq!(BasePoseRetention::UPPER_BODY.len(), 4);
        }

        #[test]
        fn tuning_clamps_the_stride_distance() {
            use tuning::stride_distance;
            // In range: capped by `max`, never past 8.
            assert!((stride_distance(2.0, 0.5, 3.0) - 2.0).abs() < 1e-6);
            assert!((stride_distance(5.0, 0.5, 3.0) - 3.0).abs() < 1e-6);
            assert!((stride_distance(9.0, 0.5, 3.0) - 0.0).abs() < 1e-6);
            // Below the minimum or broken: nothing.
            assert!((stride_distance(0.4, 0.5, 3.0) - 0.0).abs() < 1e-6);
            assert!((stride_distance(f32::NAN, 0.5, 3.0) - 0.0).abs() < 1e-6);
            assert!((stride_distance(f32::INFINITY, 0.5, 3.0) - 0.0).abs() < 1e-6);
        }

        #[test]
        fn tuning_folds_the_sprint_into_the_forward_weight() {
            use tuning::sprint_directions;
            // No blend leaves the four weights alone.
            let none = sprint_directions(1.0, 0.7, 0.85, 0.85, 0.0);
            assert!((none.forward - 1.0).abs() < 1e-6);
            assert!((none.backward - 0.7).abs() < 1e-6);
            // A full blend folds the other three into the forward weight.
            let all = sprint_directions(1.0, 0.7, 0.85, 0.85, 1.0);
            assert!((all.forward - (1.0 + 0.7 + 0.85 + 0.85)).abs() < 1e-6);
            assert_eq!(all.backward, 0.0);
            assert_eq!(all.left, 0.0);
            assert_eq!(all.right, 0.0);
            // Halfway blends both ways.
            let half = sprint_directions(1.0, 0.7, 0.85, 0.85, 0.5);
            assert!((half.forward - (1.0 + 1.2)).abs() < 1e-6);
            assert!((half.backward - 0.35).abs() < 1e-6);
            // A broken blend reads as none.
            let broken = sprint_directions(1.0, 0.7, 0.85, 0.85, f32::NAN);
            assert!((broken.forward - 1.0).abs() < 1e-6);
            // A blend past 1 clamps.
            let over = sprint_directions(1.0, 0.7, 0.85, 0.85, 4.0);
            assert_eq!(over.backward, 0.0);
        }

        #[test]
        fn tuning_scales_the_gait_and_the_walk_weight() {
            use tuning::{gait_amplitude, walk_weight};
            // The swing grows with speed up to the cap.
            assert!((gait_amplitude(0.5, 1.0, 1.2) - 0.5).abs() < 1e-6);
            assert!((gait_amplitude(2.0, 1.0, 1.2) - 1.2).abs() < 1e-6);
            // No cap means the Java default of 1.2.
            assert!((gait_amplitude(2.0, 1.0, f32::NAN) - 1.2).abs() < 1e-6);
            // A broken stride leaves the swing at 1; a broken speed flattens it.
            assert!((gait_amplitude(0.5, 0.0, 1.2) - 1.0).abs() < 1e-6);
            assert!((gait_amplitude(f32::NAN, 1.0, 1.2) - 0.0).abs() < 1e-6);
            // A negative speed is no swing at all.
            assert!((gait_amplitude(-1.0, 1.0, 1.2) - 0.0).abs() < 1e-6);

            // The walk weight fills in by 0.11 blocks a tick at scale 1.
            assert!((walk_weight(0.11, 1.0) - 1.0).abs() < 1e-6);
            assert!((walk_weight(0.05, 1.0) - 0.45454547).abs() < 1e-6);
            assert!((walk_weight(1.0, 1.0) - 1.0).abs() < 1e-6);
            // A smaller scale fills sooner; a broken speed is nothing.
            assert!((walk_weight(0.05, 0.5) - 0.909_090_94).abs() < 1e-6);
            assert!((walk_weight(f32::NAN, 1.0) - 0.0).abs() < 1e-6);
            // A broken scale reads as 1.
            assert!((walk_weight(0.11, f32::NAN) - 1.0).abs() < 1e-6);
        }

        #[test]
        fn tuning_lands_on_the_java_blend_distance() {
            use tuning::landing_amplitude;
            // Nothing below the onset, full at and past the Java blend distance.
            assert_eq!(landing_amplitude(0.0, 5.0), 0.0);
            assert_eq!(landing_amplitude(-1.0, 5.0), 0.0);
            assert_eq!(landing_amplitude(f32::NAN, 5.0), 0.0);
            assert!((landing_amplitude(0.843_910_6, 5.0) - 1.0).abs() < 1e-6);
            assert!((landing_amplitude(9.0, 5.0) - 1.0).abs() < 1e-6);
            // In between, the share of the blend distance the fall reached.
            assert!((landing_amplitude(0.4, 5.0) - 0.47398388).abs() < 1e-6);
            assert!((landing_amplitude(0.25, 5.0) - 0.29623994).abs() < 1e-6);
            // A pack's own full distance scales the same way.
            assert!((landing_amplitude(0.4, 0.4) - 1.0).abs() < 1e-6);
            // A broken full distance reads as the Java default of 5.
            assert!((landing_amplitude(0.4, 0.0) - 0.47398388).abs() < 1e-6);
        }

        #[test]
        fn tuning_names_the_jump_families_and_their_landing() {
            use tuning::{jump_contact, jump_families};
            assert_eq!(jump_families(false, true), vec!["jump_idle_"]);
            assert_eq!(
                jump_families(true, true),
                vec!["jump_running_", "jump_moving_"]
            );
            assert_eq!(jump_families(true, false), vec!["jump_moving_"]);

            // A family with its own landing keeps it; one without falls back to
            // the shared landing.
            let clips = |name: &str| match name {
                "jump_running_landing" => Some("running landing"),
                "jump_landing" => Some("shared landing"),
                _ => None,
            };
            assert_eq!(
                jump_contact(clips, Some("jump_running_")),
                Some("running landing")
            );
            assert_eq!(
                jump_contact(clips, Some("jump_idle_")),
                Some("shared landing")
            );
            // An empty family name goes straight to the shared landing.
            assert_eq!(jump_contact(clips, Some("")), Some("shared landing"));
            assert_eq!(jump_contact(clips, None), Some("shared landing"));
            // Nothing anywhere is nothing.
            assert_eq!(jump_contact(|_| None::<&str>, Some("jump_idle_")), None);
        }

        #[test]
        fn tuning_phases_scales_and_fades() {
            use tuning::{early_jump_landing, fade_rate, gait_phase, use_stride_scale};
            // The phase runs backwards only when asked.
            assert!((gait_phase(0.3, false) - 0.3).abs() < 1e-6);
            assert!((gait_phase(0.3, true) - 0.7).abs() < 1e-6);
            assert!((gait_phase(1.3, true) - 0.7).abs() < 1e-6);
            assert_eq!(gait_phase(2.0, true), 0.0);
            assert_eq!(gait_phase(f32::NAN, true), 0.0);
            assert!(gait_phase(f32::NAN, false).is_nan());

            // A landing inside both windows is early.
            assert!(early_jump_landing(0.1, 0.2, 0.3));
            assert!(!early_jump_landing(0.25, 0.2, 0.3));
            assert!(!early_jump_landing(-0.1, 0.2, 0.3));
            assert!(!early_jump_landing(f32::NAN, 0.2, 0.3));

            // The stride scale clamps into 0.1..1; broken reads as 1.
            assert!((use_stride_scale(0.5) - 0.5).abs() < 1e-6);
            assert!((use_stride_scale(0.01) - 0.1).abs() < 1e-6);
            assert!((use_stride_scale(4.0) - 1.0).abs() < 1e-6);
            assert!((use_stride_scale(f32::NAN) - 1.0).abs() < 1e-6);

            // The fade rate is one over the seconds, clamped into 0.05..5.
            assert!((fade_rate(0.2, 6.0) - 5.0).abs() < 1e-6);
            assert!((fade_rate(0.5, 6.0) - 2.0).abs() < 1e-6);
            assert!((fade_rate(0.01, 6.0) - 20.0).abs() < 1e-6);
            assert!((fade_rate(50.0, 6.0) - 0.2).abs() < 1e-6);
            // A broken value takes the fallback.
            assert!((fade_rate(0.0, 4.0) - 0.25).abs() < 1e-6);
            assert!((fade_rate(f32::NAN, 4.0) - 0.25).abs() < 1e-6);
        }

        #[test]
        fn tuning_wraps_degrees_the_java_way() {
            use tuning::{short_way, wrap_degrees};
            assert!((wrap_degrees(450.0) - 90.0).abs() < 1e-6);
            assert!((wrap_degrees(-90.0) - (-90.0)).abs() < 1e-6);
            assert!((wrap_degrees(-270.0) - 90.0).abs() < 1e-6);
            assert!((wrap_degrees(180.0) - (-180.0)).abs() < 1e-6);
            assert!((wrap_degrees(-180.0) - (-180.0)).abs() < 1e-6);
            assert_eq!(wrap_degrees(0.0), 0.0);
            // NaN stays NaN, like Java's `%`.
            assert!(wrap_degrees(f32::NAN).is_nan());

            let wrapped = short_way(Vec3::new(450.0, -270.0, 30.0));
            assert!((wrapped.x - 90.0).abs() < 1e-6);
            assert!((wrapped.y - 90.0).abs() < 1e-6);
            assert!((wrapped.z - 30.0).abs() < 1e-6);
        }

        #[test]
        fn tuning_leans_and_pivots_like_joml() {
            use tuning::{arm_pivot, lean_under, shoulder_on_lean};
            // No weight leaves the angles alone.
            let base = Vec3::new(10.0, 20.0, 30.0);
            assert_eq!(lean_under(base, Vec3::new(5.0, 5.0, 5.0), 0.0), base);

            // A z quarter turn composed with an x quarter turn reads back as
            // x 90, z 90 (the JOML Euler ZYX of the product).
            let leaned = lean_under(Vec3::new(90.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 90.0), 1.0);
            assert!((leaned.x - 90.0).abs() < 1e-4, "x was {}", leaned.x);
            assert!(leaned.y.abs() < 1e-4, "y was {}", leaned.y);
            assert!((leaned.z - 90.0).abs() < 1e-4, "z was {}", leaned.z);

            // Half the under-turn halves the z angle.
            let half = lean_under(Vec3::new(90.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 90.0), 0.5);
            assert!((half.z - 45.0).abs() < 1e-4, "z was {}", half.z);

            // No weight leaves the shoulder where it was.
            assert_eq!(
                shoulder_on_lean(
                    Vec3::new(1.0, 2.0, 3.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(0.0, 0.0, 90.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    0.0
                ),
                Vec3::ZERO
            );

            // The arm pivots mirror each other, 2 pixels above the middle.
            let right = arm_pivot(true);
            let left = arm_pivot(false);
            assert_eq!(right, Vec3::new(5.0, 2.0, 0.0));
            assert_eq!(left, Vec3::new(-5.0, 2.0, 0.0));
        }
    }
}

pub mod camera {
    //! Punchy Moves' shoulder camera, ported 1:1 from the Java client's
    //! `punchymoves.client.camera` package: the settings (and the config clamps),
    //! the wall-collision recovery, the avatar's camera visibility, the centre
    //! aim ray's unprojection, and the controller that layers the situation
    //! camera profiles (fence, ladder, proximity, elytra, map view, ranged,
    //! passenger) over the activation transition.
    //!
    //! Pure Rust and std-only, so the tests run without a Horizon build. The game
    //! side is three small helpers the bundle injects: `list.rs` (the Options >
    //! Mods row and the two keys), `settings.rs` and `click.rs` (the page), `key.rs`
    //! (the key hooks) and `camera_hook.rs` (each frame, `native_camera_pose`).
    //!
    //! Angles are degrees and positions are world blocks. Yaw is Minecraft's: `0`
    //! faces `+Z`, `90` faces `-X`; a positive pitch looks down. That gives the
    //! forward vector `(-sin yaw, 0, cos yaw)` and the camera's right
    //! `(cos yaw, 0, sin yaw)`.

    use std::cell::RefCell;
    use std::time::{Duration, Instant};

    // ---------------------------------------------------------------------------
    // Keys and stored setting names
    // ---------------------------------------------------------------------------

    /// The key that toggles the shoulder camera, and its default: unbound until
    /// the player picks one on Options > Mods.
    pub const TOGGLE_CAMERA: &str = "camera";
    /// The Java keybind `key.punchy_moves.camera_toggle_v`, on V out of the box.
    pub const DEFAULT_TOGGLE_CAMERA: &str = "key.keyboard.v";
    /// The Java keybind `key.punchy_moves.camera_editor`, which opens the
    /// camera's settings screen.
    pub const TOGGLE_EDITOR: &str = "camera_editor";
    pub const DEFAULT_TOGGLE_EDITOR: &str = "key.keyboard.f7";
    /// The key that toggles the orbit (free) camera, unbound by default too.
    pub const TOGGLE_ORBIT: &str = "orbit";
    /// The Java keybind `key.punchy_moves.free_camera`, on B out of the box.
    pub const DEFAULT_TOGGLE_ORBIT: &str = "key.keyboard.b";

    /// The stored setting names, the Java config file's keys, one per field.
    pub const ENABLED: &str = "enabled";
    pub const HORIZONTAL: &str = "horizontal";
    pub const VERTICAL: &str = "vertical";
    pub const DEPTH: &str = "depth";
    pub const FOV: &str = "fov";
    pub const ROTATION_YAW: &str = "rotationYaw";
    pub const ROTATION_PITCH: &str = "rotationPitch";
    pub const FENCE_CAMERA: &str = "fenceCamera";
    pub const LADDER_CAMERA: &str = "ladderCamera";
    pub const PROXIMITY_CAMERA: &str = "proximityCamera";
    pub const ELYTRA_CAMERA: &str = "elytraCamera";
    pub const MAP_CAMERA: &str = "mapCamera";
    pub const BOW_AND_CROSSBOW_CAMERA_FIX: &str = "bowAndCrossbowCameraFix";
    pub const SPEAR_AND_TRIDENT_CAMERA_FIX: &str = "spearAndTridentCameraFix";
    pub const LADDER_BODY_LOCK: &str = "ladderBodyLock";

    /// How many buttons the settings page has.
    pub const BUTTONS: usize = 15;
    /// The button index of each setting, in the page's order (the order
    /// [`ShoulderCameraSettings::click`] numbers them).
    pub const ENABLED_INDEX: usize = 0;
    pub const HORIZONTAL_INDEX: usize = 1;
    pub const VERTICAL_INDEX: usize = 2;
    pub const DEPTH_INDEX: usize = 3;
    pub const FOV_INDEX: usize = 4;
    pub const YAW_INDEX: usize = 5;
    pub const PITCH_INDEX: usize = 6;
    pub const FENCE_INDEX: usize = 7;
    pub const LADDER_INDEX: usize = 8;
    pub const PROXIMITY_INDEX: usize = 9;
    pub const ELYTRA_INDEX: usize = 10;
    pub const MAP_INDEX: usize = 11;
    pub const BOW_AND_CROSSBOW_INDEX: usize = 12;
    pub const SPEAR_AND_TRIDENT_INDEX: usize = 13;
    pub const LADDER_BODY_LOCK_INDEX: usize = 14;

    // ---------------------------------------------------------------------------
    // Small math, the Java `Mth` calls the camera package uses
    // ---------------------------------------------------------------------------

    /// `Mth.clamp` for f32.
    fn clamp(value: f32, min: f32, max: f32) -> f32 {
        value.clamp(min, max)
    }

    /// `Mth.clamp` for f64.
    fn clampd(value: f64, min: f64, max: f64) -> f64 {
        value.clamp(min, max)
    }

    /// `Mth.lerp(delta, start, end)`.
    fn lerp(delta: f32, start: f32, end: f32) -> f32 {
        start + delta * (end - start)
    }

    /// The smooth 0-to-1 curve of a blend in `0.0..=1.0`.
    fn smoothstep(t: f32) -> f32 {
        let t = clamp(t, 0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// `ShoulderCameraController.smoothStep(edge0, edge1, value)`.
    fn smooth_step(edge0: f32, edge1: f32, value: f32) -> f32 {
        smoothstep((value - edge0) / (edge1 - edge0))
    }

    // ---------------------------------------------------------------------------
    // Camera profiles
    // ---------------------------------------------------------------------------

    /// `AnimationLibrary.CameraProfile`: one situation's camera, carried by a
    /// clip's `camera` block. Complete when every field is finite and the
    /// response is positive.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CameraProfile {
        /// Blocks to the side.
        pub horizontal: f32,
        /// Blocks above the eye.
        pub vertical: f32,
        /// Blocks behind the eye.
        pub depth: f32,
        /// Field of view, in degrees.
        pub fov: f32,
        /// How fast the blend moves, per second.
        pub response: f32,
    }

    impl CameraProfile {
        /// `CameraProfile.isComplete`.
        pub fn is_complete(&self) -> bool {
            self.horizontal.is_finite()
                && self.vertical.is_finite()
                && self.depth.is_finite()
                && self.fov.is_finite()
                && self.response.is_finite()
                && self.response > 0.0
        }
    }

    /// `PlayerAnimationEngine.MapViewCamera`: the camera a held-up map drives.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct MapViewCamera {
        /// Blocks to the side.
        pub horizontal: f32,
        /// Blocks above the eye.
        pub vertical: f32,
        /// Blocks behind the eye.
        pub depth: f32,
        /// Field of view, in degrees.
        pub fov: f32,
        /// How fast the blend moves, per second.
        pub response: f32,
    }

    impl MapViewCamera {
        /// The engine's `MapViewCamera.DEFAULT`.
        pub const DEFAULT: MapViewCamera = MapViewCamera {
            horizontal: 0.0,
            vertical: 0.68,
            depth: 1.0,
            fov: 55.0,
            response: 4.5,
        };
    }

    /// `PlayerAnimationEngine.PassengerView`: the camera while riding.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct PassengerView {
        /// Blocks to the side.
        pub horizontal: f32,
        /// Blocks above the eye.
        pub vertical: f32,
        /// Blocks behind the eye.
        pub depth: f32,
        /// Field of view, in degrees.
        pub fov: f32,
        /// How fast the blend moves, per second.
        pub response: f32,
    }

    // ---------------------------------------------------------------------------
    // ShoulderCameraSettings
    // ---------------------------------------------------------------------------

    /// The Java defaults.
    pub const DEFAULT_HORIZONTAL: f32 = 1.17;
    pub const DEFAULT_VERTICAL: f32 = -0.11;
    pub const DEFAULT_DEPTH: f32 = 2.5;
    pub const DEFAULT_FOV: f32 = 70.0;
    pub const DEFAULT_ROTATION_YAW: f32 = 0.0;
    pub const DEFAULT_ROTATION_PITCH: f32 = 0.0;

    /// The shoulder camera's settings: where the camera sits behind and beside the
    /// eye, how much it turns, and in which situations it may engage at all.
    ///
    /// One click on the settings page moves each value to the next step of its
    /// table, so the page needs no text entry.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ShoulderCameraSettings {
        /// The shoulder camera may engage.
        pub enabled: bool,
        /// Blocks to the right of the eye (left when negative).
        pub horizontal: f32,
        /// Blocks above the eye.
        pub vertical: f32,
        /// Blocks behind the eye the camera sits.
        pub depth: f32,
        /// The field of view the camera draws with, in degrees.
        pub fov: f32,
        /// Degrees the camera turns right of where the player looks.
        pub rotation_yaw: f32,
        /// Degrees the camera turns down of where the player looks.
        pub rotation_pitch: f32,
        /// Engage while standing on a fence, wall or other narrow-topped block.
        pub fence_camera: bool,
        /// Engage while holding a ladder or vine.
        pub ladder_camera: bool,
        /// Engage while a block is right behind the eye.
        pub proximity_camera: bool,
        /// Engage while gliding on an elytra.
        pub elytra_camera: bool,
        /// Engage while a map is held up.
        pub map_camera: bool,
        /// Draw the aim ray from the camera while aiming a bow or crossbow.
        pub bow_and_crossbow_camera_fix: bool,
        /// Draw the aim ray while aiming a spear or trident.
        pub spear_and_trident_camera_fix: bool,
        /// Keep the body facing the ladder while climbing.
        pub ladder_body_lock: bool,
    }

    /// The steps one click moves each setting to. A value between steps counts as
    /// the first.
    const HORIZONTAL_STEPS: [f32; 5] = [0.6, 0.9, 1.17, 1.5, 1.8];
    const VERTICAL_STEPS: [f32; 5] = [-0.11, 0.0, 0.25, 0.6, 1.0];
    const DEPTH_STEPS: [f32; 5] = [1.5, 2.0, 2.5, 3.5, 5.0];
    const FOV_STEPS: [f32; 5] = [50.0, 60.0, 70.0, 85.0, 100.0];
    const YAW_STEPS: [f32; 5] = [-20.0, -10.0, 0.0, 10.0, 20.0];
    const PITCH_STEPS: [f32; 5] = [-20.0, -10.0, 0.0, 10.0, 20.0];

    impl Default for ShoulderCameraSettings {
        fn default() -> Self {
            Self {
                enabled: true,
                horizontal: DEFAULT_HORIZONTAL,
                vertical: DEFAULT_VERTICAL,
                depth: DEFAULT_DEPTH,
                fov: DEFAULT_FOV,
                rotation_yaw: DEFAULT_ROTATION_YAW,
                rotation_pitch: DEFAULT_ROTATION_PITCH,
                fence_camera: true,
                ladder_camera: true,
                proximity_camera: true,
                elytra_camera: true,
                map_camera: true,
                bow_and_crossbow_camera_fix: true,
                spear_and_trident_camera_fix: true,
                ladder_body_lock: true,
            }
        }
    }

    impl ShoulderCameraSettings {
        /// The defaults.
        pub fn new() -> Self {
            Self::default()
        }

        /// The Java setters' clamps.
        pub fn set_horizontal(&mut self, value: f32) {
            self.horizontal = clamp(value, -2.0, 2.0);
        }

        /// The Java setters' clamps.
        pub fn set_vertical(&mut self, value: f32) {
            self.vertical = clamp(value, -1.5, 2.0);
        }

        /// The Java setters' clamps.
        pub fn set_depth(&mut self, value: f32) {
            self.depth = clamp(value, 1.0, 8.0);
        }

        /// The Java setters' clamps.
        pub fn set_fov(&mut self, value: f32) {
            self.fov = clamp(value, 30.0, 110.0);
        }

        /// The Java setters' clamps (`rotationDegrees`: plus or minus 30).
        pub fn set_rotation_yaw(&mut self, value: f32) {
            self.rotation_yaw = clamp(value, -30.0, 30.0);
        }

        /// The Java setters' clamps (`rotationDegrees`: plus or minus 30).
        pub fn set_rotation_pitch(&mut self, value: f32) {
            self.rotation_pitch = clamp(value, -30.0, 30.0);
        }

        /// Reads the settings from stored values (`get` reads one key); a missing
        /// or unreadable value keeps its default, and every number passes through
        /// the Java setter's clamp.
        pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
            let mut out = Self::default();
            let flag =
                |key: &str, fallback: bool| get(key).map_or(fallback, |v| v.trim() != "false");
            let number = |key: &str, clamp_fn: fn(f32) -> f32| {
                get(key)
                    .and_then(|v| v.trim().parse::<f32>().ok())
                    .filter(|v| v.is_finite())
                    .map(clamp_fn)
            };
            out.enabled = flag(ENABLED, true);
            if let Some(v) = number(HORIZONTAL, |v| clamp(v, -2.0, 2.0)) {
                out.horizontal = v;
            }
            if let Some(v) = number(VERTICAL, |v| clamp(v, -1.5, 2.0)) {
                out.vertical = v;
            }
            if let Some(v) = number(DEPTH, |v| clamp(v, 1.0, 8.0)) {
                out.depth = v;
            }
            if let Some(v) = number(FOV, |v| clamp(v, 30.0, 110.0)) {
                out.fov = v;
            }
            if let Some(v) = number(ROTATION_YAW, |v| clamp(v, -30.0, 30.0)) {
                out.rotation_yaw = v;
            }
            if let Some(v) = number(ROTATION_PITCH, |v| clamp(v, -30.0, 30.0)) {
                out.rotation_pitch = v;
            }
            out.fence_camera = flag(FENCE_CAMERA, true);
            out.ladder_camera = flag(LADDER_CAMERA, true);
            out.proximity_camera = flag(PROXIMITY_CAMERA, true);
            out.elytra_camera = flag(ELYTRA_CAMERA, true);
            out.map_camera = flag(MAP_CAMERA, true);
            out.bow_and_crossbow_camera_fix = flag(BOW_AND_CROSSBOW_CAMERA_FIX, true);
            out.spear_and_trident_camera_fix = flag(SPEAR_AND_TRIDENT_CAMERA_FIX, true);
            out.ladder_body_lock = flag(LADDER_BODY_LOCK, true);
            out
        }

        /// The settings page's buttons, in the order [`ShoulderCameraSettings::click`]
        /// numbers them.
        pub fn labels(&self) -> Vec<String> {
            let on = |b: bool| if b { "ON" } else { "OFF" };
            vec![
                format!("Shoulder camera: {}", on(self.enabled)),
                format!("Sideways: {:.2} blocks", self.horizontal),
                format!("Height: {:.2} blocks", self.vertical),
                format!("Back: {:.1} blocks", self.depth),
                format!("Field of view: {:.0} degrees", self.fov),
                format!("Turn right: {:.0} degrees", self.rotation_yaw),
                format!("Turn down: {:.0} degrees", self.rotation_pitch),
                format!("On fences: {}", on(self.fence_camera)),
                format!("On ladders: {}", on(self.ladder_camera)),
                format!("Beside walls: {}", on(self.proximity_camera)),
                format!("While gliding: {}", on(self.elytra_camera)),
                format!("With a map: {}", on(self.map_camera)),
                format!(
                    "Bow and crossbow aim fix: {}",
                    on(self.bow_and_crossbow_camera_fix)
                ),
                format!(
                    "Spear and trident aim fix: {}",
                    on(self.spear_and_trident_camera_fix)
                ),
                format!("Ladder body lock: {}", on(self.ladder_body_lock)),
            ]
        }

        /// A click on button `i` of [`ShoulderCameraSettings::labels`]; an index
        /// past the page does nothing.
        pub fn click(&mut self, i: usize) {
            match i {
                0 => self.enabled = !self.enabled,
                1 => self.set_horizontal(cycle(&HORIZONTAL_STEPS, self.horizontal)),
                2 => self.set_vertical(cycle(&VERTICAL_STEPS, self.vertical)),
                3 => self.set_depth(cycle(&DEPTH_STEPS, self.depth)),
                4 => self.set_fov(cycle(&FOV_STEPS, self.fov)),
                5 => self.set_rotation_yaw(cycle(&YAW_STEPS, self.rotation_yaw)),
                6 => self.set_rotation_pitch(cycle(&PITCH_STEPS, self.rotation_pitch)),
                7 => self.fence_camera = !self.fence_camera,
                8 => self.ladder_camera = !self.ladder_camera,
                9 => self.proximity_camera = !self.proximity_camera,
                10 => self.elytra_camera = !self.elytra_camera,
                11 => self.map_camera = !self.map_camera,
                12 => self.bow_and_crossbow_camera_fix = !self.bow_and_crossbow_camera_fix,
                13 => self.spear_and_trident_camera_fix = !self.spear_and_trident_camera_fix,
                14 => self.ladder_body_lock = !self.ladder_body_lock,
                _ => {}
            }
        }

        /// What to store, key by key.
        pub fn stored(&self) -> [(&'static str, String); BUTTONS] {
            let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
            [
                (ENABLED, flag(self.enabled)),
                (HORIZONTAL, format!("{:.2}", self.horizontal)),
                (VERTICAL, format!("{:.2}", self.vertical)),
                (DEPTH, format!("{:.2}", self.depth)),
                (FOV, format!("{:.1}", self.fov)),
                (ROTATION_YAW, format!("{:.1}", self.rotation_yaw)),
                (ROTATION_PITCH, format!("{:.1}", self.rotation_pitch)),
                (FENCE_CAMERA, flag(self.fence_camera)),
                (LADDER_CAMERA, flag(self.ladder_camera)),
                (PROXIMITY_CAMERA, flag(self.proximity_camera)),
                (ELYTRA_CAMERA, flag(self.elytra_camera)),
                (MAP_CAMERA, flag(self.map_camera)),
                (
                    BOW_AND_CROSSBOW_CAMERA_FIX,
                    flag(self.bow_and_crossbow_camera_fix),
                ),
                (
                    SPEAR_AND_TRIDENT_CAMERA_FIX,
                    flag(self.spear_and_trident_camera_fix),
                ),
                (LADDER_BODY_LOCK, flag(self.ladder_body_lock)),
            ]
        }

        /// Flips [`ShoulderCameraSettings::enabled`] (the camera key).
        pub fn toggle(&mut self) {
            self.enabled = !self.enabled;
        }
    }

    /// The next value of a step table; a value between steps counts as the first.
    fn cycle(steps: &[f32], value: f32) -> f32 {
        let at = steps
            .iter()
            .position(|step| (step - value).abs() < 1e-4)
            .unwrap_or(0);
        steps[(at + 1) % steps.len()]
    }

    // ---------------------------------------------------------------------------
    // CameraCollisionRecovery
    // ---------------------------------------------------------------------------

    /// Keeps the camera from crossing a wall without popping when the wall is
    /// gone: it follows the fraction along the arm at once when the arm shortens,
    /// and eases back out exponentially (`RECOVERY_RESPONSE` per second) when it
    /// lengthens.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CameraCollisionRecovery {
        /// How far along the arm the camera may sit, `0.0..=1.0`.
        fraction: f64,
    }

    /// `CameraCollisionRecovery.RECOVERY_RESPONSE`: the exponential recovery's
    /// rate, per second.
    pub const RECOVERY_RESPONSE: f32 = 7.0;

    impl CameraCollisionRecovery {
        /// A camera at full reach.
        pub fn new() -> Self {
            Self::default()
        }

        /// The fraction along the arm the camera sits at, `0.0..=1.0`.
        pub fn fraction(&self) -> f32 {
            self.fraction as f32
        }

        /// One frame with `blocked` the longest unblocked fraction the game found
        /// (`1.0` when nothing blocks) over `dt` seconds; returns the new fraction.
        ///
        /// A shorter arm is taken at once, so the camera never sits inside the
        /// wall for a frame; a longer one is eased out exponentially. A broken
        /// target reads as `0.0`.
        pub fn update(&mut self, blocked: f64, dt: f32) -> f64 {
            let target = if blocked.is_finite() {
                clampd(blocked, 0.0, 1.0)
            } else {
                0.0
            };
            if target <= self.fraction {
                self.fraction = target;
                return self.fraction;
            }
            let dt = clamp(dt, 0.0, 0.1) as f64;
            let step = 1.0 - (-RECOVERY_RESPONSE as f64 * dt).exp();
            self.fraction += (target - self.fraction) * step;
            if target - self.fraction < 1.0e-4 {
                self.fraction = target;
            }
            self.fraction
        }

        /// Back to full reach.
        pub fn reset(&mut self) {
            self.fraction = 1.0;
        }
    }

    impl Default for CameraCollisionRecovery {
        fn default() -> Self {
            Self { fraction: 1.0 }
        }
    }

    // ---------------------------------------------------------------------------
    // AvatarCameraVisibility
    // ---------------------------------------------------------------------------

    /// `AvatarCameraVisibility`: how drawn the local avatar is with the shoulder
    /// camera up — skipped outright when the camera is inside the body, faded by
    /// the angle to the camera and the distance behind the player otherwise.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct AvatarCameraVisibility {
        /// `0.15..=1.0` while drawn.
        pub alpha: f32,
        /// The avatar is not submitted at all this frame.
        pub skip_avatar: bool,
    }

    /// The fully drawn answer.
    const AVATAR_VISIBLE: AvatarCameraVisibility = AvatarCameraVisibility {
        alpha: 1.0,
        skip_avatar: false,
    };

    /// `AvatarCameraVisibility.sample`.
    ///
    /// `moved` and `to_avatar` are the two position deltas from the camera
    /// (`a - camera` and `b - camera`), `dir_a` and `dir_b` the two directions
    /// the visibility is measured against, and the sizes are the player's bounding
    /// box width and height and eye height, in blocks.
    #[allow(clippy::too_many_arguments)] // the Java sample takes the same eight
    pub fn avatar_camera_visibility(
        moved: [f64; 3],
        to_avatar: [f64; 3],
        dir_a: [f64; 3],
        dir_b: [f64; 3],
        bb_width: f64,
        bb_height: f64,
        eye_height: f64,
        activating: bool,
    ) -> AvatarCameraVisibility {
        if !activating && distance_squared(moved, to_avatar) <= 1.0e-8 {
            return AVATAR_VISIBLE;
        }
        let width = bb_width.max(0.001);
        let len = length(to_avatar);
        let near = width * 0.75;
        if len < near {
            return AvatarCameraVisibility {
                alpha: 1.0,
                skip_avatar: true,
            };
        }
        let far = width * 2.5;
        if len >= far {
            return AVATAR_VISIBLE;
        }
        let across = dot(to_avatar, dir_a).abs();
        let along = dot(to_avatar, dir_b);
        let half = width * 0.5;
        let eye = eye_height.max(0.001);
        let above_eye = (bb_height - eye).max(0.001);
        if across >= half || along <= -eye || along >= above_eye {
            return AVATAR_VISIBLE;
        }
        let side = clampd(across / half, 0.0, 1.0);
        let stacked = if along >= 0.0 {
            clampd(along / above_eye, 0.0, 1.0)
        } else {
            clampd(-along / eye, 0.0, 1.0)
        };
        let angled = clamp((side * side + stacked * stacked).sqrt() as f32, 0.15, 1.0);
        let t = clampd((len - near) / (far - near), 0.0, 1.0);
        let eased = (t * t * (3.0 - 2.0 * t)) as f32;
        AvatarCameraVisibility {
            alpha: lerp(eased, angled, 1.0),
            skip_avatar: false,
        }
    }

    /// The squared distance between two points.
    fn distance_squared(a: [f64; 3], b: [f64; 3]) -> f64 {
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        dot(d, d)
    }

    /// The dot product of two vectors.
    fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    /// The length of a vector.
    fn length(a: [f64; 3]) -> f64 {
        dot(a, a).sqrt()
    }

    // ---------------------------------------------------------------------------
    // CenterAimRay
    // ---------------------------------------------------------------------------

    /// A 4×4 matrix in column-major order, the order Joml's `Matrix4f` fills a
    /// `FloatBuffer` with: elements `4*c..4*c+4` are column `c`.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Mat4 {
        pub(crate) columns: [[f32; 4]; 4],
    }

    impl Mat4 {
        /// Reads a column-major 16-float array.
        pub fn from_column_major(values: &[f32; 16]) -> Self {
            let mut columns = [[0.0f32; 4]; 4];
            for (c, column) in columns.iter_mut().enumerate() {
                *column = [
                    values[c * 4],
                    values[c * 4 + 1],
                    values[c * 4 + 2],
                    values[c * 4 + 3],
                ];
            }
            Self { columns }
        }

        /// The matrix times another: `self * other` (apply `other` first).
        pub fn mul(&self, other: &Mat4) -> Mat4 {
            let mut out = [[0.0f32; 4]; 4];
            for (c, column) in out.iter_mut().enumerate() {
                for (r, cell) in column.iter_mut().enumerate() {
                    let mut sum = 0.0f32;
                    for k in 0..4 {
                        sum += self.columns[k][r] * other.columns[c][k];
                    }
                    *cell = sum;
                }
            }
            Mat4 { columns: out }
        }

        /// The matrix times a column vector.
        pub fn transform(&self, v: [f32; 4]) -> [f32; 4] {
            let mut out = [0.0f32; 4];
            for (r, cell) in out.iter_mut().enumerate() {
                let mut sum = 0.0f32;
                for (c, value) in v.iter().enumerate() {
                    sum += self.columns[c][r] * value;
                }
                *cell = sum;
            }
            out
        }

        /// The determinant.
        pub fn determinant(&self) -> f32 {
            let m = self.columns;
            let mut det = 0.0f32;
            for c in 0..4 {
                // Laplace expansion along the first row, via 3x3 minors.
                let minor = |skip: usize| {
                    let mut sub = [[0.0f32; 3]; 3];
                    let mut sc = 0usize;
                    for (cc, column) in m.iter().enumerate() {
                        if cc == skip {
                            continue;
                        }
                        for r in 0..3 {
                            sub[r][sc] = column[r + 1];
                        }
                        sc += 1;
                    }
                    sub[0][0] * (sub[1][1] * sub[2][2] - sub[1][2] * sub[2][1])
                        - sub[0][1] * (sub[1][0] * sub[2][2] - sub[1][2] * sub[2][0])
                        + sub[0][2] * (sub[1][0] * sub[2][1] - sub[1][1] * sub[2][0])
                };
                let sign = if c % 2 == 0 { 1.0 } else { -1.0 };
                det += sign * m[c][0] * minor(c);
            }
            det
        }

        /// Whether every element is finite.
        pub fn is_finite(&self) -> bool {
            self.columns.iter().flatten().all(|c| c.is_finite())
        }

        /// The translation column.
        pub fn translation(&self) -> [f32; 3] {
            [self.columns[3][0], self.columns[3][1], self.columns[3][2]]
        }

        /// The scale factors per axis, from the column lengths.
        pub fn scale_factors(&self) -> [f32; 3] {
            [
                (self.columns[0][0] * self.columns[0][0]
                    + self.columns[0][1] * self.columns[0][1]
                    + self.columns[0][2] * self.columns[0][2])
                    .sqrt(),
                (self.columns[1][0] * self.columns[1][0]
                    + self.columns[1][1] * self.columns[1][1]
                    + self.columns[1][2] * self.columns[1][2])
                    .sqrt(),
                (self.columns[2][0] * self.columns[2][0]
                    + self.columns[2][1] * self.columns[2][1]
                    + self.columns[2][2] * self.columns[2][2])
                    .sqrt(),
            ]
        }

        /// `getUnnormalizedRotation`: the rotation the upper-left 3x3 carries,
        /// as a quaternion.
        pub fn unnormalized_rotation(&self) -> super::engine::Quat {
            // Shear-free rotation extraction from the matrix, via the standard
            // ZYX decomposition scaled by the column lengths.
            let scale = self.scale_factors();
            let mut m = [[0.0f32; 3]; 3];
            for (r, row) in m.iter_mut().enumerate() {
                for (c, value) in row.iter_mut().enumerate() {
                    *value = self.columns[c][r] / scale[c];
                }
            }
            let x = (-m[1][2]).atan2(m[2][2]);
            let y = m[0][2].clamp(-1.0, 1.0).asin();
            let z = (-m[0][1]).atan2(m[0][0]);
            super::engine::Quat::from_zyx_degrees(super::library::Vec3::new(
                x.to_degrees(),
                y.to_degrees(),
                z.to_degrees(),
            ))
        }

        /// The inverse, by Gauss-Jordan with partial pivoting; `None` when
        /// singular.
        pub fn invert(&self) -> Option<Mat4> {
            let mut a = [[0.0f64; 8]; 4];
            for (c, column) in self.columns.iter().enumerate() {
                for (r, value) in column.iter().enumerate() {
                    a[r][c] = *value as f64;
                }
                a[c][4 + c] = 1.0;
            }
            for col in 0..4 {
                let pivot = (col..4)
                    .max_by(|x, y| a[*x][col].abs().total_cmp(&a[*y][col].abs()))
                    .filter(|row| a[*row][col].abs() > 1.0e-12)?;
                a.swap(col, pivot);
                let pivot_row = a[col];
                let pivot_value = pivot_row[col];
                for (r, row) in a.iter_mut().enumerate() {
                    if r == col {
                        continue;
                    }
                    let factor = row[col] / pivot_value;
                    if factor != 0.0 {
                        for (c, cell) in row.iter_mut().enumerate() {
                            *cell -= factor * pivot_row[c];
                        }
                    }
                }
                for cell in a[col].iter_mut() {
                    *cell /= pivot_value;
                }
            }
            let mut columns = [[0.0f32; 4]; 4];
            for c in 0..4 {
                for r in 0..4 {
                    columns[c][r] = a[r][4 + c] as f32;
                }
            }
            Some(Mat4 { columns })
        }
    }

    /// The ray from the centre of the picture, out of the camera: what the
    /// crosshair points along, whatever the camera does.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CenterAimRay {
        /// Where the ray starts, in world blocks.
        pub origin: [f64; 3],
        /// Its unit direction.
        pub direction: [f64; 3],
    }

    impl CenterAimRay {
        /// The ray of a camera at `origin` looking along `yaw` and `pitch`.
        pub fn from_look(origin: [f64; 3], yaw: f32, pitch: f32) -> Self {
            Self {
                origin,
                direction: look_direction(yaw, pitch),
            }
        }

        /// The ray of a camera pose.
        pub fn from_pose(pose: &CameraPose) -> Self {
            Self::from_look(pose.position, pose.yaw, pose.pitch)
        }

        /// The point `distance` along the ray.
        pub fn at(&self, distance: f64) -> [f64; 3] {
            [
                self.origin[0] + self.direction[0] * distance,
                self.origin[1] + self.direction[1] * distance,
                self.origin[2] + self.direction[2] * distance,
            ]
        }

        /// `CenterAimRay.unproject`: the ray the middle of the picture looks
        /// along, from the view matrix and the projection matrix (both read as
        /// column-major 16-float arrays). The math runs in `f32`, like Joml's.
        ///
        /// Returns `None` when either matrix is singular or the result is not a
        /// usable finite ray.
        pub fn unproject(view: &[f32; 16], projection: &[f32; 16]) -> Option<Self> {
            let combined = Mat4::from_column_major(projection)
                .mul(&Mat4::from_column_major(view))
                .invert()?;
            let direction_point = combined.transform([0.0, 0.0, 1.0, 0.0]);
            let mid_point = combined.transform([0.0, 0.0, 0.5, 1.0]);
            let finite4 = |v: [f32; 4]| v.iter().all(|c| c.is_finite());
            if !finite4(direction_point) || !finite4(mid_point) {
                return None;
            }
            if direction_point[3].abs() < 1.0e-7 || mid_point[3].abs() < 1.0e-7 {
                return None;
            }
            let origin = [
                (direction_point[0] / direction_point[3]) as f64,
                (direction_point[1] / direction_point[3]) as f64,
                (direction_point[2] / direction_point[3]) as f64,
            ];
            let through = [
                (mid_point[0] / mid_point[3]) as f64,
                (mid_point[1] / mid_point[3]) as f64,
                (mid_point[2] / mid_point[3]) as f64,
            ];
            let direction = [
                through[0] - origin[0],
                through[1] - origin[1],
                through[2] - origin[2],
            ];
            if !direction.iter().all(|c| c.is_finite()) || !origin.iter().all(|c| c.is_finite()) {
                return None;
            }
            let squared = dot(direction, direction);
            if squared < 1.0e-12 {
                return None;
            }
            let scale = squared.sqrt();
            Some(Self {
                origin,
                direction: [
                    direction[0] / scale,
                    direction[1] / scale,
                    direction[2] / scale,
                ],
            })
        }
    }

    /// The unit direction `yaw` and `pitch` look along, yaw `0` facing `+Z` and a
    /// positive pitch looking down.
    pub fn look_direction(yaw: f32, pitch: f32) -> [f64; 3] {
        let yaw = (yaw as f64).to_radians();
        let pitch = (pitch as f64).to_radians();
        let (sin_yaw, cos_yaw) = (yaw.sin(), yaw.cos());
        let (sin_pitch, cos_pitch) = (pitch.sin(), pitch.cos());
        [-sin_yaw * cos_pitch, -sin_pitch, cos_yaw * cos_pitch]
    }

    // ---------------------------------------------------------------------------
    // ShoulderCameraController
    // ---------------------------------------------------------------------------

    /// The camera this frame is drawn from.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CameraPose {
        /// Where the camera sits, in world blocks.
        pub position: [f64; 3],
        /// Its yaw in degrees.
        pub yaw: f32,
        /// Its pitch in degrees.
        pub pitch: f32,
        /// How far the shoulder camera has engaged, `0.0..=1.0`.
        pub weight: f32,
    }

    impl CameraPose {
        /// The centre-screen aim ray of this pose.
        pub fn aim_ray(&self) -> CenterAimRay {
            CenterAimRay::from_look(self.position, self.yaw, self.pitch)
        }
    }

    /// `CAMERA_ENTER_SECONDS`: how long the activation blend takes to engage.
    pub const CAMERA_ENTER_SECONDS: f32 = 0.9;
    /// `CAMERA_EXIT_SECONDS`: how long it takes to disengage.
    pub const CAMERA_EXIT_SECONDS: f32 = 0.72;
    /// `FIRST_PERSON_RENDER_GRACE_NANOS`: how long the avatar stays hidden after
    /// the camera hands back to first person.
    pub const FIRST_PERSON_RENDER_GRACE: Duration = Duration::from_nanos(200_000_000);
    /// The response a profile without one falls back to.
    pub const DEFAULT_PROFILE_RESPONSE: f32 = 4.0;
    /// The depth below which the transition hides the local player.
    pub const TRANSITION_HIDE_DEPTH: f32 = 0.58;
    /// How far a free-camera turn moves per mouse pixel, in degrees.
    pub const FREE_CAMERA_TURN: f32 = 0.15;

    /// Blends the shoulder camera in and out and layers the situation camera
    /// profiles over it, the Java `ShoulderCameraController`.
    ///
    /// The Java class keeps its state in statics; here it is an instance the game
    /// glue owns through [`controller`], so tests can build their own.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ShoulderCameraController {
        /// Whether the vanilla camera is in third person back right now; the
        /// glue sets this each frame, `isActive` needs it.
        pub third_person: bool,
        activated: bool,
        requested: bool,
        activation_blend: f32,
        transition_horizontal: f32,
        transition_vertical: f32,
        transition_depth: f32,
        transition_fov: f32,
        fence_blend: f32,
        fence_latched: bool,
        proximity_blend: f32,
        proximity_latched: bool,
        proximity_mining_hold: f32,
        elytra_blend: f32,
        ladder_blend: f32,
        map_view_blend: f32,
        ranged_blend: f32,
        passenger_blend: f32,
        passenger_view: Option<PassengerView>,
        fence_profile: Option<CameraProfile>,
        proximity_profile: Option<CameraProfile>,
        elytra_profile: Option<CameraProfile>,
        ladder_profile: Option<CameraProfile>,
        ranged_profile: Option<CameraProfile>,
        map_view: MapViewCamera,
        hide_local_player_until: Option<Instant>,
        avatar_alpha: f32,
        avatar_skip: bool,
        spyglass_takeover: bool,
        free_camera: bool,
        free_camera_yaw: f32,
        free_camera_pitch: f32,
        vanilla_perspective_override: bool,
        collision: CameraCollisionRecovery,
    }

    impl Default for ShoulderCameraController {
        fn default() -> Self {
            Self {
                third_person: true,
                activated: false,
                requested: false,
                activation_blend: 0.0,
                transition_horizontal: 0.0,
                transition_vertical: 0.0,
                transition_depth: 0.0,
                transition_fov: 0.0,
                fence_blend: 0.0,
                fence_latched: false,
                proximity_blend: 0.0,
                proximity_latched: false,
                proximity_mining_hold: 0.0,
                elytra_blend: 0.0,
                ladder_blend: 0.0,
                map_view_blend: 0.0,
                ranged_blend: 0.0,
                passenger_blend: 0.0,
                passenger_view: None,
                fence_profile: None,
                proximity_profile: None,
                elytra_profile: None,
                ladder_profile: None,
                ranged_profile: None,
                map_view: MapViewCamera::DEFAULT,
                hide_local_player_until: None,
                avatar_alpha: 1.0,
                avatar_skip: false,
                spyglass_takeover: false,
                free_camera: false,
                free_camera_yaw: 0.0,
                free_camera_pitch: 0.0,
                vanilla_perspective_override: false,
                collision: CameraCollisionRecovery::default(),
            }
        }
    }

    /// What [`ShoulderCameraController::toggle`] and the free camera's toggle
    /// tell the player.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CameraMessage {
        /// The shoulder camera turned on.
        CameraEnabled,
        /// The shoulder camera turned off.
        CameraDisabled,
        /// The free camera needs the shoulder camera first.
        FreeCameraRequiresCustom,
        /// The free camera turned on.
        FreeCameraEnabled,
        /// The free camera turned off.
        FreeCameraDisabled,
    }

    impl CameraMessage {
        /// The translation key the game glue shows.
        pub fn key(self) -> &'static str {
            match self {
                CameraMessage::CameraEnabled => "punchy_moves.camera.enabled",
                CameraMessage::CameraDisabled => "punchy_moves.camera.disabled",
                CameraMessage::FreeCameraRequiresCustom => {
                    "punchy_moves.free_camera.requires_custom"
                }
                CameraMessage::FreeCameraEnabled => "punchy_moves.free_camera.enabled",
                CameraMessage::FreeCameraDisabled => "punchy_moves.free_camera.disabled",
            }
        }
    }

    impl ShoulderCameraController {
        /// A controller resting on vanilla's camera.
        pub fn new() -> Self {
            Self::default()
        }

        /// `isActive`: engaged and the vanilla camera is in third person back.
        pub fn is_active(&self) -> bool {
            self.activated && self.third_person
        }

        /// `hasCustomCamera`.
        pub fn has_custom_camera(&self) -> bool {
            self.activated && self.third_person
        }

        /// `isActivationTransitionRunning`.
        pub fn is_activation_transition_running(&self) -> bool {
            self.activated && (!self.requested || self.activation_blend < 1.0)
        }

        /// How far the activation blend has gone, `0.0..=1.0`.
        pub fn activation_blend(&self) -> f32 {
            self.activation_blend
        }

        /// Whether the camera is asked for.
        pub fn requested(&self) -> bool {
            self.requested
        }

        /// `updateActivationTransition`: eases the activation blend, resumes a
        /// saved camera, and drops the camera when vanilla left third person.
        pub fn update_activation_transition(&mut self, dt: f32, settings: &ShoulderCameraSettings) {
            if self.activated && !self.third_person {
                self.on_vanilla_perspective_change();
            }
            self.resume_saved_camera(settings);
            if self.activated {
                let seconds = if self.requested {
                    CAMERA_ENTER_SECONDS
                } else {
                    CAMERA_EXIT_SECONDS
                };
                let direction = if self.requested { 1.0 } else { -1.0 };
                self.activation_blend = clamp(
                    self.activation_blend + direction * clamp(dt, 0.0, 0.1) / seconds,
                    0.0,
                    1.0,
                );
            }
        }

        /// `updateSpyglassCameraTransition`: the spyglass takes the camera over
        /// and hands it back.
        pub fn update_spyglass_camera_transition(
            &mut self,
            taking: bool,
            settings: &ShoulderCameraSettings,
        ) {
            if taking {
                if !self.spyglass_takeover && self.third_person && self.activated && self.requested
                {
                    self.spyglass_takeover = true;
                    self.stop_free_camera(true);
                    self.capture_transition_target(true, settings);
                    self.requested = false;
                }
            } else if self.spyglass_takeover {
                self.spyglass_takeover = false;
                self.activated = true;
                self.requested = true;
                self.capture_transition_target(false, settings);
            }
        }

        /// `finishActivationFrame`: once the exit blend has settled, hand back to
        /// first person and hide the player for the render grace.
        pub fn finish_activation_frame(&mut self, now: Instant) {
            if self.activated && !self.requested && self.activation_blend <= 0.0 {
                self.hide_local_player_until = Some(now + FIRST_PERSON_RENDER_GRACE);
                self.activated = false;
                self.reset_dynamic_transitions();
            }
        }

        /// `updateFenceTransition`: `disengage` and `engage` are the two latch
        /// inputs the game glue reads off the block under the player.
        pub fn update_fence_transition(
            &mut self,
            disengage: bool,
            engage: bool,
            dt: f32,
            profile: Option<CameraProfile>,
        ) {
            if profile.is_some_and(|p| p.is_complete()) {
                self.fence_profile = profile;
            }
            let complete = self.fence_profile.is_some_and(|p| p.is_complete());
            if !complete {
                self.fence_latched = false;
            } else if engage {
                self.fence_latched = true;
            } else if disengage {
                self.fence_latched = false;
            }
            let target = if self.fence_latched { 1.0 } else { 0.0 };
            let response = 1.0 - (-self.response(self.fence_profile) * clamp(dt, 0.0, 0.1)).exp();
            self.fence_blend = lerp(response, self.fence_blend, target);
            if (self.fence_blend - target).abs() < 1.0e-4 {
                self.fence_blend = target;
            }
        }

        /// `updateProximityTransition`: `distance` is how far the block behind
        /// the eye is, `mining` whether the player is breaking a block, and
        /// `blocked` whether something else holds the camera off.
        #[allow(clippy::too_many_arguments)]
        pub fn update_proximity_transition(
            &mut self,
            distance: f64,
            mining: bool,
            blocked: bool,
            dt: f32,
            profile: Option<CameraProfile>,
            proximity_blocks: f32,
            mining_blocks: f32,
            hold_seconds: f32,
            engage_response: f32,
            release_response: f32,
        ) {
            if profile.is_some_and(|p| p.is_complete()) {
                self.proximity_profile = profile;
            }
            let complete = self.proximity_profile.is_some_and(|p| p.is_complete());
            if complete {
                let near = proximity_blocks.max(0.0);
                let far = near.max(mining_blocks);
                if mining && distance.is_finite() && distance <= far as f64 {
                    self.proximity_mining_hold = hold_seconds.max(0.0);
                } else {
                    self.proximity_mining_hold =
                        (self.proximity_mining_hold - dt.max(0.0)).max(0.0);
                }
                let close = !blocked && distance.is_finite() && distance <= near as f64;
                self.proximity_latched = close || self.proximity_mining_hold > 1.0e-4;
            } else {
                self.proximity_latched = false;
                self.proximity_mining_hold = 0.0;
            }
            let target = if self.proximity_latched { 1.0 } else { 0.0 };
            let response = if self.proximity_latched {
                engage_response.max(0.1)
            } else {
                release_response.max(0.1)
            };
            let step = 1.0 - (-response * clamp(dt, 0.0, 0.1)).exp();
            self.proximity_blend = lerp(step, self.proximity_blend, target);
            if (self.proximity_blend - target).abs() < 1.0e-4 {
                self.proximity_blend = target;
            }
        }

        /// `updateElytraTransition`.
        pub fn update_elytra_transition(
            &mut self,
            gliding: bool,
            dt: f32,
            profile: Option<CameraProfile>,
        ) {
            if profile.is_some_and(|p| p.is_complete()) {
                self.elytra_profile = profile;
            }
            let engaged = gliding && self.elytra_profile.is_some_and(|p| p.is_complete());
            let target = if engaged { 1.0 } else { 0.0 };
            let response = 1.0 - (-self.response(self.elytra_profile) * clamp(dt, 0.0, 0.1)).exp();
            self.elytra_blend = lerp(response, self.elytra_blend, target);
            if (self.elytra_blend - target).abs() < 1.0e-4 {
                self.elytra_blend = target;
            }
        }

        /// `updateLadderTransition`.
        pub fn update_ladder_transition(
            &mut self,
            climbing: bool,
            dt: f32,
            profile: Option<CameraProfile>,
        ) {
            if profile.is_some_and(|p| p.is_complete()) {
                self.ladder_profile = profile;
            }
            let engaged = climbing && self.ladder_profile.is_some_and(|p| p.is_complete());
            let target = if engaged { 1.0 } else { 0.0 };
            let response = 1.0 - (-self.response(self.ladder_profile) * clamp(dt, 0.0, 0.1)).exp();
            self.ladder_blend = lerp(response, self.ladder_blend, target);
            if (self.ladder_blend - target).abs() < 1.0e-4 {
                self.ladder_blend = target;
            }
        }

        /// `updateMapViewTransition`.
        pub fn update_map_view_transition(&mut self, held: bool, dt: f32) {
            let target = if held { 1.0 } else { 0.0 };
            let response = 1.0 - (-self.map_view.response.max(0.1) * clamp(dt, 0.0, 0.1)).exp();
            self.map_view_blend = lerp(response, self.map_view_blend, target);
            if (self.map_view_blend - target).abs() < 1.0e-4 {
                self.map_view_blend = target;
            }
        }

        /// `updateRangedCameraTransition`.
        pub fn update_ranged_camera_transition(
            &mut self,
            aiming: bool,
            dt: f32,
            profile: Option<CameraProfile>,
        ) {
            if profile.is_some_and(|p| p.is_complete()) {
                self.ranged_profile = profile;
            }
            let target = if aiming && self.ranged_profile.is_some_and(|p| p.is_complete()) {
                1.0
            } else {
                0.0
            };
            let response = 1.0 - (-self.response(self.ranged_profile) * clamp(dt, 0.0, 0.1)).exp();
            self.ranged_blend = lerp(response, self.ranged_blend, target);
            if (self.ranged_blend - target).abs() < 1.0e-4 {
                self.ranged_blend = target;
            }
        }

        /// `updatePassengerTransition`.
        pub fn update_passenger_transition(&mut self, view: Option<PassengerView>, dt: f32) {
            if view.is_some() {
                self.passenger_view = view;
            }
            let target = if view.is_none() { 0.0 } else { 1.0 };
            let response = match view {
                Some(passenger) => passenger.response,
                None => self
                    .passenger_view
                    .map_or(4.0, |passenger| passenger.response),
            };
            let step = 1.0 - (-response.max(0.1) * clamp(dt, 0.0, 0.1)).exp();
            self.passenger_blend = lerp(step, self.passenger_blend, target);
            if (self.passenger_blend - target).abs() < 1.0e-4 {
                self.passenger_blend = target;
                if target == 0.0 {
                    self.passenger_view = None;
                }
            }
        }

        /// `resetDynamicTransitions`.
        pub fn reset_dynamic_transitions(&mut self) {
            self.fence_blend = 0.0;
            self.fence_latched = false;
            self.proximity_blend = 0.0;
            self.proximity_latched = false;
            self.proximity_mining_hold = 0.0;
            self.elytra_blend = 0.0;
            self.ladder_blend = 0.0;
            self.map_view_blend = 0.0;
            self.ranged_blend = 0.0;
            self.passenger_blend = 0.0;
            self.passenger_view = None;
            self.fence_profile = None;
            self.proximity_profile = None;
            self.elytra_profile = None;
            self.ladder_profile = None;
            self.ranged_profile = None;
            self.avatar_alpha = 1.0;
            self.avatar_skip = false;
        }

        /// `updateLocalAvatarCameraVisibility`: reworks the avatar's alpha from
        /// the camera position, two player positions and two directions.
        #[allow(clippy::too_many_arguments)]
        pub fn update_local_avatar_camera_visibility(
            &mut self,
            active: bool,
            bb_width: f64,
            bb_height: f64,
            eye_height: f64,
            camera: [f64; 3],
            pos_a: [f64; 3],
            pos_b: [f64; 3],
            dir_a: [f64; 3],
            dir_b: [f64; 3],
        ) {
            self.avatar_alpha = 1.0;
            self.avatar_skip = false;
            if active && self.is_active() {
                let visibility = avatar_camera_visibility(
                    [
                        pos_a[0] - camera[0],
                        pos_a[1] - camera[1],
                        pos_a[2] - camera[2],
                    ],
                    [
                        pos_b[0] - camera[0],
                        pos_b[1] - camera[1],
                        pos_b[2] - camera[2],
                    ],
                    dir_a,
                    dir_b,
                    bb_width,
                    bb_height,
                    eye_height,
                    self.is_activation_transition_running(),
                );
                self.avatar_alpha = visibility.alpha;
                self.avatar_skip = visibility.skip_avatar;
            }
        }

        /// `localAvatarAlpha`.
        pub fn local_avatar_alpha(&self) -> f32 {
            clamp(self.avatar_alpha, 0.0, 1.0)
        }

        /// `shouldSkipLocalAvatarForCameraCollision`.
        pub fn should_skip_local_avatar_for_camera_collision(&self) -> bool {
            self.is_active() && self.avatar_skip
        }

        /// `effectiveHorizontal`.
        pub fn effective_horizontal(&self, settings: &ShoulderCameraSettings) -> f32 {
            let contextual = self.contextual_horizontal(self.smooth_proximity_blend(), settings);
            if self.activated && self.activation_blend < 1.0 {
                self.transition_horizontal * smooth_step(0.18, 1.0, self.activation_blend)
            } else {
                contextual
            }
        }

        /// `effectiveVertical`.
        pub fn effective_vertical(&self, settings: &ShoulderCameraSettings) -> f32 {
            let contextual = self.contextual_vertical(self.smooth_proximity_blend(), settings);
            if self.activated && self.activation_blend < 1.0 {
                self.transition_vertical * smooth_step(0.12, 1.0, self.activation_blend)
            } else {
                contextual
            }
        }

        /// `effectiveDepth`.
        pub fn effective_depth(&self, settings: &ShoulderCameraSettings) -> f32 {
            let contextual = self.contextual_depth(self.smooth_proximity_blend(), settings);
            if self.activated && self.activation_blend < 1.0 {
                self.transition_depth * smooth_step(0.0, 1.0, self.activation_blend)
            } else {
                contextual
            }
        }

        /// `effectiveFov`: `vanilla_fov` is the options' field of view.
        pub fn effective_fov(&self, settings: &ShoulderCameraSettings, vanilla_fov: f32) -> f32 {
            let contextual = self.contextual_fov(self.smooth_proximity_blend(), settings);
            if self.activated && self.activation_blend < 1.0 {
                lerp(
                    smooth_step(0.3, 1.0, self.activation_blend),
                    vanilla_fov,
                    self.transition_fov,
                )
            } else {
                contextual
            }
        }

        /// `shouldHideLocalPlayerDuringTransition`.
        pub fn should_hide_local_player_during_transition(
            &self,
            settings: &ShoulderCameraSettings,
            now: Instant,
        ) -> bool {
            self.is_active()
                && (self
                    .hide_local_player_until
                    .is_some_and(|until| now < until)
                    || (self.is_activation_transition_running()
                        && self.effective_depth(settings) < TRANSITION_HIDE_DEPTH))
        }

        /// `proximityNeutralHorizontal`: the horizontal offset with the proximity
        /// blend forced to neutral.
        pub fn proximity_neutral_horizontal(&self, settings: &ShoulderCameraSettings) -> f32 {
            self.contextual_horizontal(0.0, settings)
        }

        /// `proximityNeutralVertical`.
        pub fn proximity_neutral_vertical(&self, settings: &ShoulderCameraSettings) -> f32 {
            self.contextual_vertical(0.0, settings)
        }

        /// `proximityNeutralDepth`.
        pub fn proximity_neutral_depth(&self, settings: &ShoulderCameraSettings) -> f32 {
            self.contextual_depth(0.0, settings)
        }

        fn contextual_horizontal(&self, proximity: f32, settings: &ShoulderCameraSettings) -> f32 {
            let base = lerp(
                self.smooth_fence_blend(),
                settings.horizontal,
                self.profile_horizontal(self.fence_profile, settings),
            );
            let with_proximity = lerp(
                proximity,
                base,
                self.profile_horizontal(self.proximity_profile, settings),
            );
            let with_ladder = lerp(
                self.smooth_ladder_blend(),
                with_proximity,
                self.profile_horizontal(self.ladder_profile, settings),
            );
            let with_elytra = lerp(
                self.smooth_elytra_blend(),
                with_ladder,
                self.profile_horizontal(self.elytra_profile, settings),
            );
            let with_passenger = match self.passenger_view {
                Some(passenger) => lerp(
                    self.smooth_passenger_blend(),
                    with_elytra,
                    passenger.horizontal,
                ),
                None => with_elytra,
            };
            let with_map = lerp(
                self.smooth_map_view_blend(),
                with_passenger,
                self.map_view.horizontal,
            );
            lerp(
                self.smooth_ranged_blend(),
                with_map,
                self.profile_horizontal(self.ranged_profile, settings),
            )
        }

        fn contextual_vertical(&self, proximity: f32, settings: &ShoulderCameraSettings) -> f32 {
            let base = lerp(
                self.smooth_fence_blend(),
                settings.vertical,
                self.profile_vertical(self.fence_profile, settings),
            );
            let with_proximity = lerp(
                proximity,
                base,
                self.profile_vertical(self.proximity_profile, settings),
            );
            let with_ladder = lerp(
                self.smooth_ladder_blend(),
                with_proximity,
                self.profile_vertical(self.ladder_profile, settings),
            );
            let with_elytra = lerp(
                self.smooth_elytra_blend(),
                with_ladder,
                self.profile_vertical(self.elytra_profile, settings),
            );
            let with_passenger = match self.passenger_view {
                Some(passenger) => lerp(
                    self.smooth_passenger_blend(),
                    with_elytra,
                    passenger.vertical,
                ),
                None => with_elytra,
            };
            let with_map = lerp(
                self.smooth_map_view_blend(),
                with_passenger,
                self.map_view.vertical,
            );
            lerp(
                self.smooth_ranged_blend(),
                with_map,
                self.profile_vertical(self.ranged_profile, settings),
            )
        }

        fn contextual_depth(&self, proximity: f32, settings: &ShoulderCameraSettings) -> f32 {
            let base = lerp(
                self.smooth_fence_blend(),
                settings.depth,
                self.profile_depth(self.fence_profile, settings),
            );
            let with_proximity = lerp(
                proximity,
                base,
                self.profile_depth(self.proximity_profile, settings),
            );
            let with_ladder = lerp(
                self.smooth_ladder_blend(),
                with_proximity,
                self.profile_depth(self.ladder_profile, settings),
            );
            let with_elytra = lerp(
                self.smooth_elytra_blend(),
                with_ladder,
                self.profile_depth(self.elytra_profile, settings),
            );
            let with_passenger = match self.passenger_view {
                Some(passenger) => {
                    lerp(self.smooth_passenger_blend(), with_elytra, passenger.depth)
                }
                None => with_elytra,
            };
            let with_map = lerp(
                self.smooth_map_view_blend(),
                with_passenger,
                self.map_view.depth,
            );
            lerp(
                self.smooth_ranged_blend(),
                with_map,
                self.profile_depth(self.ranged_profile, settings),
            )
        }

        fn contextual_fov(&self, proximity: f32, settings: &ShoulderCameraSettings) -> f32 {
            let base = lerp(
                self.smooth_fence_blend(),
                settings.fov,
                self.profile_fov(self.fence_profile, settings),
            );
            let with_proximity = lerp(
                proximity,
                base,
                self.profile_fov(self.proximity_profile, settings),
            );
            let with_ladder = lerp(
                self.smooth_ladder_blend(),
                with_proximity,
                self.profile_fov(self.ladder_profile, settings),
            );
            let with_elytra = lerp(
                self.smooth_elytra_blend(),
                with_ladder,
                self.profile_fov(self.elytra_profile, settings),
            );
            let with_passenger = match self.passenger_view {
                Some(passenger) => lerp(self.smooth_passenger_blend(), with_elytra, passenger.fov),
                None => with_elytra,
            };
            let with_map = lerp(
                self.smooth_map_view_blend(),
                with_passenger,
                self.map_view.fov,
            );
            lerp(
                self.smooth_ranged_blend(),
                with_map,
                self.profile_fov(self.ranged_profile, settings),
            )
        }

        fn smooth_fence_blend(&self) -> f32 {
            smoothstep(self.fence_blend)
        }

        fn smooth_proximity_blend(&self) -> f32 {
            smoothstep(self.proximity_blend)
        }

        fn smooth_elytra_blend(&self) -> f32 {
            smoothstep(self.elytra_blend)
        }

        fn smooth_ladder_blend(&self) -> f32 {
            smoothstep(self.ladder_blend)
        }

        fn smooth_map_view_blend(&self) -> f32 {
            smoothstep(self.map_view_blend)
        }

        fn smooth_ranged_blend(&self) -> f32 {
            smoothstep(self.ranged_blend)
        }

        fn smooth_passenger_blend(&self) -> f32 {
            smoothstep(self.passenger_blend)
        }

        fn profile_horizontal(
            &self,
            profile: Option<CameraProfile>,
            settings: &ShoulderCameraSettings,
        ) -> f32 {
            profile
                .filter(CameraProfile::is_complete)
                .map_or(settings.horizontal, |p| p.horizontal)
        }

        fn profile_vertical(
            &self,
            profile: Option<CameraProfile>,
            settings: &ShoulderCameraSettings,
        ) -> f32 {
            profile
                .filter(CameraProfile::is_complete)
                .map_or(settings.vertical, |p| p.vertical)
        }

        fn profile_depth(
            &self,
            profile: Option<CameraProfile>,
            settings: &ShoulderCameraSettings,
        ) -> f32 {
            profile
                .filter(CameraProfile::is_complete)
                .map_or(settings.depth, |p| p.depth)
        }

        fn profile_fov(
            &self,
            profile: Option<CameraProfile>,
            settings: &ShoulderCameraSettings,
        ) -> f32 {
            profile
                .filter(CameraProfile::is_complete)
                .map_or(settings.fov, |p| p.fov)
        }

        fn response(&self, profile: Option<CameraProfile>) -> f32 {
            profile
                .filter(CameraProfile::is_complete)
                .map_or(DEFAULT_PROFILE_RESPONSE, |p| p.response)
        }

        /// `toggle`: flips the camera and reports the message to show. The glue
        /// persists `settings` and applies the returned free-camera restore.
        pub fn toggle(
            &mut self,
            settings: &mut ShoulderCameraSettings,
        ) -> (CameraMessage, Option<(f32, f32)>) {
            self.vanilla_perspective_override = false;
            self.hide_local_player_until = None;
            let engaging = !self.requested;
            let mut restore = None;
            if !engaging {
                restore = self.stop_free_camera(true);
            }
            if engaging {
                if !self.activated {
                    self.activation_blend = 0.0;
                }
                self.activated = true;
                self.capture_transition_target(false, settings);
            } else {
                self.capture_transition_target(true, settings);
            }
            self.requested = engaging;
            settings.enabled = engaging;
            (
                if engaging {
                    CameraMessage::CameraEnabled
                } else {
                    CameraMessage::CameraDisabled
                },
                restore,
            )
        }

        /// `onVanillaPerspectiveChange`.
        pub fn on_vanilla_perspective_change(&mut self) {
            self.vanilla_perspective_override = true;
            self.hide_local_player_until = None;
            let _ = self.stop_free_camera(true);
            self.activated = false;
            self.requested = false;
            self.activation_blend = 0.0;
            self.spyglass_takeover = false;
            self.reset_dynamic_transitions();
        }

        /// `setEnabledFromEditor`.
        pub fn set_enabled_from_editor(
            &mut self,
            enabled: bool,
            settings: &mut ShoulderCameraSettings,
        ) -> Option<(f32, f32)> {
            self.vanilla_perspective_override = false;
            self.hide_local_player_until = None;
            let mut restore = None;
            if !enabled {
                restore = self.stop_free_camera(true);
            }
            if enabled {
                if !self.activated {
                    self.activation_blend = 0.0;
                }
                self.activated = true;
                self.capture_transition_target(false, settings);
            } else if self.activated {
                self.capture_transition_target(true, settings);
            }
            self.requested = enabled;
            settings.enabled = enabled;
            restore
        }

        /// `captureTransitionTarget`: while disengaging, the camera eases towards
        /// where it is now; while engaging, towards the settings.
        fn capture_transition_target(&mut self, exiting: bool, settings: &ShoulderCameraSettings) {
            self.transition_horizontal = if exiting {
                self.contextual_horizontal(self.smooth_proximity_blend(), settings)
            } else {
                settings.horizontal
            };
            self.transition_vertical = if exiting {
                self.contextual_vertical(self.smooth_proximity_blend(), settings)
            } else {
                settings.vertical
            };
            self.transition_depth = if exiting {
                self.contextual_depth(self.smooth_proximity_blend(), settings)
            } else {
                settings.depth
            };
            self.transition_fov = if exiting {
                self.contextual_fov(self.smooth_proximity_blend(), settings)
            } else {
                settings.fov
            };
        }

        /// `toggleFreeCamera`: `player_yaw` and `player_pitch` are the player's
        /// rotation to remember; the restore pair goes back to the player.
        pub fn toggle_free_camera(
            &mut self,
            player_yaw: f32,
            player_pitch: f32,
        ) -> (CameraMessage, Option<(f32, f32)>) {
            if !self.is_active() {
                return (CameraMessage::FreeCameraRequiresCustom, None);
            }
            let mut restore = None;
            if self.free_camera {
                restore = self.stop_free_camera(true);
            } else {
                self.free_camera = true;
                self.free_camera_yaw = player_yaw;
                self.free_camera_pitch = player_pitch;
            }
            (
                if self.free_camera {
                    CameraMessage::FreeCameraEnabled
                } else {
                    CameraMessage::FreeCameraDisabled
                },
                restore,
            )
        }

        /// `turnFreeCamera`.
        pub fn turn_free_camera(&mut self, dx: f64, dy: f64) -> bool {
            if !self.free_camera {
                return false;
            }
            if !self.is_active() {
                self.free_camera = false;
                return false;
            }
            self.free_camera_yaw += dx as f32 * FREE_CAMERA_TURN;
            self.free_camera_pitch = clamp(
                self.free_camera_pitch + dy as f32 * FREE_CAMERA_TURN,
                -90.0,
                90.0,
            );
            true
        }

        /// `isFreeCameraActive`.
        pub fn is_free_camera_active(&self) -> bool {
            self.free_camera && self.is_active()
        }

        /// The free camera's remembered yaw.
        pub fn free_camera_yaw(&self) -> f32 {
            self.free_camera_yaw
        }

        /// The free camera's remembered pitch.
        pub fn free_camera_pitch(&self) -> f32 {
            self.free_camera_pitch
        }

        /// `stopFreeCamera`: returns the rotation to put back on the player.
        pub fn stop_free_camera(&mut self, restore: bool) -> Option<(f32, f32)> {
            if self.free_camera {
                let pair = (self.free_camera_yaw, self.free_camera_pitch);
                if restore {
                    self.free_camera = false;
                    return Some(pair);
                }
                self.free_camera = false;
            }
            None
        }

        /// `resumeSavedCamera`: a third-person camera left over from before the
        /// mod loaded re-engages the shoulder camera.
        pub fn resume_saved_camera(&mut self, settings: &ShoulderCameraSettings) {
            if !self.vanilla_perspective_override
                && !self.activated
                && !self.spyglass_takeover
                && settings.enabled
                && self.third_person
            {
                self.activation_blend = 0.0;
                self.activated = true;
                self.requested = true;
                self.capture_transition_target(false, settings);
            }
        }

        /// The camera this frame, from the eye `eye` the vanilla camera sits at,
        /// the player's `yaw` and `pitch`, and the settings: the effective
        /// offsets, scaled by the activation blend and pulled in by the wall
        /// recovery.
        pub fn compute_pose(
            &self,
            eye: [f64; 3],
            yaw: f32,
            pitch: f32,
            settings: &ShoulderCameraSettings,
        ) -> CameraPose {
            let weight = clamp(self.activation_blend, 0.0, 1.0);
            let horizontal = self.effective_horizontal(settings) as f64 * weight as f64;
            let vertical = self.effective_vertical(settings) as f64 * weight as f64;
            let depth = self.effective_depth(settings) as f64
                * weight as f64
                * self.collision.fraction() as f64;

            let (sin_yaw, cos_yaw) = (
                (yaw as f64).to_radians().sin(),
                (yaw as f64).to_radians().cos(),
            );
            // Forward at yaw 0 is +Z, right is +X; the camera sits back along
            // forward and out along right.
            let forward = [-sin_yaw, 0.0, cos_yaw];
            let right = [cos_yaw, 0.0, sin_yaw];
            let position = [
                eye[0] + right[0] * horizontal - forward[0] * depth,
                eye[1] + vertical,
                eye[2] + right[2] * horizontal - forward[2] * depth,
            ];
            let pose_yaw = yaw + settings.rotation_yaw * weight;
            let pose_pitch = (pitch + settings.rotation_pitch * weight).clamp(-90.0, 90.0);
            CameraPose {
                position,
                yaw: pose_yaw,
                pitch: pose_pitch,
                weight,
            }
        }

        /// The wall recovery.
        pub fn collision(&self) -> &CameraCollisionRecovery {
            &self.collision
        }

        /// The wall recovery, to change it.
        pub fn collision_mut(&mut self) -> &mut CameraCollisionRecovery {
            &mut self.collision
        }

        /// The map view camera the blends read.
        pub fn map_view(&self) -> MapViewCamera {
            self.map_view
        }

        /// Replaces the map view camera (the engine loaded a new one).
        pub fn set_map_view_camera(&mut self, map_view: MapViewCamera) {
            self.map_view = map_view;
        }
    }

    // ---------------------------------------------------------------------------
    // The frame the hooks call
    // ---------------------------------------------------------------------------

    #[derive(Default)]
    struct State {
        /// Read on the first frame, then kept.
        settings: Option<ShoulderCameraSettings>,
        /// The controller's state, while a camera is on.
        controller: Option<ShoulderCameraController>,
        /// When `frame_dt` last ran, for the frame's delta.
        last_frame: Option<Instant>,
    }

    thread_local! {
        static STATE: RefCell<State> = RefCell::new(State::default());
    }

    /// The settings, read once through `get`.
    pub fn settings(get: impl Fn(&str) -> Option<String>) -> ShoulderCameraSettings {
        STATE.with_borrow_mut(|s| {
            *s.settings
                .get_or_insert_with(|| ShoulderCameraSettings::read(get))
        })
    }

    /// Replaces the settings (the settings page or a key changed them).
    pub fn set_settings(settings: ShoulderCameraSettings) {
        STATE.with_borrow_mut(|s| s.settings = Some(settings));
    }

    /// Runs `f` on the controller, creating it on the first call.
    pub fn controller<T>(f: impl FnOnce(&mut ShoulderCameraController) -> T) -> T {
        STATE.with_borrow_mut(|s| {
            f(s.controller
                .get_or_insert_with(ShoulderCameraController::new))
        })
    }

    /// Forgets the controller: the next frame starts from vanilla's camera again.
    pub fn reset() {
        STATE.with_borrow_mut(|s| s.controller = None);
    }

    /// Seconds since the last call, for the blend's frame step: the first call and
    /// a stalled frame both read `1/20`, and no step exceeds the transitions'
    /// `0.1` clamp.
    pub fn frame_dt() -> f32 {
        STATE.with_borrow_mut(|s| {
            let now = Instant::now();
            let dt = s
                .last_frame
                .map_or(1.0 / 20.0, |last| now.duration_since(last).as_secs_f32());
            s.last_frame = Some(now);
            dt.clamp(0.0, 0.1)
        })
    }

    /// Advances the camera by `dt` seconds with `blocked` the unblocked fraction
    /// the game found, and returns the pose to draw from, or `None` to keep
    /// vanilla's.
    pub fn frame_pose(
        eye: [f64; 3],
        yaw: f32,
        pitch: f32,
        dt: f32,
        blocked: f32,
        settings: &ShoulderCameraSettings,
    ) -> Option<CameraPose> {
        if !settings.enabled {
            reset();
            return None;
        }
        STATE.with_borrow_mut(|state| {
            let controller = state
                .controller
                .get_or_insert_with(ShoulderCameraController::new);
            controller.resume_saved_camera(settings);
            controller.update_activation_transition(dt, settings);
            controller.collision_mut().update(blocked as f64, dt);
            if controller.activation_blend() <= 0.0 {
                None
            } else {
                Some(controller.compute_pose(eye, yaw, pitch, settings))
            }
        })
    }

    /// The camera key: flips the controller and the settings, and reports the
    /// message to show and the free-camera rotation to put back on the player.
    pub fn toggle_camera(
        get: impl Fn(&str) -> Option<String>,
    ) -> (ShoulderCameraSettings, CameraMessage, Option<(f32, f32)>) {
        STATE.with_borrow_mut(|s| {
            let settings = s
                .settings
                .get_or_insert_with(|| ShoulderCameraSettings::read(get));
            let controller = s
                .controller
                .get_or_insert_with(ShoulderCameraController::new);
            let (message, restore) = controller.toggle(settings);
            (*settings, message, restore)
        })
    }

    /// The orbit key: toggles the free camera; the restore pair goes back on the
    /// player.
    pub fn toggle_orbit(
        get: impl Fn(&str) -> Option<String>,
        player_yaw: f32,
        player_pitch: f32,
    ) -> (CameraMessage, Option<(f32, f32)>) {
        STATE.with_borrow_mut(|s| {
            s.settings
                .get_or_insert_with(|| ShoulderCameraSettings::read(get));
            let controller = s
                .controller
                .get_or_insert_with(ShoulderCameraController::new);
            controller.toggle_free_camera(player_yaw, player_pitch)
        })
    }

    /// A free-camera mouse turn; `false` when the free camera is not running.
    pub fn turn_orbit(dx: f64, dy: f64) -> bool {
        controller(|c| c.turn_free_camera(dx, dy))
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn stored_values<'a>(
            values: &'a [(&'a str, &'a str)],
        ) -> impl Fn(&str) -> Option<String> + 'a {
            move |key| {
                values
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.to_string())
            }
        }

        #[test]
        fn defaults_are_usable_and_complete() {
            let settings = ShoulderCameraSettings::default();
            assert!(settings.enabled);
            assert_eq!(settings.horizontal, 1.17);
            assert_eq!(settings.vertical, -0.11);
            assert_eq!(settings.depth, 2.5);
            assert_eq!(settings.fov, 70.0);
            assert_eq!(settings.rotation_yaw, 0.0);
            assert_eq!(settings.rotation_pitch, 0.0);
            assert!(settings.fence_camera);
            assert!(settings.ladder_camera);
            assert!(settings.proximity_camera);
            assert!(settings.elytra_camera);
            assert!(settings.map_camera);
            assert!(settings.bow_and_crossbow_camera_fix);
            assert!(settings.spear_and_trident_camera_fix);
            assert!(settings.ladder_body_lock);
        }

        #[test]
        fn the_setters_clamp_like_the_java_config() {
            let mut settings = ShoulderCameraSettings::default();
            settings.set_horizontal(5.0);
            assert_eq!(settings.horizontal, 2.0);
            settings.set_vertical(-4.0);
            assert_eq!(settings.vertical, -1.5);
            settings.set_depth(0.2);
            assert_eq!(settings.depth, 1.0);
            settings.set_fov(200.0);
            assert_eq!(settings.fov, 110.0);
            settings.set_rotation_yaw(45.0);
            assert_eq!(settings.rotation_yaw, 30.0);
            settings.set_rotation_pitch(-45.0);
            assert_eq!(settings.rotation_pitch, -30.0);
        }

        #[test]
        fn the_button_indices_name_the_settings_they_change() {
            assert_eq!(ENABLED_INDEX, 0);
            assert_eq!(HORIZONTAL_INDEX, 1);
            assert_eq!(VERTICAL_INDEX, 2);
            assert_eq!(DEPTH_INDEX, 3);
            assert_eq!(FOV_INDEX, 4);
            assert_eq!(YAW_INDEX, 5);
            assert_eq!(PITCH_INDEX, 6);
            assert_eq!(FENCE_INDEX, 7);
            assert_eq!(LADDER_INDEX, 8);
            assert_eq!(PROXIMITY_INDEX, 9);
            assert_eq!(ELYTRA_INDEX, 10);
            assert_eq!(MAP_INDEX, 11);
            assert_eq!(BOW_AND_CROSSBOW_INDEX, 12);
            assert_eq!(SPEAR_AND_TRIDENT_INDEX, 13);
            assert_eq!(LADDER_BODY_LOCK_INDEX, 14);
            assert_eq!(BUTTONS, 15);
        }

        #[test]
        fn read_keeps_defaults_when_a_key_is_missing_and_clamps_numbers() {
            let settings = ShoulderCameraSettings::read(stored_values(&[
                (HORIZONTAL, "9"),
                (DEPTH, "0.5"),
                (FENCE_CAMERA, "false"),
                (LADDER_BODY_LOCK, "false"),
            ]));
            assert_eq!(settings.horizontal, 2.0, "9 clamps to 2");
            assert_eq!(settings.depth, 1.0, "0.5 clamps up to 1");
            assert!(!settings.fence_camera);
            assert!(!settings.ladder_body_lock);
            assert_eq!(settings.vertical, DEFAULT_VERTICAL, "missing keeps default");
            assert!(settings.elytra_camera, "missing keeps the default true");
        }

        #[test]
        fn click_cycles_every_button_and_wraps() {
            let mut settings = ShoulderCameraSettings::default();
            settings.click(HORIZONTAL_INDEX);
            assert_eq!(settings.horizontal, 1.5, "1.17 is a step, so the next one");
            settings.click(FOV_INDEX);
            assert_eq!(settings.fov, 85.0, "70 is a step, so the next one");
            settings.click(FOV_INDEX);
            assert_eq!(settings.fov, 100.0);
            settings.click(FOV_INDEX);
            assert_eq!(settings.fov, 50.0, "wrapped past the end");
            settings.click(LADDER_BODY_LOCK_INDEX);
            assert!(!settings.ladder_body_lock);
            settings.click(BUTTONS + 3);
            assert!(!settings.ladder_body_lock, "past the page does nothing");
        }

        #[test]
        fn settings_round_trip_through_stored() {
            let mut settings = ShoulderCameraSettings::default();
            settings.click(ENABLED_INDEX);
            settings.set_horizontal(0.75);
            settings.set_rotation_yaw(12.0);
            let stored = settings.stored();
            assert_eq!(stored.len(), BUTTONS);
            let pairs: Vec<(&str, &str)> = stored.iter().map(|(k, v)| (&**k, &**v)).collect();
            let reread = ShoulderCameraSettings::read(stored_values(&pairs));
            assert_eq!(reread, settings);
            assert_eq!(stored[5].0, ROTATION_YAW);
            assert_eq!(stored[12].0, BOW_AND_CROSSBOW_CAMERA_FIX);
        }

        #[test]
        fn collision_pulls_in_at_once_and_recovers_exponentially() {
            let mut recovery = CameraCollisionRecovery::new();
            assert_eq!(recovery.fraction(), 1.0);
            // A shorter arm is taken at once.
            assert_eq!(recovery.update(0.25, 0.05), 0.25);
            // A longer arm eases out: one 0.1 s step closes 1 - e^-0.7 of the gap.
            let step = 1.0 - (-7.0f64 * 0.1).exp();
            let grown = recovery.update(1.0, 0.1);
            let expected = 0.25 + (1.0 - 0.25) * step;
            assert!((grown - expected).abs() < 1e-4, "{grown} vs {expected}");
            // A broken target reads as 0.0 and pulls the camera all the way in.
            recovery.reset();
            assert_eq!(recovery.update(f64::NAN, 0.05), 0.0);
        }

        #[test]
        fn the_avatar_is_skipped_up_close_and_faded_inside_the_band() {
            let width = 0.6;
            let height = 1.8;
            let eye = 1.62;
            // Inside 0.75 widths of the camera: skipped outright.
            let close = avatar_camera_visibility(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, -0.2],
                [0.0, 0.0, -1.0],
                [0.0, 1.0, 0.0],
                width,
                height,
                eye,
                false,
            );
            assert!(close.skip_avatar);
            // Far beyond 2.5 widths: fully drawn.
            let far = avatar_camera_visibility(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, -5.0],
                [0.0, 0.0, -1.0],
                [0.0, 1.0, 0.0],
                width,
                height,
                eye,
                false,
            );
            assert!(!far.skip_avatar);
            assert_eq!(far.alpha, 1.0);
            // In between, straight down the camera's axis: the distance fade
            // only, from the 0.15 angle floor towards 1.
            let mid = avatar_camera_visibility(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, -1.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                width,
                height,
                eye,
                false,
            );
            assert!(!mid.skip_avatar);
            assert!((mid.alpha - 1.0).abs() > 1.0e-3);
            assert!(mid.alpha > 0.15);
            // Just outside the skip radius, dead centre: the 0.15 angle floor.
            let aside = avatar_camera_visibility(
                [0.0, 0.0, 0.0],
                [0.001, 0.0, -0.46],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                width,
                height,
                eye,
                false,
            );
            assert!((aside.alpha - 0.15).abs() < 0.01, "{}", aside.alpha);
        }

        #[test]
        fn unproject_aims_through_the_middle_of_the_picture() {
            // A view that translates the world by (1, 2, 3) and a projection
            // whose clip w is `1 - z`: the ray through NDC (0, 0) starts at
            // (-1, -2, -2) and runs straight down -Z.
            let view: [f32; 16] = [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                1.0, 2.0, 3.0, 1.0, // translation column
            ];
            let projection: [f32; 16] = [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, -1.0, // clip w = 1 - z
                0.0, 0.0, 0.0, 1.0,
            ];
            let ray = CenterAimRay::unproject(&view, &projection).expect("a usable ray");
            for (got, want) in ray.origin.iter().zip([-1.0, -2.0, -2.0]) {
                assert!((got - want).abs() < 1e-5, "{:?} vs {:?}", ray.origin, want);
            }
            assert!((ray.direction[0]).abs() < 1e-6);
            assert!((ray.direction[1]).abs() < 1e-6);
            assert!((ray.direction[2] + 1.0).abs() < 1e-6);
            let through = ray.at(3.0);
            assert!((through[2] + 5.0).abs() < 1e-5);
            // A singular matrix has no ray.
            let singular: [f32; 16] = [0.0; 16];
            assert!(CenterAimRay::unproject(&singular, &projection).is_none());
        }

        #[test]
        fn the_activation_blend_rises_and_falls_at_its_rates() {
            let mut settings = ShoulderCameraSettings::default();
            let mut controller = ShoulderCameraController::new();
            assert!(!controller.is_active());
            let (message, _) = controller.toggle(&mut settings);
            assert_eq!(message, CameraMessage::CameraEnabled);
            assert!(settings.enabled);
            assert!(controller.is_active());
            // 0.45 s of a 0.9 s engagement is half way.
            for _ in 0..5 {
                controller.update_activation_transition(0.09, &settings);
            }
            assert!((controller.activation_blend() - 0.5).abs() < 1e-4);
            // Back out at the slower-looking but shorter exit rate.
            let (message, _) = controller.toggle(&mut settings);
            assert_eq!(message, CameraMessage::CameraDisabled);
            assert!(!settings.enabled);
            for _ in 0..4 {
                controller.update_activation_transition(0.09, &settings);
            }
            assert!(
                (controller.activation_blend()).abs() < 1e-3,
                "0.36 s of the 0.72 s exit empties the 0.5 blend"
            );
            // Mid-exit with the depth still low, the player is hidden.
            assert!(
                controller.should_hide_local_player_during_transition(&settings, Instant::now())
            );
            // Settled, the frame hands back to first person.
            for _ in 0..20 {
                controller.update_activation_transition(0.1, &settings);
            }
            controller.finish_activation_frame(Instant::now());
            assert!(!controller.is_active());
            assert!(
                !controller.should_hide_local_player_during_transition(&settings, Instant::now())
            );
        }

        #[test]
        fn a_profile_blends_the_situation_camera_over_the_settings() {
            let settings = ShoulderCameraSettings::default();
            let mut controller = ShoulderCameraController::new();
            controller.activated = true;
            controller.requested = true;
            controller.activation_blend = 1.0;
            let profile = CameraProfile {
                horizontal: 2.0,
                vertical: 1.0,
                depth: 4.0,
                fov: 90.0,
                response: 10.0,
            };
            // The fence latch needs a complete profile and the setting on.
            for _ in 0..30 {
                controller.update_fence_transition(false, true, 0.1, Some(profile));
            }
            assert!((controller.fence_blend - 1.0).abs() < 1e-3);
            // A complete profile replaces the held one; an incomplete one does not.
            let broken = CameraProfile {
                response: f32::NAN,
                ..profile
            };
            controller.update_fence_transition(true, false, 0.1, Some(broken));
            assert_eq!(
                controller.profile_horizontal(controller.fence_profile, &settings),
                2.0
            );
            // Settled, the effective offset is the profile's.
            for _ in 0..30 {
                controller.update_fence_transition(false, true, 0.1, None);
            }
            assert!((controller.effective_horizontal(&settings) - 2.0).abs() < 1e-3);
            assert!((controller.effective_depth(&settings) - 4.0).abs() < 1e-3);
            assert!((controller.effective_fov(&settings, 70.0) - 90.0).abs() < 1e-3);
        }

        #[test]
        fn the_proximity_camera_latches_and_holds_while_mining() {
            let mut controller = ShoulderCameraController::new();
            controller.activated = true;
            controller.requested = true;
            controller.activation_blend = 1.0;
            let profile = CameraProfile {
                horizontal: 0.0,
                vertical: 0.5,
                depth: 1.0,
                fov: 70.0,
                response: 8.0,
            };
            // Close to a wall: latched.
            controller.update_proximity_transition(
                0.4,
                false,
                false,
                0.1,
                Some(profile),
                0.5,
                1.0,
                0.3,
                6.0,
                3.0,
            );
            assert!(controller.proximity_latched);
            // Walk away: it releases and eases back.
            controller
                .update_proximity_transition(3.0, false, false, 0.1, None, 0.5, 1.0, 0.3, 6.0, 3.0);
            assert!(!controller.proximity_latched);
            // Mining within the mining distance starts the hold.
            controller
                .update_proximity_transition(0.9, true, false, 0.1, None, 0.5, 1.0, 0.3, 6.0, 3.0);
            assert!(controller.proximity_latched);
            // The hold survives walking away for its seconds, then expires.
            controller
                .update_proximity_transition(3.0, false, false, 0.1, None, 0.5, 1.0, 0.3, 6.0, 3.0);
            assert!(controller.proximity_latched, "hold still running");
            controller
                .update_proximity_transition(3.0, false, false, 0.2, None, 0.5, 1.0, 0.3, 6.0, 3.0);
            assert!(!controller.proximity_latched, "hold expired");
        }

        #[test]
        fn the_free_camera_turns_and_restores() {
            let mut controller = ShoulderCameraController::new();
            let (message, _) = controller.toggle_free_camera(20.0, 5.0);
            assert_eq!(message, CameraMessage::FreeCameraRequiresCustom);
            controller.activated = true;
            controller.requested = true;
            let (message, restore) = controller.toggle_free_camera(20.0, 5.0);
            assert_eq!(message, CameraMessage::FreeCameraEnabled);
            assert_eq!(restore, None);
            assert!(controller.is_free_camera_active());
            // 10 pixels right and 1000 up: yaw moves 1.5, pitch clamps at 90.
            assert!(controller.turn_free_camera(10.0, 1000.0));
            assert!((controller.free_camera_yaw() - 21.5).abs() < 1e-4);
            assert_eq!(controller.free_camera_pitch(), 90.0);
            let (message, restore) = controller.toggle_free_camera(0.0, 0.0);
            assert_eq!(message, CameraMessage::FreeCameraDisabled);
            assert_eq!(restore, Some((21.5, 90.0)));
            assert!(!controller.is_free_camera_active());
        }

        #[test]
        fn a_disengaged_camera_leaves_the_eye_alone() {
            let settings = ShoulderCameraSettings::default();
            let controller = ShoulderCameraController::new();
            let pose = controller.compute_pose([1.0, 2.0, 3.0], 0.0, 0.0, &settings);
            assert_eq!(pose.position, [1.0, 2.0, 3.0]);
            assert_eq!(pose.weight, 0.0);
        }

        #[test]
        fn an_engaged_camera_offsets_the_eye_by_the_settings() {
            let mut settings = ShoulderCameraSettings::default();
            let mut controller = ShoulderCameraController::new();
            controller.toggle(&mut settings);
            controller.activation_blend = 1.0;
            controller.resume_saved_camera(&settings);
            let pose = controller.compute_pose([1.0, 2.0, 3.0], 0.0, 0.0, &settings);
            // Facing +Z, the camera sits 1.17 right, 0.11 down... no: vertical
            // -0.11 lowers it, and 2.5 back along -Z.
            assert!((pose.position[0] - (1.0 + 1.17)).abs() < 1e-6);
            assert!((pose.position[1] - (2.0 - 0.11)).abs() < 1e-6);
            assert!((pose.position[2] - (3.0 - 2.5)).abs() < 1e-6);
            assert_eq!(pose.weight, 1.0);
        }

        #[test]
        fn the_aim_ray_leaves_the_camera_along_its_look() {
            let pose = CameraPose {
                position: [0.0, 64.0, 0.0],
                yaw: 90.0,
                pitch: 0.0,
                weight: 1.0,
            };
            let ray = pose.aim_ray();
            // Yaw 90 faces -X.
            assert!((ray.direction[0] + 1.0).abs() < 1e-9);
            assert!((ray.direction[2]).abs() < 1e-9);
            let at = ray.at(4.0);
            assert!((at[0] + 4.0).abs() < 1e-9);
            assert!((at[1] - 64.0).abs() < 1e-9);
        }

        #[test]
        fn the_frame_keeps_no_camera_while_it_is_off() {
            let settings = ShoulderCameraSettings {
                enabled: false,
                ..ShoulderCameraSettings::default()
            };
            STATE.with_borrow_mut(|s| *s = State::default());
            assert!(frame_pose([0.0; 3], 0.0, 0.0, 0.05, 1.0, &settings).is_none());
            STATE.with_borrow_mut(|s| *s = State::default());
        }

        #[test]
        fn frame_dt_is_bounded() {
            STATE.with_borrow_mut(|s| *s = State::default());
            let first = frame_dt();
            assert!(
                (first - 1.0 / 20.0).abs() < 1e-6,
                "the first frame reads 1/20"
            );
            let second = frame_dt();
            assert!(second <= 0.1, "no step exceeds the transitions' clamp");
            STATE.with_borrow_mut(|s| *s = State::default());
        }

        #[test]
        fn the_spyglass_takes_the_camera_and_hands_it_back() {
            let mut settings = ShoulderCameraSettings::default();
            let mut controller = ShoulderCameraController::new();
            controller.toggle(&mut settings);
            controller.update_spyglass_camera_transition(true, &settings);
            assert!(controller.spyglass_takeover);
            assert!(!controller.requested);
            controller.update_spyglass_camera_transition(false, &settings);
            assert!(!controller.spyglass_takeover);
            assert!(controller.requested);
            assert!(controller.activated);
        }

        #[test]
        fn a_vanilla_perspective_change_drops_everything() {
            let mut settings = ShoulderCameraSettings::default();
            let mut controller = ShoulderCameraController::new();
            controller.toggle(&mut settings);
            controller.update_fence_transition(
                false,
                true,
                0.1,
                Some(CameraProfile {
                    horizontal: 2.0,
                    vertical: 1.0,
                    depth: 4.0,
                    fov: 90.0,
                    response: 10.0,
                }),
            );
            controller.on_vanilla_perspective_change();
            assert!(!controller.activated);
            assert!(!controller.requested);
            assert_eq!(controller.activation_blend(), 0.0);
            assert_eq!(controller.fence_blend, 0.0);
            assert!(controller.fence_profile.is_none());
        }

        #[test]
        fn a_saved_third_person_camera_re_engages() {
            let settings = ShoulderCameraSettings::default();
            let mut controller = ShoulderCameraController::new();
            controller.resume_saved_camera(&settings);
            assert!(controller.activated);
            assert!(controller.requested);
            // The settings off, it stays off.
            let mut off = settings;
            off.enabled = false;
            let mut controller = ShoulderCameraController::new();
            controller.resume_saved_camera(&off);
            assert!(!controller.activated);
        }
    }
}

pub mod engine {
    //! Punchy Moves' player animation engine: the humanoid pose it builds, the
    //! controllers that feed it, and the per-player state the game's pose hooks
    //! read.
    //!
    //! Pure Rust and std-only, so the tests run without a Horizon build. Nothing
    //! here knows about the game: a pose is seven [`PartTransform`]s, and the
    //! hooks in the bundle translate them into the game's own part poses
    //! (`h_pose.rs`), its whole-model matrix (`h_body.rs`) and the state the
    //! models are posed with (`h_pose_start.rs`).
    //!
    //! Units: rotations are radians in every part (what a model part takes),
    //! positions are blocks, and angles the settings or signals speak in are
    //! degrees and converted once here.

    use super::library::{Clip, LocomotionInfluence, OverlayMode, Pose, ResolvedOverlay, Vec3};
    use std::collections::HashMap;
    use std::time::Instant;

    use super::faces::{FaceQuads, FaceRig};
    use super::signals::{BasePoseRetention, MotionSignals};

    /// The hand action an emote plays: our emote glue's marker, not a Java type.
    /// The Java emote system plays a named clip; the game glue translates this
    /// into its own hand swing.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ItemAction {
        /// The hand swings.
        #[default]
        Swing,
        /// The hand holds something up.
        Use,
        /// The hand eats.
        Eat,
        /// The hand charges a draw.
        Charge,
    }

    impl ItemAction {
        /// The action the signals imply, for the emote the player just played.
        pub fn from_signals(signals: &MotionSignals) -> Self {
            if signals.charging_crossbow || signals.charging_trident {
                ItemAction::Charge
            } else if signals.mining {
                ItemAction::Swing
            } else {
                ItemAction::Use
            }
        }
    }

    /// The numbers the engine's own controllers run at. The pack's tuning lives
    /// in [`super::signals::tuning`]; these are the engine's defaults.
    #[derive(Debug, Clone)]
    pub struct EngineTuning {
        /// Blocks of travel per full gait cycle.
        pub stride_distance: f32,
        /// Swing scale of the gait at walking speed.
        pub gait_amplitude: f32,
        /// Weight change per second when easing between poses.
        pub fade_rate: f32,
        /// The turn-in-place settings the controllers read.
        pub turn_settings: TurnSettings,
    }

    impl Default for EngineTuning {
        fn default() -> Self {
            Self {
                stride_distance: 2.0,
                gait_amplitude: 1.0,
                fade_rate: 6.0,
                turn_settings: TurnSettings::default(),
            }
        }
    }

    impl EngineTuning {
        /// The engine's defaults.
        pub fn new() -> Self {
            Self::default()
        }
    }

    /// One input axis clamped into `-1.0..=1.0`; NaN becomes `0.0`.
    fn clamp_axis(value: f32) -> f32 {
        if value.is_nan() {
            0.0
        } else {
            value.clamp(-1.0, 1.0)
        }
    }

    /// The length of the movement input vector, capped at `1.0`.
    fn input_length(signals: &MotionSignals) -> f32 {
        let forward = clamp_axis(signals.walk_forward);
        let left = clamp_axis(signals.walk_left);
        (forward * forward + left * left).sqrt().min(1.0)
    }

    /// Whether the player is in water on or under the surface.
    fn swimming(signals: &MotionSignals) -> bool {
        signals.surface_swimming || signals.submerged_swimming
    }

    /// `current` moved towards `target` by at most `rate * dt`.
    fn fade(current: f32, target: f32, dt: f32, rate: f32) -> f32 {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let rate = if rate.is_finite() { rate.max(0.0) } else { 0.0 };
        approach(current, target, rate * dt)
    }

    // ---------------------------------------------------------------------------
    // HumanoidPose
    // ---------------------------------------------------------------------------

    /// The seven parts of the humanoid player model, in draw order.
    pub const PART_NAMES: [&str; 7] = [
        "head",
        "body",
        "right_arm",
        "left_arm",
        "right_leg",
        "left_leg",
        "cape",
    ];

    /// One part's transform: a rotation about each model axis and an offset, all
    /// added on top of whatever the body already has. A positive `x_rot` swings a
    /// limb forward, a positive `y_rot` turns it left, a positive `z_rot` rolls it.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct PartTransform {
        pub x_rot: f32,
        pub y_rot: f32,
        pub z_rot: f32,
        pub x: f32,
        pub y: f32,
        pub z: f32,
        /// The scale multipliers the part carries (1.0 keeps the vanilla size).
        pub x_scale: f32,
        pub y_scale: f32,
        pub z_scale: f32,
    }

    impl PartTransform {
        /// A part that adds nothing.
        pub const IDENTITY: PartTransform = PartTransform {
            x_rot: 0.0,
            y_rot: 0.0,
            z_rot: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            x_scale: 1.0,
            y_scale: 1.0,
            z_scale: 1.0,
        };

        /// This transform blended towards `other` by `t`, clamped to `0.0..=1.0`.
        pub fn lerp(self, other: PartTransform, t: f32) -> PartTransform {
            let t = if t.is_finite() {
                t.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mix = |a: f32, b: f32| a + (b - a) * t;
            PartTransform {
                x_rot: mix(self.x_rot, other.x_rot),
                y_rot: mix(self.y_rot, other.y_rot),
                z_rot: mix(self.z_rot, other.z_rot),
                x: mix(self.x, other.x),
                y: mix(self.y, other.y),
                z: mix(self.z, other.z),
                x_scale: mix(self.x_scale, other.x_scale),
                y_scale: mix(self.y_scale, other.y_scale),
                z_scale: mix(self.z_scale, other.z_scale),
            }
        }

        /// This transform scaled by `k`.
        pub fn scaled(self, k: f32) -> PartTransform {
            PartTransform {
                x_rot: self.x_rot * k,
                y_rot: self.y_rot * k,
                z_rot: self.z_rot * k,
                x: self.x * k,
                y: self.y * k,
                z: self.z * k,
                x_scale: self.x_scale,
                y_scale: self.y_scale,
                z_scale: self.z_scale,
            }
        }

        /// Whether the transform changes nothing at all.
        pub fn is_identity(self) -> bool {
            self == PartTransform::IDENTITY
        }
    }

    /// A whole humanoid pose: the seven parts plus how much of the animation
    /// shows (`blend`) and how much of each bone's base pose is kept
    /// (`retention`, see [`BasePoseRetention`]).
    #[derive(Debug, Clone, PartialEq)]
    pub struct HumanoidPose {
        pub head: PartTransform,
        pub body: PartTransform,
        pub right_arm: PartTransform,
        pub left_arm: PartTransform,
        pub right_leg: PartTransform,
        pub left_leg: PartTransform,
        pub cape: PartTransform,
        /// How much of this pose shows, `0.0..=1.0`.
        pub blend: f32,
        /// How much of the base pose each bone keeps, `0.0..=1.0` per bone.
        pub retention: BasePoseRetention,
    }

    impl Default for HumanoidPose {
        fn default() -> Self {
            Self {
                head: PartTransform::IDENTITY,
                body: PartTransform::IDENTITY,
                right_arm: PartTransform::IDENTITY,
                left_arm: PartTransform::IDENTITY,
                right_leg: PartTransform::IDENTITY,
                left_leg: PartTransform::IDENTITY,
                cape: PartTransform::IDENTITY,
                blend: 1.0,
                retention: BasePoseRetention::default(),
            }
        }
    }

    impl HumanoidPose {
        /// Every part adding nothing, at full blend with no retention.
        pub fn identity() -> Self {
            Self::default()
        }

        /// The parts by name, in [`PART_NAMES`] order.
        pub fn parts(&self) -> [(&'static str, PartTransform); 7] {
            [
                ("head", self.head),
                ("body", self.body),
                ("right_arm", self.right_arm),
                ("left_arm", self.left_arm),
                ("right_leg", self.right_leg),
                ("left_leg", self.left_leg),
                ("cape", self.cape),
            ]
        }

        /// One part's transform by name, or `None` when the name is not a part.
        pub fn part(&self, name: &str) -> Option<PartTransform> {
            self.parts()
                .into_iter()
                .find(|(part, _)| *part == name)
                .map(|(_, transform)| transform)
        }

        /// One part's transform by name, to change.
        pub fn part_mut(&mut self, name: &str) -> Option<&mut PartTransform> {
            match name {
                "head" => Some(&mut self.head),
                "body" => Some(&mut self.body),
                "right_arm" => Some(&mut self.right_arm),
                "left_arm" => Some(&mut self.left_arm),
                "right_leg" => Some(&mut self.right_leg),
                "left_leg" => Some(&mut self.left_leg),
                "cape" => Some(&mut self.cape),
                _ => None,
            }
        }

        /// This pose blended towards `next` by `t`: how a partial tick samples
        /// the pose between the last two updates.
        pub fn lerp(&self, next: &HumanoidPose, t: f32) -> HumanoidPose {
            let t = if t.is_finite() {
                t.clamp(0.0, 1.0)
            } else {
                0.0
            };
            HumanoidPose {
                head: self.head.lerp(next.head, t),
                body: self.body.lerp(next.body, t),
                right_arm: self.right_arm.lerp(next.right_arm, t),
                left_arm: self.left_arm.lerp(next.left_arm, t),
                right_leg: self.right_leg.lerp(next.right_leg, t),
                left_leg: self.left_leg.lerp(next.left_leg, t),
                cape: self.cape.lerp(next.cape, t),
                blend: self.blend + (next.blend - self.blend) * t,
                retention: next.retention.clone(),
            }
        }

        /// This pose laid over `base`: each part moves from the base towards this
        /// pose by `blend * (1 - retention[part])`, so a bone the retention keeps
        /// stays where the base put it.
        pub fn over(&self, base: &HumanoidPose) -> HumanoidPose {
            let blend = if self.blend.is_finite() {
                self.blend.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut out = base.clone();
            for (name, animated) in self.parts() {
                let kept = self.retention.value(name, false);
                let kept = if kept.is_nan() {
                    0.0
                } else {
                    kept.clamp(0.0, 1.0)
                };
                let amount = blend * (1.0 - kept);
                if let Some(current) = out.part_mut(name) {
                    *current = current.lerp(animated, amount);
                }
            }
            out.blend = self.blend;
            out.retention = self.retention.clone();
            out
        }
    }

    // ---------------------------------------------------------------------------
    // LocomotionController
    // ---------------------------------------------------------------------------

    /// A full walk's speed in blocks a second: what a movement input of `1.0`
    /// carries the player along at.
    pub const WALK_SPEED: f32 = 4.317;

    /// How far a limb swings at full stride, in radians.
    pub const LIMB_SWING: f32 = 0.85;

    /// The gait: advances the walk cycle with the distance travelled, eases the
    /// stride in and out, and counts how long the player has been running.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct LocomotionController {
        /// The walk cycle's phase, a fraction in `0.0..1.0`.
        phase: f32,
        /// The stride amount, `0.0..=1.0`, eased.
        stride: f32,
        /// The fall weight, `0.0..=1.0`, eased.
        lift: f32,
        /// Seconds the player has been running.
        run_seconds: f32,
    }

    /// One update's gait.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct LocomotionFrame {
        /// The walk cycle's phase, degrees in `0.0..360.0`.
        pub phase: f32,
        /// How much the legs swing, `0.0..=1.0`.
        pub stride: f32,
        /// The fall or landing weight, `0.0..=1.0`.
        pub lift: f32,
        /// The movement input's length, `0.0..=1.0`.
        pub speed: f32,
        /// Seconds of running so far.
        pub run_seconds: f32,
    }

    impl Default for LocomotionController {
        fn default() -> Self {
            Self {
                phase: 0.0,
                stride: 0.0,
                lift: 0.0,
                run_seconds: 0.0,
            }
        }
    }

    impl LocomotionController {
        pub fn new() -> Self {
            Self::default()
        }

        /// The gait phase, a fraction in `0.0..1.0`.
        pub fn phase(&self) -> f32 {
            self.phase
        }

        /// The eased stride amount.
        pub fn stride(&self) -> f32 {
            self.stride
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }

        /// One frame from the movement signals.
        pub fn update(
            &mut self,
            signals: &MotionSignals,
            dt: f32,
            tuning: &EngineTuning,
        ) -> LocomotionFrame {
            let dt = if dt.is_finite() {
                dt.clamp(0.0, 0.25)
            } else {
                0.0
            };
            let speed = if signals.moving {
                input_length(signals)
            } else {
                0.0
            };
            // The cycle advances with the blocks travelled, so it never outruns
            // the feet however the frame rate moves. The stride is the pack's
            // ask, clamped the way `MovementTuning.strideDistance` clamps it.
            let stride = super::signals::tuning::stride_distance(tuning.stride_distance, 0.5, 8.0);
            let distance = speed * WALK_SPEED * dt;
            if stride > 0.0 && distance.is_finite() {
                self.phase = gait_wrap(self.phase + distance / stride);
            }
            let target_stride = if signals.moving { speed.max(0.35) } else { 0.0 };
            self.stride = fade(self.stride, target_stride, dt, tuning.fade_rate);
            self.lift = fade(
                self.lift,
                signals.fall_weight as f32 / super::signals::HEAVY as f32,
                dt,
                tuning.fade_rate,
            );
            if signals.running && signals.moving {
                self.run_seconds += dt;
            } else {
                self.run_seconds = 0.0;
            }
            LocomotionFrame {
                phase: self.phase * 360.0,
                stride: self.stride,
                lift: self.lift,
                speed,
                run_seconds: self.run_seconds,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // TurnInPlaceController
    // ---------------------------------------------------------------------------

    // ---------------------------------------------------------------------------
    // Quaternions, the Joml calls the pose math needs
    // ---------------------------------------------------------------------------

    /// A rotation quaternion, `(w, x, y, z)`, mirroring the Joml calls the Java
    /// makes on pose rotations: `rotationZYX(z, y, x)` to build and
    /// `getEulerAnglesZYX` to read back, in degrees at the edges.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Quat {
        pub w: f32,
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl Quat {
        /// The identity rotation.
        pub const IDENTITY: Quat = Quat {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        /// `new Quaternionf().rotationZYX(z, y, x)` from a pose rotation in
        /// degrees: about Z first, then Y, then X.
        pub fn from_zyx_degrees(rotation: Vec3) -> Self {
            let (zx, yx, xx) = (
                (rotation.z as f64).to_radians() * 0.5,
                (rotation.y as f64).to_radians() * 0.5,
                (rotation.x as f64).to_radians() * 0.5,
            );
            let (sz, cz) = (zx.sin() as f32, zx.cos() as f32);
            let (sy, cy) = (yx.sin() as f32, yx.cos() as f32);
            let (sx, cx) = (xx.sin() as f32, xx.cos() as f32);
            // q = qX * qY * qZ
            Self {
                w: cx * cy * cz - sx * sy * sz,
                x: sx * cy * cz + cx * sy * sz,
                y: cx * sy * cz - sx * cy * sz,
                z: cx * cy * sz + sx * sy * cz,
            }
        }

        /// `getEulerAnglesZYX`, back in degrees: `(x, y, z)` rotations.
        pub fn to_euler_zyx_degrees(self) -> Vec3 {
            let m = self.to_matrix3();
            // R = Rx(a) Ry(b) Rz(c): b from m02, a from -m12/m22, c from -m01/m00.
            let x = (-m[1][2]).atan2(m[2][2]);
            let y = m[0][2].clamp(-1.0, 1.0).asin();
            let z = (-m[0][1]).atan2(m[0][0]);
            Vec3::new(
                (x as f64 * 180.0 / std::f64::consts::PI) as f32,
                (y as f64 * 180.0 / std::f64::consts::PI) as f32,
                (z as f64 * 180.0 / std::f64::consts::PI) as f32,
            )
        }

        /// The rotation matrix, row-major `[[m00, m01, m02], ...]`.
        pub fn to_matrix3(self) -> [[f32; 3]; 3] {
            let (w, x, y, z) = (self.w, self.x, self.y, self.z);
            [
                [
                    1.0 - 2.0 * (y * y + z * z),
                    2.0 * (x * y - w * z),
                    2.0 * (x * z + w * y),
                ],
                [
                    2.0 * (x * y + w * z),
                    1.0 - 2.0 * (x * x + z * z),
                    2.0 * (y * z - w * x),
                ],
                [
                    2.0 * (x * z - w * y),
                    2.0 * (y * z + w * x),
                    1.0 - 2.0 * (x * x + y * y),
                ],
            ]
        }

        /// `slerp`: the shortest path towards `other` by `t`.
        pub fn slerp(self, other: Quat, t: f32) -> Quat {
            if t <= 0.0 {
                return self;
            }
            if t >= 1.0 {
                return other;
            }
            let mut dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;
            let (ox, oy, oz, ow) = if dot < 0.0 {
                dot = -dot;
                (-other.x, -other.y, -other.z, -other.w)
            } else {
                (other.x, other.y, other.z, other.w)
            };
            if dot > 1.0 - 1.0e-6 {
                // Nearly parallel: lerp and normalize.
                let out = Self {
                    w: self.w + (ow - self.w) * t,
                    x: self.x + (ox - self.x) * t,
                    y: self.y + (oy - self.y) * t,
                    z: self.z + (oz - self.z) * t,
                };
                return out.normalized();
            }
            let theta = dot.acos();
            let sin_theta = theta.sin();
            let a = ((1.0 - t) * theta).sin() / sin_theta;
            let b = (t * theta).sin() / sin_theta;
            Self {
                w: self.w * a + ow * b,
                x: self.x * a + ox * b,
                y: self.y * a + oy * b,
                z: self.z * a + oz * b,
            }
        }

        /// `normalize`.
        pub fn normalized(self) -> Quat {
            let squared = self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z;
            if squared < 1.0e-12 {
                return Quat::IDENTITY;
            }
            let scale = 1.0 / squared.sqrt();
            Self {
                w: self.w * scale,
                x: self.x * scale,
                y: self.y * scale,
                z: self.z * scale,
            }
        }

        /// The Hamilton product, `self * other`.
        #[allow(clippy::should_implement_trait)]
        pub fn mul(self, other: Quat) -> Quat {
            Self {
                w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
                x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
                y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
                z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
            }
        }

        /// `invert`.
        pub fn invert(self) -> Quat {
            Self {
                w: self.w,
                x: -self.x,
                y: -self.y,
                z: -self.z,
            }
        }

        /// Rotates a vector.
        pub fn transform(self, v: Vec3) -> Vec3 {
            let qv = Quat {
                w: 0.0,
                x: v.x,
                y: v.y,
                z: v.z,
            };
            let out = self.mul(qv).mul(self.invert());
            Vec3::new(out.x, out.y, out.z)
        }
    }

    /// `ClipSequencePlayer.quaternion`: a pose rotation in degrees as a
    /// quaternion.
    fn quaternion(rotation: Vec3) -> Quat {
        Quat::from_zyx_degrees(rotation)
    }

    // ---------------------------------------------------------------------------
    // MovingPoseBlend
    // ---------------------------------------------------------------------------

    /// `MovingPoseBlend`: a chain of sources blended into each other, the newest
    /// easing in over the running one and dropping it once settled.
    struct MovingPoseBlend<S: MovingSource> {
        root: MovingNode<S>,
    }

    impl<S: MovingSource> MovingPoseBlend<S> {
        /// A blend resting on the identity source.
        fn new(identity: impl Fn() -> S + 'static) -> Self {
            Self {
                root: MovingNode::Leaf {
                    source: MovingSourceNode::Identity(std::rc::Rc::new(identity)),
                },
            }
        }

        /// `retarget`: the new source eases in over `seconds`.
        fn retarget(&mut self, source: S, seconds: f32) {
            self.root.commit();
            let leaf = MovingNode::Leaf {
                source: MovingSourceNode::Live(source),
            };
            self.root = if seconds > 0.0 {
                MovingNode::Blend {
                    from: Box::new(std::mem::replace(&mut self.root, leaf.clone())),
                    to: Box::new(leaf),
                    duration: seconds.max(0.0),
                    previous_elapsed: 0.0,
                    elapsed: 0.0,
                }
            } else {
                leaf
            };
        }

        fn tick(&mut self, dt: f32) {
            let root = std::mem::replace(
                &mut self.root,
                MovingNode::Leaf {
                    source: MovingSourceNode::Identity(std::rc::Rc::new(|| {
                        unreachable!("replaced on the next line")
                    })),
                },
            );
            self.root = root.tick(dt.max(0.0));
        }

        fn sample(&self, bone: &str, partial: f32) -> S::Output {
            self.root.sample(bone, partial.clamp(0.0, 1.0))
        }

        fn transitioning(&self) -> bool {
            matches!(self.root, MovingNode::Blend { .. })
        }

        fn clear(&mut self, identity: impl Fn() -> S + 'static) {
            self.root = MovingNode::Leaf {
                source: MovingSourceNode::Identity(std::rc::Rc::new(identity)),
            };
        }
    }

    /// What a moving source produces and how it steps.
    trait MovingSource: Clone {
        type Output: BlendMixable;

        fn sample(&self, bone: &str, partial: f32) -> Self::Output;
        fn tick(&mut self, dt: f32) {
            let _ = dt;
        }
        fn commit(&mut self) {}
        fn set_times(&mut self, _previous_time: f32, _time: f32, _rate: f32) {}
        fn sync_playback(
            &mut self,
            _previous_time: f32,
            _time: f32,
            _previous_overlay: f32,
            _overlay: f32,
            _rate: f32,
        ) {
        }
    }

    #[derive(Clone)]
    enum MovingSourceNode<S: MovingSource> {
        /// The resting source: the identity.
        Identity(std::rc::Rc<dyn Fn() -> S>),
        /// A live playback.
        Live(S),
    }

    impl<S: MovingSource> MovingSourceNode<S> {
        fn sample(&self, bone: &str, partial: f32) -> S::Output {
            match self {
                MovingSourceNode::Identity(make) => make().sample(bone, partial),
                MovingSourceNode::Live(source) => source.sample(bone, partial),
            }
        }

        fn tick(&mut self, dt: f32) {
            if let MovingSourceNode::Live(source) = self {
                source.tick(dt);
            }
        }

        #[allow(
            dead_code,
            reason = "the leaf half of the node API, reached through MovingNode"
        )]
        fn set_times(&mut self, previous_time: f32, time: f32, rate: f32) {
            if let MovingSourceNode::Live(source) = self {
                source.set_times(previous_time, time, rate);
            }
        }

        #[allow(
            dead_code,
            reason = "the leaf half of the node API, reached through MovingNode"
        )]
        fn sync_playback(
            &mut self,
            previous_time: f32,
            time: f32,
            previous_overlay: f32,
            overlay: f32,
            rate: f32,
        ) {
            if let MovingSourceNode::Live(source) = self {
                source.sync_playback(previous_time, time, previous_overlay, overlay, rate);
            }
        }
    }

    enum MovingNode<S: MovingSource> {
        Leaf {
            source: MovingSourceNode<S>,
        },
        Blend {
            from: Box<MovingNode<S>>,
            to: Box<MovingNode<S>>,
            duration: f32,
            previous_elapsed: f32,
            elapsed: f32,
        },
    }

    impl<S: MovingSource> Clone for MovingNode<S> {
        fn clone(&self) -> Self {
            match self {
                MovingNode::Leaf { source } => MovingNode::Leaf {
                    source: source.clone(),
                },
                MovingNode::Blend {
                    from,
                    to,
                    duration,
                    previous_elapsed,
                    elapsed,
                } => MovingNode::Blend {
                    from: from.clone(),
                    to: to.clone(),
                    duration: *duration,
                    previous_elapsed: *previous_elapsed,
                    elapsed: *elapsed,
                },
            }
        }
    }

    impl<S: MovingSource> MovingNode<S> {
        fn sample(&self, bone: &str, partial: f32) -> S::Output {
            match self {
                MovingNode::Leaf { source } => source.sample(bone, partial),
                MovingNode::Blend {
                    from,
                    to,
                    duration,
                    previous_elapsed,
                    elapsed,
                } => {
                    let t = ((previous_elapsed + (elapsed - previous_elapsed) * partial)
                        / duration.max(1.0e-6))
                    .clamp(0.0, 1.0);
                    from.sample(bone, partial)
                        .mixed_with(&to.sample(bone, partial), t * t * (3.0 - 2.0 * t))
                }
            }
        }

        fn tick(self, dt: f32) -> Self {
            match self {
                MovingNode::Leaf { mut source } => {
                    source.tick(dt);
                    MovingNode::Leaf { source }
                }
                MovingNode::Blend {
                    mut from,
                    mut to,
                    duration,
                    previous_elapsed: _,
                    mut elapsed,
                } => {
                    from = Box::new(from.tick(dt));
                    to = Box::new(to.tick(dt));
                    let previous_elapsed = elapsed;
                    elapsed += dt;
                    if previous_elapsed >= duration {
                        *to
                    } else {
                        MovingNode::Blend {
                            from,
                            to,
                            duration,
                            previous_elapsed,
                            elapsed,
                        }
                    }
                }
            }
        }

        fn set_times(&mut self, previous_time: f32, time: f32, rate: f32) {
            if let MovingNode::Leaf {
                source: MovingSourceNode::Live(source),
            } = self
            {
                source.set_times(previous_time, time, rate);
            }
        }

        fn sync_playback(
            &mut self,
            previous_time: f32,
            time: f32,
            previous_overlay: f32,
            overlay: f32,
            rate: f32,
        ) {
            if let MovingNode::Leaf {
                source: MovingSourceNode::Live(source),
            } = self
            {
                source.sync_playback(previous_time, time, previous_overlay, overlay, rate);
            }
        }

        fn commit(&mut self) {
            match self {
                MovingNode::Leaf { source } => source.tick(0.0),
                MovingNode::Blend {
                    from,
                    to,
                    previous_elapsed,
                    elapsed,
                    ..
                } => {
                    *previous_elapsed = *elapsed;
                    from.commit();
                    to.commit();
                }
            }
        }
    }

    /// The mix a moving blend applies between two outputs.
    trait BlendMixable {
        fn mixed_with(&self, other: &Self, t: f32) -> Self;
    }

    // ---------------------------------------------------------------------------
    // ClipSequencePlayer
    // ---------------------------------------------------------------------------

    /// `ClipSequencePlayer.Sample`: one bone's pose from the playback, with its
    /// weight and the base-pose retention it carries.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Sample {
        pub pose: Pose,
        pub weight: f32,
        pub base_retain: f32,
        pub legacy_retain_weight: f32,
    }

    impl Sample {
        /// `Sample.empty`.
        pub fn empty() -> Self {
            Self {
                pose: Pose::identity(),
                weight: 0.0,
                base_retain: 0.0,
                legacy_retain_weight: 1.0,
            }
        }

        /// `withWeight`.
        pub fn with_weight(self, weight: f32) -> Self {
            Self { weight, ..self }
        }

        /// `retention`.
        pub fn retention(self, legacy: f32) -> f32 {
            self.base_retain + self.legacy_retain_weight * legacy
        }
    }

    impl BlendMixable for Sample {
        fn mixed_with(&self, other: &Self, t: f32) -> Self {
            mix_samples(*self, *other, t)
        }
    }

    impl BlendMixable for Pose {
        fn mixed_with(&self, other: &Self, t: f32) -> Self {
            mix(*self, *other, t)
        }
    }

    /// One playback running one clip with its overlays.
    #[derive(Debug, Clone, PartialEq)]
    struct Playback {
        clip: Option<Clip>,
        overlays: Vec<ResolvedOverlay>,
        looping: bool,
        held: std::collections::HashMap<String, Sample>,
        retention: BasePoseRetention,
        left: bool,
        base: Option<Clip>,
        time: f32,
        previous_time: f32,
        overlay_time: f32,
        previous_overlay_time: f32,
        rate: f32,
    }

    impl Playback {
        fn new(
            clip: Option<Clip>,
            overlays: Vec<ResolvedOverlay>,
            looping: bool,
            held: std::collections::HashMap<String, Sample>,
            retention: BasePoseRetention,
            left: bool,
            base: Option<Clip>,
        ) -> Self {
            Self {
                clip,
                overlays,
                looping,
                held,
                retention,
                left,
                base,
                time: 0.0,
                previous_time: 0.0,
                overlay_time: 0.0,
                previous_overlay_time: 0.0,
                rate: 1.0,
            }
        }
    }

    impl MovingSource for Playback {
        type Output = Sample;

        fn sample(&self, bone: &str, partial: f32) -> Sample {
            let retain = self.retention.value(bone, self.left);
            let sampled = clip_sequence_current(
                self.clip.as_ref(),
                &self.overlays,
                self.looping,
                &self.held,
                bone,
                self.previous_time + (self.time - self.previous_time) * partial,
                self.previous_overlay_time
                    + (self.overlay_time - self.previous_overlay_time) * partial,
                self.base.as_ref(),
            );
            if self.retention.value(bone, self.left).is_finite() {
                Sample {
                    base_retain: retain,
                    ..sampled
                }
            } else {
                sampled
            }
        }

        fn tick(&mut self, dt: f32) {
            self.previous_time = self.time;
            self.previous_overlay_time = self.overlay_time;
            self.time += dt * self.rate;
            self.overlay_time += dt * self.rate;
        }
    }

    /// `ClipSequencePlayer`: one clip playing with its overlays, cross-blending
    /// into each new `play`.
    pub struct ClipSequencePlayer {
        clip: Option<Clip>,
        base: Option<Clip>,
        overlays: Vec<ResolvedOverlay>,
        tracks: std::collections::HashSet<String>,
        direct_tracks: std::collections::HashSet<String>,
        duration: f32,
        current: Option<Playback>,
        blend: MovingPoseBlend<Playback>,
    }

    impl Default for ClipSequencePlayer {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ClipSequencePlayer {
        /// A stopped player.
        pub fn new() -> Self {
            Self {
                clip: None,
                base: None,
                overlays: Vec::new(),
                tracks: std::collections::HashSet::new(),
                direct_tracks: std::collections::HashSet::new(),
                duration: 0.16,
                current: None,
                blend: MovingPoseBlend::new(|| Playback {
                    clip: None,
                    overlays: Vec::new(),
                    looping: false,
                    held: std::collections::HashMap::new(),
                    retention: BasePoseRetention::default(),
                    left: false,
                    base: None,
                    time: 0.0,
                    previous_time: 0.0,
                    overlay_time: 0.0,
                    previous_overlay_time: 0.0,
                    rate: 1.0,
                }),
            }
        }

        /// `play`, the full form: `hold_start` snapshots the running pose, the
        /// retention and handedness ride along, and `base` adds a pose on top.
        #[allow(clippy::too_many_arguments)]
        pub fn play(
            &mut self,
            clip: Option<Clip>,
            overlays: Vec<ResolvedOverlay>,
            looping: bool,
            transition_seconds: f32,
            hold_start: bool,
            retention: BasePoseRetention,
            left: bool,
            base: Option<Clip>,
        ) {
            let mut held = std::collections::HashMap::new();
            if hold_start {
                for bone in &self.tracks {
                    held.insert(bone.clone(), self.sample(bone, 1.0));
                }
            }
            let mut additive = Vec::new();
            let mut merged = clip;
            if let Some(base_clip) = &merged
                && !overlays.is_empty()
            {
                for overlay in &overlays {
                    if overlay.mode == OverlayMode::Additive {
                        additive.push(overlay.clone());
                    }
                }
                merged = Some(merge_exclusive_tracks(base_clip, &overlays));
            }
            self.overlays = additive;
            self.base = base.clone();
            self.duration = if transition_seconds.is_finite() {
                transition_seconds.max(0.0)
            } else {
                0.16
            };
            let playback = Playback::new(
                merged.clone(),
                self.overlays.clone(),
                looping,
                held,
                retention,
                left,
                base,
            );
            let retarget = if self.current.is_some() {
                self.duration
            } else {
                0.0
            };
            self.blend.retarget(playback.clone(), retarget);
            self.current = Some(playback);
            self.clip = merged;
            let clips: Vec<Clip> = self
                .clip
                .iter()
                .cloned()
                .chain(self.overlays.iter().map(|o| o.clip.clone()))
                .chain(self.base.iter().cloned())
                .collect();
            for clip in &clips {
                self.add_tracks(clip);
            }
        }

        fn add_tracks(&mut self, clip: &Clip) {
            for bone in clip.bones.keys() {
                self.direct_tracks.insert(bone.clone());
                self.tracks.insert(bone.clone());
            }
            if clip.bones.contains_key("waist") {
                for bone in ["body", "head", "left_arm", "right_arm"] {
                    self.tracks.insert(bone.to_string());
                }
            }
        }

        /// `tick`.
        pub fn tick(&mut self, dt: f32) {
            self.blend.tick(dt);
            if let Some(current) = &mut self.current {
                current.tick(dt);
            }
            if !self.blend.transitioning() {
                self.tracks.clear();
                self.direct_tracks.clear();
                let held: Vec<String> = self
                    .current
                    .as_ref()
                    .map_or(Vec::new(), |current| current.held.keys().cloned().collect());
                self.tracks.extend(held);
                let clips: Vec<Clip> = self
                    .clip
                    .iter()
                    .cloned()
                    .chain(self.overlays.iter().map(|o| o.clip.clone()))
                    .chain(self.base.iter().cloned())
                    .collect();
                for clip in &clips {
                    self.add_tracks(clip);
                }
            }
        }

        /// `seek`.
        pub fn seek(&mut self, time: f32) {
            let length = self.clip.as_ref().map(|clip| clip.length);
            if let (Some(current), Some(length)) = (&mut self.current, length) {
                current.time = time.clamp(0.0, length);
                self.sync_current();
            }
        }

        /// Copies the current playback's clock into the blend's live leaf, so
        /// the sampled side sees seeks and rate changes at once.
        fn sync_current(&mut self) {
            if let Some(current) = &self.current {
                let (previous_time, time, previous_overlay, overlay, rate) = (
                    current.previous_time,
                    current.time,
                    current.previous_overlay_time,
                    current.overlay_time,
                    current.rate,
                );
                self.blend
                    .root
                    .sync_playback(previous_time, time, previous_overlay, overlay, rate);
            }
        }

        /// `setPlaybackRate`.
        pub fn set_playback_rate(&mut self, rate: f32) {
            if let Some(current) = &mut self.current {
                current.rate = if rate.is_finite() { rate.max(0.0) } else { 1.0 };
                self.sync_current();
            }
        }

        /// `renderTime`.
        pub fn render_time(&self, partial: f32) -> f32 {
            self.current.as_ref().map_or(0.0, |current| {
                current.previous_time + (current.time - current.previous_time) * partial
            })
        }

        /// `holdLastFrame`.
        pub fn hold_last_frame(&mut self) {
            let length = self.clip.as_ref().map(|clip| clip.length);
            if let (Some(current), Some(length)) = (&mut self.current, length) {
                current.previous_time = length;
                current.time = length;
                self.sync_current();
            }
        }

        /// `finished`.
        pub fn finished(&self) -> bool {
            match (&self.clip, &self.current) {
                (None, _) => true,
                (Some(clip), Some(current)) => !current.looping && current.time >= clip.length,
                (Some(_), None) => true,
            }
        }

        /// `progress`.
        pub fn progress(&self) -> f32 {
            match (&self.clip, &self.current) {
                (None, _) => 1.0,
                (Some(clip), Some(current)) => current.time / clip.length.max(0.001),
                (Some(_), None) => 1.0,
            }
        }

        /// `active`.
        pub fn active(&self) -> bool {
            self.clip.is_some() || self.blend.transitioning()
        }

        /// The clip playing.
        pub fn clip(&self) -> Option<&Clip> {
            self.clip.as_ref()
        }

        /// `hasDirectTrack`.
        pub fn has_direct_track(&self, bone: &str) -> bool {
            self.direct_tracks.contains(bone)
        }

        /// `stop`.
        pub fn stop(&mut self, transition_seconds: f32) {
            if self.clip.is_some() {
                self.play(
                    None,
                    Vec::new(),
                    false,
                    transition_seconds,
                    false,
                    BasePoseRetention::default(),
                    false,
                    None,
                );
            }
        }

        /// `clear`.
        pub fn clear(&mut self) {
            self.clip = None;
            self.base = None;
            self.current = None;
            self.overlays = Vec::new();
            self.tracks.clear();
            self.direct_tracks.clear();
            self.blend.clear(|| Playback {
                clip: None,
                overlays: Vec::new(),
                looping: false,
                held: std::collections::HashMap::new(),
                retention: BasePoseRetention::default(),
                left: false,
                base: None,
                time: 0.0,
                previous_time: 0.0,
                overlay_time: 0.0,
                previous_overlay_time: 0.0,
                rate: 1.0,
            });
        }

        /// `sample`.
        pub fn sample(&self, bone: &str, partial: f32) -> Sample {
            self.blend.sample(bone, partial)
        }
    }

    /// `ClipSequencePlayer.current`: one bone's sample from a clip and its
    /// overlays, falling back to the held snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn clip_sequence_current(
        clip: Option<&Clip>,
        overlays: &[ResolvedOverlay],
        looping: bool,
        held: &std::collections::HashMap<String, Sample>,
        bone: &str,
        time: f32,
        overlay_time: f32,
        base: Option<&Clip>,
    ) -> Sample {
        let Some(clip) = clip else {
            return Sample::empty();
        };
        let mut found = has_track(Some(clip), bone);
        let mut pose = if looping {
            clip.sample(bone, time)
        } else {
            clip.sample_once(bone, time)
        };
        for overlay in overlays {
            if overlay.applies_to(bone) && has_track(Some(&overlay.clip), bone) {
                let overlay_pose = overlay.clip.sample(bone, overlay_time);
                found = true;
                match overlay.mode {
                    OverlayMode::Additive => {
                        pose = Pose {
                            position: pose.position + overlay_pose.position,
                            rotation: pose.rotation + overlay_pose.rotation,
                            scale: pose.scale + overlay_pose.scale - Vec3::new(1.0, 1.0, 1.0),
                        };
                    }
                    OverlayMode::Override | OverlayMode::Exclusive => {
                        pose = overlay_pose;
                    }
                }
                if overlay.mode == OverlayMode::Exclusive {
                    break;
                }
            }
        }
        if let Some(base) = base
            && has_track(Some(base), bone)
        {
            let base_pose = base.sample_once(bone, base.length);
            pose = Pose {
                position: base_pose.position + pose.position,
                rotation: base_pose.rotation + pose.rotation,
                scale: base_pose.scale + pose.scale - Vec3::new(1.0, 1.0, 1.0),
            };
            found = true;
        }
        if found {
            Sample {
                pose,
                weight: 1.0,
                base_retain: 0.0,
                legacy_retain_weight: 1.0,
            }
        } else {
            held.get(bone).copied().unwrap_or_else(Sample::empty)
        }
    }

    /// `ClipSequencePlayer.hasTrack`.
    pub fn has_track(clip: Option<&Clip>, bone: &str) -> bool {
        let Some(clip) = clip else {
            return false;
        };
        clip.bones.contains_key(bone)
            || clip.bones.contains_key("waist")
                && matches!(bone, "body" | "head" | "left_arm" | "right_arm")
    }

    /// `ClipSequencePlayer.mergeExclusiveTracks`.
    pub fn merge_exclusive_tracks(clip: &Clip, overlays: &[ResolvedOverlay]) -> Clip {
        let mut bones = clip.bones.clone();
        for overlay in overlays {
            if overlay.mode != OverlayMode::Additive {
                for (bone, track) in &overlay.clip.bones {
                    if overlay.applies_to(bone) {
                        bones.insert(bone.clone(), track.clone());
                    }
                }
            }
        }
        Clip::new(clip.name.clone(), clip.length, clip.loop_, bones)
    }

    /// `ClipSequencePlayer.mixSamples`.
    pub fn mix_samples(a: Sample, b: Sample, t: f32) -> Sample {
        if t <= 0.0 {
            return a;
        }
        if t >= 1.0 {
            return b;
        }
        let wa = a.weight * (1.0 - t);
        let wb = b.weight * t;
        let total = wa + wb;
        if total < 1.0e-5 {
            return Sample::empty();
        }
        let share = wb / total;
        Sample {
            pose: mix(a.pose, b.pose, share),
            weight: total,
            base_retain: a.base_retain + (b.base_retain - a.base_retain) * share,
            legacy_retain_weight: a.legacy_retain_weight
                + (b.legacy_retain_weight - a.legacy_retain_weight) * share,
        }
    }

    /// `ClipSequencePlayer.mix`.
    pub fn mix(a: Pose, b: Pose, t: f32) -> Pose {
        if t <= 0.0 {
            return a;
        }
        if t >= 1.0 {
            return b;
        }
        Pose {
            position: a.position.lerp(b.position, t),
            rotation: mix_rotation(a.rotation, b.rotation, t),
            scale: a.scale.lerp(b.scale, t),
        }
    }

    /// `ClipSequencePlayer.mixRotation`: slerp through quaternions, read back as
    /// degrees.
    pub fn mix_rotation(a: Vec3, b: Vec3, t: f32) -> Vec3 {
        if t <= 0.0 {
            return a;
        }
        if t >= 1.0 {
            return b;
        }
        quaternion(a)
            .slerp(quaternion(b), t)
            .normalized()
            .to_euler_zyx_degrees()
    }

    /// `ClipSequencePlayer.applyRotationOffset`.
    pub fn apply_rotation_offset(base: Vec3, from: Vec3, to: Vec3, t: f32) -> Vec3 {
        let offset = quaternion(from).invert().mul(quaternion(to));
        quaternion(base)
            .mul(Quat::IDENTITY.slerp(offset, t))
            .normalized()
            .to_euler_zyx_degrees()
    }

    /// `ClipSequencePlayer.compose`: the sample over the base pose, keeping
    /// `retain` of the base.
    pub fn compose(base: Pose, sample: Sample, legacy_retain: f32) -> Pose {
        let weight = sample.weight.clamp(0.0, 1.0);
        if weight <= 0.0 {
            return base;
        }
        let retain = sample.retention(legacy_retain).clamp(0.0, 1.0);
        let composed = Pose {
            position: base.position * retain + sample.pose.position,
            rotation: retained_rotation(base.rotation, sample.pose.rotation, retain),
            scale: (base.scale - Vec3::new(1.0, 1.0, 1.0)) * retain + sample.pose.scale,
        };
        if weight >= 0.99999 {
            return composed;
        }
        Pose {
            position: base.position.lerp(composed.position, weight),
            rotation: quaternion(base.rotation)
                .slerp(quaternion(composed.rotation), weight)
                .normalized()
                .to_euler_zyx_degrees(),
            scale: base.scale.lerp(composed.scale, weight),
        }
    }

    /// `ClipSequencePlayer.retainedRotation`.
    fn retained_rotation(base: Vec3, over: Vec3, retain: f32) -> Vec3 {
        if retain != 0.0 && base.length_squared() != 0.0 {
            quaternion(base * retain)
                .mul(quaternion(over))
                .normalized()
                .to_euler_zyx_degrees()
        } else {
            over
        }
    }

    // ---------------------------------------------------------------------------
    // ClipPoseBlend
    // ---------------------------------------------------------------------------

    /// `ClipPoseBlend.Playback`: one clip running against the render clock.
    #[derive(Debug, Clone, PartialEq)]
    struct ClipPosePlayback {
        clip: Clip,
        carry_clip: Clip,
        overlays: Vec<ResolvedOverlay>,
        carry_overlays: Vec<ResolvedOverlay>,
        looping: bool,
        previous_time: f32,
        time: f32,
        rate: f32,
    }

    impl MovingSource for ClipPosePlayback {
        type Output = Pose;

        fn sample(&self, bone: &str, partial: f32) -> Pose {
            let mut bone = bone;
            let carry = bone.starts_with("carry:");
            if carry {
                bone = &bone["carry:".len()..];
            }
            let mut time = self.previous_time + (self.time - self.previous_time) * partial;
            if self.looping {
                let length = self.clip.length.max(1.0e-4);
                time = positive_modulo(time, length);
            }
            clip_sequence_current(
                if carry {
                    Some(&self.carry_clip)
                } else {
                    Some(&self.clip)
                },
                if carry {
                    &self.carry_overlays
                } else {
                    &self.overlays
                },
                self.looping,
                &std::collections::HashMap::new(),
                bone,
                time,
                time,
                None,
            )
            .pose
        }

        fn tick(&mut self, dt: f32) {
            self.commit();
            self.time += dt * self.rate;
        }

        fn commit(&mut self) {
            self.previous_time = self.time;
        }

        fn set_times(&mut self, previous_time: f32, time: f32, rate: f32) {
            self.previous_time = previous_time;
            self.time = time;
            self.rate = rate;
        }
    }

    /// `ClipPoseBlend`: one clip's pose, blended into each new `start`.
    pub struct ClipPoseBlend {
        blend: MovingPoseBlend<ClipPosePlayback>,
    }

    impl Default for ClipPoseBlend {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ClipPoseBlend {
        /// A blend resting on identity.
        pub fn new() -> Self {
            Self {
                blend: MovingPoseBlend::new(|| ClipPosePlayback {
                    clip: Clip::new(String::new(), 0.0, false, std::collections::HashMap::new()),
                    carry_clip: Clip::new(
                        String::new(),
                        0.0,
                        false,
                        std::collections::HashMap::new(),
                    ),
                    overlays: Vec::new(),
                    carry_overlays: Vec::new(),
                    looping: false,
                    previous_time: 0.0,
                    time: 0.0,
                    rate: 1.0,
                }),
            }
        }

        /// `start`.
        pub fn start(
            &mut self,
            clip: Option<Clip>,
            time: f32,
            rate: f32,
            looping: bool,
            blend_seconds: f32,
            overlays: Vec<ResolvedOverlay>,
        ) {
            let Some(clip) = clip else {
                return;
            };
            let playback = ClipPosePlayback {
                carry_clip: clip.without_local_arms(),
                carry_overlays: overlays
                    .iter()
                    .map(|overlay| ResolvedOverlay {
                        clip: overlay.clip.without_local_arms(),
                        mode: overlay.mode,
                        bones: overlay.bones.clone(),
                    })
                    .collect(),
                clip,
                overlays: overlays
                    .iter()
                    .filter(|overlay| overlay.mode == OverlayMode::Additive)
                    .cloned()
                    .collect(),
                looping,
                previous_time: time,
                time,
                rate,
            };
            let retarget = if self.has_target() {
                blend_seconds
            } else {
                0.0
            };
            self.blend.retarget(playback, retarget);
        }

        fn has_target(&self) -> bool {
            !matches!(
                &self.blend.root,
                MovingNode::Leaf {
                    source: MovingSourceNode::Identity(_),
                }
            )
        }

        /// `tick`.
        pub fn tick(&mut self, dt: f32, previous_time: f32, time: f32, rate: f32) {
            self.blend.tick(dt);
            self.blend.root.set_times(previous_time, time, rate);
        }

        /// `sample`.
        pub fn sample(&self, bone: &str, partial: f32) -> Pose {
            self.blend.sample(bone, partial)
        }

        /// `sampleCarry`.
        pub fn sample_carry(&self, bone: &str, partial: f32) -> Pose {
            self.blend.sample(&format!("carry:{}", bone), partial)
        }

        /// `clear`.
        pub fn clear(&mut self) {
            self.blend.clear(|| ClipPosePlayback {
                clip: Clip::new(String::new(), 0.0, false, std::collections::HashMap::new()),
                carry_clip: Clip::new(String::new(), 0.0, false, std::collections::HashMap::new()),
                overlays: Vec::new(),
                carry_overlays: Vec::new(),
                looping: false,
                previous_time: 0.0,
                time: 0.0,
                rate: 1.0,
            });
        }
    }

    // ---------------------------------------------------------------------------
    // TurnInPlaceController (+ CameraTurnInPlace, KeyframedTurnInPlace)
    // ---------------------------------------------------------------------------

    /// `TurnInPlaceController.ENTRY_SECONDS`.
    pub const TURN_ENTRY_SECONDS: f32 = 0.08;
    /// `TurnInPlaceController.ALIGNMENT_EPSILON`.
    pub const TURN_ALIGNMENT_EPSILON: f32 = 1.0;

    /// Which turn-in-place playback a setting asks for.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum TurnMode {
        /// One clip scaled by how far the turn went.
        Scaled,
        /// Left and right step clips.
        #[default]
        Steps,
        /// The camera-following tracker.
        Camera,
        /// The keyframed two-turn tracker (`dynamic` and `procedural`).
        Dynamic,
    }

    /// `TurnInPlaceController.Settings`: what a pack configures, the Java
    /// record's fields.
    #[derive(Debug, Clone, PartialEq)]
    pub struct TurnSettings {
        pub enabled: bool,
        pub threshold: f32,
        pub delay: f32,
        pub fade: f32,
        pub mode: TurnMode,
        pub reference: f32,
        pub double_turn: f32,
        pub template: Option<Clip>,
        pub left: Option<Clip>,
        pub right: Option<Clip>,
        pub left_add: Option<Clip>,
        pub right_add: Option<Clip>,
    }

    impl Default for TurnSettings {
        fn default() -> Self {
            Self {
                enabled: false,
                threshold: 45.0,
                delay: 2.0,
                fade: 0.12,
                mode: TurnMode::Steps,
                reference: 45.0,
                double_turn: 75.0,
                template: None,
                left: None,
                right: None,
                left_add: None,
                right_add: None,
            }
        }
    }

    impl TurnSettings {
        /// `Settings.from(library)`: the `turn_in_place` settings off the
        /// library, with the Java defaults and clamps.
        pub fn from_library(library: &super::library::AnimationLibrary) -> Self {
            let mode = match library
                .bindings()
                .string_setting(None, None, "turn_in_place_mode", "steps")
                .as_str()
            {
                "scaled" => TurnMode::Scaled,
                "camera" => TurnMode::Camera,
                "dynamic" | "procedural" => TurnMode::Dynamic,
                _ => TurnMode::Steps,
            };
            let template = library.clip(&library.bindings().string_setting(
                None,
                None,
                "turn_in_place_clip",
                "",
            ));
            let left = library.clip(&library.bindings().string_setting(
                None,
                None,
                "turn_in_place_left",
                if mode == TurnMode::Dynamic {
                    "step_left"
                } else {
                    ""
                },
            ));
            let right = library.clip(&library.bindings().string_setting(
                None,
                None,
                "turn_in_place_right",
                if mode == TurnMode::Dynamic {
                    "step_right"
                } else {
                    ""
                },
            ));
            let valid = |clip: Option<&Clip>| {
                clip.is_some_and(|clip| {
                    has_track(Some(clip), "left_leg")
                        && has_track(Some(clip), "right_leg")
                        && clip.length.is_finite()
                        && clip.length > 0.0
                })
            };
            let enabled = match mode {
                TurnMode::Scaled => valid(template),
                _ => valid(left) && valid(right),
            };
            let setting = |key: &str, default: f32, min: f32, max: f32| {
                let value = library.bindings().numeric_setting(
                    None,
                    None,
                    &format!("turn_in_place_{}", key),
                    default,
                );
                if value.is_finite() {
                    value.clamp(min, max)
                } else {
                    default
                }
            };
            Self {
                enabled: enabled
                    && library
                        .bindings()
                        .numeric_setting(None, None, "turn_in_place_enabled", 0.0)
                        > 0.5,
                threshold: if mode == TurnMode::Dynamic {
                    setting("minimum_turn_degrees", 2.0, 2.0, 90.0)
                } else {
                    setting("minimum_turn_degrees", 45.0, 2.0, 90.0)
                },
                delay: if mode == TurnMode::Dynamic {
                    setting("turn_delay_seconds", 0.12, 0.0, 0.3)
                } else {
                    setting(
                        "look_idle_seconds",
                        if mode == TurnMode::Camera { 0.0 } else { 2.0 },
                        0.0,
                        10.0,
                    )
                },
                fade: setting("fade_out_seconds", 0.12, 0.05, 0.4),
                mode,
                reference: setting("reference_degrees", 45.0, 1.0, 180.0),
                double_turn: setting("double_turn_degrees", 75.0, 30.0, 180.0),
                template: template.cloned(),
                left: left.cloned(),
                right: right.cloned(),
                left_add: library
                    .clip(&library.bindings().string_setting(
                        None,
                        None,
                        "turn_in_place_left_add",
                        "",
                    ))
                    .cloned(),
                right_add: library
                    .clip(&library.bindings().string_setting(
                        None,
                        None,
                        "turn_in_place_right_add",
                        "",
                    ))
                    .cloned(),
            }
        }
    }

    /// `CameraTurnInPlace`: the camera-mode tracker.
    #[derive(Debug, Clone, PartialEq)]
    struct CameraTurnInPlace {
        state: CameraTurnState,
        previous_look: f32,
        target_yaw: f32,
        body_yaw: f32,
        tracking_start_yaw: f32,
        time: f32,
        strength: f32,
        retarget_time: f32,
        last_motion_time: f64,
        retargeting: bool,
        clip: Option<Clip>,
        addition: Option<Clip>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    enum CameraTurnState {
        #[default]
        Idle,
        Tracking,
        Animating,
    }

    /// `CameraTurnInPlace.INPUT_GAP_SECONDS`.
    const CAMERA_INPUT_GAP: f32 = 0.05;
    /// `CameraTurnInPlace.RETARGET_SECONDS`.
    const CAMERA_RETARGET_SECONDS: f32 = 0.08;
    /// `CameraTurnInPlace.ANGLE_EPSILON`.
    const CAMERA_ANGLE_EPSILON: f32 = 0.001;

    impl CameraTurnInPlace {
        fn reset(&mut self, body_yaw: f32, look: f32, time: f64) {
            self.state = CameraTurnState::Idle;
            self.previous_look = look;
            self.body_yaw = body_yaw;
            self.tracking_start_yaw = body_yaw;
            self.target_yaw = body_yaw + wrap_degrees360(look - body_yaw);
            self.last_motion_time = time;
            self.time = 0.0;
            self.strength = 0.0;
            self.retarget_time = 0.0;
            self.retargeting = false;
            self.clip = None;
            self.addition = None;
        }

        fn update(&mut self, look: f32, time: f64, dt: f32, settings: &TurnSettings) -> f32 {
            let delta = wrap_degrees360(look - self.previous_look);
            self.previous_look = look;
            self.target_yaw += delta;
            let moved = delta.abs() > 1.0e-6;
            if moved {
                self.last_motion_time = time;
            }
            if self.state == CameraTurnState::Animating {
                let length = self.clip.as_ref().map_or(0.0, |clip| clip.length);
                self.time = length.min(self.time + dt);
                if self.time < length {
                    return self.body_yaw;
                }
                self.clip = None;
                self.strength = 0.0;
                self.state = CameraTurnState::Idle;
                if wrap_degrees360(self.target_yaw - self.body_yaw).abs() > CAMERA_ANGLE_EPSILON {
                    self.begin_tracking(true);
                }
                return self.body_yaw;
            }
            if self.state == CameraTurnState::Idle {
                if !moved {
                    return self.body_yaw;
                }
                self.begin_tracking(false);
            }
            if self.retargeting {
                self.retarget_time = CAMERA_RETARGET_SECONDS.min(self.retarget_time + dt);
                let t = self.retarget_time / CAMERA_RETARGET_SECONDS;
                let eased = t * t * (3.0 - 2.0 * t);
                self.body_yaw = self.tracking_start_yaw
                    + wrap_degrees360(self.target_yaw - self.tracking_start_yaw) * eased;
                self.retargeting = self.retarget_time < CAMERA_RETARGET_SECONDS;
            } else {
                self.body_yaw = self.target_yaw;
            }
            let swung = self.body_yaw - self.tracking_start_yaw;
            self.strength = (swung / settings.reference).clamp(0.0, 1.0);
            self.clip = if swung < 0.0 {
                settings.left.clone()
            } else {
                settings.right.clone()
            };
            self.addition = if swung < 0.0 {
                settings.left_add.clone()
            } else {
                settings.right_add.clone()
            };
            self.time = 0.0;
            if !self.retargeting
                && !moved
                && (time - self.last_motion_time) as f32 >= CAMERA_INPUT_GAP - 1.0e-6
            {
                if self.strength > 0.0 {
                    self.state = CameraTurnState::Animating;
                } else {
                    self.state = CameraTurnState::Idle;
                    self.clip = None;
                }
            }
            self.body_yaw
        }

        fn begin_tracking(&mut self, retarget: bool) {
            self.state = CameraTurnState::Tracking;
            self.tracking_start_yaw = self.body_yaw;
            self.target_yaw = self.body_yaw + wrap_degrees360(self.target_yaw - self.body_yaw);
            self.time = 0.0;
            self.strength = 0.0;
            self.retarget_time = 0.0;
            self.retargeting = retarget;
        }

        fn sample(&self, bone: &str) -> Pose {
            if let (Some(clip), false) = (&self.clip, self.strength == 0.0) {
                let pose = sample_authored(clip, self.addition.as_ref(), bone, self.time);
                return Pose {
                    position: pose.position * self.strength,
                    rotation: pose.rotation * self.strength,
                    scale: Vec3::new(1.0, 1.0, 1.0).lerp(pose.scale, self.strength),
                };
            }
            Pose::identity()
        }

        fn active(&self) -> bool {
            self.clip.is_some() && self.strength > 0.0
        }
    }

    /// `KeyframedTurnInPlace`: the two-turn tracker.
    #[derive(Debug, Clone, PartialEq)]
    struct KeyframedTurnInPlace {
        from: HashMap<String, Pose>,
        now: f32,
        gesture_time: f32,
        last_motion: f32,
        reverse_distance: f32,
        restart_since: f32,
        large_since: f32,
        body_yaw: f32,
        previous_look: f32,
        look_target: f32,
        origin_yaw: f32,
        gesture_direction: f32,
        start_yaw: f32,
        target_yaw: f32,
        time: f32,
        strength: f32,
        playback_rate: f32,
        body_duration: f32,
        entry_duration: f32,
        turns: u8,
        gesture: bool,
        stopped: bool,
        two_turns: bool,
        restart_pending: bool,
        clip: Option<Clip>,
        addition: Option<Clip>,
    }

    /// The keyframed tracker's constants.
    const KEYFRAMED_QUIET_SECONDS: f32 = 0.12;
    const KEYFRAMED_FIRST_BODY_SECONDS: f32 = 0.36;
    const KEYFRAMED_SECOND_BODY_SECONDS: f32 = 0.3;
    const KEYFRAMED_FIRST_ENTRY_SECONDS: f32 = 0.1;
    const KEYFRAMED_SECOND_ENTRY_SECONDS: f32 = 0.06;
    const KEYFRAMED_SECOND_PLAYBACK_RATE: f32 = 1.4;
    const KEYFRAMED_REVERSAL_DEGREES: f32 = 3.0;
    const KEYFRAMED_FIRST_TURN_SHARE: f32 = 0.48;
    const KEYFRAMED_MAX_BLEND: f64 = 1.333_333_333_33;
    const KEYFRAMED_BLEND_RESPONSE_SECONDS: f32 = 0.04;
    const KEYFRAMED_MAX_COLLECTION_SECONDS: f32 = 0.3;
    const KEYFRAMED_BONES: [&str; 6] = [
        "left_leg",
        "right_leg",
        "left_arm",
        "right_arm",
        "body",
        "head",
    ];

    impl Default for KeyframedTurnInPlace {
        fn default() -> Self {
            let mut out = Self {
                from: HashMap::new(),
                now: 0.0,
                gesture_time: 0.0,
                last_motion: 0.0,
                reverse_distance: 0.0,
                restart_since: 0.0,
                large_since: f32::NAN,
                body_yaw: 0.0,
                previous_look: 0.0,
                look_target: 0.0,
                origin_yaw: 0.0,
                gesture_direction: 0.0,
                start_yaw: 0.0,
                target_yaw: 0.0,
                time: 0.0,
                strength: 0.0,
                playback_rate: 1.0,
                body_duration: KEYFRAMED_FIRST_BODY_SECONDS,
                entry_duration: KEYFRAMED_FIRST_ENTRY_SECONDS,
                turns: 0,
                gesture: false,
                stopped: false,
                two_turns: false,
                restart_pending: false,
                clip: None,
                addition: None,
            };
            out.reset(0.0, 0.0);
            out
        }
    }

    impl KeyframedTurnInPlace {
        fn reset(&mut self, body_yaw: f32, look: f32) {
            self.body_yaw = body_yaw;
            self.origin_yaw = body_yaw;
            self.start_yaw = body_yaw;
            self.target_yaw = body_yaw;
            self.previous_look = look;
            self.look_target = body_yaw + wrap_degrees360(look - body_yaw);
            self.now = 0.0;
            self.gesture_time = 0.0;
            self.reverse_distance = 0.0;
            self.time = 0.0;
            self.strength = 0.0;
            self.restart_since = 0.0;
            self.large_since = f32::NAN;
            self.playback_rate = 1.0;
            self.body_duration = KEYFRAMED_FIRST_BODY_SECONDS;
            self.entry_duration = KEYFRAMED_FIRST_ENTRY_SECONDS;
            self.last_motion = f32::NEG_INFINITY;
            self.turns = 0;
            self.gesture = false;
            self.stopped = false;
            self.two_turns = false;
            self.restart_pending = false;
            self.clip = None;
            self.addition = None;
            self.from.clear();
        }

        fn update(
            &mut self,
            look: f32,
            dt: f32,
            key_pressed: bool,
            settings: &TurnSettings,
        ) -> f32 {
            self.now += dt;
            let delta = wrap_degrees360(look - self.previous_look);
            self.previous_look = look;
            self.look_target += delta;
            if delta.abs() > 1.0e-6 {
                let restart_wanted =
                    self.clip.is_none() && (self.turns == 2 || self.turns == 1 && !self.two_turns);
                if !self.gesture
                    || restart_wanted
                    || self.clip.is_none() && self.turns == 0 && self.stopped
                {
                    self.start_gesture(delta.signum(), self.now);
                }
                if self.gesture && delta * self.gesture_direction < 0.0 {
                    self.reverse_distance += delta;
                } else {
                    self.reverse_distance = 0.0;
                }
                if self.reverse_distance.abs() >= KEYFRAMED_REVERSAL_DEGREES
                    || self.clip.is_some()
                        && (self.turns == 2 || self.turns == 1 && !self.two_turns)
                {
                    if self.clip.is_none() {
                        self.start_gesture(delta.signum(), self.now);
                    } else if !self.restart_pending {
                        self.restart_pending = true;
                        self.restart_since = self.now;
                    }
                }
                self.last_motion = self.now;
            } else if key_pressed && !self.gesture && (self.look_target - self.body_yaw).abs() > 1.0
            {
                self.start_gesture((self.look_target - self.body_yaw).signum(), self.now);
                self.last_motion = self.now - KEYFRAMED_QUIET_SECONDS;
            }
            self.stopped =
                self.gesture && self.now - self.last_motion >= KEYFRAMED_QUIET_SECONDS - 1.0e-6;
            if let Some(clip) = &self.clip {
                let length = clip.length;
                let mut swung = (self.look_target - self.start_yaw)
                    * (self.target_yaw - self.start_yaw).signum();
                if self.turns == 1 && self.two_turns {
                    swung *= KEYFRAMED_FIRST_TURN_SHARE;
                }
                let target_strength = blend_strength(
                    (self.target_yaw - self.start_yaw).abs().max(swung),
                    settings.reference,
                );
                let response = -(-dt / KEYFRAMED_BLEND_RESPONSE_SECONDS).exp_m1();
                self.strength += (target_strength - self.strength).max(0.0) * response;
                self.time = length.min(self.time + dt * self.playback_rate);
                self.body_yaw = self.start_yaw
                    + (self.target_yaw - self.start_yaw)
                        * ease_turn((self.time / self.body_duration.min(length)).min(1.0));
                if self.time >= self.entry_duration {
                    self.from.clear();
                }
                if self.time >= length - 1.0e-4 {
                    self.time = length;
                    self.body_yaw = self.target_yaw;
                    self.clip = None;
                    self.addition = None;
                    self.strength = 0.0;
                    self.from.clear();
                    if self.restart_pending {
                        self.start_gesture(
                            (self.look_target - self.body_yaw).signum(),
                            self.restart_since,
                        );
                        self.stopped =
                            self.now - self.last_motion >= KEYFRAMED_QUIET_SECONDS - 1.0e-6;
                    }
                }
            }
            let mut pending = self.look_target
                - if self.turns == 0 {
                    self.body_yaw
                } else {
                    self.origin_yaw
                };
            let large = pending.abs() >= settings.double_turn - 0.001;
            if !self.gesture || self.turns != 0 || !large {
                self.large_since = f32::NAN;
            } else if self.large_since.is_nan() {
                self.large_since = self.now;
            }
            let collected = self.stopped
                || large
                    && (pending.abs() * KEYFRAMED_FIRST_TURN_SHARE
                        >= settings.reference * KEYFRAMED_MAX_BLEND as f32
                        || self.now - self.large_since
                            >= KEYFRAMED_MAX_COLLECTION_SECONDS - 1.0e-6);
            if self.gesture
                && self.clip.is_none()
                && self.turns == 0
                && (key_pressed || self.now - self.gesture_time >= settings.delay - 1.0e-6)
                && collected
                && pending.abs() >= if key_pressed { 1.0 } else { settings.threshold }
            {
                self.origin_yaw = self.body_yaw;
                pending = self.look_target - self.origin_yaw;
                self.two_turns = pending.abs() >= settings.double_turn - 0.001;
                let first = if self.two_turns {
                    pending * KEYFRAMED_FIRST_TURN_SHARE
                } else {
                    pending
                };
                self.begin(self.body_yaw + first, false, settings);
                self.turns = 1;
            } else if self.gesture
                && self.turns == 1
                && self.two_turns
                && self.clip.is_none()
                && self.stopped
            {
                if (self.look_target - self.body_yaw).abs()
                    >= if key_pressed { 1.0 } else { settings.threshold }
                {
                    self.begin(self.look_target, true, settings);
                    self.turns = 2;
                } else {
                    self.two_turns = false;
                }
            }
            self.body_yaw
        }

        fn start_gesture(&mut self, direction: f32, at: f32) {
            self.gesture = true;
            self.stopped = false;
            self.two_turns = false;
            self.gesture_time = at;
            self.origin_yaw = self.body_yaw;
            self.gesture_direction = direction;
            self.reverse_distance = 0.0;
            self.large_since = f32::NAN;
            self.turns = 0;
            self.restart_pending = false;
        }

        fn begin(&mut self, target: f32, second: bool, settings: &TurnSettings) {
            let mut from = HashMap::new();
            for bone in KEYFRAMED_BONES {
                from.insert(bone.to_string(), self.sample(bone));
            }
            self.start_yaw = self.body_yaw;
            self.target_yaw = target;
            self.strength = blend_strength((target - self.start_yaw).abs(), settings.reference);
            self.playback_rate = if second {
                KEYFRAMED_SECOND_PLAYBACK_RATE
            } else {
                1.0
            };
            self.body_duration = if second {
                KEYFRAMED_SECOND_BODY_SECONDS
            } else {
                KEYFRAMED_FIRST_BODY_SECONDS
            };
            self.entry_duration = if second {
                KEYFRAMED_SECOND_ENTRY_SECONDS
            } else {
                KEYFRAMED_FIRST_ENTRY_SECONDS
            };
            self.clip = if target < self.body_yaw {
                settings.left.clone()
            } else {
                settings.right.clone()
            };
            self.addition = if target < self.body_yaw {
                settings.left_add.clone()
            } else {
                settings.right_add.clone()
            };
            self.time = 0.0;
            self.from = from;
        }

        fn sample(&self, bone: &str) -> Pose {
            let Some(clip) = &self.clip else {
                return Pose::identity();
            };
            let pose = sample_authored(clip, self.addition.as_ref(), bone, self.time);
            let scaled = Pose {
                position: pose.position * self.strength,
                rotation: pose.rotation * self.strength,
                scale: Vec3::new(1.0, 1.0, 1.0).lerp(pose.scale, self.strength),
            };
            if self.time >= self.entry_duration {
                return scaled;
            }
            mix(
                self.from.get(bone).cloned().unwrap_or_else(Pose::identity),
                scaled,
                ease_turn((self.time / self.entry_duration).min(1.0)),
            )
        }

        fn active(&self) -> bool {
            self.clip.is_some()
        }
    }

    /// `blendStrength`.
    fn blend_strength(degrees: f32, reference: f32) -> f32 {
        ((degrees / reference) as f64).clamp(0.0, KEYFRAMED_MAX_BLEND) as f32
    }

    /// The turn controllers' smoothstep.
    fn ease_turn(t: f32) -> f32 {
        t * t * (3.0 - 2.0 * t)
    }

    /// The turn controllers' degree wrap.
    fn wrap_degrees360(value: f32) -> f32 {
        let mut wrapped = value % 360.0;
        if wrapped >= 180.0 {
            wrapped -= 360.0;
        }
        if wrapped < -180.0 {
            wrapped += 360.0;
        }
        wrapped
    }

    /// `TurnInPlaceController`: the body yaw standing still, with the mode the
    /// settings pick.
    #[derive(Debug, Clone, PartialEq)]
    pub struct TurnInPlaceController {
        initialized: bool,
        managed: bool,
        releasing: bool,
        last_time: f64,
        last_look_change: f64,
        body_yaw: f32,
        quiet_yaw: f32,
        quiet_pitch: f32,
        start_yaw: f32,
        target_yaw: f32,
        progress: f32,
        weight: f32,
        time: f32,
        release_time: f32,
        release_yaw: f32,
        strength: f32,
        clip: Option<Clip>,
        mirrored: bool,
        camera_turn: CameraTurnInPlace,
        keyframed_turn: KeyframedTurnInPlace,
        settings_addition: Option<Clip>,
        camera_mode: bool,
        dynamic_mode: bool,
    }

    impl Default for TurnInPlaceController {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TurnInPlaceController {
        /// A controller resting on the vanilla body yaw.
        pub fn new() -> Self {
            Self {
                initialized: false,
                managed: false,
                releasing: false,
                last_time: 0.0,
                last_look_change: 0.0,
                body_yaw: 0.0,
                quiet_yaw: 0.0,
                quiet_pitch: 0.0,
                start_yaw: 0.0,
                target_yaw: 0.0,
                progress: 0.0,
                weight: 0.0,
                time: 0.0,
                release_time: 0.0,
                release_yaw: 0.0,
                strength: 1.0,
                clip: None,
                mirrored: false,
                camera_turn: CameraTurnInPlace {
                    state: CameraTurnState::Idle,
                    previous_look: 0.0,
                    target_yaw: 0.0,
                    body_yaw: 0.0,
                    tracking_start_yaw: 0.0,
                    time: 0.0,
                    strength: 0.0,
                    retarget_time: 0.0,
                    last_motion_time: 0.0,
                    retargeting: false,
                    clip: None,
                    addition: None,
                },
                keyframed_turn: KeyframedTurnInPlace::default(),
                settings_addition: None,
                camera_mode: false,
                dynamic_mode: false,
            }
        }

        /// `update`: `body_yaw` in, the look yaw and pitch, the animation time,
        /// `compatible` whether the body is free to turn in place, `stationary`
        /// whether the gait says the player stands still, `key_pressed` whether
        /// the player holds a key that would turn, and the settings.
        #[allow(clippy::too_many_arguments)]
        pub fn update(
            &mut self,
            body_yaw: f32,
            look_yaw: f32,
            look_pitch: f32,
            time: f64,
            compatible: bool,
            stationary: bool,
            key_pressed: bool,
            settings: &TurnSettings,
        ) -> f32 {
            if !(body_yaw.is_finite()
                && look_yaw.is_finite()
                && look_pitch.is_finite()
                && time.is_finite())
            {
                self.clear();
                return body_yaw;
            }
            if !self.initialized || time < self.last_time || time - self.last_time > 0.5 {
                self.reset(body_yaw, look_yaw, look_pitch, time);
            }
            if !compatible || !settings.enabled {
                self.reset(body_yaw, look_yaw, look_pitch, time);
                return body_yaw;
            }
            let dt = (time - self.last_time) as f32;
            if dt <= 0.0 {
                return if self.managed {
                    self.body_yaw
                } else {
                    body_yaw
                };
            }
            self.last_time = time;
            if wrap_degrees360(look_yaw - self.quiet_yaw).abs() > 0.15
                || (look_pitch - self.quiet_pitch).abs() > 0.15
            {
                self.quiet_yaw = look_yaw;
                self.quiet_pitch = look_pitch;
                self.last_look_change = time;
            }
            if !stationary {
                // The feet moved: hand the yaw back to them over the fade.
                self.camera_mode = false;
                self.dynamic_mode = false;
                self.clip = None;
                self.weight = 0.0;
                if !self.managed {
                    self.body_yaw = body_yaw;
                    self.reset_continuous_turns(look_yaw, time);
                    return body_yaw;
                }
                if !self.releasing {
                    self.releasing = true;
                    self.release_yaw = self.body_yaw;
                    self.release_time = 0.0;
                }
                self.release_time += dt;
                let eased = ease_turn((self.release_time / settings.fade).min(1.0));
                self.body_yaw =
                    self.release_yaw + wrap_degrees360(body_yaw - self.release_yaw) * eased;
                if eased >= 1.0 {
                    self.managed = false;
                    self.releasing = false;
                    self.body_yaw = body_yaw;
                }
                self.reset_continuous_turns(look_yaw, time);
                self.last_look_change = time;
                return self.body_yaw;
            }
            // Stationary: the playback runs, in the mode the settings pick.
            self.managed = true;
            self.releasing = false;
            self.dynamic_mode = settings.mode == TurnMode::Dynamic;
            if self.dynamic_mode {
                self.camera_mode = false;
                self.body_yaw = self
                    .keyframed_turn
                    .update(look_yaw, dt, key_pressed, settings);
                self.weight = if self.keyframed_turn.active() {
                    1.0
                } else {
                    0.0
                };
                return self.body_yaw;
            }
            if settings.mode == TurnMode::Camera {
                self.camera_mode = true;
                self.body_yaw = self.camera_turn.update(look_yaw, time, dt, settings);
                self.weight = if self.camera_turn.active() { 1.0 } else { 0.0 };
                return self.body_yaw;
            }
            self.camera_mode = false;
            if let Some(clip) = &self.clip {
                let length = clip.length;
                self.time = length.min(self.time + dt);
                self.progress = self.progress.max(rotation_progress(clip, self.time));
                self.body_yaw = self.start_yaw
                    + wrap_degrees360(self.target_yaw - self.start_yaw) * self.progress;
                let fade = settings.fade.min(length * 0.2);
                self.weight = self.strength
                    * ease_turn((self.time / TURN_ENTRY_SECONDS).min(1.0))
                    * ease_turn(((length - self.time) / fade).min(1.0));
                if self.time >= length {
                    self.body_yaw = self.target_yaw;
                    self.clip = None;
                    self.weight = 0.0;
                }
                return self.body_yaw;
            }
            let delta = wrap_degrees360(look_yaw - self.body_yaw);
            if delta.abs() > TURN_ALIGNMENT_EPSILON
                && (key_pressed
                    || delta.abs() >= settings.threshold
                    || time - self.last_look_change >= settings.delay as f64)
            {
                self.start_yaw = self.body_yaw;
                self.target_yaw = self.body_yaw + delta;
                self.mirrored = settings.mode == TurnMode::Scaled && delta < 0.0;
                self.clip = if settings.mode == TurnMode::Scaled {
                    settings.template.clone()
                } else if delta < 0.0 {
                    settings.left.clone()
                } else {
                    settings.right.clone()
                };
                self.settings_addition = if settings.mode == TurnMode::Scaled {
                    None
                } else if delta < 0.0 {
                    settings.left_add.clone()
                } else {
                    settings.right_add.clone()
                };
                self.strength = (delta.abs() / settings.reference).clamp(0.0, 1.0);
                self.progress = 0.0;
                self.time = 0.0;
                self.weight = 0.0;
            }
            self.body_yaw
        }

        /// `sample`: the pose one leg or arm carries this frame.
        pub fn sample(&self, bone: &str) -> Pose {
            if self.dynamic_mode {
                return self.keyframed_turn.sample(bone);
            }
            if self.camera_mode {
                return self.camera_turn.sample(bone);
            }
            let Some(clip) = &self.clip else {
                return Pose::identity();
            };
            if !self.mirrored {
                return sample_authored(clip, self.settings_addition.as_ref(), bone, self.time);
            }
            let mirrored_bone = match bone {
                "left_leg" => "right_leg",
                "right_leg" => "left_leg",
                "left_arm" => "right_arm",
                "right_arm" => "left_arm",
                other => other,
            };
            let pose = sample_authored(
                clip,
                self.settings_addition.as_ref(),
                mirrored_bone,
                self.time,
            );
            Pose {
                position: Vec3::new(-pose.position.x, pose.position.y, pose.position.z),
                rotation: Vec3::new(pose.rotation.x, -pose.rotation.y, -pose.rotation.z),
                scale: pose.scale,
            }
        }

        /// `sampleAuthored`: the clip's pose plus the settings addition's.
        pub fn sample_authored(
            clip: &Clip,
            addition: Option<&Clip>,
            bone: &str,
            time: f32,
        ) -> Pose {
            sample_authored(clip, addition, bone, time)
        }

        /// `apply`: the pose added onto a part, scaled by `weight`.
        pub fn apply_part(part: &mut PartTransform, pose: &Pose, weight: f32) {
            if weight <= 0.0 {
                return;
            }
            part.x += pose.position.x * weight;
            part.y -= pose.position.y * weight;
            part.z += pose.position.z * weight;
            part.x_rot += pose.rotation.x.to_radians() * weight;
            part.y_rot += pose.rotation.y.to_radians() * weight;
            part.z_rot += pose.rotation.z.to_radians() * weight;
            part.x_scale *= 1.0 + (pose.scale.x - 1.0) * weight;
            part.y_scale *= 1.0 + (pose.scale.y - 1.0) * weight;
            part.z_scale *= 1.0 + (pose.scale.z - 1.0) * weight;
        }

        /// How drawn the turn playback is this frame.
        pub fn weight(&self) -> f32 {
            self.weight
        }

        /// The body yaw the controller holds.
        pub fn body_yaw(&self) -> f32 {
            self.body_yaw
        }

        fn reset(&mut self, body_yaw: f32, look_yaw: f32, look_pitch: f32, time: f64) {
            self.initialized = true;
            self.last_time = time;
            self.body_yaw = body_yaw;
            self.quiet_yaw = look_yaw;
            self.quiet_pitch = look_pitch;
            self.last_look_change = time;
            self.managed = false;
            self.releasing = false;
            self.weight = 0.0;
            self.time = 0.0;
            self.progress = 0.0;
            self.clip = None;
            self.mirrored = false;
            self.settings_addition = None;
            self.camera_mode = false;
            self.dynamic_mode = false;
            self.reset_continuous_turns(look_yaw, time);
        }

        fn reset_continuous_turns(&mut self, look: f32, time: f64) {
            self.camera_turn.reset(self.body_yaw, look, time);
            self.keyframed_turn.reset(self.body_yaw, look);
        }

        /// `clear`.
        pub fn clear(&mut self) {
            self.reset(0.0, 0.0, 0.0, 0.0);
            self.initialized = false;
        }
    }

    /// `TurnInPlaceController.rotationProgress`.
    fn rotation_progress(clip: &Clip, time: f32) -> f32 {
        let start = clip.sample_once("root", 0.0).rotation.y;
        let end = clip.sample_once("root", clip.length).rotation.y;
        if (end - start).abs() < 0.001 {
            ease_turn((time / clip.length).clamp(0.0, 1.0))
        } else {
            ((clip.sample_once("root", time).rotation.y - start) / (end - start)).clamp(0.0, 1.0)
        }
    }

    /// `TurnInPlaceController.sampleAuthored`.
    fn sample_authored(clip: &Clip, addition: Option<&Clip>, bone: &str, time: f32) -> Pose {
        let pose = clip.sample_once(bone, time);
        let Some(addition) = addition else {
            return pose;
        };
        let mut extra = addition.sample_once(bone, time);
        if time > addition.length && clip.length > addition.length {
            let fade =
                ease_turn(((clip.length - time) / (clip.length - addition.length)).clamp(0.0, 1.0));
            extra = Pose {
                position: extra.position * fade,
                rotation: extra.rotation * fade,
                scale: Vec3::new(1.0, 1.0, 1.0).lerp(extra.scale, fade),
            };
        }
        Pose {
            position: pose.position + extra.position,
            rotation: pose.rotation + extra.rotation,
            scale: pose.scale + extra.scale - Vec3::new(1.0, 1.0, 1.0),
        }
    }

    /// `Mth.lerp(delta, start, end)`.
    fn lerp(delta: f32, start: f32, end: f32) -> f32 {
        start + delta * (end - start)
    }

    /// The Java `positiveModulo`: the remainder taken positive.
    fn positive_modulo(value: f32, modulus: f32) -> f32 {
        (value % modulus + modulus) % modulus
    }

    // ---------------------------------------------------------------------------
    // DirectionChangeController
    // ---------------------------------------------------------------------------

    /// `DirectionChangeController.MIN_SPEED`.
    pub const DIRECTION_MIN_SPEED: f32 = 0.04;
    /// `DirectionChangeController.MAX_DEGREES`.
    pub const DIRECTION_MAX_DEGREES: f32 = 3.0;

    /// `DirectionChangeController`: the lean the travel direction's changes add,
    /// in degrees, per tick.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct DirectionChangeController {
        last_tick: i32,
        slow_ticks: i32,
        tracking: bool,
        travel_x: f32,
        travel_z: f32,
        previous_pitch: f32,
        pitch: f32,
        previous_roll: f32,
        roll: f32,
    }

    impl DirectionChangeController {
        /// `tick`: `dx`/`dz` the movement since the last tick, `yaw` the player's
        /// yaw in degrees, `alive` whether the player is still there.
        pub fn tick(&mut self, tick: i32, dx: f64, dz: f64, yaw: f32, alive: bool) {
            if tick == self.last_tick {
                return;
            }
            let stale =
                self.last_tick == i32::MIN || tick < self.last_tick || (tick - self.last_tick) > 2;
            self.last_tick = tick;
            if alive && !stale && dx.is_finite() && dz.is_finite() && yaw.is_finite() {
                self.previous_pitch = self.pitch;
                self.previous_roll = self.roll;
                let distance = dx.hypot(dz);
                let mut pitch = 0.0f32;
                let mut roll = 0.0f32;
                if distance >= DIRECTION_MIN_SPEED as f64 {
                    let nx = (dx / distance) as f32;
                    let nz = (dz / distance) as f32;
                    if self.tracking {
                        let turn = nx - self.travel_x;
                        let sideways = nz - self.travel_z;
                        let radians = yaw as f64 * (std::f64::consts::PI / 180.0);
                        let (sin, cos) = radians.sin_cos();
                        let forward = turn * cos as f32 + sideways * sin as f32;
                        let across = -turn * sin as f32 + sideways * cos as f32;
                        let scaled = (distance as f32 / 0.215).clamp(0.0, 1.0);
                        roll = (-forward * 5.0 * scaled)
                            .clamp(-DIRECTION_MAX_DEGREES, DIRECTION_MAX_DEGREES);
                        pitch = (-across * 5.0 * scaled)
                            .clamp(-DIRECTION_MAX_DEGREES, DIRECTION_MAX_DEGREES);
                    } else {
                        self.travel_x = nx;
                        self.travel_z = nz;
                        self.tracking = true;
                    }
                    self.travel_x = lerp(0.45, self.travel_x, nx);
                    self.travel_z = lerp(0.45, self.travel_z, nz);
                    self.slow_ticks = 0;
                } else {
                    self.slow_ticks += 1;
                    if self.slow_ticks > 3 {
                        self.tracking = false;
                    }
                }
                self.pitch = direction_approach(self.pitch, pitch);
                self.roll = direction_approach(self.roll, roll);
            } else {
                self.reset();
            }
        }

        /// `pitch` between the last two ticks.
        pub fn pitch(&self, partial: f32) -> f32 {
            lerp(partial.clamp(0.0, 1.0), self.previous_pitch, self.pitch)
        }

        /// `roll` between the last two ticks.
        pub fn roll(&self, partial: f32) -> f32 {
            lerp(partial.clamp(0.0, 1.0), self.previous_roll, self.roll)
        }

        fn reset(&mut self) {
            self.tracking = false;
            self.slow_ticks = 0;
            self.previous_pitch = 0.0;
            self.pitch = 0.0;
            self.previous_roll = 0.0;
            self.roll = 0.0;
        }
    }

    /// `DirectionChangeController.approach`.
    fn direction_approach(current: f32, target: f32) -> f32 {
        let out = lerp(0.4, current, target);
        if out.abs() < 0.001 { 0.0 } else { out }
    }

    // ---------------------------------------------------------------------------
    // TurnLeanController
    // ---------------------------------------------------------------------------

    /// `TurnLeanController.MAX_DEGREES`.
    pub const TURN_LEAN_MAX_DEGREES: f32 = 14.0;

    /// `TurnLeanController`: the sideways lean the look turn adds while the
    /// player is visible or holding jump.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct TurnLeanController {
        yaw: f32,
        previous: f32,
        current: f32,
        last_tick: i32,
        visible: bool,
        holding_jump: bool,
    }

    impl TurnLeanController {
        /// `tick`: `yaw` the look yaw, `speed` the walk speed, `x_input`/`z_input`
        /// the movement inputs, `visible` whether the player is on screen,
        /// `jumping` whether the jump key is pressed, `holding_jump` whether it
        /// is held.
        #[allow(clippy::too_many_arguments)]
        pub fn tick(
            &mut self,
            tick: i32,
            yaw: f32,
            speed: f32,
            x_input: f32,
            z_input: f32,
            visible: bool,
            jumping: bool,
            holding_jump: bool,
        ) {
            if tick == self.last_tick {
                return;
            }
            if yaw.is_finite()
                && self.last_tick != i32::MIN
                && tick >= self.last_tick
                && tick - self.last_tick <= 2
            {
                let turned = wrap_degrees360(yaw - self.yaw);
                self.yaw = yaw;
                self.last_tick = tick;
                self.previous = self.current;
                self.holding_jump = holding_jump && (self.holding_jump || jumping && self.visible);
                self.visible = visible || self.holding_jump;
                if !self.holding_jump {
                    let mut target = 0.0f32;
                    if visible && speed.is_finite() && x_input.is_finite() && z_input.is_finite() {
                        let scaled = ((speed - 0.01) / 0.205).clamp(0.0, 1.25);
                        let from_turn = if turned.abs() <= 60.0 {
                            -turned * 1.4 * x_input.clamp(-1.0, 1.0)
                        } else {
                            0.0
                        };
                        let from_strafe = 6.0 * z_input.clamp(-1.0, 1.0);
                        target = ((from_turn + from_strafe) * scaled)
                            .clamp(-TURN_LEAN_MAX_DEGREES, TURN_LEAN_MAX_DEGREES);
                    }
                    let rate = if target == 0.0 { 0.3 } else { 0.38 };
                    self.current += (target - self.current) * rate;
                    if self.current.abs() < 0.001 {
                        self.current = 0.0;
                    }
                }
            } else {
                self.yaw = if yaw.is_finite() { yaw } else { 0.0 };
                self.previous = 0.0;
                self.current = 0.0;
                self.visible = false;
                self.holding_jump = false;
                self.last_tick = tick;
            }
        }

        /// `sample` between the last two ticks.
        pub fn sample(&self, partial: f32) -> f32 {
            if self.visible {
                lerp(partial.clamp(0.0, 1.0), self.previous, self.current)
            } else {
                0.0
            }
        }

        /// `apply`: the lean rotates a part about the world Y through the part's
        /// own rotation, keeping the head's top in place; `extra` adds degrees
        /// and `extra_z` a sideways offset in degrees.
        pub fn apply_part(&self, part: &mut PartTransform, partial: f32, extra_z: f32, extra: f32) {
            let degrees =
                (self.sample(partial) + extra).clamp(-TURN_LEAN_MAX_DEGREES, TURN_LEAN_MAX_DEGREES);
            if degrees.abs() < 0.001 && extra_z.abs() < 0.001 {
                return;
            }
            let own = Quat::from_zyx_degrees(Vec3::new(
                part.x_rot.to_degrees(),
                part.y_rot.to_degrees(),
                part.z_rot.to_degrees(),
            ));
            let leaned = Quat::from_zyx_degrees(Vec3::new(degrees, 0.0, extra_z));
            let pivot = Vec3::new(0.0, 24.0 * part.y_scale, 0.0);
            let moved = own.transform(pivot) - leaned.transform(pivot);
            part.x += moved.x;
            part.y += moved.y;
            part.z += moved.z;
            let euler = leaned.to_euler_zyx_degrees();
            part.x_rot = euler.x.to_radians();
            part.y_rot = euler.y.to_radians();
            part.z_rot = euler.z.to_radians();
        }
    }

    // ---------------------------------------------------------------------------
    // CreativeFlightState
    // ---------------------------------------------------------------------------

    /// `CreativeFlightState.Motion`: the hover detector for remote players in
    /// creative.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CreativeFlightMotion {
        deltas: [f64; 8],
        tick: i32,
        y: f64,
        count: usize,
        cursor: usize,
        flying: bool,
    }

    impl Default for CreativeFlightMotion {
        fn default() -> Self {
            Self {
                deltas: [0.0; 8],
                tick: i32::MIN,
                y: 0.0,
                count: 0,
                cursor: 0,
                flying: false,
            }
        }
    }

    impl CreativeFlightMotion {
        /// `Motion.update`: `y` the player's height, `eligible` whether every
        /// other flying reason is out.
        pub fn update(&mut self, tick: i32, y: f64, eligible: bool) -> bool {
            if !eligible || self.tick == i32::MIN || tick < self.tick || tick - self.tick > 2 {
                self.count = 0;
                self.cursor = 0;
                self.flying = false;
                self.tick = tick;
                self.y = y;
                return false;
            }
            if tick == self.tick {
                return self.flying;
            }
            self.deltas[self.cursor] = (y - self.y) / (tick - self.tick) as f64;
            self.cursor = (self.cursor + 1) % self.deltas.len();
            self.count = self.deltas.len().min(self.count + 1);
            self.tick = tick;
            self.y = y;
            let lowest = self.deltas[..self.count]
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let highest = self.deltas[..self.count]
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            if self.count == self.deltas.len()
                && highest - lowest < 0.035
                && lowest > -0.65
                && highest < 0.65
            {
                self.flying = true;
            } else if self.flying {
                // A steady descent ends the hover: the three newest deltas
                // (just behind the cursor) sink faster than the three before
                // them.
                let mut newest = 0.0;
                let mut older = 0.0;
                for j in 0..3 {
                    older += self.deltas[(self.cursor + j) % self.deltas.len()] / 3.0;
                    newest += self.deltas
                        [(self.cursor + self.deltas.len() - 3 + j) % self.deltas.len()]
                        / 3.0;
                }
                if newest < -0.2 && newest - older < -0.18 {
                    self.flying = false;
                }
            }
            self.flying
        }

        /// Whether the hover is on.
        pub fn flying(&self) -> bool {
            self.flying
        }
    }

    // ---------------------------------------------------------------------------
    // ElytraFlightAlignment
    // ---------------------------------------------------------------------------

    /// `ElytraFlightAlignment.MESH_SCALE`.
    pub const ELYTRA_MESH_SCALE: f32 = 0.9375;
    /// `ElytraFlightAlignment.MODEL_BASE`.
    pub const ELYTRA_MODEL_BASE: f32 = 1.501;

    /// `ElytraFlightAlignment`: the glide pose's height matched to the hitbox.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct ElytraFlightAlignment {
        normal_height: f32,
        fast_height: f32,
        target_height: f32,
        configured: bool,
    }

    impl ElytraFlightAlignment {
        /// `apply`: the model part's y moves by the difference between the
        /// hitbox's half height and the pose's reference height.
        #[allow(clippy::too_many_arguments)]
        pub fn apply(
            &mut self,
            part: &mut PartTransform,
            hitbox_height: Option<f32>,
            pitch_scale: f32,
            weight: f32,
            blend: f32,
            normal: Option<&Clip>,
            fast: Option<&Clip>,
        ) {
            if let Some(height) = hitbox_height
                && height.is_finite()
            {
                self.target_height = height * 0.5;
            }
            let Some(_) = hitbox_height else {
                return;
            };
            // The reference heights need both clips; without them there is no
            // correction.
            let (Some(rotate), Some(fast_or_normal)) = (normal, fast.or(normal)) else {
                return;
            };
            if !self.configured {
                self.normal_height = reference_height_pair(rotate, normal);
                self.fast_height = reference_height_pair(rotate, fast);
                self.configured = true;
            }
            let _ = fast_or_normal;
            if weight <= 1.0e-5 || pitch_scale <= 1.0e-5 {
                return;
            }
            let reference =
                lerp(blend.clamp(0.0, 1.0), self.normal_height, self.fast_height) * weight;
            let offset = (self.target_height - reference) * pitch_scale.clamp(0.0, 1.0);
            part.y -= offset * 16.0 / (ELYTRA_MESH_SCALE * weight);
        }
    }

    /// `ElytraFlightAlignment.referenceHeight`: the pose's torso height over the
    /// model base, averaged over 64 samples.
    fn reference_height_pair(rotate: &Clip, over: Option<&Clip>) -> f32 {
        let mut total = 0.0f32;
        for i in 0..64 {
            let over_time = over.map(|clip| clip.length * i as f32 / 64.0);
            let mut matrix = Mat4Translate::new(0.0, -ELYTRA_MODEL_BASE, 0.0);
            apply_pose_transform(
                &mut matrix,
                rotate.sample("root", rotate.length),
                over.map(|clip| clip.sample("root", over_time.unwrap_or(0.0))),
            );
            apply_pose_transform(
                &mut matrix,
                rotate.sample("body", rotate.length),
                over.map(|clip| clip.sample("body", over_time.unwrap_or(0.0))),
            );
            total -= matrix.transform_position(0.0, 0.375, 0.0)[1] * ELYTRA_MESH_SCALE;
        }
        total / 64.0
    }

    /// A tiny translate/rotate/scale matrix chain, the Joml calls the reference
    /// height math makes.
    #[derive(Debug, Clone, Copy)]
    struct Mat4Translate {
        translation: [f32; 3],
        rotation: Quat,
        scale: [f32; 3],
    }

    impl Mat4Translate {
        fn new(x: f32, y: f32, z: f32) -> Self {
            Self {
                translation: [x, y, z],
                rotation: Quat::IDENTITY,
                scale: [1.0, 1.0, 1.0],
            }
        }

        fn transform_position(&self, x: f32, y: f32, z: f32) -> [f32; 3] {
            let v = Vec3::new(x * self.scale[0], y * self.scale[1], z * self.scale[2]);
            let rotated = self.rotation.transform(v);
            [
                rotated.x + self.translation[0],
                rotated.y + self.translation[1],
                rotated.z + self.translation[2],
            ]
        }
    }

    /// `ElytraFlightAlignment.transform`: the base pose plus the overlay's.
    fn apply_pose_transform(matrix: &mut Mat4Translate, base: Pose, over: Option<Pose>) {
        let mut position = base.position;
        let mut rotation = base.rotation;
        let mut scale = base.scale;
        if let Some(over) = over {
            position = position + over.position;
            rotation = rotation + over.rotation;
            scale = scale + over.scale - Vec3::new(1.0, 1.0, 1.0);
        }
        matrix.translation[0] += position.x / 16.0;
        matrix.translation[1] += -position.y / 16.0;
        matrix.translation[2] += position.z / 16.0;
        matrix.rotation = Quat::from_zyx_degrees(rotation).mul(matrix.rotation);
        matrix.scale = [
            matrix.scale[0] * scale.x,
            matrix.scale[1] * scale.y,
            matrix.scale[2] * scale.z,
        ];
    }

    // ---------------------------------------------------------------------------
    // SwimSurfaceAlignment
    // ---------------------------------------------------------------------------

    /// `SwimSurfaceAlignment.DT`.
    pub const SWIM_DT: f32 = 0.05;
    /// `SwimSurfaceAlignment.EYE_ABOVE_TORSO`.
    pub const SWIM_EYE_ABOVE_TORSO: f32 = 0.2;
    /// The depth the alignment falls back to.
    pub const SWIM_DEFAULT_DEPTH: f32 = 0.12;

    /// `SwimSurfaceAlignment`: keeps the swimming body at the water surface.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct SwimSurfaceAlignment {
        reference_torso_height: f32,
        has_reference: bool,
        eye_initialized: bool,
        previous_eye_height: f32,
        eye_height: f32,
        previous_weight: f32,
        weight: f32,
        previous_surface: f32,
        surface: f32,
        previous_water_y: f64,
        water_y: f64,
        depth: f32,
    }

    impl SwimSurfaceAlignment {
        /// `prepareEyeHeight`.
        pub fn prepare_eye_height(&mut self, height: f32) {
            if height.is_finite() {
                if !self.eye_initialized {
                    self.previous_eye_height = height;
                    self.eye_height = height;
                    self.eye_initialized = true;
                } else {
                    self.previous_eye_height = self.eye_height;
                    self.eye_height = lerp(1.0 - (-0.5f32).exp(), self.eye_height, height);
                }
            }
        }

        /// `tick`.
        pub fn tick(&mut self, swimming: bool, at_surface: bool, water_y: f64, depth: f32) {
            self.previous_weight = self.weight;
            self.previous_surface = self.surface;
            self.previous_water_y = self.water_y;
            self.weight = swim_approach(
                self.weight,
                if self.has_reference && swimming {
                    1.0
                } else {
                    0.0
                },
                0.25,
            );
            if swimming && self.has_reference {
                self.surface =
                    swim_approach(self.surface, if at_surface { 1.0 } else { 0.0 }, 0.25);
            }
            self.depth = if depth.is_finite() {
                depth.max(0.0)
            } else {
                SWIM_DEFAULT_DEPTH
            };
            if swimming && self.has_reference && water_y.is_finite() {
                self.water_y = if self.water_y.is_finite() {
                    self.water_y + (water_y - self.water_y) * (1.0 - (-0.5f64).exp())
                } else {
                    water_y
                };
                if !self.previous_water_y.is_finite() {
                    self.previous_water_y = self.water_y;
                }
            }
        }

        /// Sets the reference clip's torso height (the glue measures it once).
        pub fn set_reference_height(&mut self, height: f32) {
            self.reference_torso_height = height;
            self.has_reference = true;
        }

        /// `active`.
        pub fn active(&self) -> bool {
            self.weight > 1.0e-4 || self.previous_weight > 1.0e-4
        }

        /// `worldOffset`.
        pub fn world_offset(&self, player_y: f64, scale: f32, partial: f32) -> f32 {
            if !(self.eye_initialized && self.has_reference && scale > 1.0e-5) {
                return 0.0;
            }
            let amount = self.amount(partial);
            if amount <= 0.0 {
                return 0.0;
            }
            let eye = lerp(partial, self.previous_eye_height, self.eye_height);
            let torso = eye - SWIM_EYE_ABOVE_TORSO * scale;
            let water = if self.water_y.is_finite() && self.previous_water_y.is_finite() {
                self.previous_water_y + (self.water_y - self.previous_water_y) * partial as f64
            } else {
                f64::NAN
            };
            let surface = if water.is_finite() {
                smooth_swim(lerp(partial, self.previous_surface, self.surface))
            } else {
                0.0
            };
            let above = (water - player_y) as f32 - self.depth * scale;
            let offset = if surface <= 0.0 {
                torso
            } else {
                lerp(surface, torso, above)
            };
            (offset - self.reference_torso_height * scale) * amount
        }

        /// `apply`: the model part's y moves by the world offset, in model units.
        pub fn apply(&self, part: &mut PartTransform, player_y: f64, scale: f32, partial: f32) {
            let offset = self.world_offset(player_y, scale, partial);
            part.y -= offset * 16.0 / (ELYTRA_MESH_SCALE * scale.max(1.0e-5));
        }

        /// `cameraEyeHeight`.
        pub fn camera_eye_height(
            &self,
            player_y: f64,
            scale: f32,
            partial: f32,
            standing: f32,
        ) -> f32 {
            let raised = (self.reference_torso_height + SWIM_EYE_ABOVE_TORSO) * scale;
            lerp(self.amount(partial), standing, raised)
                + self.world_offset(player_y, scale, partial)
        }

        /// `clear`.
        pub fn clear(&mut self) {
            self.previous_weight = 0.0;
            self.weight = 0.0;
            self.previous_surface = 0.0;
            self.surface = 0.0;
            self.previous_water_y = f64::NAN;
            self.water_y = f64::NAN;
            self.eye_initialized = false;
        }

        fn amount(&self, partial: f32) -> f32 {
            smooth_swim(lerp(partial, self.previous_weight, self.weight))
        }
    }

    /// `SwimSurfaceAlignment.referenceHeight`: the reference clip's torso height.
    pub fn swim_reference_height(clip: &Clip) -> f32 {
        let mut total = 0.0f32;
        for i in 0..64 {
            let time = clip.length * i as f32 / 64.0;
            let mut matrix = Mat4Translate::new(0.0, -ELYTRA_MODEL_BASE, 0.0);
            apply_pose_transform(&mut matrix, clip.sample("root", time), None);
            apply_pose_transform(&mut matrix, clip.sample("body", time), None);
            total += -matrix.transform_position(0.0, 0.375, 0.0)[1] * ELYTRA_MESH_SCALE;
        }
        total / 64.0
    }

    fn swim_approach(current: f32, target: f32, step: f32) -> f32 {
        if current < target {
            target.min(current + step)
        } else {
            target.max(current - step)
        }
    }

    fn smooth_swim(value: f32) -> f32 {
        value * value * (3.0 - 2.0 * value)
    }

    // ---------------------------------------------------------------------------
    // FishingLinePhysics
    // ---------------------------------------------------------------------------

    /// `FishingLinePhysics.GRAVITY`.
    pub const FISHING_GRAVITY: f64 = 16.0;
    /// `FishingLinePhysics.STEP`: one integration step, 1/120 s.
    pub const FISHING_STEP: f64 = 0.008_333_333_333_333_333;
    /// `FishingLinePhysics.LONGEST_GAP`: the most seconds one frame may simulate.
    pub const FISHING_LONGEST_GAP: f64 = 0.25;
    /// `FishingLinePhysics.KEEP`: how much of the bob's velocity each step keeps.
    pub fn fishing_keep() -> f64 {
        (-0.013_333_333_333_333_334f64).exp()
    }

    /// `FishingLinePhysics.Swing`: the bob's verlet-ish fall along the line.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FishingSwing {
        bob: [f64; 3],
        previous: [f64; 3],
        last_pivot: [f64; 3],
        time: f64,
        carried: f64,
    }

    impl Default for FishingSwing {
        fn default() -> Self {
            Self {
                bob: [0.0; 3],
                previous: [0.0; 3],
                last_pivot: [0.0; 3],
                time: f64::NAN,
                carried: 0.0,
            }
        }
    }

    impl FishingSwing {
        /// `update`: `pivot` where the rod tip is, `length` the line's length,
        /// `time` the animation time; returns the unit vector from the pivot to
        /// the bob.
        pub fn update(&mut self, pivot: [f64; 3], length: f64, time: f64) -> [f64; 3] {
            let fresh = self.time.is_nan()
                || time < self.time
                || time - self.time > FISHING_LONGEST_GAP
                || length <= 1.0e-6;
            if fresh {
                self.bob = [pivot[0], pivot[1] - length.max(0.0), pivot[2]];
                self.previous = self.bob;
                self.last_pivot = pivot;
                self.time = time;
                self.carried = 0.0;
                return [0.0, -1.0, 0.0];
            }
            let elapsed = time - self.time + self.carried;
            let steps = (elapsed / FISHING_STEP) as i32;
            self.carried = elapsed - steps as f64 * FISHING_STEP;
            self.time = time;
            let keep = fishing_keep();
            for step in 1..=steps {
                let t = step as f64 / steps as f64;
                let anchor = [
                    self.last_pivot[0] + (pivot[0] - self.last_pivot[0]) * t,
                    self.last_pivot[1] + (pivot[1] - self.last_pivot[1]) * t,
                    self.last_pivot[2] + (pivot[2] - self.last_pivot[2]) * t,
                ];
                let velocity = [
                    (self.bob[0] - self.previous[0]) * keep,
                    (self.bob[1] - self.previous[1]) * keep,
                    (self.bob[2] - self.previous[2]) * keep,
                ];
                self.previous = self.bob;
                self.bob[0] += velocity[0];
                self.bob[1] += velocity[1] - 0.001_111_111_111_111_111_1;
                self.bob[2] += velocity[2];
                fishing_pull_in(&mut self.bob, &anchor, length);
            }
            if steps > 0 {
                self.last_pivot = pivot;
            }
            let mut out = [
                self.bob[0] - pivot[0],
                self.bob[1] - pivot[1],
                self.bob[2] - pivot[2],
            ];
            let len = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
            if len < 1.0e-9 {
                out = [0.0, -1.0, 0.0];
            } else {
                out = [out[0] / len, out[1] / len, out[2] / len];
            }
            out
        }
    }

    /// `FishingLinePhysics.pullIn`.
    fn fishing_pull_in(bob: &mut [f64; 3], anchor: &[f64; 3], length: f64) {
        let dx = bob[0] - anchor[0];
        let dy = bob[1] - anchor[1];
        let dz = bob[2] - anchor[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        if distance > length {
            let scale = length / distance;
            *bob = [
                anchor[0] + dx * scale,
                anchor[1] + dy * scale,
                anchor[2] + dz * scale,
            ];
        }
    }

    // ---------------------------------------------------------------------------
    // BoatPaddleAnimation
    // ---------------------------------------------------------------------------

    /// `BoatPaddleAnimation.STEP`: a quarter paddle stroke, in radians.
    pub const PADDLE_STEP: f32 = std::f32::consts::PI / 8.0;
    /// The phase magnitude the paddles wrap at.
    const PADDLE_PHASE_WRAP: f32 = 100.53097;
    /// How much weight a paddling boat gains per tick.
    const PADDLE_WEIGHT_PER_TICK: f32 = 0.2;

    /// `BoatPaddleAnimation.Pose`: one paddle's pitch and yaw, in radians.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct PaddlePose {
        pub pitch: f32,
        pub yaw: f32,
    }

    /// `BoatPaddleAnimation.vanillaPose`: the vanilla paddle angle for one side.
    pub fn paddle_vanilla_pose(side: i32, phase: f32) -> PaddlePose {
        let pitch = lerp(
            ((-phase).sin() + 1.0) / 2.0,
            -std::f32::consts::PI / 3.0,
            -std::f32::consts::PI / 12.0,
        );
        let yaw = lerp(
            ((-phase + 1.0).sin() + 1.0) / 2.0,
            -std::f32::consts::PI / 4.0,
            std::f32::consts::PI / 4.0,
        );
        PaddlePose {
            pitch,
            yaw: if side == 1 {
                std::f32::consts::PI - yaw
            } else {
                yaw
            },
        }
    }

    /// `BoatPaddleAnimation.oppositeSide`.
    pub fn paddle_opposite_side(pose: PaddlePose) -> PaddlePose {
        PaddlePose {
            pitch: pose.pitch,
            yaw: std::f32::consts::PI - pose.yaw,
        }
    }

    /// `BoatPaddleAnimation.Paddle`: one paddle's eased stroke.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Paddle {
        side: i32,
        direction: i32,
        phase_end: f32,
        offset: PaddlePose,
        velocity_offset: PaddlePose,
        previous_time: f32,
        time: f32,
    }

    impl Paddle {
        /// `Paddle(side, paddling, phase)`.
        pub fn new(side: i32, direction: i32, phase_end: f32) -> Self {
            Self {
                side,
                direction,
                phase_end,
                offset: PaddlePose::default(),
                velocity_offset: PaddlePose::default(),
                previous_time: 0.3,
                time: 0.3,
            }
        }

        /// `tick`.
        pub fn tick(&mut self, direction: i32, phase_end: f32) {
            if direction != self.direction {
                let pose = self.sample(1.0);
                let velocity = self.velocity(1.0);
                self.direction = direction;
                self.phase_end = phase_end;
                let target = self.target(0.0);
                let target_velocity = self.target_velocity(0.0);
                self.offset = PaddlePose {
                    pitch: pose.pitch - target.pitch,
                    yaw: pose.yaw - target.yaw,
                };
                self.velocity_offset = PaddlePose {
                    pitch: velocity.pitch - target_velocity.pitch,
                    yaw: velocity.yaw - target_velocity.yaw,
                };
                self.previous_time = 0.0;
                self.time = 0.05;
            } else {
                self.previous_time = self.time;
                self.time = (self.time + 0.05).min(0.3);
            }
            self.phase_end = phase_end;
        }

        /// `sample`.
        pub fn sample(&self, partial: f32) -> PaddlePose {
            let partial = partial.clamp(0.0, 1.0);
            let target = self.target(partial);
            let progress = self.progress(partial);
            let hold =
                1.0 - progress * progress * progress * (10.0 + progress * (-15.0 + 6.0 * progress));
            let drift = self.duration()
                * (progress
                    - progress * progress * progress * (6.0 + progress * (-8.0 + 3.0 * progress)));
            PaddlePose {
                pitch: target.pitch + self.offset.pitch * hold + self.velocity_offset.pitch * drift,
                yaw: target.yaw + self.offset.yaw * hold + self.velocity_offset.yaw * drift,
            }
        }

        /// `velocity`.
        pub fn velocity(&self, partial: f32) -> PaddlePose {
            let target = self.target_velocity(partial);
            let progress = self.progress(partial);
            let swing =
                -30.0 * progress * progress * (1.0 - progress) * (1.0 - progress) / self.duration();
            let hold = 1.0 - progress * progress * (18.0 + progress * (-32.0 + 15.0 * progress));
            PaddlePose {
                pitch: target.pitch + self.offset.pitch * swing + self.velocity_offset.pitch * hold,
                yaw: target.yaw + self.offset.yaw * swing + self.velocity_offset.yaw * hold,
            }
        }

        fn duration(&self) -> f32 {
            if self.direction != 0 { 0.15 } else { 0.3 }
        }

        fn progress(&self, partial: f32) -> f32 {
            (lerp(partial, self.previous_time, self.time) / self.duration()).clamp(0.0, 1.0)
        }

        fn phase(&self, partial: f32) -> f32 {
            lerp(
                partial,
                self.phase_end - self.direction as f32 * PADDLE_STEP,
                self.phase_end,
            )
        }

        fn target(&self, partial: f32) -> PaddlePose {
            paddle_vanilla_pose(
                self.side,
                if self.direction != 0 {
                    self.phase(partial)
                } else {
                    0.0
                },
            )
        }

        fn target_velocity(&self, partial: f32) -> PaddlePose {
            if self.direction == 0 {
                return PaddlePose::default();
            }
            let phase = self.phase(partial);
            let rate = self.direction as f32 * PADDLE_STEP / 0.05;
            let pitch = -PADDLE_STEP * (-phase).cos() * rate;
            let yaw = -std::f32::consts::PI / 4.0 * (-phase + 1.0).cos() * rate;
            PaddlePose {
                pitch,
                yaw: if self.side == 1 { -yaw } else { yaw },
            }
        }
    }

    /// `BoatPaddleAnimation.Animation`: both paddles and the rowing weight.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct BoatAnimation {
        paddles: [Paddle; 2],
        phase: [f32; 2],
        tick: i32,
        rower_id: i32,
        previous_weight: f32,
        weight: f32,
    }

    impl BoatAnimation {
        /// `new Animation(boat)`: `paddling` per side, `phase` per side, `chest`
        /// whether the boat is a chest boat (its paddles run mirrored).
        pub fn new(
            rower_id: i32,
            tick: i32,
            paddling: [bool; 2],
            phases: [f32; 2],
            chest: bool,
        ) -> Self {
            let sign = if chest { -1.0 } else { 1.0 };
            let mut paddles = [Paddle::new(0, 0, 0.0), Paddle::new(1, 0, 0.0)];
            let mut phase = [0.0f32; 2];
            for k in 0..2 {
                phase[k] = phases[k] * sign;
                paddles[k] = Paddle::new(
                    k as i32,
                    if paddling[k] { sign as i32 } else { 0 },
                    phase[k],
                );
            }
            Self {
                paddles,
                phase,
                tick,
                rower_id,
                previous_weight: 0.0,
                weight: 0.0,
            }
        }

        /// `animation(boat)`'s per-tick step: `ticks` the boat's ticks, `backing`
        /// whether the rower is steering backwards, `paddle_state` per side.
        pub fn step(
            &mut self,
            ticks: i32,
            rower_id: i32,
            backing: bool,
            paddle_state: [bool; 2],
            chest: bool,
        ) {
            let passed = ticks - self.tick;
            if !(0..=40).contains(&passed) || self.rower_id != rower_id {
                *self = BoatAnimation::new(rower_id, ticks, paddle_state, self.phase, chest);
                return;
            }
            if passed > 0 {
                for _ in 0..passed {
                    self.previous_weight = self.weight;
                    self.weight = (self.weight + PADDLE_WEIGHT_PER_TICK).min(1.0);
                    for (k, stroking) in paddle_state.into_iter().enumerate() {
                        let mut direction = if stroking {
                            1
                        } else if backing {
                            -1
                        } else {
                            0
                        };
                        if chest {
                            direction = -direction;
                        }
                        self.phase[k] += direction as f32 * PADDLE_STEP;
                        if self.phase[k].abs() > PADDLE_PHASE_WRAP {
                            self.phase[k] %= std::f32::consts::TAU;
                        }
                        self.paddles[k].tick(direction, self.phase[k]);
                    }
                }
                self.tick = ticks;
            }
        }

        /// `weight`: the eased rowing weight at `partial`.
        pub fn weight(&self, partial: f32) -> f32 {
            let f = lerp(partial.clamp(0.0, 1.0), self.previous_weight, self.weight);
            f * f * f * (10.0 + f * (-15.0 + 6.0 * f))
        }

        /// The two paddles' samples at `partial`.
        pub fn sample(&self, partial: f32) -> [PaddlePose; 2] {
            [
                self.paddles[0].sample(partial),
                self.paddles[1].sample(partial),
            ]
        }
    }

    /// `BoatPaddleAnimation.RiderMotion.Response`: the critically damped follow.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct MotionResponse {
        rate: f32,
        minimum: f32,
        maximum: f32,
        time: f64,
        value: f32,
        velocity: f32,
    }

    impl MotionResponse {
        fn new(rate: f32, minimum: f32, maximum: f32) -> Self {
            Self {
                rate,
                minimum,
                maximum,
                time: f64::NAN,
                value: 0.0,
                velocity: 0.0,
            }
        }

        fn update(&mut self, target: f32, time: f64) -> f32 {
            if !(target.is_finite() && time.is_finite()) {
                return self.value;
            }
            let dt = time - self.time;
            if !self.time.is_finite() || dt < 0.0 || dt > 0.5 {
                self.time = time;
                self.value = target;
                self.velocity = 0.0;
                return self.value;
            }
            if dt == 0.0 {
                return self.value;
            }
            let error = self.value - target;
            let decay = (-self.rate as f64 * dt).exp() as f32;
            let carried = (self.velocity + self.rate * error) * dt as f32;
            self.value = target + (error + carried) * decay;
            self.velocity = (self.velocity - self.rate * carried) * decay;
            self.value = self.value.clamp(self.minimum, self.maximum);
            self.time = time;
            self.value
        }
    }

    /// `BoatPaddleAnimation.RiderMotion`: how the rower's body follows the boat.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct RiderMotion {
        torso: MotionResponse,
        pitch: [MotionResponse; 2],
        yaw: [MotionResponse; 2],
    }

    impl Default for RiderMotion {
        fn default() -> Self {
            Self {
                torso: MotionResponse::new(16.0, -std::f32::consts::PI / 4.0, 0.610_865_24),
                pitch: [
                    MotionResponse::new(20.0, -1.2, 1.2),
                    MotionResponse::new(20.0, -1.2, 1.2),
                ],
                yaw: [
                    MotionResponse::new(20.0, -1.2, 1.2),
                    MotionResponse::new(20.0, -1.2, 1.2),
                ],
            }
        }
    }

    impl RiderMotion {
        /// `paddle`: the rider's paddle pose following the boat's.
        pub fn paddle(
            &mut self,
            side: usize,
            boat: PaddlePose,
            rider: PaddlePose,
            time: f64,
        ) -> PaddlePose {
            PaddlePose {
                pitch: boat.pitch + self.pitch[side].update(rider.pitch - boat.pitch, time),
                yaw: boat.yaw + self.yaw[side].update(rider.yaw - boat.yaw, time),
            }
        }

        /// `torso`.
        pub fn torso(&mut self, target: f32, time: f64) -> f32 {
            self.torso.update(target, time)
        }
    }

    // ---------------------------------------------------------------------------
    // ElytraOuterRotation, HeldItemAnimation
    // ---------------------------------------------------------------------------

    /// `ElytraOuterRotation.correctionDegrees`: the extra pitch the gliding body
    /// needs so the wings meet the flight path.
    pub fn elytra_correction_degrees(
        flying: bool,
        gliding: bool,
        paused: bool,
        weight: f32,
    ) -> f32 {
        if flying && gliding && !paused && weight.is_finite() {
            90.0 * weight.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// `HeldItemAnimation`: the matrix a held item's model is drawn through, from
    /// the locomotion pose's position, rotation and scale (model units).
    pub fn held_item_matrix(position: Vec3, rotation: Vec3, scale: Vec3) -> Mat4Builder {
        let mut out = Mat4Builder::identity();
        // The Java translates by (-x, -z, y) / 16.
        out = out.translate(-position.x / 16.0, -position.z / 16.0, position.y / 16.0);
        if rotation.length_squared() > 1.0e-6 {
            let q = Quat::from_zyx_degrees(Vec3::new(-rotation.x, -rotation.y, -rotation.z));
            out = out.rotate(q);
        }
        out.scale(scale.x.max(0.001), scale.y.max(0.001), scale.z.max(0.001))
    }

    /// A small matrix builder for the held-item transform, column-major.
    pub struct Mat4Builder([f32; 16]);

    impl Mat4Builder {
        /// The identity.
        pub fn identity() -> Self {
            let mut values = [0.0f32; 16];
            values[0] = 1.0;
            values[5] = 1.0;
            values[10] = 1.0;
            values[15] = 1.0;
            Self(values)
        }

        /// The raw column-major values.
        pub fn values(&self) -> &[f32; 16] {
            &self.0
        }

        /// Starts from an existing column-major matrix.
        pub fn from_matrix(values: &[f32; 16]) -> Self {
            Self(*values)
        }

        fn multiply(&self, other: &[f32; 16]) -> [f32; 16] {
            let a = super::camera::Mat4::from_column_major(&self.0);
            let b = super::camera::Mat4::from_column_major(other);
            let out = a.mul(&b);
            let mut values = [0.0f32; 16];
            for c in 0..4 {
                for r in 0..4 {
                    values[c * 4 + r] = out.columns[c][r];
                }
            }
            values
        }

        /// `translate`.
        pub fn translate(self, x: f32, y: f32, z: f32) -> Self {
            let mut t = [0.0f32; 16];
            t[0] = 1.0;
            t[5] = 1.0;
            t[10] = 1.0;
            t[15] = 1.0;
            t[12] = x;
            t[13] = y;
            t[14] = z;
            Self(self.multiply(&t))
        }

        /// `rotate`.
        pub fn rotate(self, q: Quat) -> Self {
            let m = q.to_matrix3();
            let mut r = [0.0f32; 16];
            // Column-major rotation matrix.
            r[0] = m[0][0];
            r[1] = m[1][0];
            r[2] = m[2][0];
            r[4] = m[0][1];
            r[5] = m[1][1];
            r[6] = m[2][1];
            r[8] = m[0][2];
            r[9] = m[1][2];
            r[10] = m[2][2];
            r[15] = 1.0;
            Self(self.multiply(&r))
        }

        /// `scale`.
        pub fn scale(self, x: f32, y: f32, z: f32) -> Self {
            let mut s = [0.0f32; 16];
            s[0] = x;
            s[5] = y;
            s[10] = z;
            s[15] = 1.0;
            Self(self.multiply(&s))
        }
    }

    // ---------------------------------------------------------------------------
    // PoseOwnershipBlend, OuterPoseOwnershipBlend
    // ---------------------------------------------------------------------------

    /// One ownership track's state.
    #[derive(Debug, Clone, PartialEq)]
    struct OwnershipTrack {
        last: Option<HashMap<String, [f32; 9]>>,
        source: Option<HashMap<String, [f32; 9]>>,
        external: bool,
        enabled: bool,
        time: f32,
        started: f32,
        duration: f32,
    }

    impl Default for OwnershipTrack {
        fn default() -> Self {
            Self {
                last: None,
                source: None,
                external: false,
                enabled: false,
                time: f32::NAN,
                started: 0.0,
                duration: 0.0,
            }
        }
    }

    /// `PoseOwnershipBlend`: the mod's pose hands the parts over to vanilla (or
    /// takes them back) through a smooth blend, per part, reading and writing the
    /// nine floats a model part carries.
    #[derive(Debug, Clone, Default)]
    pub struct PoseOwnershipBlend {
        tracks: HashMap<u64, OwnershipTrack>,
    }

    impl PoseOwnershipBlend {
        /// `apply`: `parts` are the nine floats each model part carries
        /// (position, rotation, scale), `external` whether the mod owns them
        /// this frame, `time` the animation time, `enabled` whether blending is
        /// on at all.
        pub fn apply(
            &mut self,
            key: u64,
            parts: &mut [(&'static str, &mut [f32; 9])],
            external: bool,
            time: f32,
            enabled: bool,
        ) {
            let track = self.tracks.entry(key).or_default();
            let continuous = time.is_finite()
                && track.time.is_finite()
                && time >= track.time
                && time - track.time <= 5.0
                && track.last.is_some();
            if !enabled || !track.enabled || !continuous {
                track.source = None;
            } else if external != track.external {
                track.source = track.last.take();
                track.started = time;
                track.duration = if external { 3.0 } else { 4.0 };
            }
            let mut weight = match &track.source {
                None => 1.0,
                Some(_) => ((time - track.started) / track.duration).clamp(0.0, 1.0),
            };
            weight = weight * weight * (3.0 - 2.0 * weight);
            let mut last: HashMap<String, [f32; 9]> = HashMap::new();
            for (name, values) in parts.iter_mut() {
                let mut current = **values;
                let source = track
                    .source
                    .as_ref()
                    .filter(|_| weight < 1.0)
                    .and_then(|source| source.get(*name).copied());
                if let Some(source) = source {
                    for i in 0..9 {
                        let mut delta = current[i] - source[i];
                        if (3..=5).contains(&i) {
                            delta = delta.sin().atan2(delta.cos());
                        }
                        current[i] = source[i] + delta * weight;
                    }
                }
                last.insert((*name).to_string(), current);
                **values = current;
            }
            if weight >= 1.0 {
                track.source = None;
            }
            track.last = Some(last);
            track.external = external;
            track.time = time;
            track.enabled = enabled;
        }
    }

    /// `OuterPoseOwnershipBlend.Rigid`: the outer transform's translation and
    /// rotation.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct RigidPose {
        position: [f32; 3],
        rotation: Quat,
    }

    /// One outer track's state.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct OuterTrack {
        last: Option<RigidPose>,
        source: Option<RigidPose>,
        external: bool,
        time: f32,
        started: f32,
        duration: f32,
    }

    impl Default for OuterTrack {
        fn default() -> Self {
            Self {
                last: None,
                source: None,
                external: false,
                time: f32::NAN,
                started: 0.0,
                duration: 0.0,
            }
        }
    }

    /// `OuterPoseOwnershipBlend`: the whole-model matrix the mod draws with
    /// hands over to vanilla (or takes back) through a smooth blend, keeping the
    /// transform rigid.
    #[derive(Debug, Clone, Default)]
    pub struct OuterPoseOwnershipBlend {
        tracks: HashMap<u64, OuterTrack>,
    }

    impl OuterPoseOwnershipBlend {
        /// `apply`: `outer` is the mod's matrix (column-major), `render` the
        /// vanilla one this frame would draw with, both read as 16 floats.
        pub fn apply(
            &mut self,
            key: u64,
            outer: Option<&[f32; 16]>,
            render: &mut [f32; 16],
            external: bool,
            time: f32,
            enabled: bool,
        ) {
            let Some(outer) = outer else {
                return;
            };
            if !enabled || !time.is_finite() {
                self.tracks.remove(&key);
                return;
            }
            let outer_matrix = super::camera::Mat4::from_column_major(outer);
            if outer_matrix.determinant().abs() < 1.0e-8 {
                self.tracks.remove(&key);
                return;
            }
            let render_matrix = super::camera::Mat4::from_column_major(render);
            // The delta from the mod's matrix to the render's, made rigid.
            let delta = outer_matrix.invert().map(|inv| inv.mul(&render_matrix));
            let Some(delta) = delta else {
                self.tracks.remove(&key);
                return;
            };
            let scale = delta.scale_factors();
            if !delta.is_finite()
                || scale.iter().any(|s| (s - 1.0).abs() > 0.001)
                || (delta.determinant() - 1.0).abs() > 0.001
            {
                self.tracks.remove(&key);
                return;
            }
            let rigid = RigidPose {
                position: delta.translation(),
                rotation: delta.unnormalized_rotation().normalized(),
            };
            let track = self.tracks.entry(key).or_default();
            let continuous = track.time.is_finite()
                && time >= track.time
                && time - track.time <= 5.0
                && track.last.is_some();
            if !continuous {
                track.source = None;
            } else if external != track.external {
                track.source = track.last;
                track.started = time;
                track.duration = if external { 3.0 } else { 4.0 };
            }
            let mut weight = match track.source {
                None => 1.0,
                Some(_) => ((time - track.started) / track.duration).clamp(0.0, 1.0),
            };
            weight = weight * weight * (3.0 - 2.0 * weight);
            let blended = match track.source {
                Some(source) if weight < 1.0 => RigidPose {
                    position: [
                        source.position[0] + (rigid.position[0] - source.position[0]) * weight,
                        source.position[1] + (rigid.position[1] - source.position[1]) * weight,
                        source.position[2] + (rigid.position[2] - source.position[2]) * weight,
                    ],
                    rotation: source.rotation.slerp(rigid.rotation, weight),
                },
                _ => rigid,
            };
            // render = outer * translate(blended.position) * rotate(blended.rotation)
            let applied = Mat4Builder::from_matrix(outer)
                .translate(
                    blended.position[0],
                    blended.position[1],
                    blended.position[2],
                )
                .rotate(blended.rotation);
            *render = *applied.values();
            track.last = Some(blended);
            track.external = external;
            track.time = time;
            if weight >= 1.0 {
                track.source = None;
            }
        }
    }

    /// `ClimbFacing.yaw`: the yaw climbing a block faces, from its `facing` (or
    /// its boolean direction flags, or the climbable neighbours), the position
    /// across the block face, the player's yaw, and the climbable-direction test.
    pub fn climb_facing_yaw(
        facing: Option<&str>,
        boolean_flags: &[(&str, bool)],
        climbable: impl Fn(&str) -> bool,
        across_x: f64,
        across_z: f64,
        yaw: f32,
    ) -> f32 {
        if let Some(facing) = facing
            && matches!(facing, "north" | "south" | "west" | "east")
        {
            return climb_opposite_yaw(facing);
        }
        let mut directions: Vec<&str> = Vec::new();
        for (flag, on) in boolean_flags {
            if *on && matches!(*flag, "north" | "south" | "west" | "east") {
                directions.push(flag);
            }
        }
        if directions.is_empty() {
            for direction in ["north", "south", "west", "east"] {
                if climbable(direction) {
                    directions.push(direction);
                }
            }
        }
        if directions.is_empty() {
            return yaw;
        }
        let x = across_x.clamp(0.0, 1.0);
        let z = across_z.clamp(0.0, 1.0);
        let mut best = directions[0];
        for direction in &directions {
            if climb_distance(direction, x, z) < climb_distance(best, x, z) {
                best = direction;
            }
        }
        for direction in &directions {
            let yaw_of = climb_direction_yaw(direction);
            if wrap_degrees360(yaw_of - yaw).abs() < 0.01
                && climb_distance(direction, x, z) <= climb_distance(best, x, z) + 0.075
            {
                return yaw_of;
            }
        }
        climb_direction_yaw(best)
    }

    /// The yaw a direction faces, the Java `Direction.toYRot`.
    fn climb_direction_yaw(direction: &str) -> f32 {
        match direction {
            "south" => 0.0,
            "west" => 90.0,
            "north" => 180.0,
            "east" => -90.0,
            _ => 0.0,
        }
    }

    /// `direction.getOpposite().toYRot()`.
    fn climb_opposite_yaw(direction: &str) -> f32 {
        match direction {
            "north" => climb_direction_yaw("south"),
            "south" => climb_direction_yaw("north"),
            "west" => climb_direction_yaw("east"),
            "east" => climb_direction_yaw("west"),
            _ => 0.0,
        }
    }

    /// `ClimbFacing.distance`: how far across the block face the direction's
    /// surface sits.
    fn climb_distance(direction: &str, x: f64, z: f64) -> f64 {
        match direction {
            "north" => z,
            "south" => 1.0 - z,
            "west" => x,
            "east" => 1.0 - x,
            _ => f64::INFINITY,
        }
    }

    // ---------------------------------------------------------------------------
    // ItemSequenceController
    // ---------------------------------------------------------------------------

    /// `ItemSequenceController.TICK_SECONDS` — every controller steps at 20 Hz.
    pub const CONTROLLER_TICK_SECONDS: f32 = 0.05;
    /// The default transition, the Java `0.16F`.
    pub const SEQUENCE_TRANSITION_SECONDS: f32 = 0.16;
    /// The default locomotion retain, the Java `0.15F`.
    pub const SEQUENCE_RETAIN: f32 = 0.15;
    /// The stop transition the interrupted sequences use.
    pub const SEQUENCE_STOP_SECONDS: f32 = 0.08;

    /// `ItemSequenceController.Input`: what one tick knows about the player's
    /// hands. Items are registry ids (`None` when the hand is empty); the game
    /// glue builds this.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ItemInput {
        pub main: Option<String>,
        pub off: Option<String>,
        pub using_stack: Option<String>,
        /// `true` when the used hand is the off hand.
        pub used_off_hand: bool,
        /// `true` when the main arm is the left one.
        pub main_arm_left: bool,
        pub slot: i32,
        pub tick: i32,
        pub using_ticks: i32,
        pub vertical_speed: f64,
        pub grounded: bool,
        pub using: bool,
        /// Whether this client is the player itself (not a remote one).
        pub local: bool,
        /// Whether the fishing bobber is out.
        pub hook_present: bool,
    }

    impl Default for ItemInput {
        fn default() -> Self {
            Self {
                main: None,
                off: None,
                using_stack: None,
                used_off_hand: false,
                main_arm_left: false,
                slot: 0,
                tick: 0,
                using_ticks: 0,
                vertical_speed: 0.0,
                grounded: true,
                using: false,
                local: true,
                hook_present: false,
            }
        }
    }

    impl ItemInput {
        /// `Input.item(hand)`.
        pub fn item(&self, off_hand: bool) -> Option<&String> {
            if off_hand {
                self.off.as_ref()
            } else {
                self.main.as_ref()
            }
        }
    }

    /// `ItemSequenceController`: the held item's state machine, the Java class
    /// ported state for state. The library may be absent (no pack loaded); the
    /// behavior then reads blank and the playback stops.
    pub struct ItemSequenceController {
        playback: ClipSequencePlayer,
        owner: Option<String>,
        off_hand: bool,
        left: bool,
        initialized: bool,
        was_using: bool,
        hook_was_present: bool,
        airborne: bool,
        slam_started: bool,
        other_hand_occupied: bool,
        confirmed_action: bool,
        upper_body_only: bool,
        hands_only_use: bool,
        continuous_use: bool,
        riding: bool,
        behavior: String,
        phase: String,
        binding_type: String,
        attack_index: usize,
        last_attack_tick: i32,
        use_ticks: i32,
        owner_slot: i32,
        retain: f32,
        previous_retain: f32,
        target_retain: f32,
        legs_retain: f32,
        arms_only_influence: bool,
        movement_blend: f32,
        previous_movement_blend: f32,
        moving_legs_influence: f32,
        previous_legs_influence: f32,
        target_legs_influence: f32,
        requested_charge_time: f32,
        locomotion_influence: Option<LocomotionInfluence>,
        suppressed_arms: i32,
    }

    impl Default for ItemSequenceController {
        fn default() -> Self {
            Self::new()
        }
    }

    /// The library calls the state machine makes, bundled so the machine reads
    /// the same either way.
    struct SeqLibrary<'a> {
        library: Option<&'a super::library::AnimationLibrary>,
    }

    impl<'a> SeqLibrary<'a> {
        fn new(library: Option<&'a super::library::AnimationLibrary>) -> Self {
            Self { library }
        }

        fn behavior(&self, item: Option<&String>) -> String {
            match (self.library, item) {
                (Some(library), Some(item)) => library.bindings().string_action_setting(
                    Some(item),
                    None,
                    "item_sequence",
                    "behavior",
                    "",
                ),
                _ => String::new(),
            }
        }

        fn numeric(&self, item: Option<&String>, action: &str, key: &str, default: f32) -> f32 {
            match (self.library, item) {
                (Some(library), Some(item)) => library.bindings().numeric_action_setting(
                    Some(item),
                    None,
                    action,
                    key,
                    default,
                ),
                _ => default,
            }
        }

        fn clip(
            &self,
            item: Option<&String>,
            action: &str,
            left: bool,
            other: bool,
        ) -> Option<Clip> {
            let library = self.library?;
            library.handed_clip_for(item.map(String::as_str), None, action, left, other)
        }

        fn clips(&self, item: Option<&String>, action: &str, left: bool, other: bool) -> Vec<Clip> {
            match self.library {
                Some(library) => {
                    library.handed_clips_for(item.map(String::as_str), None, action, left, other)
                }
                None => Vec::new(),
            }
        }

        fn overlays(
            &self,
            item: Option<&String>,
            action: &str,
            left: bool,
        ) -> Vec<ResolvedOverlay> {
            match self.library {
                Some(library) => {
                    library.overlays_resolved(item.map(String::as_str), None, action, left, false)
                }
                None => Vec::new(),
            }
        }

        fn retention(&self, item: Option<&String>, action: &str) -> BasePoseRetention {
            match self.library {
                Some(library) => {
                    library.base_pose_retention_for(item.map(String::as_str), None, action)
                }
                None => BasePoseRetention::default(),
            }
        }

        fn influence(&self, item: Option<&String>, action: &str) -> Option<LocomotionInfluence> {
            self.library.and_then(|library| {
                library.configured_locomotion_influence(item.map(String::as_str), None, action)
            })
        }

        fn retrigger(&self, item: Option<&String>, action: &str, default: f32) -> f32 {
            match self.library {
                Some(library) => {
                    library.retrigger_progress(item.map(String::as_str), None, action, default)
                }
                None => default,
            }
        }
    }

    /// The additive retention the shield reactions compose with, the Java
    /// `ADDITIVE`.
    #[allow(dead_code)]
    fn additive_retention() -> BasePoseRetention {
        let mut retention = BasePoseRetention::default();
        for bone in [
            "root",
            "waist",
            "body",
            "head",
            "left_arm",
            "right_arm",
            "left_leg",
            "right_leg",
        ] {
            retention.set(bone, 1.0);
        }
        retention
    }

    impl ItemSequenceController {
        /// A controller at rest.
        pub fn new() -> Self {
            Self {
                playback: ClipSequencePlayer::new(),
                owner: None,
                off_hand: false,
                left: false,
                initialized: false,
                was_using: false,
                hook_was_present: false,
                airborne: false,
                slam_started: false,
                other_hand_occupied: false,
                confirmed_action: false,
                upper_body_only: false,
                hands_only_use: false,
                continuous_use: false,
                riding: false,
                behavior: String::new(),
                phase: String::new(),
                binding_type: String::new(),
                attack_index: 0,
                last_attack_tick: i32::MIN,
                use_ticks: 0,
                owner_slot: -1,
                retain: SEQUENCE_RETAIN,
                previous_retain: SEQUENCE_RETAIN,
                target_retain: SEQUENCE_RETAIN,
                legs_retain: f32::NAN,
                arms_only_influence: false,
                movement_blend: 0.0,
                previous_movement_blend: 0.0,
                moving_legs_influence: 1.0,
                previous_legs_influence: 1.0,
                target_legs_influence: 1.0,
                requested_charge_time: f32::NAN,
                locomotion_influence: None,
                suppressed_arms: 0,
            }
        }

        /// `behavior`.
        pub fn behavior(
            library: Option<&super::library::AnimationLibrary>,
            item: Option<&String>,
        ) -> String {
            SeqLibrary::new(library).behavior(item)
        }

        /// `ownsMain`.
        pub fn owns_main(
            &self,
            library: Option<&super::library::AnimationLibrary>,
            item: Option<&String>,
        ) -> bool {
            !Self::behavior(library, item).is_empty()
        }

        /// `active`.
        pub fn active(&self) -> bool {
            self.playback.active()
        }

        /// `hasClip`.
        pub fn has_clip(&self) -> bool {
            self.playback.clip().is_some()
        }

        /// `ownsDirectTrack`.
        pub fn owns_direct_track(&self, bone: &str) -> bool {
            self.playback.clip().is_some() && self.playback.has_direct_track(bone)
        }

        /// `hasLocomotionInfluence`.
        pub fn has_locomotion_influence(&self) -> bool {
            self.locomotion_influence.is_some()
        }

        /// `allowsStationarySteps`.
        pub fn allows_stationary_steps(&self) -> bool {
            !self.playback.active() || self.phase == "idle" || self.phase == "idle_one_handed"
        }

        /// `setHandsOnlyUse`.
        pub fn set_hands_only_use(&mut self, hands_only: bool) {
            self.hands_only_use = hands_only;
        }

        /// `setRiding`.
        pub fn set_riding(&mut self, riding: bool) {
            self.riding = riding;
        }

        /// `interruptRiding`.
        pub fn interrupt_riding(&mut self) {
            self.playback.stop(0.1);
            self.phase = String::new();
            self.was_using = false;
        }

        /// `clearRiding`.
        pub fn clear_riding(&mut self) {
            self.playback.clear();
            self.phase = String::new();
            self.was_using = false;
            self.hook_was_present = false;
            self.initialized = false;
            self.owner = None;
            self.owner_slot = -1;
        }

        /// `replacesFalling`.
        pub fn replaces_falling(&self) -> bool {
            let legs = self.playback.has_direct_track("root")
                || self.playback.has_direct_track("right_leg")
                || self.playback.has_direct_track("left_leg");
            Self::replaces_falling_for(
                &self.behavior,
                &self.phase,
                self.off_hand,
                self.playback.clip().is_some(),
            ) && legs
                && (self.phase != "slam" || self.airborne)
        }

        /// `replacesFalling(behavior, phase, hand, hasClip)`.
        pub fn replaces_falling_for(
            behavior: &str,
            phase: &str,
            off_hand: bool,
            has_clip: bool,
        ) -> bool {
            has_clip && !off_hand && behavior == "mace" && (phase == "charge" || phase == "slam")
        }

        /// `slamStarted`.
        pub fn slam_started(&self) -> bool {
            self.slam_started
        }

        /// `startsMaceCharge`.
        pub fn starts_mace_charge(
            grounded: bool,
            vertical_speed: f64,
            fall_height: f64,
            charge_height: f64,
            charging: bool,
        ) -> bool {
            !grounded && vertical_speed < 0.0 && (charging || fall_height > charge_height)
        }

        /// `canMaceSmash`.
        pub fn can_mace_smash(attacking: bool, distance: f64, threshold: f64) -> bool {
            !attacking && distance > threshold
        }

        /// `tick`, the Java signature with the arguments named after the
        /// LocomotionController call site.
        #[allow(clippy::too_many_arguments)]
        pub fn tick(
            &mut self,
            input: &ItemInput,
            library: Option<&super::library::AnimationLibrary>,
            body_available: bool,
            attack_allowed: bool,
            grounded: bool,
            fall_distance: f64,
            use_item: bool,
            use_off_hand: bool,
            use_stack: Option<&String>,
            use_event: &str,
            slot_changed: bool,
            movement: f32,
            projected_landing: f64,
        ) {
            self.requested_charge_time = f32::NAN;
            let seq = SeqLibrary::new(library);
            self.update(
                &seq,
                input,
                body_available,
                attack_allowed,
                grounded,
                fall_distance,
                use_item,
                use_off_hand,
                use_stack,
                use_event,
                slot_changed,
                movement,
                projected_landing,
            );
            self.playback.tick(CONTROLLER_TICK_SECONDS);
            if self.phase == "charge" && self.requested_charge_time.is_finite() {
                self.playback.seek(self.requested_charge_time);
            }
        }

        /// `suspend`.
        pub fn suspend(&mut self) {
            self.playback.stop(0.18);
            self.playback.tick(CONTROLLER_TICK_SECONDS);
            self.phase = String::new();
            self.was_using = false;
        }

        /// `clear`.
        pub fn clear(&mut self) {
            self.playback.clear();
            self.phase = String::new();
            self.was_using = false;
            self.hook_was_present = false;
            self.initialized = false;
            self.owner = None;
            self.owner_slot = -1;
            self.confirmed_action = false;
            self.behavior = String::new();
            self.binding_type = String::new();
            self.attack_index = 0;
            self.retain = SEQUENCE_RETAIN;
            self.target_retain = SEQUENCE_RETAIN;
            self.legs_retain = f32::NAN;
            self.moving_legs_influence = 1.0;
            self.target_legs_influence = 1.0;
            self.locomotion_influence = None;
            self.suppressed_arms = 0;
        }

        /// `sample`.
        pub fn sample(&self, bone: &str, partial: f32) -> Sample {
            if self.riding
                && !(bone != "root"
                    && !bone.ends_with("_leg")
                    && (!bone.ends_with("_arm") || self.playback.has_direct_track(bone)))
            {
                return Sample::empty();
            }
            if self.hands_only_use && !bone.ends_with("_arm") && !bone.ends_with("_item") {
                return Sample::empty();
            }
            if !self.upper_body_only
                || (self.confirmed_action || self.continuous_use)
                    && bone != "root"
                    && bone != "left_leg"
                    && bone != "right_leg"
            {
                let movement = lerp(partial, self.previous_movement_blend, self.movement_blend);
                let legs = lerp(
                    partial,
                    self.previous_legs_influence,
                    self.moving_legs_influence,
                );
                let sampled = self.playback.sample(bone, partial);
                Self::with_moving_legs(sampled, bone, movement, legs)
            } else {
                Sample::empty()
            }
        }

        /// `withMovingLegs`.
        pub fn with_moving_legs(
            sample: Sample,
            bone: &str,
            movement: f32,
            legs_influence: f32,
        ) -> Sample {
            if bone != "left_leg" && bone != "right_leg" {
                return sample;
            }
            sample.with_weight(sample.weight * (1.0 - movement * (1.0 - legs_influence)))
        }

        /// `retain`.
        pub fn retain(&self, bone: &str, partial: f32) -> f32 {
            self.retain_for(bone, partial, false, false)
        }

        /// `retain(bone, partial, sprinting, jumping)`.
        pub fn retain_for(&self, bone: &str, partial: f32, sprinting: bool, jumping: bool) -> f32 {
            let arm = if bone == "left_arm" {
                1
            } else if bone == "right_arm" {
                2
            } else {
                0
            };
            if (self.suppressed_arms & arm) != 0 {
                return 0.0;
            }
            let hand_blocks = self.locomotion_influence.as_ref().is_some_and(|influence| {
                influence.hand_only && (if self.left { arm != 1 } else { arm != 2 })
            });
            if self.locomotion_influence.is_none()
                || (arm == 0 && self.arms_only_influence)
                || hand_blocks
            {
                self.legacy_retain(bone, partial)
            } else {
                self.locomotion_influence
                    .as_ref()
                    .map_or(0.0, |influence| influence.select(sprinting, jumping))
            }
        }

        fn legacy_retain(&self, bone: &str, partial: f32) -> f32 {
            let retain = lerp(partial, self.previous_retain, self.retain).clamp(0.0, 1.0);
            if bone != "left_leg" && bone != "right_leg" {
                return retain;
            }
            if self.legs_retain.is_finite() {
                return self.legs_retain.clamp(0.0, 1.0);
            }
            let movement = lerp(partial, self.previous_movement_blend, self.movement_blend);
            retain + (1.0 - retain) * movement
        }

        /// The `update` body, the Java method with its flags named.
        #[allow(clippy::too_many_arguments)]
        fn update(
            &mut self,
            seq: &SeqLibrary,
            input: &ItemInput,
            body_available: bool,
            attack_allowed: bool,
            _grounded: bool,
            fall_distance: f64,
            use_item: bool,
            use_off_hand: bool,
            use_stack: Option<&String>,
            use_event: &str,
            slot_changed: bool,
            movement: f32,
            projected_landing: f64,
        ) {
            self.slam_started = false;
            self.airborne = !input.grounded;
            self.upper_body_only = !body_available;
            self.previous_movement_blend = self.movement_blend;
            let mut blend = ((movement - 0.001) / 0.007).clamp(0.0, 1.0);
            blend = blend * blend * (3.0 - 2.0 * blend);
            self.movement_blend += (blend - self.movement_blend) * 0.4;
            self.previous_legs_influence = self.moving_legs_influence;
            self.moving_legs_influence +=
                (self.target_legs_influence - self.moving_legs_influence) * 0.4;
            self.previous_retain = self.retain;
            self.retain += (self.target_retain - self.retain) * 0.3;
            let hook_present = input.hook_present;
            let supports = use_item
                && Self::supports_use(
                    &seq.behavior(Some(use_stack.unwrap_or(&String::new()))),
                    use_event,
                );
            let same_slot = self.owner_slot == input.slot
                && !slot_changed
                && (self.owner == input.item(self.off_hand).cloned()
                    || input.item(self.off_hand).is_none()
                    || self.behavior == "bucket"
                        && seq.behavior(input.item(self.off_hand)) == "bucket");
            let using_behavior = seq.behavior(use_stack);
            self.continuous_use = self.hands_only_use
                && input.using
                && (using_behavior == "bow" || using_behavior == "trident");
            let keep_bow = self.hands_only_use
                && same_slot
                && self.was_using
                && (self.behavior == "bow" || self.behavior == "trident");
            if !body_available && !supports && !self.continuous_use && !keep_bow {
                if self.confirmed_action && same_slot && !self.playback.finished() {
                    self.hook_was_present = hook_present;
                } else {
                    self.playback.stop(SEQUENCE_STOP_SECONDS);
                    self.phase = String::new();
                    self.was_using = false;
                    self.hook_was_present = hook_present;
                }
                return;
            }
            let other_behaved = !supports && input.using && !seq.behavior(use_stack).is_empty();
            let off_item = input.off.clone();
            let continues_off = self.off_hand
                && !slot_changed
                && !attack_allowed
                && self.owner_slot == input.slot
                && Self::continues_offhand_action(
                    &self.phase,
                    self.was_using,
                    self.playback.finished(),
                )
                && (self.owner == off_item
                    || off_item.is_none()
                    || self.behavior == "bucket" && seq.behavior(off_item.as_ref()) == "bucket");
            let hand_off = Self::select_hand(
                other_behaved,
                input.used_off_hand,
                supports,
                use_off_hand,
                continues_off,
            );
            let was_occupied = self.other_hand_occupied;
            self.other_hand_occupied = input.item(!hand_off).is_some();
            let held = if other_behaved || supports {
                use_stack.cloned()
            } else if continues_off {
                self.owner.clone()
            } else {
                input.main.clone()
            };
            let behavior = seq.behavior(held.as_ref());
            let bucket_carry = self.behavior == "bucket"
                && behavior == "bucket"
                && self.off_hand == hand_off
                && self.owner_slot == input.slot
                && !slot_changed
                && !supports;
            let confirmed_keep = self.confirmed_action
                && !self.playback.finished()
                && input.item(self.off_hand).is_none()
                && self.owner_slot == input.slot
                && !slot_changed
                && !use_item;
            let changed =
                self.owner != held || self.off_hand != hand_off || self.behavior != behavior;
            if changed && bucket_carry {
                self.owner = held;
                if !self.phase.starts_with("use") || self.playback.finished() {
                    self.enter(seq, "idle", true, f32::INFINITY, None);
                }
            } else if changed && !confirmed_keep {
                let first = !self.initialized;
                self.initialized = true;
                self.owner = held;
                self.off_hand = hand_off;
                self.left = Self::visual_left(input, self.off_hand);
                self.behavior = behavior;
                self.owner_slot = input.slot;
                self.was_using = false;
                self.attack_index = 0;
                self.phase = String::new();
                if self.behavior.is_empty() {
                    self.playback.stop(0.18);
                } else if !first && !supports {
                    let phase = self.equip_phase(seq, input);
                    self.enter(seq, &phase, false, f32::INFINITY, None);
                }
            } else if slot_changed && !self.behavior.is_empty() && !supports {
                self.left = Self::visual_left(input, self.off_hand);
                let phase = self.equip_phase(seq, input);
                self.enter(seq, &phase, false, f32::INFINITY, None);
            }
            if self.behavior.is_empty() {
                self.hook_was_present = hook_present;
                return;
            }
            if self.left != Self::visual_left(input, self.off_hand) {
                self.left = Self::visual_left(input, self.off_hand);
                let phase = if self.phase.is_empty() {
                    "idle".to_string()
                } else {
                    self.phase.clone()
                };
                let looped = Self::is_loop_phase(&phase);
                self.enter(seq, &phase, looped, f32::INFINITY, None);
            }
            if was_occupied != self.other_hand_occupied {
                self.refresh_held_hand_variant(seq);
            }
            let using_this_hand = !supports
                && input.using
                && input.used_off_hand == self.off_hand
                && self.owner == input.using_stack;
            if self.behavior == "bow" || self.behavior == "trident" {
                if using_this_hand {
                    self.use_ticks = input.using_ticks;
                    if !self.was_using {
                        self.enter(seq, "charge", false, f32::INFINITY, None);
                    }
                    if self.phase == "charge" {
                        let charge_ticks =
                            seq.numeric(self.owner.as_ref(), "item_sequence", "charge_ticks", 20.0);
                        if let Some(clip) = self.playback.clip() {
                            self.requested_charge_time = clip.length
                                * (self.use_ticks as f32 / charge_ticks.max(1.0)).min(1.0);
                            if self.requested_charge_time >= clip.length
                                && self.clip_for(seq, "hold").is_some()
                            {
                                self.enter(seq, "hold", true, f32::INFINITY, None);
                            }
                        }
                    }
                } else if self.was_using {
                    let remote = !input.local;
                    let release = use_item && use_event == "release"
                        || remote
                            && self.use_ticks as f32
                                >= seq.numeric(
                                    self.owner.as_ref(),
                                    "item_sequence",
                                    "minimum_charge_ticks",
                                    3.0,
                                );
                    if release {
                        self.enter(seq, "release", false, f32::INFINITY, None);
                        self.confirmed_action = true;
                    } else {
                        self.enter(seq, "idle", true, f32::INFINITY, None);
                    }
                } else if use_item && use_event == "release" {
                    self.enter(seq, "release", false, f32::INFINITY, None);
                    self.confirmed_action = true;
                }
                self.was_using = using_this_hand;
            }
            if body_available && attack_allowed && !supports && !self.off_hand && !using_this_hand {
                self.attack(
                    seq,
                    attack_allowed,
                    fall_distance,
                    input.tick,
                    projected_landing,
                );
            }
            if body_available
                && !self.off_hand
                && self.behavior == "mace"
                && self.phase != "slam"
                && !self.phase.starts_with("attack")
            {
                let charging = self.phase == "charge";
                let start = Self::starts_mace_charge(
                    input.grounded,
                    input.vertical_speed,
                    projected_landing,
                    seq.numeric(self.owner.as_ref(), "item_sequence", "charge_height", 3.0) as f64,
                    charging,
                );
                if start && !charging {
                    self.enter(seq, "charge", false, f32::INFINITY, None);
                } else if !start && charging {
                    self.enter(seq, "idle", true, f32::INFINITY, None);
                }
            }
            let use_this = supports && use_off_hand == self.off_hand;
            if use_this {
                self.use_action(seq, Self::one_handed_for(input, self.off_hand), use_event);
            }
            if self.behavior == "fishing" && !use_this {
                if hook_present && !self.hook_was_present && self.phase != "cast" {
                    self.enter(seq, "cast", false, f32::INFINITY, None);
                }
                if !hook_present && self.hook_was_present && self.phase != "reel" {
                    self.enter(seq, "reel", false, f32::INFINITY, None);
                }
            }
            if self.playback.finished()
                && !using_this_hand
                && self.phase != "charge"
                && self.phase != "idle_one_handed"
            {
                if self.behavior == "fishing" && hook_present {
                    if self.phase != "fishing" {
                        self.enter(seq, "fishing", true, f32::INFINITY, None);
                    }
                } else if self.phase != "idle" {
                    self.enter(seq, "idle", true, f32::INFINITY, None);
                }
            }
            if self.phase.is_empty() {
                let phase = if self.behavior == "fishing" && hook_present {
                    "fishing"
                } else {
                    "idle"
                };
                self.enter(seq, phase, true, f32::INFINITY, None);
            }
            if self.phase == "idle" || self.phase == "idle_one_handed" {
                let wanted = if Self::one_handed_for(input, self.off_hand)
                    && self.clip_for(seq, "idle_one_handed").is_some()
                {
                    "idle_one_handed"
                } else {
                    "idle"
                };
                if self.phase != wanted {
                    self.enter(seq, wanted, true, f32::INFINITY, None);
                }
            }
            self.hook_was_present = hook_present;
        }

        /// `attack`.
        fn attack(
            &mut self,
            seq: &SeqLibrary,
            attacking: bool,
            distance: f64,
            tick: i32,
            _projected_landing: f64,
        ) {
            if self.behavior == "mace"
                && Self::can_mace_smash(
                    attacking,
                    distance,
                    seq.numeric(self.owner.as_ref(), "item_sequence", "smash_distance", 1.5) as f64,
                )
            {
                self.enter(seq, "slam", false, f32::INFINITY, None);
                self.slam_started = true;
            } else if self.ready(seq) {
                if self.behavior != "flint" && self.behavior != "shears" {
                    let mut list = seq.clips(
                        self.owner.as_ref(),
                        "sequence_attack",
                        self.left,
                        self.other_hand_occupied,
                    );
                    if list.is_empty() && self.behavior == "throwable" {
                        self.enter(seq, "use", false, f32::INFINITY, None);
                    } else {
                        let plain = list.is_empty();
                        if plain {
                            list = seq.clips(
                                self.owner.as_ref(),
                                "attack",
                                self.left,
                                self.other_hand_occupied,
                            );
                        }
                        if !list.is_empty() {
                            if tick - self.last_attack_tick > 30 {
                                self.attack_index = 0;
                            }
                            let index = self.attack_index % list.len();
                            self.attack_index += 1;
                            let clip = list[index].clone();
                            let name = format!("attack_{}", index + 1);
                            self.enter_with(
                                seq,
                                &name,
                                Some(clip),
                                false,
                                f32::INFINITY,
                                if plain { Some("attack") } else { None },
                            );
                            self.last_attack_tick = tick;
                        }
                    }
                } else {
                    self.enter(seq, "use", false, f32::INFINITY, None);
                }
            }
        }

        /// `refreshHeldHandVariant`.
        fn refresh_held_hand_variant(&mut self, seq: &SeqLibrary) {
            if self.phase == "charge" || self.phase == "hold" || self.phase == "fishing" {
                let clip = self.clip_for(seq, &self.phase);
                if let Some(clip) = clip {
                    let differs = self
                        .playback
                        .clip()
                        .is_none_or(|playing| playing.name != clip.name);
                    if differs {
                        let progress = self.playback.progress();
                        let confirmed = self.confirmed_action;
                        let looped = Self::is_loop_phase(&self.phase);
                        let phase = self.phase.clone();
                        self.enter(seq, &phase, looped, f32::INFINITY, None);
                        if self.phase != "hold" {
                            self.playback.seek(progress * clip.length);
                        }
                        self.confirmed_action = confirmed;
                    }
                }
            }
        }

        /// `use`.
        fn use_action(&mut self, seq: &SeqLibrary, one_handed: bool, event: &str) {
            if self.behavior == "fishing" {
                let reel = event == "reel" || self.hook_was_present && event != "cast";
                let phase = if reel { "reel" } else { "cast" };
                self.enter_accepted_use(seq, phase);
            } else if self.behavior == "flint"
                || self.behavior == "shears"
                || self.behavior == "bucket"
                || self.behavior == "throwable"
            {
                let phase = if one_handed && self.clip_for(seq, "use_one_handed").is_some() {
                    "use_one_handed"
                } else {
                    "use"
                };
                self.enter_accepted_use(seq, phase);
            }
        }

        /// `ready`.
        fn ready(&self, seq: &SeqLibrary) -> bool {
            if self.phase != "idle"
                && self.phase != "idle_one_handed"
                && self.phase != "fishing"
                && self.phase != "charge"
                && !self.phase.starts_with("equip")
            {
                let percent = seq.retrigger(self.owner.as_ref(), "item_sequence", 70.0) * 100.0;
                self.playback.progress()
                    >= seq.retrigger(self.owner.as_ref(), &self.binding_type, percent)
            } else {
                true
            }
        }

        /// `selectHand`.
        pub fn select_hand(
            other_behaved: bool,
            used_off_hand: bool,
            supports: bool,
            use_off_hand: bool,
            continues_off: bool,
        ) -> bool {
            if other_behaved {
                used_off_hand
            } else if supports {
                use_off_hand
            } else {
                continues_off
            }
        }

        /// `supportsUse`.
        pub fn supports_use(behavior: &str, event: &str) -> bool {
            match behavior {
                "bow" | "trident" => event == "release",
                "fishing" | "flint" | "shears" | "bucket" | "throwable" => true,
                _ => false,
            }
        }

        /// `continuesOffhandAction`.
        pub fn continues_offhand_action(phase: &str, was_using: bool, finished: bool) -> bool {
            if !was_using || phase != "charge" && phase != "hold" {
                !finished
                    && (phase == "release"
                        || phase.starts_with("use")
                        || phase == "cast"
                        || phase == "reel"
                        || phase == "fishing")
            } else {
                true
            }
        }

        /// `equipPhase`.
        fn equip_phase(&self, seq: &SeqLibrary, input: &ItemInput) -> String {
            if Self::one_handed_for(input, self.off_hand)
                && self.clip_for(seq, "equip_one_handed").is_some()
            {
                "equip_one_handed".to_string()
            } else {
                "equip".to_string()
            }
        }

        /// `oneHanded`: the other hand carries something.
        fn one_handed_for(input: &ItemInput, off_hand: bool) -> bool {
            input.item(!off_hand).is_some()
        }

        /// `visualLeft`.
        fn visual_left(input: &ItemInput, off_hand: bool) -> bool {
            // The main arm's side, flipped when the item sits in the off hand.
            if off_hand {
                !input.main_arm_left
            } else {
                input.main_arm_left
            }
        }

        /// `clip(action)`: the sequence-prefixed clip for the phase.
        fn clip_for(&self, seq: &SeqLibrary, action: &str) -> Option<Clip> {
            seq.clip(
                self.owner.as_ref(),
                &format!("sequence_{}", action),
                self.left,
                self.other_hand_occupied,
            )
        }

        /// `enter(phase, looped)`.
        fn enter(
            &mut self,
            seq: &SeqLibrary,
            phase: &str,
            looped: bool,
            transition: f32,
            named: Option<&str>,
        ) {
            let clip = self.clip_for(seq, phase);
            self.enter_with(seq, phase, clip, looped, transition, named);
        }

        /// `overIdle`.
        fn over_idle(&self, seq: &SeqLibrary, phase: &str) -> Option<Clip> {
            if !phase.starts_with("idle")
                && !phase.starts_with("equip")
                && seq.numeric(self.owner.as_ref(), &self.binding_type, "over_idle", 0.0) > 0.5
            {
                self.clip_for(seq, "idle")
            } else {
                None
            }
        }

        /// `enterAcceptedUse`.
        fn enter_accepted_use(&mut self, seq: &SeqLibrary, phase: &str) {
            self.enter(seq, phase, false, 0.08, None);
            self.confirmed_action = true;
        }

        /// `enter(phase, clip, looped, transition, named)`.
        fn enter_with(
            &mut self,
            seq: &SeqLibrary,
            phase: &str,
            clip: Option<Clip>,
            looped: bool,
            transition: f32,
            named: Option<&str>,
        ) {
            self.confirmed_action = false;
            if self.riding
                && !(phase.starts_with("idle") || phase.starts_with("equip") || phase == "fishing")
            {
                self.phase = phase.to_string();
                self.playback.stop(0.28);
                return;
            }
            let mut phase = phase.to_string();
            let mut clip = clip;
            let renamed = if named.is_none() {
                self.one_handed_phase(seq, &phase, self.other_hand_occupied)
            } else {
                phase.clone()
            };
            if renamed != phase {
                phase = renamed;
                clip = self.clip_for(seq, &phase);
            }
            self.phase = phase.clone();
            self.binding_type = named.map_or_else(
                || format!("sequence_{}", self.settings_phase(seq, &phase)),
                str::to_string,
            );
            self.arms_only_influence = named.is_some();
            self.target_retain = self.action_setting(seq, "locomotion_retain", SEQUENCE_RETAIN);
            self.locomotion_influence = seq.influence(self.owner.as_ref(), &self.binding_type);
            if self.locomotion_influence.is_none() && self.binding_type.ends_with("_one_handed") {
                self.locomotion_influence = seq.influence(
                    self.owner.as_ref(),
                    &self.binding_type[..self.binding_type.len() - "_one_handed".len()],
                );
            }
            if self.locomotion_influence.is_none() {
                self.locomotion_influence = seq.influence(self.owner.as_ref(), "item_sequence");
            }
            self.suppressed_arms =
                self.action_setting(seq, "disable_locomotion_arm_swing", 0.0) as i32;
            self.legs_retain = self.action_setting(seq, "legs_locomotion_retain", f32::NAN);
            self.target_legs_influence = self
                .action_setting(seq, "moving_legs_influence", 1.0)
                .clamp(0.0, 1.0);
            let retention = self
                .retention_of(seq, &self.binding_type)
                .over(&self.retention_of(seq, "item_sequence"));
            let transition = transition.min(self.action_setting(
                seq,
                "transition_seconds",
                SEQUENCE_TRANSITION_SECONDS,
            ));
            let over_idle = self.over_idle(seq, &phase);
            self.playback.play(
                clip.clone(),
                seq.overlays(self.owner.as_ref(), &self.binding_type, self.left),
                looped && clip.as_ref().is_some_and(|clip| clip.loop_),
                transition,
                phase == "hold",
                retention,
                self.left,
                over_idle,
            );
            if self.action_setting(seq, "start_at_end", 0.0) > 0.5 {
                self.playback.hold_last_frame();
            }
        }

        /// `oneHandedPhase`.
        fn one_handed_phase(&self, seq: &SeqLibrary, phase: &str, other_occupied: bool) -> String {
            if !other_occupied || phase.ends_with("_one_handed") {
                return phase.to_string();
            }
            if phase != "use" && phase != "equip" && phase != "idle" && !phase.starts_with("attack")
            {
                return phase.to_string();
            }
            let named = format!("{}_one_handed", phase);
            if self.clip_for(seq, &named).is_none() {
                phase.to_string()
            } else {
                named
            }
        }

        /// `actionSetting`.
        fn action_setting(&self, seq: &SeqLibrary, key: &str, default: f32) -> f32 {
            let mut value = seq.numeric(self.owner.as_ref(), "item_sequence", key, default);
            if self.binding_type.ends_with("_one_handed") {
                let base = &self.binding_type[..self.binding_type.len() - "_one_handed".len()];
                value = seq.numeric(self.owner.as_ref(), base, key, value);
            }
            seq.numeric(self.owner.as_ref(), &self.binding_type, key, value)
        }

        /// `basePoseRetention(stack, action)`.
        fn retention_of(&self, seq: &SeqLibrary, action: &str) -> BasePoseRetention {
            seq.retention(self.owner.as_ref(), action)
        }

        /// `settingsPhase`.
        fn settings_phase(&self, seq: &SeqLibrary, phase: &str) -> String {
            if self.other_hand_occupied && !phase.ends_with("_one_handed") {
                let named = format!("{}_one_handed", phase);
                if self.clip_for(seq, &named).is_none() {
                    phase.to_string()
                } else {
                    named
                }
            } else {
                phase.to_string()
            }
        }

        /// `isLoopPhase`.
        pub fn is_loop_phase(phase: &str) -> bool {
            phase == "idle" || phase == "idle_one_handed" || phase == "hold" || phase == "fishing"
        }
    }

    /// `ItemUseController.mirrorFallback`.
    pub fn mirror_fallback(clip: Option<&Clip>, left: bool) -> bool {
        clip.is_some_and(|clip| left && !clip.name.ends_with("_left"))
    }

    /// `ItemUseController.sampledBone`.
    pub fn sampled_bone(bone: &str, mirror: bool) -> String {
        if !mirror {
            return bone.to_string();
        }
        if let Some(rest) = bone.strip_prefix("left_") {
            format!("right_{}", rest)
        } else if let Some(rest) = bone.strip_prefix("right_") {
            format!("left_{}", rest)
        } else {
            bone.to_string()
        }
    }

    /// `ItemUseController.sample`: the playback's sample, mirrored when the hand
    /// asks for it.
    pub fn item_use_sample(
        playback: &ClipSequencePlayer,
        bone: &str,
        partial: f32,
        mirror: bool,
    ) -> Sample {
        let sample = playback.sample(&sampled_bone(bone, mirror), partial);
        if mirror && sample.weight > 0.0 {
            Sample {
                pose: Pose {
                    position: Vec3::new(
                        -sample.pose.position.x,
                        sample.pose.position.y,
                        sample.pose.position.z,
                    ),
                    rotation: Vec3::new(
                        sample.pose.rotation.x,
                        -sample.pose.rotation.y,
                        -sample.pose.rotation.z,
                    ),
                    scale: sample.pose.scale,
                },
                ..sample
            }
        } else {
            sample
        }
    }

    // ---------------------------------------------------------------------------
    // CrouchSequenceController
    // ---------------------------------------------------------------------------

    /// `CrouchSequenceController.BLEND_SECONDS`.
    pub const CROUCH_BLEND_SECONDS: f32 = 0.1;
    /// The crouch amount's per-tick limits, the Java `-0.2F`/`0.4F`.
    pub const CROUCH_AMOUNT_OUT: f32 = 0.2;
    pub const CROUCH_AMOUNT_IN: f32 = 0.4;
    /// The stop transition, the Java `0.125F`.
    pub const CROUCH_STOP_SECONDS: f32 = 0.125;

    /// `CrouchSequenceController`: the crouch's enter/hold/exit playback.
    pub struct CrouchSequenceController {
        playback: ClipSequencePlayer,
        initialized: bool,
        crouching: bool,
        phase: String,
        amount: f32,
        previous_amount: f32,
        single_pose: bool,
    }

    impl Default for CrouchSequenceController {
        fn default() -> Self {
            Self::new()
        }
    }

    impl CrouchSequenceController {
        /// A controller at rest.
        pub fn new() -> Self {
            Self {
                playback: ClipSequencePlayer::new(),
                initialized: false,
                crouching: false,
                phase: String::new(),
                amount: 0.0,
                previous_amount: 0.0,
                single_pose: false,
            }
        }

        /// `tick`, the full form: `crouching`, `sprinting`, the enter/hold/exit
        /// clips and the playback rate. The eight arguments mirror the Java
        /// signature.
        #[allow(clippy::too_many_arguments)]
        pub fn tick(
            &mut self,
            crouching: bool,
            sprinting: bool,
            enter: Option<Clip>,
            hold: Option<Clip>,
            exit: Option<Clip>,
            rate: f32,
            single_pose: bool,
        ) {
            if self.single_pose != single_pose {
                self.cancel();
            }
            self.single_pose = single_pose;
            if single_pose {
                self.tick_single_pose(crouching, enter.or(hold), rate);
                return;
            }
            if !crouching {
                self.cancel();
                return;
            }
            self.previous_amount = self.amount;
            let target = if !sprinting || (enter.is_none() && hold.is_none()) {
                0.0
            } else {
                1.0
            };
            self.amount += (target - self.amount).clamp(-CROUCH_AMOUNT_OUT, CROUCH_AMOUNT_IN);
            if !self.initialized {
                self.initialized = true;
                self.crouching = crouching;
                if crouching {
                    self.hold(hold, enter);
                }
            } else if crouching != self.crouching {
                self.crouching = crouching;
                if crouching {
                    if enter.is_some() && enter != hold {
                        self.play("enter", enter, false);
                    } else {
                        self.hold(hold, enter);
                    }
                } else if let Some(exit) = exit {
                    self.play("exit", Some(exit), false);
                } else {
                    self.stop();
                }
            } else if crouching && self.phase == "enter" && self.playback.finished() {
                self.hold(hold, enter);
            } else if !crouching && self.phase == "exit" && self.playback.finished() {
                self.stop();
            } else if crouching
                && self.phase == "hold"
                && hold.is_some()
                && self.playback.clip() != hold.as_ref()
            {
                self.hold(hold, enter);
            }
            self.playback.set_playback_rate(rate);
            self.playback.tick(CONTROLLER_TICK_SECONDS);
        }

        fn tick_single_pose(&mut self, crouching: bool, clip: Option<Clip>, rate: f32) {
            if !crouching {
                self.cancel();
                return;
            }
            self.previous_amount = self.amount;
            let target = if crouching && clip.is_some() {
                1.0
            } else {
                0.0
            };
            self.amount += (target - self.amount).clamp(-CROUCH_AMOUNT_OUT, CROUCH_AMOUNT_IN);
            if crouching && clip.is_some() {
                if !self.crouching || self.playback.clip() != clip.as_ref() {
                    self.play("enter", clip.clone(), false);
                }
                if self.playback.finished() {
                    self.phase = "hold".to_string();
                    self.playback.hold_last_frame();
                }
            } else if self.crouching || self.playback.clip().is_some() {
                self.stop();
            }
            self.initialized = true;
            self.crouching = crouching;
            self.playback
                .set_playback_rate(if self.phase == "hold" { 0.0 } else { rate });
            self.playback.tick(CONTROLLER_TICK_SECONDS);
        }

        /// Whether the single-pose mode is on.
        pub fn single_pose(&self) -> bool {
            self.single_pose
        }

        fn hold(&mut self, hold: Option<Clip>, enter: Option<Clip>) {
            let clip = hold.or(enter);
            let Some(clip) = clip else {
                self.stop();
                return;
            };
            if self.playback.clip() != Some(&clip) || self.phase != "hold" {
                let looped = clip.loop_;
                self.play("hold", Some(clip), looped);
            }
            if !self.playback.clip().is_some_and(|clip| clip.loop_) {
                self.playback.hold_last_frame();
            }
        }

        fn play(&mut self, phase: &str, clip: Option<Clip>, looped: bool) {
            self.phase = phase.to_string();
            self.playback.play(
                clip,
                Vec::new(),
                looped,
                CROUCH_BLEND_SECONDS,
                false,
                BasePoseRetention::default(),
                false,
                None,
            );
        }

        fn stop(&mut self) {
            self.phase = String::new();
            self.playback.stop(CROUCH_STOP_SECONDS);
        }

        /// `sample`.
        pub fn sample(&self, bone: &str, partial: f32) -> Sample {
            self.playback.sample(bone, partial)
        }

        /// `isActive`.
        pub fn is_active(&self) -> bool {
            self.playback.active()
        }

        /// The clip playing.
        pub fn clip(&self) -> Option<&Clip> {
            self.playback.clip()
        }

        /// The phase name.
        pub fn phase(&self) -> &str {
            &self.phase
        }

        /// `renderTime`.
        pub fn render_time(&self, partial: f32) -> f32 {
            self.playback.render_time(partial)
        }

        /// `renderAmount`.
        pub fn render_amount(&self, partial: f32) -> f32 {
            let amount = (self.previous_amount + (self.amount - self.previous_amount) * partial)
                .clamp(0.0, 1.0);
            amount * amount * (3.0 - 2.0 * amount)
        }

        /// `cancel`.
        pub fn cancel(&mut self) {
            self.playback.clear();
            self.initialized = false;
            self.crouching = false;
            self.single_pose = false;
            self.phase = String::new();
            self.previous_amount = 0.0;
            self.amount = 0.0;
        }
    }

    /// The glue's crouch state: the controller plus the vanilla-shaped amount
    /// the pose builder reads when no pack carries crouch clips.
    #[derive(Default)]
    pub struct CrouchSequenceState {
        pub controller: CrouchSequenceController,
    }

    impl CrouchSequenceState {
        fn new() -> Self {
            Self {
                controller: CrouchSequenceController::new(),
            }
        }
    }

    /// The glue's item state: the controller plus whether an emote plays.
    #[derive(Default)]
    pub struct ItemSequenceState {
        pub controller: ItemSequenceController,
        emote: Option<(ItemAction, f32)>,
    }

    impl ItemSequenceState {
        fn new() -> Self {
            Self {
                controller: ItemSequenceController::new(),
                emote: None,
            }
        }

        /// The emote hand action our glue plays, with its seconds left.
        pub fn play_emote(&mut self, action: ItemAction) {
            self.emote = Some((action, 0.6));
        }

        /// Steps the emote action down; `Some(action)` while it plays.
        pub fn tick_emote(&mut self, dt: f32) -> Option<ItemAction> {
            let Some((action, seconds)) = &mut self.emote else {
                return None;
            };
            *seconds -= dt;
            if *seconds <= 0.0 {
                self.emote = None;
                None
            } else {
                Some(*action)
            }
        }

        /// The action playing, if any.
        pub fn emote(&self) -> Option<ItemAction> {
            self.emote.map(|(action, _)| action)
        }
    }

    /// The gait phase wrapped into `0.0..1.0` (`x - floor(x)`).
    fn gait_wrap(phase: f32) -> f32 {
        phase - phase.floor()
    }

    /// `a` moved towards `b` by at most `step`.
    fn approach(a: f32, b: f32, step: f32) -> f32 {
        if !a.is_finite() {
            return b;
        }
        let step = if step.is_finite() { step.max(0.0) } else { 0.0 };
        let gap = b - a;
        if gap.abs() <= step {
            b
        } else if gap > 0.0 {
            a + step
        } else {
            a - step
        }
    }

    // ---------------------------------------------------------------------------
    // PlayerAnimationEngine
    // ---------------------------------------------------------------------------

    /// The whole-model transform the body hook multiplies in: how the player
    /// leans, banks, bobs, squashes and rolls.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct RootMotion {
        /// Forward lean, radians about x (positive leans forward).
        pub lean: f32,
        /// Sideways bank, radians about z (positive leans right).
        pub bank: f32,
        /// Vertical bob, blocks.
        pub bob: f32,
        /// Vertical squash, `0.0..=1.0`.
        pub squash: f32,
        /// Roll about y, radians (gliding and swimming).
        pub roll: f32,
    }

    /// One player's animation state: the signals, the controllers, and the pose
    /// the last two updates built.
    pub struct PlayerState {
        /// The signals of the last update.
        pub signals: MotionSignals,
        pub locomotion: LocomotionController,
        /// The gait of the last update, kept so a read costs no step.
        pub gait: LocomotionFrame,
        /// The gait before it, for a partial-tick read.
        previous_gait: LocomotionFrame,
        pub turn: TurnInPlaceController,
        /// The item sequence state machine (the Java `ItemSequenceController`).
        pub items: super::engine::ItemSequenceState,
        /// The boat's paddles, while rowing.
        pub boat: BoatAnimation,
        /// The fishing line's bob.
        pub fishing: FishingSwing,
        /// The direction-change lean.
        pub direction: DirectionChangeController,
        /// The sideways lean from turning.
        pub turn_lean: TurnLeanController,
        /// The creative-flight hover detector.
        pub flight: CreativeFlightMotion,
        /// The swimming surface alignment.
        pub swim: SwimSurfaceAlignment,
        /// The crouch sequence state machine.
        pub crouch: CrouchSequenceState,
        /// The glide height alignment.
        pub elytra: ElytraFlightAlignment,
        /// The pose ownership blends.
        pub ownership: PoseOwnershipBlend,
        pub outer_ownership: OuterPoseOwnershipBlend,
        /// The player's face: blinks, eyes and cues.
        pub face: FaceRig,
        /// Whether the player is rowing a boat.
        pub riding: bool,
        /// Whether a fishing line is out.
        pub fishing_line: bool,
        /// The emote hand action our glue plays (the Java emote system plays a
        /// named clip; this is the game-side stand-in).
        pub emote_action: Option<(ItemAction, f32)>,
        /// The pose input the last `begin_pose` carried (the yaw and pitch the
        /// turn controllers read).
        pub pose_input: PoseInput,
        /// The animated pose the last update built.
        pose: HumanoidPose,
        /// The pose before it, for a partial-tick blend.
        previous: HumanoidPose,
        /// Seconds of the player's animation clock.
        clock: f32,
        /// How much of the animation shows, `0.0..=1.0`.
        blend: f32,
        /// The bones whose base pose this player keeps.
        pub retention: BasePoseRetention,
    }

    impl Default for PlayerState {
        fn default() -> Self {
            Self {
                signals: MotionSignals::new(),
                locomotion: LocomotionController::new(),
                gait: LocomotionFrame::default(),
                previous_gait: LocomotionFrame::default(),
                turn: TurnInPlaceController::new(),
                items: ItemSequenceState::new(),
                boat: BoatAnimation::new(0, 0, [false, false], [0.0, 0.0], false),
                fishing: FishingSwing::default(),
                direction: DirectionChangeController::default(),
                turn_lean: TurnLeanController::default(),
                flight: CreativeFlightMotion::default(),
                swim: SwimSurfaceAlignment::default(),
                crouch: CrouchSequenceState::new(),
                elytra: ElytraFlightAlignment::default(),
                ownership: PoseOwnershipBlend::default(),
                outer_ownership: OuterPoseOwnershipBlend::default(),
                face: FaceRig::default(),
                riding: false,
                fishing_line: false,
                emote_action: None,
                pose_input: PoseInput::default(),
                pose: HumanoidPose::identity(),
                previous: HumanoidPose::identity(),
                clock: 0.0,
                blend: 0.0,
                retention: BasePoseRetention::default(),
            }
        }
    }

    impl PlayerState {
        /// Seconds of animation so far.
        pub fn clock(&self) -> f32 {
            self.clock
        }

        /// How much of the animation shows, `0.0..=1.0`.
        pub fn blend(&self) -> f32 {
            self.blend
        }

        /// The pose the last update built.
        pub fn pose(&self) -> &HumanoidPose {
            &self.pose
        }

        /// One update: advance every controller and rebuild the pose. The
        /// item-sequence machine only runs when the glue hands it a library and
        /// input (see [`PlayerAnimationEngine::tick_items`]); without one it
        /// stays idle, like a player with an empty hand.
        fn advance(&mut self, signals: MotionSignals, dt: f32, tuning: &EngineTuning) {
            self.previous = self.pose.clone();
            self.signals = signals;
            self.clock += dt;
            self.face
                .update(&signals, &super::faces::current_settings(), dt);
            let loco = self.locomotion.update(&signals, dt, tuning);
            self.previous_gait = self.gait;
            self.gait = loco;
            // Turn in place, the Java update, driven from the pose input's yaw.
            let input = self.pose_input;
            let settings = tuning.turn_settings.clone();
            self.turn.update(
                input.yaw,
                input.yaw,
                input.pitch,
                self.clock as f64,
                !signals.elytra && !signals.climbing && !signals.crawling,
                !signals.moving,
                false,
                &settings,
            );
            // The crouch machine: no clips here, so it tracks the amount only.
            self.crouch.controller.tick(
                signals.crouching,
                signals.running,
                None,
                None,
                None,
                1.0,
                false,
            );
            // The direction-change lean and the turn lean, per tick.
            self.direction.tick(signals.tick, 0.0, 0.0, input.yaw, true);
            self.turn_lean.tick(
                signals.tick,
                input.yaw,
                signals.walk_forward,
                0.0,
                signals.walk_forward,
                true,
                signals.jumped != 0,
                false,
            );
            // The creative-flight hover.
            self.flight
                .update(signals.tick, 0.0, signals.creative_flight);
            // The swim alignment.
            self.swim.tick(
                signals.surface_swimming || signals.submerged_swimming,
                signals.swim_vertical > 0,
                f64::NAN,
                SWIM_DEFAULT_DEPTH,
            );
            // The boat's paddles, while rowing.
            if self.riding {
                let paddling = signals.moving;
                self.boat
                    .step(signals.tick, 0, false, [paddling, paddling], false);
            }
            // The emote hand action runs down.
            let _ = self.items.tick_emote(dt);
            self.blend = approach(self.blend, 1.0, 8.0 * dt);
            let animated = self.build_pose(&loco, tuning);
            self.pose = HumanoidPose {
                blend: self.blend,
                retention: self.retention.clone(),
                ..animated
            }
            .over(&HumanoidPose::identity());
        }

        /// The whole-model transform for this player at `partial`.
        pub fn root_motion(&self, partial: f32) -> RootMotion {
            let partial = if partial.is_finite() {
                partial.clamp(0.0, 1.0)
            } else {
                1.0
            };
            // Read between the last two updates, so a partial tick moves the
            // model smoothly instead of in whole ticks.
            let mix = |a: f32, b: f32| a + (b - a) * partial;
            let loco = LocomotionFrame {
                phase: mix(self.previous_gait.phase, self.gait.phase),
                stride: mix(self.previous_gait.stride, self.gait.stride),
                lift: mix(self.previous_gait.lift, self.gait.lift),
                speed: mix(self.previous_gait.speed, self.gait.speed),
                run_seconds: self.gait.run_seconds,
            };
            let phase = loco.phase.to_radians();
            let stride = loco.stride;
            let glide = if self.signals.elytra { 1.0 } else { 0.0 };
            RootMotion {
                lean: stride * 0.12 + glide * 0.45,
                bank: self.signals.elytra_bank * glide,
                bob: phase.cos() * 0.03 * stride - loco.lift * 0.05,
                squash: loco.lift * 0.12,
                roll: self.signals.swim_vertical as f32
                    * if swimming(&self.signals) { 0.35 } else { 0.0 },
            }
        }
    }

    /// The pose the gait and the ported controllers add up to: the vanilla
    /// comparison shape, with the turn-in-place, crouch, direction and lean
    /// controllers layered on the way the Java `apply` calls do.
    impl PlayerState {
        fn build_pose(&self, loco: &LocomotionFrame, tuning: &EngineTuning) -> HumanoidPose {
            let phase = loco.phase.to_radians();
            let (stride, speed) = (loco.stride.clamp(0.0, 1.0), loco.speed.clamp(0.0, 1.0));
            let swing = phase.cos() * LIMB_SWING * stride * tuning.gait_amplitude;
            let mut pose = HumanoidPose::identity();
            let signals = &self.signals;
            let glide = if signals.elytra { 1.0 } else { 0.0 };
            let swim = if swimming(signals) { 1.0 } else { 0.0 };
            let crouch = self.crouch.controller.render_amount(1.0);
            let climb = if signals.climbing { 1.0 } else { 0.0 };
            let busy = self.items.emote().is_some() as i32 as f32;

            // Legs: opposite swings, still while gliding or swimming, tucked
            // while climbing.
            pose.right_leg.x_rot = swing;
            pose.left_leg.x_rot = -swing;
            let turn_yaw = self.turn.body_yaw();
            pose.left_leg.y_rot = (input_yaw(self) - turn_yaw).to_radians() * 0.1;
            pose.right_leg.y_rot = pose.left_leg.y_rot;

            // Arms: the legs' opposite swing on foot; back and out gliding; up
            // climbing; the emote's hand action.
            let arm_swing = -swing * 1.05;
            pose.right_arm.x_rot = arm_swing;
            pose.left_arm.x_rot = -arm_swing;
            let action = self.items.emote();
            let charge = if action == Some(ItemAction::Charge) {
                1.4 * busy
            } else {
                0.0
            };
            let use_arm = if action == Some(ItemAction::Use) || action == Some(ItemAction::Eat) {
                -0.9 * busy
            } else {
                0.0
            };
            let swing_arm = if action == Some(ItemAction::Swing) {
                (self.clock * std::f32::consts::PI).sin() * 1.2 * busy
            } else {
                0.0
            };
            let paddles = self.boat.sample(1.0);
            let rowing = self.riding as i32 as f32;
            pose.right_arm.x_rot += -swing_arm - charge - use_arm - paddles[1].pitch * 1.1 * rowing;
            pose.left_arm.x_rot += -paddles[0].pitch * 1.1 * rowing;
            pose.right_arm.z_rot = -(0.09 + 0.35 * glide + 0.25 * swim) - 0.35 * climb;
            pose.left_arm.z_rot = 0.09 + 0.35 * glide + 0.25 * swim + 0.35 * climb;

            // Body: crouches (through the crouch machine's amount), dips on a
            // landing, leans into a run and while gliding.
            pose.body.y = -0.12 * crouch - 0.10 * loco.lift;
            pose.body.x_rot = speed * 0.10 + 0.45 * glide - 0.18 * climb;
            pose.body.y_rot = (input_yaw(self) - turn_yaw).to_radians() * 0.35;

            // Head: takes most of the turn, looks along the climb, and banks
            // with the glide.
            pose.head.y_rot = (input_yaw(self) - turn_yaw).to_radians() * 0.65;
            pose.head.x_rot = -(signals.climb_direction as f32) * 0.4 * climb - 0.20 * glide;
            pose.head.z_rot = signals.elytra_bank * 0.35 * glide;

            // The turn-in-place playback lands on the limbs, body and head, the
            // Java `apply`'s weights.
            let weight = self.turn.weight();
            if weight > 0.0 {
                let partial = 1.0;
                TurnInPlaceController::apply_part(
                    &mut pose.left_leg,
                    &self.turn.sample("left_leg"),
                    weight * 1.0,
                );
                TurnInPlaceController::apply_part(
                    &mut pose.right_leg,
                    &self.turn.sample("right_leg"),
                    weight * 1.0,
                );
                TurnInPlaceController::apply_part(
                    &mut pose.left_arm,
                    &self.turn.sample("left_arm"),
                    weight * 1.0,
                );
                TurnInPlaceController::apply_part(
                    &mut pose.right_arm,
                    &self.turn.sample("right_arm"),
                    weight * 1.0,
                );
                TurnInPlaceController::apply_part(
                    &mut pose.body,
                    &self.turn.sample("body"),
                    weight,
                );
                TurnInPlaceController::apply_part(
                    &mut pose.head,
                    &self.turn.sample("head"),
                    weight,
                );
                let _ = partial;
            }

            // Cape: streams behind with speed and glide, lifts when falling.
            pose.cape.x_rot = -(speed * 0.25 + 0.55 * glide) - 0.30 * loco.lift;
            pose.cape.z_rot = swing * 0.15;

            pose.blend = 1.0;
            pose
        }
    }

    /// The yaw the pose input last carried.
    fn input_yaw(state: &PlayerState) -> f32 {
        state.pose_input.yaw
    }

    /// Every player's animation state, keyed by entity id, with the tuning they
    /// share.
    #[derive(Default)]
    pub struct PlayerAnimationEngine {
        players: HashMap<u64, PlayerState>,
        tuning: EngineTuning,
    }

    impl PlayerAnimationEngine {
        /// An engine with the given tuning.
        pub fn new(tuning: EngineTuning) -> Self {
            Self {
                players: HashMap::new(),
                tuning,
            }
        }

        /// The tuning every player reads.
        pub fn tuning(&self) -> &EngineTuning {
            &self.tuning
        }

        /// Replaces the tuning; the next update uses it.
        pub fn set_tuning(&mut self, tuning: EngineTuning) {
            self.tuning = tuning;
        }

        /// One update for `entity`; the state is created on its first update.
        pub fn update(&mut self, entity: u64, signals: MotionSignals, dt: f32) -> &PlayerState {
            let dt = if dt.is_finite() {
                dt.clamp(0.0, 0.25)
            } else {
                0.0
            };
            let tuning = &self.tuning;
            let state = self.players.entry(entity).or_default();
            state.advance(signals, dt, tuning);
            state
        }

        /// One update for `entity`, carrying the pose input (the body yaw and
        /// look pitch the turn controllers read) the render handed over.
        pub fn update_with_input(
            &mut self,
            entity: u64,
            signals: MotionSignals,
            dt: f32,
            input: PoseInput,
        ) -> &PlayerState {
            let dt = if dt.is_finite() {
                dt.clamp(0.0, 0.25)
            } else {
                0.0
            };
            let tuning = &self.tuning;
            {
                let state = self.players.entry(entity).or_default();
                state.pose_input = input;
                state.advance(signals, dt, tuning);
            }
            self.players.get(&entity).expect("just inserted")
        }

        /// `entity`'s state, if it has been updated.
        pub fn state(&self, entity: u64) -> Option<&PlayerState> {
            self.players.get(&entity)
        }

        /// `entity`'s state, to change (the vehicle and fishing flags live there).
        pub fn state_mut(&mut self, entity: u64) -> Option<&mut PlayerState> {
            self.players.get_mut(&entity)
        }

        /// The pose to draw for `entity` at `partial` ticks past the last update:
        /// the two latest poses blended, or the identity pose for an entity the
        /// engine has never seen.
        pub fn sample_pose(&self, entity: u64, partial: f32) -> HumanoidPose {
            let partial = if partial.is_finite() {
                partial.clamp(0.0, 1.0)
            } else {
                1.0
            };
            match self.players.get(&entity) {
                Some(state) if partial >= 1.0 => state.pose.clone(),
                Some(state) => state.previous.lerp(&state.pose, partial),
                None => HumanoidPose::identity(),
            }
        }

        /// The whole-model transform for `entity`, if it has been updated.
        pub fn root_motion(&self, entity: u64, partial: f32) -> Option<RootMotion> {
            self.players
                .get(&entity)
                .map(|state| state.root_motion(partial))
        }

        /// `entity`'s face quads for this frame, if it has a state.
        pub fn face_quads(&self, entity: u64) -> Option<FaceQuads> {
            let state = self.players.get(&entity)?;
            Some(state.face.quads(&super::faces::current_settings()))
        }

        /// Sets how much of each bone's base pose `entity` keeps.
        pub fn set_retention(&mut self, entity: u64, retention: BasePoseRetention) {
            self.players.entry(entity).or_default().retention = retention;
        }

        /// Whether `entity` has a state.
        pub fn contains(&self, entity: u64) -> bool {
            self.players.contains_key(&entity)
        }

        /// Forgets `entity`; whether it was there.
        pub fn remove(&mut self, entity: u64) -> bool {
            self.players.remove(&entity).is_some()
        }

        /// Forgets every player (a world unload, a dimension change).
        pub fn clear(&mut self) {
            self.players.clear();
        }

        /// How many players the engine holds.
        pub fn len(&self) -> usize {
            self.players.len()
        }

        /// Whether the engine holds no player.
        pub fn is_empty(&self) -> bool {
            self.players.is_empty()
        }
    }

    // ---------------------------------------------------------------------------
    // The frame the game's hooks call
    // ---------------------------------------------------------------------------

    /// What the game's model pose state gives the engine.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct PoseInput {
        /// The entity's age in ticks.
        pub age: f32,
        /// The body yaw, in degrees.
        pub yaw: f32,
        /// The look pitch, in degrees.
        pub pitch: f32,
        /// Its limb speed, `0.0..=1.0`.
        pub speed: f32,
        /// Its walk position, in the units the gait cycle advances with.
        pub position: f32,
        /// Ticks past the last update, for the pose sampled.
        pub partial: f32,
    }

    impl PoseInput {
        /// The movement signals this pose state implies: walking at its limb
        /// speed and nothing else.
        pub fn signals(self) -> MotionSignals {
            let speed = if self.speed.is_finite() {
                self.speed.clamp(0.0, 1.0)
            } else {
                0.0
            };
            MotionSignals {
                tick: if self.age.is_finite() {
                    self.age.max(0.0) as i32
                } else {
                    0
                },
                grounded: true,
                moving: speed > 0.01,
                running: speed >= 0.99,
                walk_forward: speed,
                ..MotionSignals::new()
            }
        }
    }

    /// One pose being built: the entity and the tick it is being drawn at.
    #[derive(Debug, Clone, Copy)]
    struct Posing {
        entity: u64,
        partial: f32,
    }

    #[derive(Default)]
    struct Glue {
        engine: PlayerAnimationEngine,
        /// Seconds since the last `begin_frame`.
        frame_dt: f32,
        last_frame: Option<Instant>,
        /// The entity whose pose is being built.
        posing: Option<Posing>,
    }

    thread_local! {
        static GLUE: std::cell::RefCell<Option<Glue>> = const { std::cell::RefCell::new(None) };
    }

    /// Runs `f` with this thread's engine.
    pub fn with_engine<R>(f: impl FnOnce(&mut PlayerAnimationEngine) -> R) -> R {
        GLUE.with_borrow_mut(|glue| {
            let glue = glue.get_or_insert_with(|| Glue {
                engine: PlayerAnimationEngine::default(),
                frame_dt: 1.0 / 20.0,
                last_frame: None,
                posing: None,
            });
            f(&mut glue.engine)
        })
    }

    /// Starts a frame and returns its length in seconds: the first call and a
    /// stalled frame both read `1/20`, so nothing jumps.
    pub fn begin_frame() -> f32 {
        GLUE.with_borrow_mut(|slot| {
            let glue = slot.get_or_insert_with(|| Glue {
                engine: PlayerAnimationEngine::default(),
                frame_dt: 1.0 / 20.0,
                last_frame: None,
                posing: None,
            });
            let now = Instant::now();
            let dt = glue
                .last_frame
                .map_or(1.0 / 20.0, |last| now.duration_since(last).as_secs_f32());
            glue.last_frame = Some(now);
            glue.frame_dt = dt.clamp(0.0, 1.0 / 20.0);
            glue.frame_dt
        })
    }

    /// The current frame's length, for a pose asked for without a frame first.
    pub fn frame_dt() -> f32 {
        GLUE.with_borrow(|slot| slot.as_ref().map_or(1.0 / 20.0, |glue| glue.frame_dt))
    }

    /// Tells the engine which entity the player model is being posed for, and
    /// steps its animation by this frame.
    pub fn begin_pose(entity: u64, input: PoseInput) {
        let dt = frame_dt();
        let signals = input.signals();
        GLUE.with_borrow_mut(|slot| {
            let glue = slot.get_or_insert_with(|| Glue {
                engine: PlayerAnimationEngine::default(),
                frame_dt: dt,
                last_frame: None,
                posing: None,
            });
            glue.engine.update_with_input(entity, signals, dt, input);
            glue.posing = Some(Posing {
                entity,
                partial: if input.partial.is_finite() {
                    input.partial.clamp(0.0, 1.0)
                } else {
                    1.0
                },
            });
        });
    }

    /// The pose for the entity whose model is being posed, or `None` when the
    /// game posed one the engine knows nothing about.
    pub fn finish_pose() -> Option<HumanoidPose> {
        GLUE.with_borrow_mut(|slot| {
            let glue = slot.as_mut()?;
            let posing = glue.posing.take()?;
            Some(glue.engine.sample_pose(posing.entity, posing.partial))
        })
    }

    /// `entity`'s face quads, or `None` when it has no state.
    pub fn sampled_face_quads(entity: u64) -> Option<FaceQuads> {
        GLUE.with_borrow(|slot| {
            slot.as_ref()
                .and_then(|glue| glue.engine.face_quads(entity))
        })
    }

    /// `entity`'s head transform, for a face drawn on the front of the head.
    pub fn sampled_head_pose(entity: u64) -> Option<PartTransform> {
        GLUE.with_borrow(|slot| {
            slot.as_ref()
                .and_then(|glue| glue.engine.state(entity))
                .map(|state| state.pose.head)
        })
    }

    /// `entity`'s whole-model transform, or `None` when it has no state.
    pub fn sampled_root_motion(entity: u64, partial: f32) -> Option<RootMotion> {
        GLUE.with_borrow(|slot| {
            slot.as_ref()
                .and_then(|glue| glue.engine.root_motion(entity, partial))
        })
    }

    /// Forgets every player and the frame clock (a world unload).
    pub fn reset() {
        GLUE.with_borrow_mut(|slot| {
            if let Some(glue) = slot.as_mut() {
                glue.engine.clear();
                glue.posing = None;
                glue.frame_dt = 1.0 / 20.0;
                glue.last_frame = None;
            }
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn clip(name: &str, length: f32, bones: &[(&str, [f32; 3])]) -> Clip {
            let mut map = std::collections::HashMap::new();
            for (bone, rotation) in bones {
                // One linear channel: the rotation ramps 0..=value over the clip.
                let mut keys = Vec::new();
                for t in [0.0f32, length] {
                    keys.push((
                        t,
                        super::super::library::Key {
                            pre: Vec3::new(
                                rotation[0] * t / length,
                                rotation[1] * t / length,
                                rotation[2] * t / length,
                            ),
                            post: Vec3::new(
                                rotation[0] * t / length,
                                rotation[1] * t / length,
                                rotation[2] * t / length,
                            ),
                            interpolation: "linear".into(),
                            easing: None,
                        },
                    ));
                }
                map.insert(
                    bone.to_string(),
                    super::super::library::BoneTrack::new(
                        super::super::library::Channel::new(keys.clone(), Vec3::ZERO),
                        super::super::library::Channel::new(keys.clone(), Vec3::ZERO),
                        super::super::library::Channel::new(keys, Vec3::ONE),
                    ),
                );
            }
            Clip::new(name.to_string(), length, false, map)
        }

        #[test]
        fn a_step_turn_eases_the_body_across_the_clip() {
            let left = clip("step_left", 0.3, &[("root", [0.0, -45.0, 0.0])]);
            let right = clip("step_right", 0.3, &[("root", [0.0, 45.0, 0.0])]);
            let settings = TurnSettings {
                enabled: true,
                threshold: 45.0,
                delay: 2.0,
                fade: 0.12,
                mode: TurnMode::Steps,
                reference: 45.0,
                double_turn: 75.0,
                template: None,
                left: Some(left),
                right: Some(right),
                left_add: None,
                right_add: None,
            };
            let mut turn = TurnInPlaceController::new();
            // The look jumps 60 degrees right while standing: the body follows
            // through the clip.
            let mut time = 0.0;
            turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
            time += 0.05;
            turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
            time += 0.05;
            turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
            assert!(turn.weight() > 0.0, "the step plays");
            // Half way through the 0.3 s clip the body has turned most of the way.
            let mut yaw = 0.0;
            for _ in 0..4 {
                time += 0.05;
                yaw = turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
            }
            assert!(yaw > 20.0, "{}", yaw);
            // Settled at the target.
            for _ in 0..20 {
                time += 0.05;
                yaw = turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
            }
            assert!((yaw - 60.0).abs() < 1e-3, "{}", yaw);
            assert!((turn.weight()).abs() < 1e-4, "the playback is spent");
        }

        #[test]
        fn a_step_turn_releases_back_to_the_feet_while_moving() {
            let right = clip("step_right", 0.3, &[("root", [0.0, 45.0, 0.0])]);
            let settings = TurnSettings {
                enabled: true,
                threshold: 45.0,
                delay: 2.0,
                fade: 0.12,
                mode: TurnMode::Steps,
                reference: 45.0,
                double_turn: 75.0,
                template: None,
                left: None,
                right: Some(right),
                left_add: None,
                right_add: None,
            };
            let mut turn = TurnInPlaceController::new();
            let mut time = 0.0;
            turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
            time += 0.05;
            turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
            // Walking again hands the yaw back to the feet over the fade.
            time += 0.05;
            let yaw = turn.update(10.0, 60.0, 0.0, time, true, false, false, &settings);
            assert!(yaw < 60.0);
            for _ in 0..10 {
                time += 0.05;
                let yaw = turn.update(10.0, 60.0, 0.0, time, true, false, false, &settings);
                let _ = yaw;
            }
            time += 0.05;
            let yaw = turn.update(10.0, 60.0, 0.0, time, true, false, false, &settings);
            assert!((yaw - 10.0).abs() < 1e-3, "{}", yaw);
        }

        #[test]
        fn the_camera_turn_tracks_the_look() {
            let right = clip("step_right", 0.3, &[("root", [0.0, 45.0, 0.0])]);
            let settings = TurnSettings {
                enabled: true,
                threshold: 45.0,
                delay: 2.0,
                fade: 0.12,
                mode: TurnMode::Camera,
                reference: 45.0,
                double_turn: 75.0,
                template: None,
                left: None,
                right: Some(right),
                left_add: None,
                right_add: None,
            };
            let mut turn = TurnInPlaceController::new();
            let mut time = 0.0f64;
            // The look moves 30 degrees between calls: the body tracks it.
            turn.update(0.0, 0.0, 0.0, time, true, true, false, &settings);
            time += 0.05;
            let yaw = turn.update(0.0, 30.0, 0.0, time, true, true, false, &settings);
            assert!((yaw - 30.0).abs() < 1e-3, "{}", yaw);
            // The clip plays once the look settles past the input gap.
            time += 0.05;
            turn.update(30.0, 30.0, 0.0, time, true, true, false, &settings);
            assert!(turn.weight() > 0.0 || turn.body_yaw() == 30.0);
        }

        #[test]
        fn the_keyframed_turn_splits_a_double_turn() {
            let left = clip("step_left", 0.3, &[("root", [0.0, -45.0, 0.0])]);
            let right = clip("step_right", 0.3, &[("root", [0.0, 45.0, 0.0])]);
            let settings = TurnSettings {
                enabled: true,
                threshold: 45.0,
                delay: 0.12,
                fade: 0.12,
                mode: TurnMode::Dynamic,
                reference: 45.0,
                double_turn: 75.0,
                template: None,
                left: Some(left),
                right: Some(right),
                left_add: None,
                right_add: None,
            };
            let mut turn = TurnInPlaceController::new();
            let mut time = 0.0f64;
            // A 160-degree look swing: the first turn covers 48% of it.
            turn.update(0.0, 0.0, 0.0, time, true, true, false, &settings);
            time += 0.05;
            turn.update(0.0, 160.0, 0.0, time, true, true, false, &settings);
            // The swing splits: the first clip passes near 48% of it, the
            // second completes the turn.
            let mut yaw = 0.0;
            let mut hit_mid = false;
            for _ in 0..40 {
                time += 0.05;
                yaw = turn.update(160.0, 160.0, 0.0, time, true, true, false, &settings);
                if (yaw - 160.0 * 0.48).abs() < 12.0 {
                    hit_mid = true;
                }
            }
            assert!(hit_mid, "the first clip covered about half the swing");
            assert!((yaw - 160.0).abs() < 2.0, "{}", yaw);
        }

        #[test]
        fn playback_plays_stops_and_holds() {
            let mut player = ClipSequencePlayer::new();
            let clip = clip("wave", 0.4, &[("right_arm", [0.0, 0.0, 30.0])]);
            player.play(
                Some(clip.clone()),
                Vec::new(),
                false,
                0.1,
                false,
                BasePoseRetention::default(),
                false,
                None,
            );
            assert!(player.active());
            assert!(!player.finished());
            player.tick(0.2);
            assert!(
                (player.progress() - 0.5).abs() < 1e-4,
                "{}",
                player.progress()
            );
            // The rotation ramps linearly to half its value.
            let sample = player.sample("right_arm", 1.0);
            assert!(
                (sample.pose.rotation.z - 15.0).abs() < 1e-3,
                "{}",
                sample.pose.rotation.z
            );
            player.hold_last_frame();
            assert!((player.render_time(1.0) - 0.4).abs() < 1e-4);
            assert!(player.finished());
            player.stop(0.1);
            player.tick(0.2);
            player.tick(0.05);
            assert!(!player.active());
        }

        #[test]
        fn playback_cross_blends_into_the_next_clip() {
            let mut player = ClipSequencePlayer::new();
            let a = clip("a", 0.4, &[("right_arm", [0.0, 0.0, 30.0])]);
            let b = clip("b", 0.4, &[("right_arm", [0.0, 0.0, 60.0])]);
            player.play(
                Some(a),
                Vec::new(),
                false,
                0.2,
                false,
                BasePoseRetention::default(),
                false,
                None,
            );
            player.tick(0.1);
            player.play(
                Some(b),
                Vec::new(),
                false,
                0.2,
                false,
                BasePoseRetention::default(),
                false,
                None,
            );
            // Mid-blend the sample sits between the two clips.
            player.tick(0.1);
            let sample = player.sample("right_arm", 1.0);
            assert!(
                sample.pose.rotation.z > 0.0 && sample.pose.rotation.z < 60.0,
                "{}",
                sample.pose.rotation.z
            );
            // The blend settles on the new clip.
            for _ in 0..20 {
                player.tick(0.05);
            }
            let sample = player.sample("right_arm", 1.0);
            assert!((sample.pose.rotation.z - 60.0).abs() < 1e-3);
        }

        #[test]
        fn exclusive_overlays_replace_their_tracks() {
            let base = clip(
                "base",
                0.4,
                &[("right_arm", [0.0, 0.0, 30.0]), ("head", [0.0, 10.0, 0.0])],
            );
            let over = clip("over", 0.4, &[("right_arm", [0.0, 0.0, 90.0])]);
            let merged = merge_exclusive_tracks(
                &base,
                &[ResolvedOverlay {
                    clip: over.clone(),
                    mode: OverlayMode::Override,
                    bones: Vec::new(),
                }],
            );
            let sample = merged.sample_once("right_arm", 0.2);
            assert!(
                (sample.rotation.z - 45.0).abs() < 1e-3,
                "the overlay's track wins"
            );
            let head = merged.sample_once("head", 0.2);
            assert!((head.rotation.y - 5.0).abs() < 1e-3, "the others stay");
            assert!(has_track(Some(&merged), "right_arm"));
            assert!(!has_track(None, "right_arm"));
        }

        #[test]
        fn compose_keeps_the_retained_part_of_the_base() {
            let base = Pose {
                position: Vec3::ZERO,
                rotation: Vec3::new(10.0, 0.0, 0.0),
                scale: Vec3::ONE,
            };
            let sample = Sample {
                pose: Pose {
                    position: Vec3::ZERO,
                    rotation: Vec3::new(30.0, 0.0, 0.0),
                    scale: Vec3::ONE,
                },
                weight: 1.0,
                base_retain: 0.5,
                legacy_retain_weight: 0.0,
            };
            let composed = compose(base, sample, 0.0);
            // Half the base rotation kept, plus the clip's.
            assert!(
                composed.rotation.x > 30.0 && composed.rotation.x < 40.0,
                "{}",
                composed.rotation.x
            );
            // Zero weight leaves the base alone.
            let untouched = compose(
                base,
                Sample {
                    weight: 0.0,
                    ..sample
                },
                0.0,
            );
            assert_eq!(untouched, base);
        }

        #[test]
        fn mix_rotation_goes_the_short_way() {
            let mixed = mix_rotation(Vec3::new(170.0, 0.0, 0.0), Vec3::new(-170.0, 0.0, 0.0), 0.5);
            assert!(
                mixed.x.abs() < 1e-3
                    || (mixed.x - 180.0).abs() < 1e-3
                    || (mixed.x + 180.0).abs() < 1e-3,
                "{}",
                mixed.x
            );
            assert_eq!(
                mix_rotation(Vec3::new(10.0, 0.0, 0.0), Vec3::new(30.0, 0.0, 0.0), 0.0).x,
                10.0
            );
            assert_eq!(
                mix_rotation(Vec3::new(10.0, 0.0, 0.0), Vec3::new(30.0, 0.0, 0.0), 1.0).x,
                30.0
            );
        }

        #[test]
        fn the_clip_pose_blend_runs_its_clock() {
            let mut blend = ClipPoseBlend::new();
            let clip = clip("pose", 0.4, &[("body", [0.0, 0.0, 40.0])]);
            blend.start(Some(clip), 0.0, 1.0, false, 0.0, Vec::new());
            blend.tick(0.2, 0.0, 0.2, 1.0);
            let pose = blend.sample("body", 1.0);
            assert!((pose.rotation.z - 20.0).abs() < 1e-3, "{}", pose.rotation.z);
            // The carry form reads the same clip without the local arms.
            let carry = blend.sample_carry("body", 1.0);
            assert!((carry.rotation.z - 20.0).abs() < 1e-3);
            blend.clear();
            let cleared = blend.sample("body", 1.0);
            assert_eq!(cleared, Pose::identity());
        }

        #[test]
        fn the_direction_change_leans_within_three_degrees() {
            let mut direction = DirectionChangeController::default();
            // Walk north for two ticks, then turn hard east: the lean clamps at
            // 3 degrees.
            direction.tick(1, 0.0, 1.0, 0.0, true);
            direction.tick(2, 0.0, 1.0, 0.0, true);
            let mut peak = 0.0f32;
            for tick in 3..13 {
                direction.tick(tick, 1.0, 0.0, 90.0, true);
                peak = peak.max(direction.pitch(1.0).abs());
            }
            assert!(peak <= 3.0 + 1e-3, "{}", peak);
            assert!(peak > 0.5, "{}", peak);
            // Standing still long enough resets the tracker.
            for tick in 13..21 {
                direction.tick(tick, 0.0, 0.0, 90.0, true);
            }
            assert!(
                direction.pitch(1.0).abs() < 0.01,
                "{}",
                direction.pitch(1.0)
            );
        }

        #[test]
        fn the_turn_lean_scales_with_speed_and_clamps() {
            let mut lean = TurnLeanController::default();
            // Full speed, a hard look turn and full sideways input: the lean
            // clamps at 14.
            for tick in 1..20 {
                lean.tick(
                    tick,
                    -60.0 * tick as f32,
                    0.215,
                    1.0,
                    1.0,
                    true,
                    false,
                    false,
                );
            }
            assert!(
                (lean.sample(1.0).abs() - 14.0).abs() < 1.0,
                "{}",
                lean.sample(1.0)
            );
            // Standing still eases back to zero.
            for tick in 20..40 {
                lean.tick(tick, 0.0, 0.0, 0.0, 0.0, true, false, false);
            }
            assert!(lean.sample(1.0).abs() < 0.5, "{}", lean.sample(1.0));
            // Invisible, the lean reads zero.
            let mut lean = TurnLeanController::default();
            lean.tick(1, 0.0, 0.215, 1.0, 1.0, false, false, false);
            assert_eq!(lean.sample(1.0), 0.0);
        }

        #[test]
        fn the_creative_flight_hover_needs_steady_heights() {
            let mut motion = CreativeFlightMotion::default();
            // Eight ticks at the same height: the hover is on.
            let mut flying = false;
            for tick in 0..10 {
                flying = motion.update(tick, 64.0, true);
            }
            assert!(flying);
            // A steady descent ends the hover while the fall builds up.
            let mut ended = false;
            for tick in 10..24 {
                if !motion.update(tick, 64.0 - (tick - 10) as f64 * 0.4, true) {
                    ended = true;
                }
            }
            assert!(ended, "the accelerating descent ended the hover");
            // On the ground it never starts.
            let mut motion = CreativeFlightMotion::default();
            for tick in 0..10 {
                assert!(!motion.update(tick, 64.0, false));
            }
        }

        #[test]
        fn the_swim_alignment_eases_toward_the_surface() {
            let mut swim = SwimSurfaceAlignment::default();
            swim.set_reference_height(0.6);
            swim.prepare_eye_height(1.62);
            swim.tick(true, true, 63.0, 0.12);
            for _ in 0..20 {
                swim.tick(true, true, 63.0, 0.12);
            }
            assert!(swim.active());
            // The eye height smoothed towards its target.
            swim.prepare_eye_height(1.42);
            swim.prepare_eye_height(1.42);
            let eye = swim.camera_eye_height(62.0, 1.0, 1.0, 1.62);
            assert!(eye < 1.62, "{}", eye);
            swim.clear();
            assert!(!swim.active());
        }

        #[test]
        fn the_fishing_bob_falls_and_is_pulled_in() {
            let mut swing = FishingSwing::default();
            // The first frame hangs straight down the line's length.
            let direction = swing.update([0.0, 64.0, 0.0], 2.0, 0.0);
            assert_eq!(direction, [0.0, -1.0, 0.0]);
            // Time passing: the bob stays on the line, so the direction keeps
            // pointing down the line.
            let direction = swing.update([0.0, 64.0, 0.0], 2.0, 0.5);
            assert!(direction[1] < 0.0, "{:?}", direction);
            // A broken frame restarts the swing.
            let direction = swing.update([0.0, 64.0, 0.0], 2.0, 100.0);
            assert_eq!(direction, [0.0, -1.0, 0.0]);
        }

        #[test]
        fn the_paddles_ease_between_strokes() {
            // The vanilla pose at phase 0 for the left paddle.
            let rest = paddle_vanilla_pose(0, 0.0);
            assert!(
                (rest.pitch - (-std::f32::consts::PI / 3.0 - std::f32::consts::PI / 12.0) / 2.0)
                    .abs()
                    < 1e-4
            );
            let mut paddle = Paddle::new(0, 1, 0.0);
            // One stroke step moves the phase by an eighth of a turn.
            paddle.tick(1, PADDLE_STEP);
            let mid = paddle.sample(0.5);
            assert!(mid.pitch.is_finite());
            // Stopping eases back over the longer 0.3 s.
            paddle.tick(0, 0.0);
            for _ in 0..8 {
                paddle.tick(0, 0.0);
            }
            let settled = paddle.sample(1.0);
            assert!(
                (settled.pitch - paddle_vanilla_pose(0, 0.0).pitch).abs() < 1e-3,
                "{:?}",
                settled
            );
        }

        #[test]
        fn the_boat_weight_cubes_in() {
            let mut boat = BoatAnimation::new(1, 0, [false, false], [0.0, 0.0], false);
            boat.step(1, 1, false, [true, true], false);
            let early = boat.weight(1.0);
            boat.step(2, 1, false, [true, true], false);
            boat.step(3, 1, false, [true, true], false);
            let later = boat.weight(1.0);
            assert!(later > early);
            assert!(later <= 1.0);
            // The vanilla pose mirrors across the boat.
            let left = paddle_vanilla_pose(0, 0.0);
            let mirrored = paddle_opposite_side(left);
            assert!((mirrored.yaw - (std::f32::consts::PI - left.yaw)).abs() < 1e-5);
        }

        #[test]
        fn the_elytra_correction_is_ninety_degrees_of_the_weight() {
            assert_eq!(elytra_correction_degrees(true, true, false, 1.0), 90.0);
            assert_eq!(elytra_correction_degrees(true, true, false, 0.5), 45.0);
            assert_eq!(
                elytra_correction_degrees(true, true, false, 2.0),
                90.0,
                "clamped"
            );
            assert_eq!(
                elytra_correction_degrees(true, true, true, 1.0),
                0.0,
                "paused"
            );
            assert_eq!(elytra_correction_degrees(false, true, false, 1.0), 0.0);
            assert_eq!(elytra_correction_degrees(true, true, false, f32::NAN), 0.0);
        }

        #[test]
        fn the_held_item_matrix_translates_by_sixteenths() {
            let matrix = held_item_matrix(Vec3::new(16.0, 0.0, -32.0), Vec3::ZERO, Vec3::ONE);
            let values = matrix.values();
            // The translation column: (-1, 2, 0).
            assert!((values[12] - (-1.0)).abs() < 1e-5);
            assert!((values[13] - 2.0).abs() < 1e-5);
            assert!((values[14] - 0.0).abs() < 1e-5);
            // A rotation turns the matrix.
            let turned = held_item_matrix(Vec3::ZERO, Vec3::new(0.0, 0.0, 90.0), Vec3::ONE);
            assert!((turned.values()[0] - 90.0f32.to_radians().cos()).abs() < 1e-5);
        }

        #[test]
        fn the_pose_ownership_blend_eases_between_owners() {
            let mut blend = PoseOwnershipBlend::default();
            let mut parts: Vec<(&'static str, [f32; 9])> =
                vec![("head", [0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0])];
            // The mod owns the part first: the pose is the mod's.
            let mut heads: Vec<(&'static str, &mut [f32; 9])> = vec![("head", &mut parts[0].1)];
            blend.apply(1, &mut heads, true, 0.0, true);
            // Vanilla takes over: over 3 seconds the rotation eases back.
            let mut heads: Vec<(&'static str, &mut [f32; 9])> = vec![("head", &mut parts[0].1)];
            blend.apply(1, &mut heads, false, 0.1, true);
            let after = parts[0].1[3];
            assert!(
                (after - 0.5).abs() < 1e-6,
                "the blend starts where the mod left it"
            );
            // Settled, vanilla's value stands: vanilla writes its own pose into
            // the parts every frame, then the blend mixes the snapshot away.
            for step in 1..60 {
                let time = 0.1 + step as f32 * 0.1;
                parts[0].1[3] = 0.0;
                let mut heads: Vec<(&'static str, &mut [f32; 9])> = vec![("head", &mut parts[0].1)];
                blend.apply(1, &mut heads, false, time, true);
            }
            assert!(parts[0].1[3].abs() < 0.01, "{}", parts[0].1[3]);
        }

        #[test]
        fn the_outer_ownership_blend_keeps_the_transform_rigid() {
            let mut blend = OuterPoseOwnershipBlend::default();
            // The mod's matrix is a pure translation; the render's is the same
            // translation, so the delta is the identity and nothing moves.
            let outer = *Mat4Builder::identity().translate(1.0, 2.0, 3.0).values();
            let mut render = outer;
            blend.apply(1, Some(&outer), &mut render, true, 0.0, true);
            assert!((render[12] - 1.0).abs() < 1e-5);
            assert!((render[13] - 2.0).abs() < 1e-5);
            assert!((render[14] - 3.0).abs() < 1e-5);
            // A scaled outer matrix is not rigid: the track is dropped.
            let mut scaled = *Mat4Builder::identity().scale(2.0, 2.0, 2.0).values();
            blend.apply(2, Some(&scaled), &mut render, true, 0.0, true);
            scaled = *Mat4Builder::identity().translate(0.0, 0.0, 0.0).values();
            blend.apply(2, Some(&scaled), &mut render, false, 0.1, true);
        }

        #[test]
        fn climb_facing_reads_the_block() {
            // A `facing` property wins, read as its opposite.
            assert_eq!(
                climb_facing_yaw(Some("north"), &[], |_| false, 0.0, 0.0, 0.0),
                climb_direction_yaw("south"),
            );
            // Boolean flags pick the nearest face.
            let yaw = climb_facing_yaw(
                None,
                &[("north", true), ("east", true)],
                |_| false,
                0.1,
                0.1,
                0.0,
            );
            assert_eq!(
                yaw,
                climb_direction_yaw("north"),
                "north is nearest at x 0.1"
            );
            // Nothing known: the player's yaw stands.
            assert_eq!(climb_facing_yaw(None, &[], |_| false, 0.5, 0.5, 42.0), 42.0);
            // The climbable neighbours break the tie.
            let yaw = climb_facing_yaw(None, &[], |d| d == "west", 0.5, 0.5, 0.0);
            assert_eq!(yaw, climb_direction_yaw("west"));
        }

        #[test]
        fn the_engine_glue_still_poses_and_reads_root_motion() {
            GLUE.with_borrow_mut(|slot| *slot = Some(Glue::default()));
            let pose = with_engine(|engine| {
                let state = engine.update(7, MotionSignals::new(), 0.05);
                state.pose().clone()
            });
            assert!(pose.right_arm.x_rot.abs() < 1e-5, "still, the arms hang");
            assert_eq!(pose.head, PartTransform::IDENTITY);
            let root = with_engine(|engine| engine.root_motion(7, 1.0));
            assert_eq!(root, Some(RootMotion::default()));
            with_engine(|engine| engine.clear());
            GLUE.with_borrow_mut(|slot| *slot = Some(Glue::default()));
        }

        #[test]
        fn the_part_transform_carries_scale() {
            let mut part = PartTransform::IDENTITY;
            TurnInPlaceController::apply_part(
                &mut part,
                &Pose {
                    position: Vec3::new(1.0, 2.0, 3.0),
                    rotation: Vec3::new(90.0, 0.0, 0.0),
                    scale: Vec3::new(2.0, 1.0, 1.0),
                },
                1.0,
            );
            assert!((part.x - 1.0).abs() < 1e-6);
            assert!(
                (part.y + 2.0).abs() < 1e-6,
                "the y flips, like the Java apply"
            );
            assert!((part.z - 3.0).abs() < 1e-6);
            assert!((part.x_rot - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
            assert!((part.x_scale - 2.0).abs() < 1e-6);
            assert!((part.y_scale - 1.0).abs() < 1e-6);
        }

        #[test]
        fn quaternion_round_trips_through_euler() {
            for (x, y, z) in [(30.0, 45.0, 60.0), (-20.0, 0.0, 90.0), (0.0, 0.0, 0.0)] {
                let q = Quat::from_zyx_degrees(Vec3::new(x, y, z));
                let back = q.to_euler_zyx_degrees();
                assert!((back.x - x).abs() < 1e-3, "{} vs {}", back.x, x);
                assert!((back.y - y).abs() < 1e-3, "{} vs {}", back.y, y);
                assert!((back.z - z).abs() < 1e-3, "{} vs {}", back.z, z);
            }
            // Slerp walks the short arc.
            let a = Quat::from_zyx_degrees(Vec3::new(170.0, 0.0, 0.0));
            let b = Quat::from_zyx_degrees(Vec3::new(-170.0, 0.0, 0.0));
            let mid = a.slerp(b, 0.5).to_euler_zyx_degrees();
            // The short arc from +170 to -170 runs through 180.
            assert!((mid.x.abs() - 180.0).abs() < 1e-2, "{}", mid.x);
        }

        #[test]
        fn the_item_sequence_starts_and_idles_without_a_library() {
            let mut controller = ItemSequenceController::new();
            let input = ItemInput::default();
            controller.tick(
                &input, None, true, true, true, 0.0, false, false, None, "", false, 0.0, 0.0,
            );
            assert!(!controller.active(), "no pack, no playback");
            assert!(!controller.replaces_falling());
            // The mace helpers read as the Java statics do.
            assert!(ItemSequenceController::starts_mace_charge(
                false, -1.0, 4.0, 3.0, false
            ));
            assert!(!ItemSequenceController::starts_mace_charge(
                true, -1.0, 4.0, 3.0, false
            ));
            assert!(ItemSequenceController::can_mace_smash(false, 2.0, 1.5));
            assert!(!ItemSequenceController::can_mace_smash(true, 2.0, 1.5));
            assert!(ItemSequenceController::supports_use("bow", "release"));
            assert!(!ItemSequenceController::supports_use("bow", "charge"));
            assert!(ItemSequenceController::supports_use("fishing", "cast"));
            assert!(ItemSequenceController::continues_offhand_action(
                "release", false, false
            ));
            assert!(!ItemSequenceController::continues_offhand_action(
                "idle", false, false
            ));
        }

        #[test]
        fn the_crouch_machine_tracks_its_amount_without_clips() {
            let mut crouch = CrouchSequenceController::new();
            for _ in 0..5 {
                crouch.tick(true, false, None, None, None, 1.0, false);
            }
            // Sprinting crouches with no clips read as zero, and the amount
            // eases down at 0.2 a tick.
            assert!(crouch.render_amount(1.0) <= 1.0);
            crouch.tick(false, false, None, None, None, 1.0, false);
            assert_eq!(crouch.phase(), "");
            assert!(!crouch.is_active());
        }

        #[test]
        fn the_turn_settings_read_the_library_defaults() {
            let library = super::super::library::AnimationLibrary::new(
                HashMap::new(),
                super::super::library::ItemAnimationBindings::new(),
            );
            let settings = TurnSettings::from_library(&library);
            assert!(!settings.enabled, "off by default");
            assert_eq!(settings.mode, TurnMode::Steps);
            assert_eq!(settings.threshold, 45.0);
            assert_eq!(settings.reference, 45.0);
            assert_eq!(settings.double_turn, 75.0);
            assert_eq!(settings.fade, 0.12);
            assert_eq!(settings.delay, 2.0);
        }

        #[test]
        fn mirror_fallback_and_sampled_bones_follow_the_java() {
            let plain = clip("swing", 0.2, &[]);
            assert!(!mirror_fallback(Some(&plain), false));
            assert!(
                mirror_fallback(Some(&plain), true),
                "no _left suffix mirrors"
            );
            let left = clip("swing_left", 0.2, &[]);
            assert!(!mirror_fallback(Some(&left), true));
            assert!(!mirror_fallback(None, true));
            assert_eq!(sampled_bone("left_arm", true), "right_arm");
            assert_eq!(sampled_bone("right_leg", true), "left_leg");
            assert_eq!(sampled_bone("body", true), "body");
            assert_eq!(sampled_bone("head", false), "head");
        }
    }
}

pub mod faces {
    //! Punchy Moves' facial expressions: the settings page, the cue table, and the
    //! per-player rig that blinks, follows the eyes and draws the face as quads on
    //! the player's head.
    //!
    //! Pure Rust and std-only, so the tests run without a Horizon build. The game
    //! side is one helper the bundle injects: `h_face.rs` appends the rig's quads
    //! after a player's body is submitted (`mobs::render::render_with_hidden`),
    //! and `h_frame.rs` keeps the settings and the animation clock.
    //!
    //! # The face textures
    //!
    //! The rig draws nothing but flat quads on the front of the head, each one a
    //! cell of one of two of the mod's own textures:
    //!
    //! - `textures/face/fx.png`, 256x256, a 2x2 sheet of soft white masks: a round
    //!   glow, a wide ellipse, a broad vertical band and a thin vertical streak.
    //!   The rig reads them as the eye's shine, the eyelid's shade, the brow's and
    //!   the mouth's strokes, and any expression's extra effect.
    //! - `textures/face/marks.png`, 4x4, a 2x2 sheet of the little marks a
    //!   reaction puts on a cheek: blush, sweat, a tear and a strain mark.
    //!
    //! Both are addressed in normalised texture space ([`FaceUv`]), so a resize of
    //! either sheet only changes the constants here.

    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::signals::MotionSignals;

    /// The effect sheet: a 2x2 atlas of white masks.
    pub const FX_TEXTURE: &str = "punchy_moves:textures/face/fx.png";
    /// The effect sheet's size in texels.
    pub const FX_SIZE: f32 = 256.0;
    /// The mark sheet: a 2x2 atlas of cheek marks.
    pub const MARKS_TEXTURE: &str = "punchy_moves:textures/face/marks.png";
    /// The mark sheet's size in texels.
    pub const MARKS_SIZE: f32 = 4.0;

    // ---------------------------------------------------------------------------
    // FaceUv
    // ---------------------------------------------------------------------------

    /// One rectangle of a face texture in normalised coordinates, `0.0..=1.0`,
    /// with `v` running down the image like every texture's.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FaceUv {
        pub u0: f32,
        pub v0: f32,
        pub u1: f32,
        pub v1: f32,
    }

    impl FaceUv {
        /// The whole texture.
        pub const FULL: FaceUv = FaceUv {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        };

        /// One cell of a `columns` by `rows` sheet, counted from the top left.
        pub fn cell(columns: u32, rows: u32, column: u32, row: u32) -> FaceUv {
            let columns = columns.max(1) as f32;
            let rows = rows.max(1) as f32;
            let column = (column as f32).min(columns - 1.0);
            let row = (row as f32).min(rows - 1.0);
            FaceUv {
                u0: column / columns,
                v0: row / rows,
                u1: (column + 1.0) / columns,
                v1: (row + 1.0) / rows,
            }
        }

        /// The rectangle's width and height in texture space.
        pub fn size(&self) -> [f32; 2] {
            [self.u1 - self.u0, self.v1 - self.v0]
        }

        /// The rectangle in texels of a `width` by `height` texture:
        /// `[x0, y0, x1, y1]`.
        pub fn px(&self, width: f32, height: f32) -> [f32; 4] {
            [
                self.u0 * width,
                self.v0 * height,
                self.u1 * width,
                self.v1 * height,
            ]
        }

        /// Whether the rectangle lies inside the texture and has a size.
        pub fn is_valid(&self) -> bool {
            let inside = |v: f32| (0.0..=1.0).contains(&v);
            inside(self.u0)
                && inside(self.v0)
                && inside(self.u1)
                && inside(self.v1)
                && self.u1 > self.u0
                && self.v1 > self.v0
        }

        /// A point of this rectangle from a point of the whole quad, `0.0..=1.0`
        /// across and down it: how a quad's corner becomes an atlas coordinate.
        pub fn at(&self, u: f32, v: f32) -> [f32; 2] {
            [
                self.u0 + (self.u1 - self.u0) * u.clamp(0.0, 1.0),
                self.v0 + (self.v1 - self.v0) * v.clamp(0.0, 1.0),
            ]
        }
    }

    /// The effect sheet's four cells, as the rig's four face layers.
    ///
    /// `fx.png`'s top left is the round glow, top right the wide ellipse, bottom
    /// left the vertical band and bottom right the thin streak.
    pub const FX_IRIS: FaceUv = FaceUv {
        u0: 0.0,
        v0: 0.0,
        u1: 0.5,
        v1: 0.5,
    };
    pub const FX_LID: FaceUv = FaceUv {
        u0: 0.5,
        v0: 0.0,
        u1: 1.0,
        v1: 0.5,
    };
    pub const FX_BROW: FaceUv = FaceUv {
        u0: 0.0,
        v0: 0.5,
        u1: 0.5,
        v1: 1.0,
    };
    pub const FX_MOUTH: FaceUv = FaceUv {
        u0: 0.5,
        v0: 0.5,
        u1: 1.0,
        v1: 1.0,
    };

    /// The mark sheet's four cells.
    pub const MARK_BLUSH: FaceUv = FaceUv {
        u0: 0.0,
        v0: 0.0,
        u1: 0.5,
        v1: 0.5,
    };
    pub const MARK_SWEAT: FaceUv = FaceUv {
        u0: 0.5,
        v0: 0.0,
        u1: 1.0,
        v1: 0.5,
    };
    pub const MARK_TEAR: FaceUv = FaceUv {
        u0: 0.0,
        v0: 0.5,
        u1: 0.5,
        v1: 1.0,
    };
    pub const MARK_VEIN: FaceUv = FaceUv {
        u0: 0.5,
        v0: 0.5,
        u1: 1.0,
        v1: 1.0,
    };

    // ---------------------------------------------------------------------------
    // FaceSettings
    // ---------------------------------------------------------------------------

    /// The face's personality: how quick the rig is to smile, scowl or look away.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum Personality {
        /// Even-tempered: the plain thresholds.
        #[default]
        Neutral,
        /// Smiles sooner and blinks a little less.
        Cheerful,
        /// Scowls at the smallest thing and blinks more.
        Grumpy,
        /// Looks away sooner and blushes more.
        Shy,
    }

    impl Personality {
        /// The personalities in the order the settings page cycles them.
        pub const ALL: [Personality; 4] = [
            Personality::Neutral,
            Personality::Cheerful,
            Personality::Grumpy,
            Personality::Shy,
        ];

        /// The label the settings page shows.
        pub fn label(self) -> &'static str {
            match self {
                Personality::Neutral => "Neutral",
                Personality::Cheerful => "Cheerful",
                Personality::Grumpy => "Grumpy",
                Personality::Shy => "Shy",
            }
        }

        /// The next personality in [`Personality::ALL`].
        pub fn next(self) -> Personality {
            let at = Personality::ALL
                .iter()
                .position(|p| *p == self)
                .unwrap_or(0);
            Personality::ALL[(at + 1) % Personality::ALL.len()]
        }

        /// How much easier this personality smiles (`1.0` is neutral).
        pub fn cheer(self) -> f32 {
            match self {
                Personality::Neutral => 1.0,
                Personality::Cheerful => 1.4,
                Personality::Grumpy => 0.6,
                Personality::Shy => 0.9,
            }
        }

        /// How much quicker this personality scowls.
        pub fn grumble(self) -> f32 {
            match self {
                Personality::Neutral => 1.0,
                Personality::Cheerful => 0.6,
                Personality::Grumpy => 1.6,
                Personality::Shy => 0.8,
            }
        }

        /// How much more this personality blushes.
        pub fn blush(self) -> f32 {
            match self {
                Personality::Shy => 1.5,
                Personality::Cheerful => 1.1,
                Personality::Neutral => 1.0,
                Personality::Grumpy => 0.7,
            }
        }
    }

    /// The face's settings: on or off, how often it blinks, whether the eyes
    /// follow the body, how strong a reaction shows and which personality wears it.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FaceSettings {
        /// The face is drawn at all.
        pub enabled: bool,
        /// Mean seconds between blinks.
        pub blink_interval: f32,
        /// The eyes follow the walk, the climb and the swim.
        pub look_tracking: bool,
        /// How strong a reaction shows, `0.0..=1.0`.
        pub reaction_intensity: f32,
        /// How quick the rig is to react.
        pub personality: Personality,
    }

    /// The blink intervals one click of the page cycles through, in seconds.
    const BLINK_STEPS: [f32; 5] = [2.0, 3.5, 5.0, 7.0, 10.0];
    /// The reaction strengths one click cycles through.
    const REACTION_STEPS: [f32; 5] = [0.25, 0.5, 0.75, 1.0, 1.25];
    /// How many buttons the face section of the settings page has.
    pub const BUTTONS: usize = 5;

    impl Default for FaceSettings {
        fn default() -> Self {
            Self {
                enabled: true,
                blink_interval: 5.0,
                look_tracking: true,
                reaction_intensity: 1.0,
                personality: Personality::Neutral,
            }
        }
    }

    impl FaceSettings {
        /// The defaults.
        pub fn new() -> Self {
            Self::default()
        }

        /// Reads the settings from stored values (`get` reads one key); a missing
        /// or unreadable value keeps its default.
        pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
            let defaults = Self::default();
            let flag =
                |key: &str, fallback: bool| get(key).map_or(fallback, |v| v.trim() != "false");
            let number = |key: &str, fallback: f32, min: f32, max: f32| {
                get(key)
                    .and_then(|v| v.trim().parse::<f32>().ok())
                    .filter(|v| v.is_finite())
                    .map(|v| v.clamp(min, max))
                    .unwrap_or(fallback)
            };
            let personality = get("personality")
                .and_then(|name| {
                    Personality::ALL
                        .into_iter()
                        .find(|p| p.label().eq_ignore_ascii_case(name.trim()))
                })
                .unwrap_or(defaults.personality);
            Self {
                enabled: flag("enabled", defaults.enabled),
                blink_interval: number("blink_interval", defaults.blink_interval, 0.5, 30.0),
                look_tracking: flag("look_tracking", defaults.look_tracking),
                reaction_intensity: number(
                    "reaction_intensity",
                    defaults.reaction_intensity,
                    0.0,
                    2.0,
                ),
                personality,
            }
        }

        /// The settings page's buttons, in the order [`FaceSettings::click`]
        /// numbers them.
        pub fn labels(&self) -> Vec<String> {
            let on = |b: bool| if b { "ON" } else { "OFF" };
            vec![
                format!("Face: {}", on(self.enabled)),
                format!("Blink every: {:.1} s", self.blink_interval),
                format!("Eyes follow: {}", on(self.look_tracking)),
                format!("Reactions: {:.0}%", self.reaction_intensity * 100.0),
                format!("Personality: {}", self.personality.label()),
            ]
        }

        /// A click on button `i` of [`FaceSettings::labels`]; an index past the
        /// section does nothing.
        pub fn click(&mut self, i: usize) {
            match i {
                0 => self.enabled = !self.enabled,
                1 => self.blink_interval = cycle(&BLINK_STEPS, self.blink_interval),
                2 => self.look_tracking = !self.look_tracking,
                3 => self.reaction_intensity = cycle(&REACTION_STEPS, self.reaction_intensity),
                4 => self.personality = self.personality.next(),
                _ => {}
            }
        }

        /// What to store, key by key.
        pub fn stored(&self) -> [(&'static str, String); BUTTONS] {
            [
                (
                    "enabled",
                    if self.enabled { "true" } else { "false" }.to_owned(),
                ),
                ("blink_interval", format!("{:.1}", self.blink_interval)),
                (
                    "look_tracking",
                    if self.look_tracking { "true" } else { "false" }.to_owned(),
                ),
                (
                    "reaction_intensity",
                    format!("{:.2}", self.reaction_intensity),
                ),
                ("personality", self.personality.label().to_owned()),
            ]
        }

        /// The strongest alpha a reaction may show at, `0.0..=1.0`.
        pub fn reaction_alpha(&self) -> f32 {
            self.reaction_intensity.clamp(0.0, 1.0)
        }

        /// The mean seconds between blinks, scaled by the personality: a grumpy
        /// face blinks sooner, a cheerful one later.
        pub fn blink_seconds(&self) -> f32 {
            let scale = match self.personality {
                Personality::Grumpy => 0.8,
                Personality::Cheerful => 1.15,
                Personality::Neutral | Personality::Shy => 1.0,
            };
            (self.blink_interval * scale).max(0.5)
        }
    }

    /// The next value of a step table; a value between steps counts as the first.
    fn cycle(steps: &[f32], value: f32) -> f32 {
        let at = steps
            .iter()
            .position(|step| (step - value).abs() < 1e-4)
            .unwrap_or(0);
        steps[(at + 1) % steps.len()]
    }

    // ---------------------------------------------------------------------------
    // Expressions and cues
    // ---------------------------------------------------------------------------

    /// One face the rig can wear.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum FaceExpression {
        /// The resting face.
        #[default]
        Idle,
        /// Eyes shut for a moment.
        Blink,
        /// A flinch: squeezed eyes, a raised brow, a small mark.
        Hurt,
        /// A smile.
        Happy,
        /// Wide eyes and open mouth.
        Surprised,
        /// Knitted brows and a flat mouth.
        Angry,
        /// Heavy lids and a drooping mouth.
        Sleepy,
        /// Water in the eyes while swimming.
        Swim,
        /// Hair and eyes streaming while gliding.
        Fly,
        /// A face an emote asked for.
        Emote,
    }

    impl FaceExpression {
        /// The built-in cue for this expression.
        pub fn cue(self) -> FaceCue {
            FaceCue::built_in(self)
        }
    }

    /// What one expression does to the face, and for how long.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FaceCue {
        /// The expression it plays.
        pub expression: FaceExpression,
        /// How long it lasts, seconds; `0.0` is a resting face that never times
        /// out.
        pub seconds: f32,
        /// The higher the priority, the sooner it wins over another cue.
        pub priority: u8,
        /// How far the lids close, `0.0..=1.0`, added to the blink.
        pub lid: f32,
        /// The brow's angle in radians: positive raises the inner ends.
        pub brow: f32,
        /// The mouth's openness, `0.0..=1.0`.
        pub mouth: f32,
        /// How far the eyes widen, `0.0..=1.0`.
        pub widen: f32,
        /// The extra effect drawn over the face, as a cell of the effect sheet.
        pub fx: Option<FaceUv>,
        /// The mark drawn on a cheek, as a cell of the mark sheet.
        pub mark: Option<FaceUv>,
    }

    impl FaceCue {
        /// The plain resting face.
        pub const IDLE: FaceCue = FaceCue {
            expression: FaceExpression::Idle,
            seconds: 0.0,
            priority: 0,
            lid: 0.0,
            brow: 0.0,
            mouth: 0.0,
            widen: 0.0,
            fx: None,
            mark: None,
        };

        /// The built-in cue of an expression.
        pub fn built_in(expression: FaceExpression) -> FaceCue {
            let base = FaceCue {
                expression,
                ..FaceCue::IDLE
            };
            match expression {
                FaceExpression::Idle => base,
                FaceExpression::Blink => FaceCue {
                    seconds: 0.18,
                    priority: 3,
                    lid: 1.0,
                    ..base
                },
                FaceExpression::Hurt => FaceCue {
                    seconds: 0.55,
                    priority: 6,
                    lid: 0.55,
                    brow: -0.22,
                    mouth: 0.35,
                    widen: 0.2,
                    mark: Some(MARK_SWEAT),
                    ..base
                },
                FaceExpression::Happy => FaceCue {
                    seconds: 2.0,
                    priority: 2,
                    lid: 0.25,
                    brow: 0.10,
                    mouth: 0.45,
                    ..base
                },
                FaceExpression::Surprised => FaceCue {
                    seconds: 1.2,
                    priority: 4,
                    lid: 0.0,
                    brow: 0.30,
                    mouth: 0.8,
                    widen: 1.0,
                    fx: Some(FX_IRIS),
                    ..base
                },
                FaceExpression::Angry => FaceCue {
                    seconds: 1.6,
                    priority: 4,
                    lid: 0.35,
                    brow: -0.32,
                    mouth: 0.15,
                    mark: Some(MARK_VEIN),
                    ..base
                },
                FaceExpression::Sleepy => FaceCue {
                    seconds: 3.0,
                    priority: 1,
                    lid: 0.65,
                    brow: -0.08,
                    mouth: 0.10,
                    fx: Some(FX_LID),
                    ..base
                },
                FaceExpression::Swim => FaceCue {
                    seconds: 0.0,
                    priority: 2,
                    lid: 0.30,
                    mouth: 0.20,
                    mark: Some(MARK_TEAR),
                    ..base
                },
                FaceExpression::Fly => FaceCue {
                    seconds: 0.0,
                    priority: 2,
                    lid: 0.45,
                    brow: 0.16,
                    mouth: 0.25,
                    fx: Some(FX_LID),
                    ..base
                },
                FaceExpression::Emote => FaceCue {
                    seconds: 1.5,
                    priority: 5,
                    mouth: 0.4,
                    ..base
                },
            }
        }

        /// Whether this cue is a resting face that never runs out.
        pub fn is_resting(self) -> bool {
            self.seconds <= 0.0
        }
    }

    /// The cue table: the built-in faces plus any emote's own.
    #[derive(Debug, Clone, Default)]
    pub struct FaceLibrary {
        /// Named cues in the order they were added (emote names first).
        cues: Vec<(String, FaceCue)>,
    }

    impl FaceLibrary {
        /// The built-in table: one cue per expression, plus the emotes Punchy
        /// Moves knows by name.
        pub fn built_in() -> Self {
            let mut library = FaceLibrary::default();
            for (name, expression) in [
                ("idle", FaceExpression::Idle),
                ("blink", FaceExpression::Blink),
                ("hurt", FaceExpression::Hurt),
                ("happy", FaceExpression::Happy),
                ("surprised", FaceExpression::Surprised),
                ("angry", FaceExpression::Angry),
                ("sleepy", FaceExpression::Sleepy),
                ("swim", FaceExpression::Swim),
                ("fly", FaceExpression::Fly),
            ] {
                library.insert(name, FaceCue::built_in(expression));
            }
            // Emote-driven cues: an emote's name picks its face.
            for (name, expression, mouth, fx) in [
                ("wave", FaceExpression::Emote, 0.35, None),
                ("clap", FaceExpression::Happy, 0.55, None),
                ("dance", FaceExpression::Happy, 0.45, Some(FX_MOUTH)),
                ("bow", FaceExpression::Emote, 0.20, None),
                ("cry", FaceExpression::Hurt, 0.60, Some(MARK_TEAR)),
                ("laugh", FaceExpression::Happy, 0.75, Some(FX_MOUTH)),
            ] {
                library.insert(
                    name,
                    FaceCue {
                        mouth,
                        fx,
                        seconds: 2.4,
                        priority: 5,
                        ..FaceCue::built_in(expression)
                    },
                );
            }
            library
        }

        /// The cue with this name, if the table has one.
        pub fn get(&self, name: &str) -> Option<FaceCue> {
            self.cues
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, cue)| *cue)
        }

        /// The cue for an expression: the table's own if it names one, the
        /// built-in otherwise.
        pub fn cue(&self, expression: FaceExpression) -> FaceCue {
            let name = match expression {
                FaceExpression::Idle => "idle",
                FaceExpression::Blink => "blink",
                FaceExpression::Hurt => "hurt",
                FaceExpression::Happy => "happy",
                FaceExpression::Surprised => "surprised",
                FaceExpression::Angry => "angry",
                FaceExpression::Sleepy => "sleepy",
                FaceExpression::Swim => "swim",
                FaceExpression::Fly => "fly",
                FaceExpression::Emote => "wave",
            };
            self.get(name)
                .unwrap_or_else(|| FaceCue::built_in(expression))
        }

        /// Adds or replaces a named cue.
        pub fn insert(&mut self, name: &str, cue: FaceCue) {
            match self.cues.iter_mut().find(|(key, _)| key == name) {
                Some((_, stored)) => *stored = cue,
                None => self.cues.push((name.to_string(), cue)),
            }
        }

        /// The cue names, in the order they were added.
        pub fn names(&self) -> impl Iterator<Item = &str> {
            self.cues.iter().map(|(name, _)| name.as_str())
        }

        /// How many cues the table holds.
        pub fn len(&self) -> usize {
            self.cues.len()
        }

        /// Whether the table is empty.
        pub fn is_empty(&self) -> bool {
            self.cues.is_empty()
        }
    }

    // ---------------------------------------------------------------------------
    // The quads the head carries
    // ---------------------------------------------------------------------------

    /// One quad of a face on the front of a head, in head-local blocks: `x` to the
    /// model's right, `y` up, `z` out through the face.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FaceQuad {
        /// The face texture it shows.
        pub texture: &'static str,
        /// Its centre in head-local blocks.
        pub center: [f32; 3],
        /// Its width and height in head-local blocks.
        pub size: [f32; 2],
        /// Its roll about the face normal, radians.
        pub roll: f32,
        /// The rectangle of the texture it shows.
        pub uv: FaceUv,
        /// The colour and opacity it multiplies in, each `0.0..=1.0`.
        pub color: [f32; 4],
    }

    impl FaceQuad {
        /// A quad that draws nothing: the empty slot of a [`FaceQuads`].
        pub const EMPTY: FaceQuad = FaceQuad {
            texture: "",
            center: [0.0; 3],
            size: [0.0; 2],
            roll: 0.0,
            uv: FaceUv::FULL,
            color: [0.0; 4],
        };

        /// The quad's four corners in head-local space, anticlockwise from the
        /// bottom left.
        pub fn corners(&self) -> [[f32; 3]; 4] {
            let half_width = self.size[0] * 0.5;
            let half_height = self.size[1] * 0.5;
            let (sin, cos) = self.roll.sin_cos();
            let corner = |x: f32, y: f32| {
                [
                    self.center[0] + x * cos - y * sin,
                    self.center[1] + x * sin + y * cos,
                    self.center[2],
                ]
            };
            [
                corner(-half_width, -half_height),
                corner(half_width, -half_height),
                corner(half_width, half_height),
                corner(-half_width, half_height),
            ]
        }

        /// The texture coordinates of [`FaceQuad::corners`], in the same order.
        pub fn uv_corners(&self) -> [[f32; 2]; 4] {
            [
                [self.uv.u0, self.uv.v1],
                [self.uv.u1, self.uv.v1],
                [self.uv.u1, self.uv.v0],
                [self.uv.u0, self.uv.v0],
            ]
        }

        /// Whether the quad has a size and is not see-through.
        pub fn is_visible(&self) -> bool {
            self.size[0] > 0.0 && self.size[1] > 0.0 && self.color[3] > 0.0
        }
    }

    /// The most quads one face draws: two eyes, two lids, two brows, a mouth, an
    /// effect and a mark.
    pub const MAX_FACE_QUADS: usize = 10;

    /// The quads a rig draws this frame, at most [`MAX_FACE_QUADS`].
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FaceQuads {
        quads: [FaceQuad; MAX_FACE_QUADS],
        len: usize,
    }

    impl Default for FaceQuads {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FaceQuads {
        /// No quads.
        pub fn new() -> Self {
            Self {
                quads: [FaceQuad::EMPTY; MAX_FACE_QUADS],
                len: 0,
            }
        }

        /// Adds a quad; `false` when the list is already full.
        pub fn push(&mut self, quad: FaceQuad) -> bool {
            if self.len >= MAX_FACE_QUADS {
                return false;
            }
            self.quads[self.len] = quad;
            self.len += 1;
            true
        }

        /// The quads, in draw order.
        pub fn as_slice(&self) -> &[FaceQuad] {
            &self.quads[..self.len]
        }

        /// The quads, in draw order.
        pub fn iter(&self) -> impl Iterator<Item = &FaceQuad> {
            self.as_slice().iter()
        }

        /// How many quads the face drew.
        pub fn len(&self) -> usize {
            self.len
        }

        /// Whether the face drew nothing.
        pub fn is_empty(&self) -> bool {
            self.len == 0
        }
    }

    // ---------------------------------------------------------------------------
    // FaceRig
    // ---------------------------------------------------------------------------

    /// How long one blink takes, seconds.
    pub const BLINK_SECONDS: f32 = 0.18;
    /// How fast the eyes swing towards where they are looking, per second.
    const LOOK_RATE: f32 = 9.0;
    /// How fast they come back to centre when tracking is off, per second.
    const LOOK_RELEASE: f32 = 4.0;
    /// How fast a reaction fades in and out, per second.
    const REACTION_RATE: f32 = 7.0;
    /// The eyes' centres, `x` from the middle of the face, in head-local blocks.
    const EYE_X: f32 = 0.125;
    /// The eyes' centre height.
    const EYE_Y: f32 = 0.30;
    /// The eyes' resting size.
    const EYE_SIZE: [f32; 2] = [0.11, 0.05];
    /// How far in front of the head's centre the face sits.
    const FACE_Z: f32 = 0.2510;
    /// The mouth's centre height.
    const MOUTH_Y: f32 = 0.215;
    /// How far the eyes may slide with the look, in blocks.
    const LOOK_SHIFT: [f32; 2] = [0.020, 0.014];

    /// One player's face: the blink timer, the eyes' look, the reaction and the
    /// cue being worn.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FaceRig {
        /// The blink timer's state.
        seed: u32,
        blink_in: f32,
        blinking: bool,
        blink_progress: f32,
        blink: f32,
        /// The eyes' look, `-1.0..=1.0` across and up.
        look: [f32; 2],
        /// How strong the reaction shows, `0.0..=1.0`.
        reaction: f32,
        /// The cue being worn.
        cue: FaceCue,
        /// Seconds left of an explicit cue; `None` when none was asked for.
        held: Option<f32>,
    }

    impl Default for FaceRig {
        fn default() -> Self {
            Self::new(0)
        }
    }

    impl FaceRig {
        /// A rig with its own blink timer seed, so two faces never blink together.
        pub fn new(seed: u32) -> Self {
            let mut rig = Self {
                seed: seed | 1,
                blink_in: 0.0,
                blinking: false,
                blink_progress: 0.0,
                blink: 0.0,
                look: [0.0, 0.0],
                reaction: 0.0,
                cue: FaceCue::IDLE,
                held: None,
            };
            rig.blink_in = rig.random01() * 5.0;
            rig
        }

        /// How far the lids are closed right now, `0.0..=1.0`.
        pub fn blink(&self) -> f32 {
            self.blink
        }

        /// Seconds until the next blink.
        pub fn blink_timer(&self) -> f32 {
            self.blink_in
        }

        /// Sets the next blink to `seconds` from now (a scene, a test).
        pub fn set_blink_in(&mut self, seconds: f32) {
            self.blink_in = if seconds.is_finite() {
                seconds.max(0.0)
            } else {
                0.0
            };
        }

        /// The eyes' look, `-1.0..=1.0` across and up.
        pub fn look(&self) -> [f32; 2] {
            self.look
        }

        /// How strong the reaction shows, `0.0..=1.0`.
        pub fn reaction(&self) -> f32 {
            self.reaction
        }

        /// The cue being worn.
        pub fn cue(&self) -> FaceCue {
            self.cue
        }

        /// The expression being worn.
        pub fn expression(&self) -> FaceExpression {
            self.cue.expression
        }

        /// Whether an explicit cue is still playing.
        pub fn is_playing(&self) -> bool {
            self.held.is_some()
        }

        /// Wears `expression` for as long as its cue lasts, whatever the signals
        /// say in the meantime.
        pub fn play(&mut self, expression: FaceExpression) {
            self.play_cue(expression.cue());
        }

        /// Wears `cue` for as long as it lasts.
        pub fn play_cue(&mut self, cue: FaceCue) {
            self.cue = cue;
            self.held = (!cue.is_resting()).then_some(cue.seconds);
        }

        /// Back to a resting face with the eyes centred.
        pub fn reset(&mut self) {
            let seed = self.seed;
            *self = Self::new(seed);
        }

        /// One frame of the face.
        pub fn update(&mut self, signals: &MotionSignals, settings: &FaceSettings, dt: f32) {
            let dt = if dt.is_finite() {
                dt.clamp(0.0, 0.25)
            } else {
                0.0
            };
            if !settings.enabled {
                self.blink = 0.0;
                self.blinking = false;
                self.look = [0.0, 0.0];
                self.reaction = 0.0;
                self.cue = FaceCue::IDLE;
                self.held = None;
                return;
            }

            // The blink: a full close and open inside `BLINK_SECONDS`, then a
            // fresh interval, jittered so a crowd never blinks in step.
            self.blink_in -= dt;
            if !self.blinking && self.blink_in <= 0.0 {
                self.blinking = true;
                self.blink_progress = 0.0;
            }
            if self.blinking {
                self.blink_progress = (self.blink_progress + dt / BLINK_SECONDS).min(1.0);
                if self.blink_progress >= 1.0 {
                    self.blinking = false;
                    self.blink_progress = 0.0;
                    self.blink_in = self.next_blink(settings);
                }
            }
            self.blink = if self.blinking {
                (std::f32::consts::PI * self.blink_progress).sin()
            } else {
                0.0
            };

            // The eyes: on the walk, up a ladder, up while swimming, down while
            // mining; centred when tracking is off.
            let target = if !settings.look_tracking {
                [0.0, 0.0]
            } else {
                look_target(signals)
            };
            let rate = if settings.look_tracking {
                LOOK_RATE
            } else {
                LOOK_RELEASE
            };
            self.look[0] = approach(self.look[0], target[0], rate * dt);
            self.look[1] = approach(self.look[1], target[1], rate * dt);

            // The reaction: a fall or a charge shows, scaled by the setting.
            let target = reaction_target(signals) * settings.reaction_alpha();
            self.reaction = approach(self.reaction, target, REACTION_RATE * dt);

            // The cue: an explicit one runs out, then the signals pick again.
            match self.held {
                Some(left) if left > dt => self.held = Some(left - dt),
                Some(_) => self.held = None,
                None => {}
            }
            if self.held.is_none() {
                self.cue = pick_cue(signals, settings);
            }
        }

        /// The seconds until the next blink, jittered around the setting.
        fn next_blink(&mut self, settings: &FaceSettings) -> f32 {
            (settings.blink_seconds() * (0.7 + 0.6 * self.random01())).max(0.5)
        }

        /// One deterministic draw in `0.0..1.0` (xorshift32).
        fn random01(&mut self) -> f32 {
            let mut x = self.seed;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.seed = x;
            (x >> 8) as f32 / 16_777_216.0
        }

        /// The quads this face draws this frame, in head-local space.
        pub fn quads(&self, settings: &FaceSettings) -> FaceQuads {
            let mut quads = FaceQuads::new();
            if !settings.enabled {
                return quads;
            }
            let alpha = if settings.reaction_alpha() > 0.0 {
                (0.5 + 0.5 * self.reaction) * settings.reaction_alpha()
            } else {
                0.5
            };
            let lid = self.cue.lid.max(self.blink).clamp(0.0, 1.0);
            let widen = self.cue.widen.clamp(0.0, 1.0);
            let size = [
                EYE_SIZE[0] * (1.0 + 0.35 * widen),
                EYE_SIZE[1] * (1.0 + 0.50 * widen),
            ];
            let shift = [self.look[0] * LOOK_SHIFT[0], self.look[1] * LOOK_SHIFT[1]];

            // The two eyes: the iris cell of the effect sheet, slid with the look.
            for side in [-1.0, 1.0] {
                quads.push(FaceQuad {
                    texture: FX_TEXTURE,
                    center: [side * EYE_X + shift[0], EYE_Y + shift[1], FACE_Z],
                    size,
                    roll: 0.0,
                    uv: FX_IRIS,
                    color: [1.0, 1.0, 1.0, alpha],
                });
            }

            // The lids: the closed share of each eye, hanging from its top.
            if lid > 0.001 {
                let height = size[1] * lid;
                let center_y = EYE_Y + size[1] * 0.5 - height * 0.5 + shift[1];
                for side in [-1.0, 1.0] {
                    quads.push(FaceQuad {
                        texture: FX_TEXTURE,
                        center: [side * EYE_X + shift[0], center_y, FACE_Z + 0.0004],
                        size: [size[0], height],
                        roll: 0.0,
                        uv: FX_LID,
                        color: [0.06, 0.06, 0.09, alpha],
                    });
                }
            }

            // The brows: the vertical band cell, tilted by the cue's mood.
            let brow = self.cue.brow.clamp(-0.6, 0.6);
            for side in [-1.0, 1.0] {
                quads.push(FaceQuad {
                    texture: FX_TEXTURE,
                    center: [side * EYE_X, EYE_Y + size[1] * 0.5 + 0.035, FACE_Z + 0.0002],
                    size: [0.10, 0.018],
                    roll: side * brow,
                    uv: FX_BROW,
                    color: [0.16, 0.11, 0.09, alpha],
                });
            }

            // The mouth: the streak cell, opened and widened by the cue.
            let mouth = self.cue.mouth.clamp(0.0, 1.0);
            quads.push(FaceQuad {
                texture: FX_TEXTURE,
                center: [0.0, MOUTH_Y, FACE_Z + 0.0002],
                size: [0.11 * (1.0 + 0.30 * mouth), 0.012 + 0.05 * mouth],
                roll: 0.0,
                uv: FX_MOUTH,
                color: [0.38, 0.13, 0.13, alpha],
            });

            // The expression's effect and its cheek mark, when it has them.
            if let Some(fx) = self.cue.fx {
                quads.push(FaceQuad {
                    texture: FX_TEXTURE,
                    center: [0.0, EYE_Y, FACE_Z - 0.0006],
                    size: [0.34, 0.34],
                    roll: 0.0,
                    uv: fx,
                    color: [1.0, 1.0, 1.0, 0.45 * alpha],
                });
            }
            if let Some(mark) = self.cue.mark {
                let blush = settings.personality.blush();
                quads.push(FaceQuad {
                    texture: MARKS_TEXTURE,
                    center: [0.145, EYE_Y - 0.02, FACE_Z + 0.0006],
                    size: [0.085, 0.085],
                    roll: 0.0,
                    uv: mark,
                    color: [1.0, 1.0, 1.0, (0.75 * alpha * blush).clamp(0.0, 1.0)],
                });
            }
            quads
        }
    }

    /// Where the eyes look: across the walk, up a ladder and in water, down while
    /// mining.
    fn look_target(signals: &MotionSignals) -> [f32; 2] {
        if signals.elytra || signals.surface_swimming || signals.submerged_swimming {
            [0.0, signals.swim_vertical as f32]
        } else if signals.mining {
            [0.0, -0.8]
        } else if signals.climbing {
            [0.0, signals.climb_direction as f32]
        } else if signals.crouching {
            [0.0, -0.35]
        } else {
            [
                signals.walk_left.clamp(-1.0, 1.0) * 0.8,
                -signals.walk_forward.clamp(-1.0, 1.0) * 0.5,
            ]
        }
    }

    /// How strong a reaction the signals ask for, before the setting scales it.
    fn reaction_target(signals: &MotionSignals) -> f32 {
        if signals.fall_weight >= super::signals::LIGHT {
            1.0
        } else if signals.charging_crossbow || signals.charging_spear || signals.charging_trident {
            0.6
        } else if signals.elytra && signals.elytra_speed > 1.0 {
            0.4
        } else if signals.mining {
            0.25
        } else {
            0.0
        }
    }

    /// The cue the movement state wears when nothing explicit is playing: the
    /// highest-priority match, the resting face when none.
    fn pick_cue(signals: &MotionSignals, settings: &FaceSettings) -> FaceCue {
        let cheer = settings.personality.cheer();
        let grumble = settings.personality.grumble();
        let mut best = FaceCue::IDLE;
        let mut candidate = |cue: FaceCue| {
            if cue.priority > best.priority {
                best = cue;
            }
        };
        if signals.fall_weight >= super::signals::LIGHT {
            candidate(FaceExpression::Hurt.cue());
        }
        if signals.mining || (signals.crouching && signals.moving && grumble > 1.0) {
            candidate(FaceExpression::Angry.cue());
        }
        if signals.fast_flight || (signals.charging_spear && signals.running) {
            candidate(FaceExpression::Surprised.cue());
        }
        if signals.running || (signals.moving && !signals.grounded && cheer > 1.0) {
            candidate(FaceExpression::Happy.cue());
        }
        if signals.elytra {
            candidate(FaceExpression::Fly.cue());
        }
        if signals.surface_swimming || signals.submerged_swimming {
            candidate(FaceExpression::Swim.cue());
        }
        if signals.crouching && !signals.moving {
            candidate(FaceExpression::Sleepy.cue());
        }
        best
    }

    // ---------------------------------------------------------------------------
    // The settings the game's hooks read
    // ---------------------------------------------------------------------------

    thread_local! {
        /// The face settings, read once from the mod's stored values.
        static SETTINGS: RefCell<Option<FaceSettings>> = const { RefCell::new(None) };
    }

    /// The settings, read once through `get`.
    pub fn settings(get: impl Fn(&str) -> Option<String>) -> FaceSettings {
        SETTINGS.with_borrow_mut(|s| *s.get_or_insert_with(|| FaceSettings::read(get)))
    }

    /// Replaces the settings (the settings page or a key changed them).
    pub fn set_settings(settings: FaceSettings) {
        SETTINGS.with_borrow_mut(|s| *s = Some(settings));
    }

    /// The settings as last read, or the defaults when nothing has read them yet.
    pub fn current_settings() -> FaceSettings {
        SETTINGS.with_borrow(|s| s.unwrap_or_default())
    }

    /// Forgets the settings (a mod reload reads them again).
    pub fn reset_settings() {
        SETTINGS.with_borrow_mut(|s| *s = None);
    }

    /// The rigs of every player, keyed by entity id: the game's head overlay
    /// reads the rig of the head it is drawing.
    #[derive(Debug, Clone, Default)]
    pub struct FaceRigs {
        rigs: HashMap<u64, FaceRig>,
    }

    impl FaceRigs {
        pub fn new() -> Self {
            Self::default()
        }

        /// The rig of `entity`, made (and seeded by its id) on first use.
        pub fn rig_mut(&mut self, entity: u64) -> &mut FaceRig {
            self.rigs
                .entry(entity)
                .or_insert_with(|| FaceRig::new(entity as u32))
        }

        /// The rig of `entity`, if it has one.
        pub fn rig(&self, entity: u64) -> Option<&FaceRig> {
            self.rigs.get(&entity)
        }

        /// Steps `entity`'s face by one frame.
        pub fn update(
            &mut self,
            entity: u64,
            signals: &MotionSignals,
            settings: &FaceSettings,
            dt: f32,
        ) {
            self.rig_mut(entity).update(signals, settings, dt);
        }

        /// `entity`'s face quads, or `None` when it has no rig.
        pub fn quads(&self, entity: u64, settings: &FaceSettings) -> Option<FaceQuads> {
            self.rig(entity).map(|rig| rig.quads(settings))
        }

        /// Forgets `entity`; whether it was there.
        pub fn remove(&mut self, entity: u64) -> bool {
            self.rigs.remove(&entity).is_some()
        }

        /// Forgets every face.
        pub fn clear(&mut self) {
            self.rigs.clear();
        }

        /// How many faces are tracked.
        pub fn len(&self) -> usize {
            self.rigs.len()
        }

        /// Whether no face is tracked.
        pub fn is_empty(&self) -> bool {
            self.rigs.is_empty()
        }
    }

    /// `a` moved towards `b` by at most `step`.
    fn approach(a: f32, b: f32, step: f32) -> f32 {
        if !a.is_finite() {
            return b;
        }
        let step = if step.is_finite() { step.max(0.0) } else { 0.0 };
        let gap = b - a;
        if gap.abs() <= step {
            b
        } else if gap > 0.0 {
            a + step
        } else {
            a - step
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn walking() -> MotionSignals {
            MotionSignals {
                moving: true,
                walk_forward: 1.0,
                grounded: true,
                ..MotionSignals::new()
            }
        }

        fn stored(values: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
            let values: Vec<(String, String)> = values
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect();
            move |key| {
                values
                    .iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| value.clone())
            }
        }

        #[test]
        fn uv_cells_tile_the_sheets() {
            let whole = FaceUv::cell(1, 1, 0, 0);
            assert_eq!(whole, FaceUv::FULL);
            assert!(whole.is_valid());

            // A 2x2 sheet's four cells cover it without overlapping.
            let mut cells = Vec::new();
            for row in 0..2 {
                for column in 0..2 {
                    let cell = FaceUv::cell(2, 2, column, row);
                    assert!(cell.is_valid());
                    assert_eq!(cell.size(), [0.5, 0.5]);
                    assert!(!cells.contains(&cell));
                    cells.push(cell);
                }
            }
            let area: f32 = cells.iter().map(|c| c.size()[0] * c.size()[1]).sum();
            assert!((area - 1.0).abs() < 1e-6);
            // A cell past the last is clamped into the sheet, never outside it.
            assert_eq!(FaceUv::cell(2, 2, 9, 9), FaceUv::cell(2, 2, 1, 1));
            assert_eq!(FaceUv::cell(0, 0, 0, 0), FaceUv::FULL);

            // The rig's own regions match the sheets it ships.
            assert_eq!(FX_IRIS.px(FX_SIZE, FX_SIZE), [0.0, 0.0, 128.0, 128.0]);
            assert_eq!(FX_LID.px(FX_SIZE, FX_SIZE), [128.0, 0.0, 256.0, 128.0]);
            assert_eq!(FX_BROW.px(FX_SIZE, FX_SIZE), [0.0, 128.0, 128.0, 256.0]);
            assert_eq!(FX_MOUTH.px(FX_SIZE, FX_SIZE), [128.0, 128.0, 256.0, 256.0]);
            assert_eq!(MARK_BLUSH.px(MARKS_SIZE, MARKS_SIZE), [0.0, 0.0, 2.0, 2.0]);
            assert_eq!(MARK_VEIN.px(MARKS_SIZE, MARKS_SIZE), [2.0, 2.0, 4.0, 4.0]);
            for uv in [
                FX_IRIS, FX_LID, FX_BROW, FX_MOUTH, MARK_BLUSH, MARK_SWEAT, MARK_TEAR, MARK_VEIN,
            ] {
                assert!(uv.is_valid(), "{:?} is not inside its sheet", uv);
            }

            // A quad's corner lands inside the sprite it shows.
            assert_eq!(FX_LID.at(0.0, 0.0), [0.5, 0.0]);
            assert_eq!(FX_LID.at(1.0, 1.0), [1.0, 0.5]);
        }

        #[test]
        fn the_library_names_every_expression_and_emote() {
            let library = FaceLibrary::built_in();
            for expression in [
                FaceExpression::Idle,
                FaceExpression::Blink,
                FaceExpression::Hurt,
                FaceExpression::Happy,
                FaceExpression::Surprised,
                FaceExpression::Angry,
                FaceExpression::Sleepy,
                FaceExpression::Swim,
                FaceExpression::Fly,
                FaceExpression::Emote,
            ] {
                let cue = library.cue(expression);
                assert!(cue.priority < 10);
            }
            // The named cues are all there, emotes included.
            for name in [
                "idle",
                "blink",
                "hurt",
                "happy",
                "surprised",
                "angry",
                "sleepy",
                "swim",
                "fly",
                "wave",
                "clap",
                "dance",
                "bow",
                "cry",
                "laugh",
            ] {
                assert!(library.get(name).is_some(), "no cue named {}", name);
            }
            assert_eq!(library.len(), 15);
            assert_eq!(library.names().count(), 15);
            assert!(library.get("no_such_face").is_none());

            // A replaced cue wins; an unknown expression still has a face.
            let mut library = library;
            library.insert(
                "happy",
                FaceCue {
                    mouth: 0.9,
                    ..FaceCue::built_in(FaceExpression::Happy)
                },
            );
            assert_eq!(library.get("happy").unwrap().mouth, 0.9);
            assert_eq!(library.cue(FaceExpression::Happy).mouth, 0.9);
            assert_eq!(
                FaceLibrary::default().cue(FaceExpression::Angry),
                FaceCue::built_in(FaceExpression::Angry)
            );
        }

        #[test]
        fn cues_only_rest_when_they_never_time_out() {
            assert!(FaceCue::IDLE.is_resting());
            assert!(FaceExpression::Idle.cue().is_resting());
            assert!(FaceExpression::Swim.cue().is_resting());
            assert!(FaceExpression::Fly.cue().is_resting());
            assert!(!FaceExpression::Hurt.cue().is_resting());
            assert!(!FaceExpression::Happy.cue().is_resting());
            // The hurt face squeezes, frowns and marks the cheek.
            let hurt = FaceExpression::Hurt.cue();
            assert!(hurt.lid > 0.0 && hurt.mouth > 0.0);
            assert!(hurt.mark.is_some());
            // The surprised face widens the eyes and opens the mouth widest.
            let surprised = FaceExpression::Surprised.cue();
            assert_eq!(surprised.widen, 1.0);
            assert!(surprised.mouth > FaceExpression::Happy.cue().mouth);
        }

        #[test]
        fn face_settings_read_keep_and_store() {
            let defaults = FaceSettings::default();
            assert_eq!(FaceSettings::read(|_| None), defaults);

            let read = FaceSettings::read(stored(&[
                ("enabled", "false"),
                ("blink_interval", "7"),
                ("look_tracking", "false"),
                ("reaction_intensity", "0.5"),
                ("personality", "grumpy"),
            ]));
            assert!(!read.enabled);
            assert_eq!(read.blink_interval, 7.0);
            assert!(!read.look_tracking);
            assert_eq!(read.reaction_intensity, 0.5);
            assert_eq!(read.personality, Personality::Grumpy);

            // Out-of-range and broken values clamp or fall back, and the labels
            // cover every button.
            let clamped = FaceSettings::read(stored(&[
                ("blink_interval", "0"),
                ("reaction_intensity", "9"),
                ("personality", "sulky"),
            ]));
            assert_eq!(clamped.blink_interval, 0.5);
            assert_eq!(clamped.reaction_intensity, 2.0);
            assert_eq!(clamped.personality, Personality::Neutral);
            assert_eq!(clamped.reaction_alpha(), 1.0);
            assert_eq!(clamped.labels().len(), BUTTONS);
            assert_eq!(clamped.stored().len(), BUTTONS);

            // The settings round-trip through what they store.
            let mut settings = defaults;
            settings.click(1);
            settings.click(3);
            settings.click(4);
            let pairs: Vec<(&str, String)> = settings.stored().to_vec();
            let values: Vec<(&str, &str)> = pairs
                .iter()
                .map(|(key, value)| (*key, value.as_str()))
                .collect();
            assert_eq!(FaceSettings::read(stored(&values)), settings);
        }

        #[test]
        fn clicks_cycle_every_button() {
            let mut settings = FaceSettings::default();
            let start = settings;
            for button in 0..BUTTONS {
                let before = settings;
                settings.click(button);
                assert_ne!(settings, before, "button {} changed nothing", button);
            }
            let before = settings;
            settings.click(BUTTONS);
            assert_eq!(settings, before);

            let mut cycled = start;
            for _ in 0..BLINK_STEPS.len() {
                cycled.click(1);
            }
            assert_eq!(cycled.blink_interval, start.blink_interval);
            assert!(BLINK_STEPS.contains(&cycled.blink_interval));

            // The personalities come round again.
            let mut personalities = start;
            for _ in 0..Personality::ALL.len() {
                personalities.click(4);
            }
            assert_eq!(personalities.personality, start.personality);
        }

        #[test]
        fn personalities_scale_the_mood() {
            assert_eq!(Personality::Neutral.next(), Personality::Cheerful);
            assert_eq!(Personality::Shy.next(), Personality::Neutral);
            assert!(Personality::Cheerful.cheer() > Personality::Neutral.cheer());
            assert!(Personality::Grumpy.grumble() > Personality::Neutral.grumble());
            assert!(Personality::Shy.blush() > Personality::Neutral.blush());
            for personality in Personality::ALL {
                assert!(!personality.label().is_empty());
            }

            // A grumpy face blinks sooner than a cheerful one.
            let grumpy = FaceSettings {
                personality: Personality::Grumpy,
                ..FaceSettings::default()
            };
            let cheerful = FaceSettings {
                personality: Personality::Cheerful,
                ..FaceSettings::default()
            };
            assert!(cheerful.blink_seconds() > grumpy.blink_seconds());
        }

        #[test]
        fn the_face_blinks_and_opens_again() {
            let settings = FaceSettings::default();
            let mut rig = FaceRig::new(7);
            let signals = MotionSignals::new();

            // Start a blink now: the lids close and open inside one blink.
            rig.set_blink_in(0.0);
            rig.update(&signals, &settings, 0.05);
            assert!(rig.blink() > 0.5, "blink was {}", rig.blink());
            for _ in 0..4 {
                rig.update(&signals, &settings, 0.05);
            }
            assert_eq!(rig.blink(), 0.0);
            // And a fresh interval, never shorter than the setting allows.
            assert!(rig.blink_timer() >= 0.5);

            // Turned off, the face stays open and nothing moves.
            let off = FaceSettings {
                enabled: false,
                ..FaceSettings::default()
            };
            rig.set_blink_in(0.0);
            rig.update(&walking(), &off, 0.1);
            assert_eq!(rig.blink(), 0.0);
            assert_eq!(rig.look(), [0.0, 0.0]);
            assert_eq!(rig.reaction(), 0.0);
            assert_eq!(rig.expression(), FaceExpression::Idle);
        }

        #[test]
        fn the_eyes_follow_the_body_and_let_go() {
            let settings = FaceSettings::default();
            let mut rig = FaceRig::new(3);
            rig.set_blink_in(100.0);

            let strafing = MotionSignals {
                moving: true,
                walk_left: 1.0,
                ..MotionSignals::new()
            };
            for _ in 0..5 {
                rig.update(&strafing, &settings, 0.1);
            }
            let look = rig.look();
            assert!(look[0] > 0.5, "the eyes did not follow: {:?}", look);
            assert!(look[1].abs() < 0.1);

            // Looking down while mining.
            let mining = MotionSignals {
                mining: true,
                ..MotionSignals::new()
            };
            for _ in 0..10 {
                rig.update(&mining, &settings, 0.1);
            }
            assert!(rig.look()[1] < -0.5);

            // Tracking off, the eyes centre again.
            let off = FaceSettings {
                look_tracking: false,
                ..FaceSettings::default()
            };
            for _ in 0..20 {
                rig.update(&mining, &off, 0.1);
            }
            assert_eq!(rig.look(), [0.0, 0.0]);
        }

        #[test]
        fn a_reaction_shows_and_fades() {
            let settings = FaceSettings::default();
            let mut rig = FaceRig::new(11);
            rig.set_blink_in(100.0);

            let falling = MotionSignals {
                fall_weight: super::super::signals::HEAVY,
                ..MotionSignals::new()
            };
            for _ in 0..10 {
                rig.update(&falling, &settings, 0.05);
            }
            assert!((rig.reaction() - 1.0).abs() < 1e-6);
            // The face shows the fall.
            assert_eq!(rig.expression(), FaceExpression::Hurt);

            // A half-strength setting halves it.
            let half = FaceSettings {
                reaction_intensity: 0.5,
                ..FaceSettings::default()
            };
            for _ in 0..10 {
                rig.update(&falling, &half, 0.05);
            }
            assert!((rig.reaction() - 0.5).abs() < 1e-6);

            // Calm again, it fades away.
            for _ in 0..40 {
                rig.update(&MotionSignals::new(), &half, 0.05);
            }
            assert!(rig.reaction() < 1e-6);
        }

        #[test]
        fn the_signals_pick_the_strongest_face() {
            let settings = FaceSettings::default();
            let mut rig = FaceRig::new(5);
            rig.set_blink_in(100.0);

            let cases = [
                (
                    MotionSignals {
                        fall_weight: super::super::signals::HEAVY,
                        running: true,
                        moving: true,
                        ..MotionSignals::new()
                    },
                    FaceExpression::Hurt,
                ),
                (
                    MotionSignals {
                        mining: true,
                        ..MotionSignals::new()
                    },
                    FaceExpression::Angry,
                ),
                (
                    MotionSignals {
                        fast_flight: true,
                        ..MotionSignals::new()
                    },
                    FaceExpression::Surprised,
                ),
                (
                    MotionSignals {
                        running: true,
                        moving: true,
                        ..MotionSignals::new()
                    },
                    FaceExpression::Happy,
                ),
                (
                    MotionSignals {
                        elytra: true,
                        ..MotionSignals::new()
                    },
                    FaceExpression::Fly,
                ),
                (
                    MotionSignals {
                        submerged_swimming: true,
                        ..MotionSignals::new()
                    },
                    FaceExpression::Swim,
                ),
                (
                    MotionSignals {
                        crouching: true,
                        ..MotionSignals::new()
                    },
                    FaceExpression::Sleepy,
                ),
                (MotionSignals::new(), FaceExpression::Idle),
            ];
            for (signals, expected) in cases {
                rig.update(&signals, &settings, 0.05);
                assert_eq!(rig.expression(), expected, "signals {:?}", signals);
            }
        }

        #[test]
        fn an_explicit_face_plays_and_lets_go() {
            let settings = FaceSettings::default();
            let mut rig = FaceRig::new(2);
            rig.set_blink_in(100.0);

            rig.play(FaceExpression::Happy);
            assert!(rig.is_playing());
            assert_eq!(rig.expression(), FaceExpression::Happy);
            // Even a fall leaves it alone while it lasts.
            let falling = MotionSignals {
                fall_weight: super::super::signals::HEAVY,
                ..MotionSignals::new()
            };
            rig.update(&falling, &settings, 0.1);
            assert_eq!(rig.expression(), FaceExpression::Happy);

            // Happy lasts two seconds; then the signals pick again.
            for _ in 0..25 {
                rig.update(&falling, &settings, 0.1);
            }
            assert!(!rig.is_playing());
            assert_eq!(rig.expression(), FaceExpression::Hurt);

            // A resting cue never holds.
            rig.play_cue(FaceCue::IDLE);
            assert!(!rig.is_playing());
            assert_eq!(rig.expression(), FaceExpression::Idle);

            rig.play(FaceExpression::Angry);
            rig.reset();
            assert!(!rig.is_playing());
            assert_eq!(rig.expression(), FaceExpression::Idle);
        }

        #[test]
        fn a_resting_face_draws_its_eyes_brows_and_mouth() {
            let settings = FaceSettings::default();
            let mut rig = FaceRig::new(9);
            rig.set_blink_in(100.0);
            rig.update(&MotionSignals::new(), &settings, 0.05);

            let quads = rig.quads(&settings);
            assert_eq!(quads.len(), 5, "two eyes, two brows, one mouth");
            for quad in quads.iter() {
                assert!(quad.is_visible());
                assert!(quad.uv.is_valid());
                assert!((0.0..=1.0).contains(&quad.color[3]));
                assert!((quad.center[2] - FACE_Z).abs() < 0.01);
            }
            // The eyes sit either side of the middle, at the same height.
            let eyes: Vec<&FaceQuad> = quads.iter().filter(|q| q.uv == FX_IRIS).collect();
            assert_eq!(eyes.len(), 2);
            assert!((eyes[0].center[0] + eyes[1].center[0]).abs() < 1e-6);
            assert_eq!(eyes[0].center[1], eyes[1].center[1]);
            // The mouth is below them.
            let mouth = quads.iter().find(|q| q.uv == FX_MOUTH).unwrap();
            assert!(mouth.center[1] < eyes[0].center[1]);

            // A blink adds one lid per eye, hung on their top halves.
            rig.set_blink_in(0.0);
            rig.update(&MotionSignals::new(), &settings, 0.04);
            let blinking = rig.quads(&settings);
            assert!(blinking.len() > quads.len());
            let lids: Vec<&FaceQuad> = blinking.iter().filter(|q| q.uv == FX_LID).collect();
            assert_eq!(lids.len(), 2);
            for lid in lids {
                assert!(lid.center[1] > eyes[0].center[1]);
            }

            // A reaction adds its mark, and no face draws with the rig off.
            rig.set_blink_in(100.0);
            rig.play(FaceExpression::Hurt);
            rig.update(&MotionSignals::new(), &settings, 0.05);
            let hurt = rig.quads(&settings);
            assert!(hurt.iter().any(|q| q.texture == MARKS_TEXTURE));
            let off = FaceSettings {
                enabled: false,
                ..FaceSettings::default()
            };
            assert!(rig.quads(&off).is_empty());
        }

        #[test]
        fn a_mouth_opens_and_a_quad_has_corners() {
            let settings = FaceSettings::default();
            let mut rig = FaceRig::new(13);
            rig.set_blink_in(100.0);
            rig.update(&MotionSignals::new(), &settings, 0.05);
            let closed = rig
                .quads(&settings)
                .iter()
                .find(|q| q.uv == FX_MOUTH)
                .copied()
                .unwrap();

            rig.play(FaceExpression::Surprised);
            rig.update(&MotionSignals::new(), &settings, 0.05);
            let quads = rig.quads(&settings);
            let open = quads.iter().find(|q| q.uv == FX_MOUTH).copied().unwrap();
            assert!(open.size[1] > closed.size[1]);
            // Surprised also widens the eyes and draws its glow.
            assert!(
                quads
                    .iter()
                    .any(|q| q.uv == FX_IRIS && q.size[0] > EYE_SIZE[0])
            );

            // A quad's corners are its rectangle, turned by its roll.
            let quad = FaceQuad {
                texture: FX_TEXTURE,
                center: [0.0, 0.0, 1.0],
                size: [0.2, 0.1],
                roll: std::f32::consts::FRAC_PI_2,
                uv: FX_IRIS,
                color: [1.0; 4],
            };
            let corners = quad.corners();
            // A quarter turn swaps the rectangle's sides.
            let width = corners[1][0] - corners[0][0];
            let height = corners[1][1] - corners[0][1];
            assert!((width * width + height * height - 0.2 * 0.2).abs() < 1e-6);
            // Every corner keeps the quad's depth.
            for corner in corners {
                assert_eq!(corner[2], 1.0);
            }
            // The UV corners start at the sprite's bottom left.
            assert_eq!(quad.uv_corners()[0], [FX_IRIS.u0, FX_IRIS.v1]);
            assert_eq!(quad.uv_corners()[2], [FX_IRIS.u1, FX_IRIS.v0]);
            // A see-through or sizeless quad draws nothing.
            assert!(
                !FaceQuad {
                    color: [1.0, 1.0, 1.0, 0.0],
                    ..quad
                }
                .is_visible()
            );
            assert!(
                !FaceQuad {
                    size: [0.0, 0.1],
                    ..quad
                }
                .is_visible()
            );
        }

        #[test]
        fn the_quad_list_holds_ten_and_no_more() {
            let mut quads = FaceQuads::new();
            assert!(quads.is_empty());
            for i in 0..MAX_FACE_QUADS {
                assert!(quads.push(FaceQuad {
                    center: [i as f32, 0.0, 0.0],
                    ..FaceQuad::EMPTY
                }));
            }
            assert_eq!(quads.len(), MAX_FACE_QUADS);
            assert!(!quads.push(FaceQuad::EMPTY));
            assert_eq!(quads.as_slice().len(), MAX_FACE_QUADS);
            assert_eq!(quads.iter().count(), MAX_FACE_QUADS);
        }

        #[test]
        fn the_rig_map_tracks_faces_per_player() {
            let settings = FaceSettings::default();
            let mut rigs = FaceRigs::new();
            assert!(rigs.is_empty());
            assert!(rigs.rig(4).is_none());

            // A face is made on first use, seeded by the entity so two players
            // never blink together.
            rigs.update(4, &MotionSignals::new(), &settings, 0.05);
            rigs.update(9, &MotionSignals::new(), &settings, 0.05);
            assert_eq!(rigs.len(), 2);
            assert!(rigs.rig(4).is_some());
            assert!(rigs.quads(4, &settings).is_some());
            assert!(rigs.quads(7, &settings).is_none());

            // The rigs keep their own timers.
            let first = rigs.rig(4).unwrap().blink_timer();
            let second = rigs.rig(9).unwrap().blink_timer();
            assert_ne!(first, second);

            assert!(rigs.remove(4));
            assert!(!rigs.remove(4));
            assert_eq!(rigs.len(), 1);
            rigs.clear();
            assert!(rigs.is_empty());
        }

        #[test]
        fn the_settings_are_read_once_and_can_be_reset() {
            reset_settings();
            assert_eq!(current_settings(), FaceSettings::default());

            let read = settings(stored(&[("blink_interval", "7")]));
            assert_eq!(read.blink_interval, 7.0);
            // A second read through another source keeps the first: read once.
            assert_eq!(settings(|_| None), read);

            set_settings(FaceSettings {
                enabled: false,
                ..FaceSettings::default()
            });
            assert!(!current_settings().enabled);
            reset_settings();
            assert_eq!(current_settings(), FaceSettings::default());
        }
    }
}

pub mod ui {
    //! Punchy Moves' screens: the emote wheel and its keybinds, the shoulder
    //! camera's tabbed settings, the colour picker, the face canvas, the tutorial,
    //! and the near-camera dither the mod hides its own geometry with.
    //!
    //! Pure Rust and std-only: every screen builds a [`HudFrame`] of plain
    //! rectangles and lines, and the bundle's `h_gui.rs` hands that frame to the
    //! game's HUD batch after the frame's own GUI. Input is the same shape: the
    //! hooks (`key.rs`, `settings.rs`, `click.rs`) call the methods here and act on
    //! the [`UiRequest`] they return.
    //!
    //! Colours are `0xAABBGGRR`, the HUD's own packing ([`rgba`]).

    use std::cell::RefCell;

    use super::camera::ShoulderCameraSettings;
    use super::engine::ItemAction;
    use super::faces::{FaceCue, FaceExpression, FaceQuads, FaceRig, FaceSettings};
    use super::signals::MotionSignals;

    // ---------------------------------------------------------------------------
    // Drawing primitives
    // ---------------------------------------------------------------------------

    /// A colour packed the way the HUD packs one: alpha high, then blue, green
    /// and red, so `rgba(1, 2, 3, 255)` is `0xFF030201`.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
        (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | r as u32
    }

    /// A rectangle in GUI pixels.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct UiRect {
        pub x: f32,
        pub y: f32,
        pub w: f32,
        pub h: f32,
    }

    impl UiRect {
        pub const fn new(x: f32, y: f32, w: f32, h: f32) -> UiRect {
            UiRect { x, y, w, h }
        }

        /// The rectangle's middle.
        pub fn center(&self) -> [f32; 2] {
            [self.x + self.w * 0.5, self.y + self.h * 0.5]
        }

        /// Whether a point lies inside it.
        pub fn contains(&self, x: f32, y: f32) -> bool {
            x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
        }

        /// The rectangle pulled in by `amount` on every side (never inverted).
        pub fn inset(&self, amount: f32) -> UiRect {
            let amount = amount.max(0.0);
            let w = (self.w - 2.0 * amount).max(0.0);
            let h = (self.h - 2.0 * amount).max(0.0);
            UiRect::new(self.x + amount, self.y + amount, w, h)
        }

        /// Its right and bottom edges.
        pub fn right(&self) -> f32 {
            self.x + self.w
        }

        pub fn bottom(&self) -> f32 {
            self.y + self.h
        }
    }

    /// The screen the screens are laid out on, in GUI pixels.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct UiLayout {
        pub width: f32,
        pub height: f32,
    }

    impl UiLayout {
        pub const fn new(width: f32, height: f32) -> UiLayout {
            UiLayout { width, height }
        }

        /// The middle of the screen.
        pub fn center(&self) -> [f32; 2] {
            [self.width * 0.5, self.height * 0.5]
        }

        /// A `w` by `h` panel in the middle, never larger than the screen.
        pub fn panel(&self, w: f32, h: f32) -> UiRect {
            let w = w.min(self.width.max(0.0));
            let h = h.min(self.height.max(0.0));
            UiRect::new((self.width - w) * 0.5, (self.height - h) * 0.5, w, h)
        }

        /// The `index`th row inside `panel`, with a line of space above the rows.
        pub fn row(&self, panel: UiRect, index: usize) -> UiRect {
            let top = panel.y + ROWS_TOP + index as f32 * ROW_HEIGHT;
            UiRect::new(
                panel.x + 8.0,
                top,
                (panel.w - 16.0).max(0.0),
                ROW_HEIGHT - 2.0,
            )
        }
    }

    /// The space a panel keeps above its first row, in GUI pixels.
    pub const ROWS_TOP: f32 = 22.0;
    /// The height of one row, in GUI pixels.
    pub const ROW_HEIGHT: f32 = 13.0;

    /// One thing the HUD draws.
    #[derive(Debug, Clone, PartialEq)]
    pub enum HudDraw {
        /// A filled rectangle.
        Rect(UiRect, u32),
        /// A filled four-cornered shape, corners in order round it.
        Quad([[f32; 2]; 4], u32),
        /// A line of text at `x, y` (top left).
        Text {
            text: String,
            x: f32,
            y: f32,
            color: u32,
            shadow: bool,
        },
    }

    /// Everything the open screens draw this frame, in draw order.
    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct HudFrame {
        draws: Vec<HudDraw>,
    }

    impl HudFrame {
        pub fn new() -> Self {
            Self::default()
        }

        /// Adds a filled rectangle.
        pub fn rect(&mut self, rect: UiRect, color: u32) {
            self.draws.push(HudDraw::Rect(rect, color));
        }

        /// Adds a filled four-cornered shape.
        pub fn quad(&mut self, corners: [[f32; 2]; 4], color: u32) {
            self.draws.push(HudDraw::Quad(corners, color));
        }

        /// Adds a line of text.
        pub fn text(&mut self, text: impl Into<String>, x: f32, y: f32, color: u32) {
            self.draws.push(HudDraw::Text {
                text: text.into(),
                x,
                y,
                color,
                shadow: true,
            });
        }

        /// Adds a line of text without its drop shadow.
        pub fn plain(&mut self, text: impl Into<String>, x: f32, y: f32, color: u32) {
            self.draws.push(HudDraw::Text {
                text: text.into(),
                x,
                y,
                color,
                shadow: false,
            });
        }

        /// The draws, in order.
        pub fn draws(&self) -> &[HudDraw] {
            &self.draws
        }

        /// How many draws the frame holds.
        pub fn len(&self) -> usize {
            self.draws.len()
        }

        /// Whether nothing is drawn.
        pub fn is_empty(&self) -> bool {
            self.draws.is_empty()
        }

        /// A dim wash over the whole screen, then a panel with a border: the
        /// backdrop every screen sits on.
        pub fn panel(&mut self, layout: &UiLayout, w: f32, h: f32, theme: &UiTheme) -> UiRect {
            self.rect(
                UiRect::new(0.0, 0.0, layout.width, layout.height),
                theme.dim,
            );
            let panel = layout.panel(w, h);
            self.rect(panel, theme.panel);
            let edge = 1.0;
            self.rect(UiRect::new(panel.x, panel.y, panel.w, edge), theme.border);
            self.rect(
                UiRect::new(panel.x, panel.bottom() - edge, panel.w, edge),
                theme.border,
            );
            self.rect(UiRect::new(panel.x, panel.y, edge, panel.h), theme.border);
            self.rect(
                UiRect::new(panel.right() - edge, panel.y, edge, panel.h),
                theme.border,
            );
            panel
        }

        /// A panel's title, centred at its top.
        pub fn title(&mut self, text: &str, panel: UiRect, theme: &UiTheme) {
            let width = text.chars().count() as f32 * GLYPH_WIDTH;
            self.text(
                text,
                (panel.x + (panel.w - width) * 0.5).max(panel.x + 4.0),
                panel.y + 6.0,
                theme.title,
            );
        }

        /// A row of a panel: its label on the left, its value on the right.
        pub fn row(
            &mut self,
            panel: UiRect,
            index: usize,
            label: &str,
            value: &str,
            selected: bool,
            theme: &UiTheme,
        ) {
            let row = UiLayout::new(panel.w, panel.h).row(panel, index);
            if selected {
                self.rect(row, theme.selected);
            }
            self.text(label, row.x + 3.0, row.y + 2.0, theme.text);
            let width = value.chars().count() as f32 * GLYPH_WIDTH;
            self.text(
                value,
                (row.right() - width - 3.0).max(row.x + 3.0),
                row.y + 2.0,
                theme.value,
            );
        }

        /// A footer line under a panel's rows.
        pub fn footer(&mut self, text: &str, panel: UiRect, theme: &UiTheme) {
            self.text(text, panel.x + 8.0, panel.bottom() - 12.0, theme.muted);
        }
    }

    /// The width one text glyph takes, for centring: the HUD font is 6 pixels
    /// wide, one more between glyphs.
    pub const GLYPH_WIDTH: f32 = 6.0;

    // ---------------------------------------------------------------------------
    // GoldUi
    // ---------------------------------------------------------------------------

    /// Converts a Java ARGB int (what GoldUi's constants are written as) to the
    /// ABGR word the HUD batch takes.
    pub const fn argb(java: i32) -> u32 {
        let v = java as u32;
        rgba(
            ((v >> 16) & 0xFF) as u8,
            ((v >> 8) & 0xFF) as u8,
            (v & 0xFF) as u8,
            ((v >> 24) & 0xFF) as u8,
        )
    }

    /// GoldUi's palette and drawing primitives, the Java `GoldUi` record for
    /// record: the same colours, the same bevels and the same pixel geometry.
    pub mod gold {
        use super::{HudFrame, UiRect, argb};

        pub const BACKDROP: u32 = argb(-1946157056);
        pub const WINDOW_EDGE: u32 = argb(-14803425);
        pub const SHADOW: u32 = argb(1493172224);
        pub const TITLEBAR: u32 = argb(-1775898);
        pub const TITLEBAR_EDGE: u32 = argb(-5131595);
        pub const TITLE_INK: u32 = argb(-14803425);
        pub const LETTERPRESS: u32 = argb(-1929379841);
        pub const RAIL: u32 = argb(-13553101);
        pub const PANE: u32 = argb(-12039862);
        pub const ROW_HOVER: u32 = argb(-11513518);
        pub const SEP: u32 = argb(-13487308);
        pub const GROOVE_LIGHT: u32 = argb(-10855588);
        pub const WELL: u32 = argb(-14737375);
        pub const OFF_FILL: u32 = argb(-13947859);
        pub const KNOB: u32 = argb(-3092012);
        pub const KNOB_HI: u32 = argb(-1841946);
        pub const TRACK: u32 = argb(-13947859);
        pub const TRACK_LIGHT: u32 = argb(-12960964);
        pub const LABEL_PLATE: u32 = argb(-1290397928);
        pub const SCROLL_THUMB: u32 = argb(-1715881283);
        pub const SCROLL_THUMB_HOVER: u32 = argb(-3092012);
        pub const TEXT_1: u32 = argb(-1);
        pub const TEXT_2: u32 = argb(-2829100);
        pub const TEXT_3: u32 = argb(-4342339);
        pub const TEXT_DIS: u32 = argb(-8487040);
        pub const BUTTON_INK: u32 = argb(-14803425);
        pub const GOLD_50: u32 = argb(-2346);
        pub const GOLD_100: u32 = argb(-5981);
        pub const GOLD_200: u32 = argb(-10138);
        pub const GOLD_300: u32 = argb(-14019);
        pub const GOLD_400: u32 = argb(-872418);
        pub const GOLD_500: u32 = argb(-2516468);
        pub const GOLD_600: u32 = argb(-5737466);
        pub const GOLD_700: u32 = argb(-8760825);
        pub const GOLD_INK: u32 = argb(-15002619);
        pub const SECONDARY_HOVER: u32 = argb(-2105118);
        pub const SECONDARY_PRESSED: u32 = argb(-4342079);
        pub const PRIMARY_DISABLED_FACE: u32 = argb(-9541028);
        pub const PRIMARY_DISABLED_INK: u32 = argb(-12961744);
        pub const WHEEL_WASH: u32 = argb(889192448);
        pub const SHORTCUT_STRIPE: u32 = argb(436207616);

        /// One bevel's five colours, the Java `Bevel` record.
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct Bevel {
            pub outline: u32,
            pub face: u32,
            pub highlight: u32,
            pub shine: u32,
            pub depth: u32,
        }

        impl Bevel {
            pub const fn new(
                outline: u32,
                face: u32,
                highlight: u32,
                shine: u32,
                depth: u32,
            ) -> Self {
                Self {
                    outline,
                    face,
                    highlight,
                    shine,
                    depth,
                }
            }

            /// `withFace`.
            pub const fn with_face(self, face: u32) -> Self {
                Self { face, ..self }
            }
        }

        pub const LIGHT: Bevel = Bevel::new(WINDOW_EDGE, KNOB, GOLD_200, GOLD_100, GOLD_700);
        pub const GOLD: Bevel = Bevel::new(WINDOW_EDGE, GOLD_400, GOLD_200, GOLD_100, GOLD_700);
        pub const KNOB_BEVEL: Bevel =
            Bevel::new(WINDOW_EDGE, KNOB, GOLD_200, GOLD_100, argb(-10789793));

        pub const LIP: i32 = 2;
        pub const TOGGLE_W: i32 = 30;
        pub const TOGGLE_H: i32 = 14;
        pub const KNOB_W: i32 = 16;
        pub const KNOB_H: i32 = 16;
        pub const TRACK_H: i32 = 6;
        pub const THUMB_W: i32 = 10;
        pub const THUMB_H: i32 = 12;
        pub const TAB_BAR: i32 = 3;

        /// `drawInk`: the letterpress shadow one pixel down, then the ink.
        pub fn draw_ink(frame: &mut HudFrame, text: &str, x: i32, y: i32, ink: u32) {
            frame.text(text, x as f32, (y + 1) as f32, LETTERPRESS);
            frame.text(text, x as f32, y as f32, ink);
        }

        /// `outline`: a one-pixel ring, or a full fill when too small.
        pub fn outline(frame: &mut HudFrame, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) {
            if x2 - x1 >= 2 && y2 - y1 >= 2 {
                frame.rect(
                    UiRect::new(x1 as f32, y1 as f32, (x2 - x1) as f32, 1.0),
                    color,
                );
                frame.rect(
                    UiRect::new(x1 as f32, (y2 - 1) as f32, (x2 - x1) as f32, 1.0),
                    color,
                );
                frame.rect(
                    UiRect::new(x1 as f32, (y1 + 1) as f32, 1.0, (y2 - y1 - 2) as f32),
                    color,
                );
                frame.rect(
                    UiRect::new((x2 - 1) as f32, (y1 + 1) as f32, 1.0, (y2 - y1 - 2) as f32),
                    color,
                );
            } else {
                frame.rect(
                    UiRect::new(x1 as f32, y1 as f32, (x2 - x1) as f32, (y2 - y1) as f32),
                    color,
                );
            }
        }

        /// `focusRing`: the focus outline one pixel out, in gold.
        pub fn focus_ring(frame: &mut HudFrame, x1: i32, y1: i32, x2: i32, y2: i32) {
            outline(frame, x1 - 1, y1 - 1, x2 + 1, y2 + 1, GOLD_200);
        }

        /// `groove`: a horizontal separator with its light edge under it.
        pub fn groove(frame: &mut HudFrame, x1: i32, x2: i32, y: i32) {
            frame.rect(UiRect::new(x1 as f32, y as f32, (x2 - x1) as f32, 1.0), SEP);
            frame.rect(
                UiRect::new(x1 as f32, (y + 1) as f32, (x2 - x1) as f32, 1.0),
                GROOVE_LIGHT,
            );
        }

        /// `vgroove`: the vertical one.
        pub fn vgroove(frame: &mut HudFrame, x: i32, y1: i32, y2: i32) {
            frame.rect(UiRect::new(x as f32, y1 as f32, 1.0, (y2 - y1) as f32), SEP);
            frame.rect(
                UiRect::new((x + 1) as f32, y1 as f32, 1.0, (y2 - y1) as f32),
                GROOVE_LIGHT,
            );
        }

        /// `bevel`: the raised plate the buttons, toggles and knobs sit on. The
        /// nine arguments mirror the Java signature.
        #[allow(clippy::too_many_arguments)]
        pub fn bevel(
            frame: &mut HudFrame,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            bevel: Bevel,
            lip: i32,
            pressed: bool,
        ) {
            let top = if pressed { y1 + lip } else { y1 };
            let depth_lip = if pressed { 0 } else { lip };
            outline(frame, x1, top, x2, y2, bevel.outline);
            let left = x1 + 1;
            let right = x2 - 1;
            let inner_top = top + 1;
            let inner_bottom = y2 - 1 - depth_lip;
            if right - left >= 2 && inner_bottom - inner_top >= 2 {
                frame.rect(
                    UiRect::new(left as f32, inner_top as f32, (right - left) as f32, 1.0),
                    bevel.highlight,
                );
                frame.rect(
                    UiRect::new(
                        left as f32,
                        (inner_top + 1) as f32,
                        1.0,
                        (inner_bottom - inner_top - 2) as f32,
                    ),
                    bevel.highlight,
                );
                frame.rect(
                    UiRect::new(
                        (right - 1) as f32,
                        (inner_top + 1) as f32,
                        1.0,
                        (inner_bottom - inner_top - 2) as f32,
                    ),
                    bevel.highlight,
                );
                frame.rect(
                    UiRect::new(
                        left as f32,
                        (inner_bottom - 1) as f32,
                        (right - left) as f32,
                        1.0,
                    ),
                    bevel.shine,
                );
                frame.rect(
                    UiRect::new(
                        (left + 1) as f32,
                        (inner_top + 1) as f32,
                        (right - left - 2) as f32,
                        (inner_bottom - inner_top - 2) as f32,
                    ),
                    bevel.face,
                );
                if depth_lip > 0 {
                    frame.rect(
                        UiRect::new(
                            left as f32,
                            inner_bottom as f32,
                            (right - left) as f32,
                            (y2 - 1 - inner_bottom) as f32,
                        ),
                        bevel.depth,
                    );
                }
            } else {
                frame.rect(
                    UiRect::new(
                        left as f32,
                        inner_top as f32,
                        (right - left) as f32,
                        (y2 - 1 - inner_top) as f32,
                    ),
                    bevel.face,
                );
            }
        }

        /// `window`: the double shadow, the edge, the title bar and the pane.
        pub fn window(
            frame: &mut HudFrame,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            title_h: i32,
            pane: u32,
        ) {
            frame.rect(
                UiRect::new(
                    (x1 + 4) as f32,
                    (y1 + 4) as f32,
                    (x2 - x1) as f32,
                    (y2 - y1) as f32,
                ),
                SHADOW,
            );
            frame.rect(
                UiRect::new(
                    (x1 + 2) as f32,
                    (y1 + 2) as f32,
                    (x2 - x1) as f32,
                    (y2 - y1) as f32,
                ),
                SHADOW,
            );
            outline(frame, x1, y1, x2, y2, WINDOW_EDGE);
            frame.rect(
                UiRect::new(
                    (x1 + 1) as f32,
                    (y1 + 1) as f32,
                    (x2 - x1 - 2) as f32,
                    (title_h - 2) as f32,
                ),
                TITLEBAR,
            );
            frame.rect(
                UiRect::new(
                    (x1 + 1) as f32,
                    (y1 + title_h - 2) as f32,
                    (x2 - x1 - 2) as f32,
                    2.0,
                ),
                TITLEBAR_EDGE,
            );
            frame.rect(
                UiRect::new(
                    (x1 + 1) as f32,
                    (y1 + title_h) as f32,
                    (x2 - x1 - 2) as f32,
                    (y2 - 1 - y1 - title_h) as f32,
                ),
                pane,
            );
        }

        /// `button`: a bevelled button with its ink centred, the Java colours for
        /// the primary (gold) and secondary (light) faces, hover and disabled.
        /// The eight arguments mirror the Java signature.
        #[allow(clippy::too_many_arguments)]
        pub fn button(
            frame: &mut HudFrame,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            label: &str,
            primary: bool,
            hovered: bool,
            enabled: bool,
        ) {
            let style = if primary { GOLD } else { LIGHT };
            let face = if !enabled {
                if primary {
                    PRIMARY_DISABLED_FACE
                } else {
                    SECONDARY_PRESSED
                }
            } else if hovered {
                if primary { GOLD_300 } else { SECONDARY_HOVER }
            } else {
                style.face
            };
            bevel(frame, x, y, x + w, y + h, style.with_face(face), LIP, false);
            let ink = if !enabled {
                if primary {
                    PRIMARY_DISABLED_INK
                } else {
                    TEXT_DIS
                }
            } else if primary {
                GOLD_INK
            } else {
                WINDOW_EDGE
            };
            let width = label.chars().count() as f32 * super::GLYPH_WIDTH;
            let text_x = x as f32 + (w as f32 - width) * 0.5;
            let top = y + 1;
            let bottom = y + h - 1 - 2;
            let text_y = top as f32 + ((bottom - top - 8) as f32) * 0.5;
            draw_ink(frame, label, text_x as i32, text_y as i32, ink);
        }

        /// `toggle`: the 30x14 switch with its 16x16 knob.
        pub fn toggle(frame: &mut HudFrame, x: i32, y: i32, on: bool, hovered: bool) {
            let top = y + 16 - TOGGLE_H;
            let height = TOGGLE_H;
            let track_start = x;
            let track_end = x + TOGGLE_W;
            let knob_x = if on { track_end - KNOB_W } else { track_start };
            let knob_end = knob_x + KNOB_W;
            frame.rect(
                UiRect::new(
                    track_start as f32,
                    top as f32,
                    (knob_x - track_start) as f32,
                    1.0,
                ),
                WINDOW_EDGE,
            );
            frame.rect(
                UiRect::new(
                    knob_end as f32,
                    top as f32,
                    (track_end - knob_end) as f32,
                    1.0,
                ),
                WINDOW_EDGE,
            );
            frame.rect(
                UiRect::new(
                    track_start as f32,
                    (top + height - 1) as f32,
                    (knob_x - track_start) as f32,
                    1.0,
                ),
                WINDOW_EDGE,
            );
            frame.rect(
                UiRect::new(
                    knob_end as f32,
                    (top + height - 1) as f32,
                    (track_end - knob_end) as f32,
                    1.0,
                ),
                WINDOW_EDGE,
            );
            if knob_x > track_start {
                frame.rect(
                    UiRect::new(
                        track_start as f32,
                        (top + 1) as f32,
                        1.0,
                        (height - 2) as f32,
                    ),
                    WINDOW_EDGE,
                );
            }
            if knob_end < track_end {
                frame.rect(
                    UiRect::new(
                        (track_end - 1) as f32,
                        (top + 1) as f32,
                        1.0,
                        (height - 2) as f32,
                    ),
                    WINDOW_EDGE,
                );
            }
            if on {
                frame.rect(
                    UiRect::new(
                        (track_start + 1) as f32,
                        (top + 1) as f32,
                        (knob_x - track_start - 1) as f32,
                        1.0,
                    ),
                    GOLD_300,
                );
                frame.rect(
                    UiRect::new(
                        (track_start + 1) as f32,
                        (top + 2) as f32,
                        (knob_x - track_start - 1) as f32,
                        (height - 3) as f32,
                    ),
                    GOLD_500,
                );
                frame.rect(
                    UiRect::new((track_start + 7) as f32, (top + 4) as f32, 1.0, 6.0),
                    GOLD_50,
                );
            } else {
                frame.rect(
                    UiRect::new(
                        knob_end as f32,
                        (top + 1) as f32,
                        (track_end - 1 - knob_end) as f32,
                        (height - 2) as f32,
                    ),
                    OFF_FILL,
                );
                let mark = track_start + 20;
                frame.rect(
                    UiRect::new((mark + 1) as f32, (top + 4) as f32, 3.0, 1.0),
                    TEXT_2,
                );
                frame.rect(
                    UiRect::new((mark + 1) as f32, (top + 9) as f32, 3.0, 1.0),
                    TEXT_2,
                );
                frame.rect(UiRect::new(mark as f32, (top + 5) as f32, 1.0, 4.0), TEXT_2);
                frame.rect(
                    UiRect::new((mark + 4) as f32, (top + 5) as f32, 1.0, 4.0),
                    TEXT_2,
                );
            }
            bevel(
                frame,
                knob_x,
                y,
                knob_end,
                y + 16,
                KNOB_BEVEL.with_face(if hovered { KNOB_HI } else { KNOB }),
                LIP,
                false,
            );
        }

        /// `slider`: the 6-pixel track with its filled span, the zero notch and
        /// the 10x12 knob; `zero` is the notch's place or negative for none,
        /// `default` the knob the reset would return to. The nine arguments
        /// mirror the Java signature.
        #[allow(clippy::too_many_arguments)]
        pub fn slider(
            frame: &mut HudFrame,
            x1: i32,
            x2: i32,
            y: i32,
            value: f32,
            zero: f32,
            default: f32,
            hovered: bool,
            dragging: bool,
        ) {
            outline(frame, x1, y, x2, y + TRACK_H, WINDOW_EDGE);
            let span = (x2 - x1 - 10).max(1) as f32;
            let value_x = x1 as f32 + 5.0 + value * span;
            let zero_x = if zero < 0.0 {
                (x1 + 1) as f32
            } else {
                x1 as f32 + 5.0 + zero * span
            };
            let left = x1 + 1;
            let right = x2 - 1;
            let start = left.max(right.min(value_x.min(zero_x).round() as i32));
            let end = left.max(right.min(value_x.max(zero_x).round() as i32));
            track_part(frame, left, start, y);
            track_part(frame, start, end, y);
            track_part(frame, end, right, y);
            let notch = (x1 as f32 + 5.0 + default * span).round() as i32;
            frame.rect(
                UiRect::new(notch as f32, (y - 4) as f32, 1.0, 3.0),
                GOLD_600,
            );
            let knob = (value_x - 5.0).round() as i32;
            let knob_top = y + 3 - THUMB_H;
            if dragging {
                outline(
                    frame,
                    knob - 1,
                    knob_top + 1,
                    knob + THUMB_W + 1,
                    knob_top + THUMB_H + 1,
                    GOLD_200,
                );
            }
            bevel(
                frame,
                knob,
                knob_top,
                knob + THUMB_W,
                knob_top + THUMB_H,
                KNOB_BEVEL.with_face(if !hovered && !dragging { KNOB } else { KNOB_HI }),
                LIP,
                dragging,
            );
        }

        fn track_part(frame: &mut HudFrame, x1: i32, x2: i32, y: i32) {
            if x2 > x1 {
                frame.rect(
                    UiRect::new(x1 as f32, (y + 1) as f32, (x2 - x1) as f32, 1.0),
                    TRACK_LIGHT,
                );
                frame.rect(
                    UiRect::new(
                        x1 as f32,
                        (y + 2) as f32,
                        (x2 - x1) as f32,
                        (TRACK_H - 3) as f32,
                    ),
                    TRACK,
                );
            }
        }

        /// `scrollbar`: the well and the thumb. The eight arguments mirror the
        /// Java signature.
        #[allow(clippy::too_many_arguments)]
        pub fn scrollbar(
            frame: &mut HudFrame,
            x: i32,
            top: i32,
            bottom: i32,
            width: i32,
            thumb_top: i32,
            thumb_bottom: i32,
            dragging: bool,
        ) {
            frame.rect(
                UiRect::new(x as f32, top as f32, width as f32, (bottom - top) as f32),
                WELL,
            );
            frame.rect(
                UiRect::new(
                    (x + 1) as f32,
                    thumb_top as f32,
                    (width - 2) as f32,
                    (thumb_bottom - thumb_top) as f32,
                ),
                if dragging {
                    SCROLL_THUMB_HOVER
                } else {
                    SCROLL_THUMB
                },
            );
        }

        /// `tab`: a left-hand navigation tab, the shown one picked out in gold.
        /// The eight arguments mirror the Java signature.
        #[allow(clippy::too_many_arguments)]
        pub fn tab(
            frame: &mut HudFrame,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            label: &str,
            selected: bool,
            hovered: bool,
        ) {
            if selected {
                frame.rect(UiRect::new(x as f32, y as f32, w as f32, 1.0), GROOVE_LIGHT);
                frame.rect(
                    UiRect::new(x as f32, (y + 1) as f32, w as f32, (h - 2) as f32),
                    PANE,
                );
                frame.rect(
                    UiRect::new(x as f32, (y + h - 1) as f32, w as f32, 1.0),
                    SEP,
                );
                frame.rect(
                    UiRect::new(x as f32, y as f32, TAB_BAR as f32, h as f32),
                    GOLD_400,
                );
            } else if hovered {
                frame.rect(
                    UiRect::new(x as f32, y as f32, w as f32, h as f32),
                    ROW_HOVER,
                );
            }
            let text_x = x + 3 + 7;
            let text_y = y + (h - 8) / 2;
            frame.text(
                label,
                text_x as f32,
                text_y as f32,
                if !selected && !hovered {
                    TEXT_2
                } else {
                    TEXT_1
                },
            );
        }

        /// `keycap`: a little bevelled key with its label centred.
        pub fn keycap(frame: &mut HudFrame, x: i32, y: i32, w: i32, h: i32, label: &str) {
            bevel(
                frame,
                x,
                y,
                x + w,
                y + h,
                LIGHT,
                if h >= 14 { 1 } else { 0 },
                false,
            );
            let width = label.chars().count() as f32 * super::GLYPH_WIDTH;
            let bottom = y + h - 1 - if h >= 14 { 1 } else { 0 };
            let text_y = (y + 1) as f32 + ((bottom - y - 1 - 8) as f32) * 0.5;
            draw_ink(
                frame,
                label,
                (x as f32 + (w as f32 - width) * 0.5) as i32,
                text_y as i32,
                WINDOW_EDGE,
            );
        }

        /// `raised`: the emote wheel's tiles, two shadow layers then a bevel.
        pub fn raised(frame: &mut HudFrame, x1: i32, y1: i32, x2: i32, y2: i32, hovered: bool) {
            frame.rect(
                UiRect::new(
                    (x1 + 4) as f32,
                    (y1 + 4) as f32,
                    (x2 - x1) as f32,
                    (y2 - y1) as f32,
                ),
                SHADOW,
            );
            frame.rect(
                UiRect::new(
                    (x1 + 2) as f32,
                    (y1 + 2) as f32,
                    (x2 - x1) as f32,
                    (y2 - y1) as f32,
                ),
                SHADOW,
            );
            bevel(
                frame,
                x1,
                y1,
                x2,
                y2,
                if hovered { GOLD } else { LIGHT },
                LIP,
                false,
            );
        }
    }

    /// The colours every screen draws with.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct UiTheme {
        /// The wash behind a panel.
        pub dim: u32,
        /// A panel's fill.
        pub panel: u32,
        /// A panel's and a row's border.
        pub border: u32,
        /// A panel's title.
        pub title: u32,
        /// Ordinary text.
        pub text: u32,
        /// A value on the right of a row.
        pub value: u32,
        /// Hints and footers.
        pub muted: u32,
        /// The selected row, wedge or swatch.
        pub selected: u32,
        /// Things that ask for a press.
        pub accent: u32,
    }

    impl Default for UiTheme {
        fn default() -> Self {
            Self {
                dim: gold::BACKDROP,
                panel: gold::PANE,
                border: gold::WINDOW_EDGE,
                title: gold::TEXT_1,
                text: gold::TEXT_1,
                value: gold::TEXT_2,
                muted: gold::TEXT_3,
                selected: gold::ROW_HOVER,
                accent: gold::GOLD_400,
            }
        }
    }

    /// The emote wheel's key, unbound until the player picks one.
    pub const EMOTE_WHEEL_KEY: &str = "emote_wheel";
    /// The emote wheel's default key.
    pub const DEFAULT_EMOTE_WHEEL_KEY: &str = "key.keyboard.unknown";

    // ---------------------------------------------------------------------------
    // Emotes
    // ---------------------------------------------------------------------------

    /// One emote: what the wheel shows, the face it wears and the hands it moves.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Emote {
        /// Its id: the settings name of its key, the name its cue is filed under.
        pub id: &'static str,
        /// The wheel's label.
        pub label: &'static str,
        /// The face it wears.
        pub expression: FaceExpression,
        /// The hand action it plays.
        pub action: ItemAction,
    }

    impl Emote {
        /// The cue this emote's face wears.
        pub fn cue(&self) -> FaceCue {
            FaceCue {
                seconds: EMOTE_SECONDS,
                priority: 5,
                ..FaceCue::built_in(self.expression)
            }
        }
    }

    /// How long an emote's face lasts, seconds.
    pub const EMOTE_SECONDS: f32 = 2.4;

    /// The emotes on the wheel, in wheel order from the top clockwise.
    pub fn emotes() -> Vec<Emote> {
        vec![
            Emote {
                id: "wave",
                label: "Wave",
                expression: FaceExpression::Emote,
                action: ItemAction::Use,
            },
            Emote {
                id: "clap",
                label: "Clap",
                expression: FaceExpression::Happy,
                action: ItemAction::Swing,
            },
            Emote {
                id: "dance",
                label: "Dance",
                expression: FaceExpression::Happy,
                action: ItemAction::Swing,
            },
            Emote {
                id: "bow",
                label: "Bow",
                expression: FaceExpression::Emote,
                action: ItemAction::Use,
            },
            Emote {
                id: "cry",
                label: "Cry",
                expression: FaceExpression::Hurt,
                action: ItemAction::Use,
            },
            Emote {
                id: "laugh",
                label: "Laugh",
                expression: FaceExpression::Happy,
                action: ItemAction::Swing,
            },
            Emote {
                id: "point",
                label: "Point",
                expression: FaceExpression::Surprised,
                action: ItemAction::Use,
            },
            Emote {
                id: "shrug",
                label: "Shrug",
                expression: FaceExpression::Emote,
                action: ItemAction::Use,
            },
        ]
    }

    /// The most emotes one wheel's keybind slots carry.
    pub const MAX_EMOTES: usize = 8;
    /// How many tiles one wheel page shows, the Java wheel's four directions.
    pub const WHEEL_PER_PAGE: usize = 4;

    /// The radial emote wheel: which emote is picked, which page is showing, and
    /// whether it is up. One page holds four tiles, one per direction round the
    /// middle, the Java `EmoteWheelScreen`.
    #[derive(Debug, Clone, PartialEq)]
    pub struct EmoteWheelState {
        open: bool,
        page: usize,
        selected: usize,
        emotes: Vec<Emote>,
    }

    impl Default for EmoteWheelState {
        fn default() -> Self {
            Self::new()
        }
    }

    impl EmoteWheelState {
        /// The wheel with the built-in emotes.
        pub fn new() -> Self {
            Self::with_emotes(emotes())
        }

        /// The wheel over a given emote list (at most [`MAX_EMOTES`]).
        pub fn with_emotes(mut emotes: Vec<Emote>) -> Self {
            emotes.truncate(MAX_EMOTES);
            Self {
                open: false,
                page: 0,
                selected: 0,
                emotes,
            }
        }

        /// Whether the wheel is up.
        pub fn is_open(&self) -> bool {
            self.open
        }

        /// Puts the wheel up, keeping the selection and its page.
        pub fn open(&mut self) {
            self.open = true;
        }

        /// Takes the wheel down and returns the emote it was pointing at, to play.
        pub fn close(&mut self) -> Option<Emote> {
            if !self.open {
                return None;
            }
            self.open = false;
            self.emote()
        }

        /// Closes without playing anything.
        pub fn cancel(&mut self) {
            self.open = false;
        }

        /// The wheel's emotes.
        pub fn emotes(&self) -> &[Emote] {
            &self.emotes
        }

        /// How many pages the wheel has, at least one, the Java
        /// `max(1, (entries + 3) / 4)`.
        pub fn pages(&self) -> usize {
            self.emotes.len().div_ceil(WHEEL_PER_PAGE).max(1)
        }

        /// The page being shown.
        pub fn page(&self) -> usize {
            self.page
        }

        /// Shows `page`, clamped to the wheel's pages.
        pub fn set_page(&mut self, page: usize) -> usize {
            self.page = page.min(self.pages() - 1);
            self.page
        }

        /// The selected emote's index.
        pub fn selected(&self) -> usize {
            self.selected
        }

        /// The selected emote.
        pub fn emote(&self) -> Option<Emote> {
            self.emotes.get(self.selected).copied()
        }

        /// Selects an emote by index (wrapping; nothing when the wheel is empty)
        /// and turns to its page.
        pub fn select(&mut self, index: usize) -> usize {
            if self.emotes.is_empty() {
                self.selected = 0;
            } else {
                self.selected = index % self.emotes.len();
                self.page = self.selected / WHEEL_PER_PAGE;
            }
            self.selected
        }

        /// Steps the selection round the wheel by `delta` emotes.
        pub fn nudge(&mut self, delta: i32) -> usize {
            if self.emotes.is_empty() {
                return 0;
            }
            let count = self.emotes.len() as i32;
            let at = (self.selected as i32 + delta).rem_euclid(count);
            self.select(at as usize)
        }

        /// Turns one page back or forward, wrapping, the Java paging buttons.
        pub fn turn_page(&mut self, delta: i32) -> usize {
            let pages = self.pages() as i32;
            self.page = (self.page as i32 + delta).rem_euclid(pages) as usize;
            self.page
        }

        /// Selects the tile a pointer at `(x, y)` on the screen lands on, the
        /// Java tiles' hit boxes; `None` keeps the selection.
        pub fn pointing_at(&mut self, layout: &UiLayout, x: f32, y: f32) -> Option<usize> {
            for tile in 0..WHEEL_PER_PAGE {
                let index = self.page * WHEEL_PER_PAGE + tile;
                if index >= self.emotes.len() {
                    break;
                }
                if self.tile_rect(layout, tile).contains(x, y) {
                    self.selected = index;
                    return Some(index);
                }
            }
            None
        }

        /// Selects the tile a pointer offset `(dx, dy)` from the middle points
        /// at: up, right, down, left, the Java tiles' order.
        pub fn pointing(&mut self, dx: f32, dy: f32) -> usize {
            if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
                return self.selected;
            }
            let tile = if dy <= -dx.abs() && dy < 0.0 {
                0
            } else if dx >= dy.abs() && dx > 0.0 {
                1
            } else if dy >= dx.abs() && dy > 0.0 {
                2
            } else if dx <= -dy.abs() && dx < 0.0 {
                3
            } else {
                return self.selected;
            };
            let index = self.page * WHEEL_PER_PAGE + tile;
            if index < self.emotes.len() {
                self.selected = index;
            }
            self.selected
        }

        /// The tile rect of the `tile`th tile of the page `index`'s emote sits
        /// on, the Java tile geometry: the four directions round the middle,
        /// sized from the screen.
        pub fn slot_rect(&self, layout: &UiLayout, index: usize) -> UiRect {
            self.tile_rect(layout, index % WHEEL_PER_PAGE)
        }

        /// The rect of tile `tile` (0 up, 1 right, 2 down, 3 left) of the page
        /// being shown, the Java `init`'s bounds.
        pub fn tile_rect(&self, layout: &UiLayout, tile: usize) -> UiRect {
            let [cx, cy] = layout.center();
            let j = cx;
            let k = cy + 3.0;
            let l = WHEEL_TILE_W.min((layout.width - 36.0) / 3.0);
            let i1 = ((layout.height - 100.0) / 3.0).clamp(34.0, WHEEL_TILE_H);
            let j1 = l + 8.0;
            let k1 = i1 + 12.0;
            let (x, y) = match tile {
                0 => (j, k - k1),
                1 => (j + j1, k),
                2 => (j, k + k1),
                _ => (j - j1, k),
            };
            UiRect::new(x - l * 0.5, y - i1 * 0.5, l, i1)
        }

        /// Draws the open wheel: the wash, the page's tiles with their labels
        /// and keycap chips, the close tile in the middle, the paging buttons
        /// and the page counter.
        pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
            if !self.open {
                return;
            }
            let _ = theme;
            frame.rect(
                UiRect::new(0.0, 0.0, layout.width, layout.height),
                super::ui::gold::WHEEL_WASH,
            );
            let [cx, cy] = layout.center();
            let j = cx;
            let k = cy + 3.0;
            for tile in 0..WHEEL_PER_PAGE {
                let index = self.page * WHEEL_PER_PAGE + tile;
                let Some(emote) = self.emotes.get(index) else {
                    break;
                };
                let slot = self.tile_rect(layout, tile);
                let hovered = index == self.selected;
                super::ui::gold::raised(
                    frame,
                    slot.x as i32,
                    slot.y as i32,
                    slot.right() as i32,
                    slot.bottom() as i32,
                    hovered,
                );
                let ink = if hovered {
                    super::ui::gold::GOLD_INK
                } else {
                    super::ui::gold::WINDOW_EDGE
                };
                let width = emote.label.chars().count() as f32 * GLYPH_WIDTH;
                frame.text(
                    emote.label,
                    slot.center()[0] - width * 0.5,
                    slot.y + 7.0,
                    ink,
                );
                // The keycap chip under the label, the Java keybind chip.
                let chip_w = 26.0;
                let chip_x = slot.center()[0] - chip_w * 0.5;
                let chip_y = slot.bottom() - 21.0;
                super::ui::gold::outline(
                    frame,
                    chip_x as i32,
                    chip_y as i32,
                    (chip_x + chip_w) as i32,
                    (chip_y + 13.0) as i32,
                    super::ui::gold::WINDOW_EDGE,
                );
                frame.rect(
                    UiRect::new(chip_x + 1.0, chip_y + 1.0, chip_w - 2.0, 11.0),
                    if hovered {
                        super::ui::gold::GOLD_700
                    } else {
                        super::ui::gold::WELL
                    },
                );
                frame.text("key", chip_x + 4.0, chip_y + 3.0, super::ui::gold::TEXT_1);
            }
            // The close tile in the middle.
            super::ui::gold::raised(
                frame,
                (j - 11.0) as i32,
                (k - 11.0) as i32,
                (j + 11.0) as i32,
                (k + 11.0) as i32,
                false,
            );
            frame.text(
                "X",
                j - GLYPH_WIDTH * 0.5,
                k - 5.0,
                super::ui::gold::WINDOW_EDGE,
            );
            // The paging buttons and the counter, when there is more than one
            // page.
            if self.pages() > 1 {
                super::ui::gold::button(
                    frame,
                    (j - 76.0) as i32,
                    (layout.height - 32.0) as i32,
                    28,
                    22,
                    "punchy_moves.ui.previous",
                    false,
                    false,
                    true,
                );
                super::ui::gold::button(
                    frame,
                    (j + 48.0) as i32,
                    (layout.height - 32.0) as i32,
                    28,
                    22,
                    "punchy_moves.ui.next_page",
                    false,
                    false,
                    true,
                );
                let counter = format!("{} / {}", self.page + 1, self.pages());
                let width = counter.chars().count() as f32 * GLYPH_WIDTH;
                frame.text(
                    counter,
                    layout.width * 0.5 - width * 0.5,
                    layout.height - 25.0,
                    super::ui::gold::TEXT_1,
                );
            }
            if self.emotes.is_empty() {
                let text = "punchy_moves.ui.no_emotes";
                let width = text.chars().count() as f32 * GLYPH_WIDTH;
                frame.text(
                    text,
                    layout.width * 0.5 - width * 0.5,
                    layout.height * 0.5 + 28.0,
                    super::ui::gold::TEXT_1,
                );
            }
        }
    }

    /// One wheel tile's widest, the Java `min(138, (width - 36) / 3)`.
    pub const WHEEL_TILE_W: f32 = 138.0;
    /// One wheel tile's tallest, the Java `min(52, max(34, (height - 100) / 3))`.
    pub const WHEEL_TILE_H: f32 = 52.0;

    /// The keys each emote is bound to: one slot per emote on the wheel.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct EmoteKeybindManager {
        slots: [Option<String>; MAX_EMOTES],
    }

    /// The setting name of slot `index`'s key.
    pub fn slot_setting(index: usize) -> String {
        format!("emote_{}", index + 1)
    }

    impl EmoteKeybindManager {
        pub fn new() -> Self {
            Self::default()
        }

        /// Reads the bindings from stored values (`get` reads one key).
        pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
            let mut manager = Self::default();
            for index in 0..MAX_EMOTES {
                if let Some(key) = get(&slot_setting(index))
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty() && v != "key.keyboard.unknown")
                {
                    manager.bind(index, &key);
                }
            }
            manager
        }

        /// The key of slot `index`, if it has one.
        pub fn slot(&self, index: usize) -> Option<&str> {
            self.slots.get(index).and_then(|slot| slot.as_deref())
        }

        /// Binds slot `index` to `key`; an unknown index or a blank key unbinds
        /// and returns `false` only for the index.
        pub fn bind(&mut self, index: usize, key: &str) -> bool {
            let Some(slot) = self.slots.get_mut(index) else {
                return false;
            };
            let key = key.trim();
            *slot = if key.is_empty() || key == "key.keyboard.unknown" {
                None
            } else {
                Some(key.to_string())
            };
            true
        }

        /// Unbinds slot `index`.
        pub fn unbind(&mut self, index: usize) {
            if let Some(slot) = self.slots.get_mut(index) {
                *slot = None;
            }
        }

        /// The slot bound to `key`, if any.
        pub fn index_of_key(&self, key: &str) -> Option<usize> {
            self.slots
                .iter()
                .position(|slot| slot.as_deref() == Some(key))
        }

        /// One line per slot, for a settings page.
        pub fn labels(&self) -> Vec<String> {
            (0..MAX_EMOTES)
                .map(|index| {
                    let emote = emotes().get(index).copied();
                    let name = emote.map_or("Emote", |e| e.label);
                    let key = self.slot(index).unwrap_or("unbound");
                    format!("{}: {}", name, key)
                })
                .collect()
        }

        /// What to store, one line per slot; an unbound slot stores the
        /// unbound key so a cleared binding stays cleared.
        pub fn stored(&self) -> Vec<(String, String)> {
            (0..MAX_EMOTES)
                .map(|index| {
                    (
                        slot_setting(index),
                        self.slot(index)
                            .unwrap_or("key.keyboard.unknown")
                            .to_string(),
                    )
                })
                .collect()
        }
    }

    /// Plays `emote` on `entity`: the face wears its cue and the hands play its
    /// action. `false` when the engine has no state for that entity.
    pub fn play_emote(entity: Option<u64>, emote: Emote) -> bool {
        let Some(entity) = entity else {
            return false;
        };
        super::engine::with_engine(|engine| {
            let Some(state) = engine.state_mut(entity) else {
                return false;
            };
            state.face.play_cue(emote.cue());
            state.items.play_emote(emote.action);
            true
        })
    }

    // ---------------------------------------------------------------------------
    // Requests
    // ---------------------------------------------------------------------------

    /// What a click on a screen asks the game to do beyond the screen's own state.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum UiRequest {
        /// Nothing beyond the screen's own state.
        None,
        /// Restart the tutorial from its first step.
        ReplayTutorial,
        /// Open the mod's keys so `name` can be rebound.
        RebindKey(&'static str),
        /// Play the wheel's emote at `index` on the local player.
        PlayEmote(usize),
        /// Close every screen and go back to the game.
        CloseAll,
    }

    // ---------------------------------------------------------------------------
    // The shoulder camera's screen
    // ---------------------------------------------------------------------------

    /// One page of the shoulder camera's screen, the Java `Page` enum.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum CameraPage {
        /// The position sliders and the on/off row.
        #[default]
        Camera,
        /// The feature toggles.
        Misc,
        /// The shortcuts and their keys.
        Shortcuts,
    }

    impl CameraPage {
        /// The pages, in navigation order.
        pub const ALL: [CameraPage; 3] =
            [CameraPage::Camera, CameraPage::Misc, CameraPage::Shortcuts];

        /// The page's label key, the Java `Page.text()`'s translation key.
        pub fn label(self) -> &'static str {
            match self {
                CameraPage::Camera => "punchy_moves.ui.camera",
                CameraPage::Misc => "punchy_moves.ui.misc",
                CameraPage::Shortcuts => "punchy_moves.ui.shortcuts",
            }
        }

        /// This page's place in [`CameraPage::ALL`].
        pub fn index(self) -> usize {
            CameraPage::ALL
                .iter()
                .position(|page| *page == self)
                .unwrap_or(0)
        }

        /// The page below (wrapping).
        pub fn next(self) -> CameraPage {
            CameraPage::ALL[(self.index() + 1) % CameraPage::ALL.len()]
        }

        /// The page above (wrapping).
        pub fn previous(self) -> CameraPage {
            CameraPage::ALL[(self.index() + CameraPage::ALL.len() - 1) % CameraPage::ALL.len()]
        }
    }

    /// One slider of the camera page: its label key, its range, the value the
    /// reset returns to, and the setting it writes, the Java `addSlider` rows.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CameraSliderRow {
        pub label: &'static str,
        pub min: f32,
        pub max: f32,
        pub default: f32,
        pub setting: usize,
    }

    /// The camera page's sliders, in screen order.
    pub const CAMERA_SLIDERS: [CameraSliderRow; 6] = [
        CameraSliderRow {
            label: "punchy_moves.ui.horizontal",
            min: -2.0,
            max: 2.0,
            default: 1.17,
            setting: super::camera::HORIZONTAL_INDEX,
        },
        CameraSliderRow {
            label: "punchy_moves.ui.vertical",
            min: -1.5,
            max: 2.0,
            default: -0.11,
            setting: super::camera::VERTICAL_INDEX,
        },
        CameraSliderRow {
            label: "punchy_moves.ui.depth",
            min: 1.0,
            max: 8.0,
            default: 2.5,
            setting: super::camera::DEPTH_INDEX,
        },
        CameraSliderRow {
            label: "punchy_moves.ui.fov",
            min: 30.0,
            max: 110.0,
            default: 70.0,
            setting: super::camera::FOV_INDEX,
        },
        CameraSliderRow {
            label: "punchy_moves.ui.rotation_yaw",
            min: -30.0,
            max: 30.0,
            default: 0.0,
            setting: super::camera::YAW_INDEX,
        },
        CameraSliderRow {
            label: "punchy_moves.ui.rotation_pitch",
            min: -30.0,
            max: 30.0,
            default: 0.0,
            setting: super::camera::PITCH_INDEX,
        },
    ];

    /// One feature toggle of the misc page: its `punchy_moves.<key>.title` key
    /// and the setting it flips, the Java `addFeature` rows. The two compat rows
    /// (`better_combat`, `spatial_gui`) have no Horizon counterpart, so they
    /// carry no setting and a click on them does nothing.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FeatureRow {
        pub key: &'static str,
        pub setting: Option<usize>,
    }

    /// The misc page's features, in screen order.
    pub const FEATURES: [FeatureRow; 10] = [
        FeatureRow {
            key: "misc.fence",
            setting: Some(super::camera::FENCE_INDEX),
        },
        FeatureRow {
            key: "misc.ladder_camera",
            setting: Some(super::camera::LADDER_INDEX),
        },
        FeatureRow {
            key: "misc.proximity",
            setting: Some(super::camera::PROXIMITY_INDEX),
        },
        FeatureRow {
            key: "misc.elytra",
            setting: Some(super::camera::ELYTRA_INDEX),
        },
        FeatureRow {
            key: "misc.map_camera",
            setting: Some(super::camera::MAP_INDEX),
        },
        FeatureRow {
            key: "misc.bow_and_crossbow_camera_fix",
            setting: Some(super::camera::BOW_AND_CROSSBOW_INDEX),
        },
        FeatureRow {
            key: "misc.spear_and_trident_camera_fix",
            setting: Some(super::camera::SPEAR_AND_TRIDENT_INDEX),
        },
        FeatureRow {
            key: "misc.ladder_lock",
            setting: Some(super::camera::LADDER_BODY_LOCK_INDEX),
        },
        FeatureRow {
            key: "misc.better_combat",
            setting: None,
        },
        FeatureRow {
            key: "misc.spatial_gui",
            setting: None,
        },
    ];

    /// How many features show at once, the Java `MAX_VISIBLE_FEATURES`.
    pub const MAX_VISIBLE_FEATURES: usize = 4;
    /// How many camera sliders show at once, the Java
    /// `MAX_VISIBLE_CAMERA_SLIDERS`.
    pub const MAX_VISIBLE_CAMERA_SLIDERS: usize = 4;
    /// How many shortcuts show at once, the Java `MAX_VISIBLE_SHORTCUTS`.
    pub const MAX_VISIBLE_SHORTCUTS: usize = 10;
    /// One feature row's height, the Java `FEATURE_HEIGHT`.
    pub const FEATURE_HEIGHT: f32 = 42.0;
    /// The gap between feature rows, the Java `FEATURE_GAP`.
    pub const FEATURE_GAP: f32 = 4.0;
    /// One camera slider's height, the Java `CAMERA_SLIDER_HEIGHT`.
    pub const CAMERA_SLIDER_HEIGHT: f32 = 34.0;
    /// The gap between camera sliders, the Java `CAMERA_SLIDER_GAP`.
    pub const CAMERA_SLIDER_GAP: f32 = 2.0;
    /// The scrollbar's gutter, the Java `SCROLLBAR_GUTTER`.
    pub const SCROLLBAR_GUTTER: f32 = 14.0;
    /// The window's title bar, the Java `TITLE_H`.
    pub const CAMERA_TITLE_H: i32 = 33;

    /// One settable row the screen shows: its label key, its value as shown, and
    /// what a click does.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Stepper {
        pub label: String,
        pub value: String,
        pub setting: Option<usize>,
    }

    /// The shoulder camera's screen: its page, its focus and its scroll positions.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ShoulderCameraUiState {
        open: bool,
        page: CameraPage,
        focus: usize,
        slider_scroll: usize,
        feature_scroll: usize,
        shortcut_scroll: usize,
    }

    impl Default for ShoulderCameraUiState {
        fn default() -> Self {
            Self {
                open: false,
                page: CameraPage::Camera,
                focus: 0,
                slider_scroll: 0,
                feature_scroll: 0,
                shortcut_scroll: 0,
            }
        }
    }

    impl ShoulderCameraUiState {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn is_open(&self) -> bool {
            self.open
        }

        pub fn open(&mut self) {
            self.open = true;
        }

        pub fn close(&mut self) {
            self.open = false;
        }

        pub fn toggle(&mut self) -> bool {
            self.open = !self.open;
            self.open
        }

        /// The page being shown.
        pub fn page(&self) -> CameraPage {
            self.page
        }

        /// Shows `page`, putting its focus back on the first row.
        pub fn select(&mut self, page: CameraPage) -> CameraPage {
            self.page = page;
            self.focus = 0;
            self.page
        }

        /// Shows the next page.
        pub fn next_tab(&mut self) -> CameraPage {
            let page = self.page.next();
            self.select(page)
        }

        /// Shows the previous page.
        pub fn previous_tab(&mut self) -> CameraPage {
            let page = self.page.previous();
            self.select(page)
        }

        /// The focused row.
        pub fn focus(&self) -> usize {
            self.focus
        }

        /// Focuses a row (clamped to the page's rows when they are known).
        pub fn focus_at(&mut self, index: usize, rows: usize) -> usize {
            self.focus = if rows == 0 { 0 } else { index.min(rows - 1) };
            self.focus
        }

        /// Steps the focus down or up the page's rows, wrapping.
        pub fn nudge_focus(&mut self, delta: i32, rows: usize) -> usize {
            if rows == 0 {
                self.focus = 0;
            } else {
                let rows = rows as i32;
                self.focus = (self.focus as i32 + delta).rem_euclid(rows) as usize;
            }
            self.focus
        }

        /// The camera page's scroll, and its furthest value.
        pub fn slider_scroll(&self) -> usize {
            self.slider_scroll
        }

        pub fn slider_scroll_max(&self) -> usize {
            CAMERA_SLIDERS
                .len()
                .saturating_sub(MAX_VISIBLE_CAMERA_SLIDERS)
        }

        /// The misc page's scroll, and its furthest value.
        pub fn feature_scroll(&self) -> usize {
            self.feature_scroll
        }

        pub fn feature_scroll_max(&self) -> usize {
            FEATURES.len().saturating_sub(MAX_VISIBLE_FEATURES)
        }

        /// The shortcuts page's scroll, and its furthest value.
        pub fn shortcut_scroll(&self) -> usize {
            self.shortcut_scroll
        }

        pub fn shortcut_scroll_max(&self) -> usize {
            Self::shortcut_count().saturating_sub(MAX_VISIBLE_SHORTCUTS)
        }

        /// The shortcuts the screen lists: the four fixed keys, then one per
        /// emote slot, the Java `EmoteKeybindManager.shortcuts`.
        pub fn shortcut_count() -> usize {
            4 + MAX_EMOTES
        }

        /// Scrolls a page by `delta` rows, clamped, the Java mouse wheel.
        pub fn scroll(&mut self, delta: i32) {
            match self.page {
                CameraPage::Camera => {
                    let max = self.slider_scroll_max();
                    self.slider_scroll = clamp_scroll(self.slider_scroll, delta, max);
                }
                CameraPage::Misc => {
                    let max = self.feature_scroll_max();
                    self.feature_scroll = clamp_scroll(self.feature_scroll, delta, max);
                }
                CameraPage::Shortcuts => {
                    let max = self.shortcut_scroll_max();
                    self.shortcut_scroll = clamp_scroll(self.shortcut_scroll, delta, max);
                }
            }
        }

        /// The rows of the page being shown.
        pub fn steppers(&self, settings: &ShoulderCameraSettings) -> Vec<Stepper> {
            let on = |b: bool| {
                if b {
                    "punchy_moves.ui.on"
                } else {
                    "punchy_moves.ui.off"
                }
            };
            let row = |label: &str, value: String, setting: Option<usize>| Stepper {
                label: label.to_string(),
                value,
                setting,
            };
            match self.page {
                CameraPage::Camera => {
                    let mut rows = vec![row(
                        "punchy_moves.ui.custom_camera",
                        on(settings.enabled).into(),
                        Some(super::camera::ENABLED_INDEX),
                    )];
                    for slider in CAMERA_SLIDERS {
                        let value = match slider.setting {
                            super::camera::HORIZONTAL_INDEX => {
                                format!("{:.2}", settings.horizontal)
                            }
                            super::camera::VERTICAL_INDEX => format!("{:.2}", settings.vertical),
                            super::camera::DEPTH_INDEX => format!("{:.2}", settings.depth),
                            super::camera::FOV_INDEX => format!("{:.0}", settings.fov),
                            super::camera::YAW_INDEX => format!("{:.0}", settings.rotation_yaw),
                            _ => format!("{:.0}", settings.rotation_pitch),
                        };
                        rows.push(row(slider.label, value, Some(slider.setting)));
                    }
                    rows
                }
                CameraPage::Misc => FEATURES
                    .iter()
                    .map(|feature| {
                        let enabled = feature
                            .setting
                            .is_some_and(|index| settings.click_value(index));
                        row(
                            &format!("punchy_moves.{}.title", feature.key),
                            on(enabled).into(),
                            feature.setting,
                        )
                    })
                    .collect(),
                CameraPage::Shortcuts => vec![
                    row("punchy_moves.key.camera_toggle_v", "rebind".into(), None),
                    row("punchy_moves.key.camera_editor", "rebind".into(), None),
                    row("punchy_moves.key.free_camera", "rebind".into(), None),
                    row("punchy_moves.key.emote_menu", "rebind".into(), None),
                ],
            }
        }

        /// The rows of the shortcuts page with the emote slots' keys read from
        /// `keys`: the four fixed shortcuts, then one row per emote slot, the
        /// Java `EmoteKeybindManager.shortcuts`.
        pub fn shortcuts(&self, keys: &EmoteKeybindManager) -> Vec<Stepper> {
            let mut rows = self.steppers(&ShoulderCameraSettings::default());
            rows.truncate(4);
            for index in 0..MAX_EMOTES {
                let emote = emotes().get(index).copied();
                let name = emote.map_or("punchy_moves.ui.emote", |e| e.label);
                rows.push(Stepper {
                    label: format!("punchy_moves.ui.emote {}", name),
                    value: keys.slot(index).unwrap_or("unbound").to_string(),
                    setting: None,
                });
            }
            rows
        }

        /// A click on row `index` of the page being shown: changes the setting or
        /// asks the game to do something only it can.
        pub fn click(&mut self, settings: &mut ShoulderCameraSettings, index: usize) -> UiRequest {
            let rows = self.steppers(settings);
            let Some(row) = rows.get(index) else {
                return UiRequest::None;
            };
            self.focus = index;
            if let Some(setting) = row.setting {
                settings.click(setting);
            }
            match self.page {
                CameraPage::Shortcuts => match index {
                    0 => UiRequest::RebindKey(super::camera::TOGGLE_CAMERA),
                    1 => UiRequest::RebindKey(super::camera::TOGGLE_EDITOR),
                    2 => UiRequest::RebindKey(super::camera::TOGGLE_ORBIT),
                    _ => UiRequest::RebindKey(EMOTE_WHEEL_KEY),
                },
                _ => UiRequest::None,
            }
        }

        /// Draws the page: the window with its rail and divider, the page header,
        /// the rows, and the scrollbars.
        pub fn draw(
            &self,
            layout: &UiLayout,
            settings: &ShoulderCameraSettings,
            theme: &UiTheme,
            frame: &mut HudFrame,
        ) {
            if !self.open {
                return;
            }
            let rows = self.steppers(settings);
            let panel = frame.panel(layout, PANEL_WIDTH, PANEL_HEIGHT, theme);
            let title_h = 33.0;
            super::ui::gold::window(
                frame,
                panel.x as i32,
                panel.y as i32,
                panel.right() as i32,
                panel.bottom() as i32,
                CAMERA_TITLE_H,
                super::ui::gold::PANE,
            );
            // The left rail and the divider groove.
            let divider = panel.x + 46.0;
            frame.rect(
                UiRect::new(
                    panel.x + 1.0,
                    panel.y + title_h,
                    divider - panel.x - 1.0,
                    panel.h - title_h - 1.0,
                ),
                super::ui::gold::RAIL,
            );
            super::ui::gold::vgroove(
                frame,
                divider as i32,
                panel.y as i32 + CAMERA_TITLE_H,
                panel.bottom() as i32 - 1,
            );
            // The navigation tabs, down the rail.
            let tab_h = 30.0f32;
            for (index, page) in CameraPage::ALL.iter().enumerate() {
                let y = panel.y + 61.0 + index as f32 * 34.0;
                super::ui::gold::tab(
                    frame,
                    (panel.x + 6.0) as i32,
                    y as i32,
                    (divider - panel.x - 12.0) as i32,
                    tab_h as i32,
                    page.label(),
                    *page == self.page,
                    index == self.focus && *page != self.page,
                );
            }
            // The page header and the groove under it.
            frame.text(
                self.page.label(),
                divider + 12.0,
                panel.y + 46.0,
                super::ui::gold::TEXT_1,
            );
            super::ui::gold::groove(
                frame,
                (divider + 12.0) as i32,
                panel.right() as i32 - 12,
                panel.y as i32 + 61,
            );
            // The rows of the page, from its scroll.
            let content_x = divider + 12.0;
            let content_w = panel.right() - content_x - 12.0;
            let visible = match self.page {
                CameraPage::Camera => MAX_VISIBLE_CAMERA_SLIDERS,
                CameraPage::Misc => MAX_VISIBLE_FEATURES,
                CameraPage::Shortcuts => MAX_VISIBLE_SHORTCUTS,
            };
            let scroll = match self.page {
                CameraPage::Camera => self.slider_scroll,
                CameraPage::Misc => self.feature_scroll,
                CameraPage::Shortcuts => self.shortcut_scroll,
            };
            let row_h = match self.page {
                CameraPage::Camera => CAMERA_SLIDER_HEIGHT + CAMERA_SLIDER_GAP,
                CameraPage::Misc => FEATURE_HEIGHT + FEATURE_GAP,
                CameraPage::Shortcuts => 16.0,
            };
            let mut y = panel.y + 70.0 + 44.0;
            for (index, row) in rows.iter().enumerate().skip(scroll).take(visible) {
                let focused = index == self.focus;
                if self.page == CameraPage::Misc {
                    if focused {
                        frame.rect(
                            UiRect::new(content_x, y, content_w, FEATURE_HEIGHT),
                            super::ui::gold::ROW_HOVER,
                        );
                    }
                    frame.text(
                        row.label.as_str(),
                        content_x + 8.0,
                        y + 8.0,
                        super::ui::gold::TEXT_1,
                    );
                    frame.text(
                        row.value.as_str(),
                        content_x + 8.0,
                        y + 20.0,
                        super::ui::gold::TEXT_2,
                    );
                    let toggle_x = (content_x + content_w - 30.0 - 8.0) as i32;
                    let enabled = row.setting.is_some_and(|s| settings.click_value(s));
                    super::ui::gold::toggle(
                        frame,
                        toggle_x,
                        y as i32 + (FEATURE_HEIGHT as i32 - 16) / 2,
                        enabled,
                        focused,
                    );
                    super::ui::gold::groove(
                        frame,
                        content_x as i32,
                        (content_x + content_w) as i32,
                        (y + FEATURE_HEIGHT + 1.0) as i32,
                    );
                } else if self.page == CameraPage::Camera {
                    frame.text(row.label.as_str(), content_x, y, super::ui::gold::TEXT_1);
                    frame.text(
                        row.value.as_str(),
                        content_x + content_w - 40.0,
                        y,
                        super::ui::gold::TEXT_2,
                    );
                    if let Some(setting) = row.setting {
                        let slider = CAMERA_SLIDERS
                            .iter()
                            .find(|slider| slider.setting == setting);
                        if let Some(slider) = slider {
                            let value = (settings_value(settings, setting) - slider.min)
                                / (slider.max - slider.min);
                            let zero = if slider.min < 0.0 && slider.max > 0.0 {
                                -slider.min / (slider.max - slider.min)
                            } else {
                                -1.0
                            };
                            let default = (slider.default.clamp(slider.min, slider.max)
                                - slider.min)
                                / (slider.max - slider.min);
                            super::ui::gold::slider(
                                frame,
                                content_x as i32,
                                (content_x + content_w - SCROLLBAR_GUTTER) as i32,
                                (y + 20.0) as i32,
                                value.clamp(0.0, 1.0),
                                zero,
                                default,
                                focused,
                                false,
                            );
                        }
                    }
                } else {
                    if focused {
                        frame.rect(
                            UiRect::new(content_x, y, content_w, row_h),
                            super::ui::gold::ROW_HOVER,
                        );
                    }
                    frame.text(
                        row.label.as_str(),
                        content_x + 4.0,
                        y + 3.0,
                        super::ui::gold::TEXT_1,
                    );
                    super::ui::gold::keycap(
                        frame,
                        (content_x + content_w * 0.62) as i32,
                        y as i32,
                        (content_w * 0.3) as i32,
                        14,
                        row.value.as_str(),
                    );
                }
                y += row_h;
            }
            // The scrollbars, when a page scrolls.
            match self.page {
                CameraPage::Camera if self.slider_scroll_max() > 0 => {
                    let (top, bottom, thumb) = scrollbar_geometry(
                        panel.y + 70.0 + 44.0,
                        panel.bottom() - 38.0,
                        CAMERA_SLIDERS.len(),
                        MAX_VISIBLE_CAMERA_SLIDERS,
                        self.slider_scroll,
                    );
                    super::ui::gold::scrollbar(
                        frame,
                        (content_x + content_w - 9.0) as i32,
                        top,
                        bottom,
                        8,
                        thumb,
                        thumb
                            + thumb_height(
                                CAMERA_SLIDERS.len(),
                                MAX_VISIBLE_CAMERA_SLIDERS,
                                bottom - top,
                            ),
                        false,
                    );
                }
                CameraPage::Misc if self.feature_scroll_max() > 0 => {
                    let (top, bottom, thumb) = scrollbar_geometry(
                        panel.y + 70.0,
                        panel.bottom() - 38.0,
                        FEATURES.len(),
                        MAX_VISIBLE_FEATURES,
                        self.feature_scroll,
                    );
                    super::ui::gold::scrollbar(
                        frame,
                        (content_x + content_w - 9.0) as i32,
                        top,
                        bottom,
                        8,
                        thumb,
                        thumb + thumb_height(FEATURES.len(), MAX_VISIBLE_FEATURES, bottom - top),
                        false,
                    );
                }
                _ => {}
            }
            frame.footer("punchy_moves.ui.settings_hint", panel, theme);
        }
    }

    /// The value a settings row reads, for the screen's rows.
    fn settings_value(settings: &ShoulderCameraSettings, index: usize) -> f32 {
        match index {
            super::camera::HORIZONTAL_INDEX => settings.horizontal,
            super::camera::VERTICAL_INDEX => settings.vertical,
            super::camera::DEPTH_INDEX => settings.depth,
            super::camera::FOV_INDEX => settings.fov,
            super::camera::YAW_INDEX => settings.rotation_yaw,
            super::camera::PITCH_INDEX => settings.rotation_pitch,
            _ => 0.0,
        }
    }

    /// Whether the settings row `index` reads on, for the toggle rows.
    trait ClickValue {
        fn click_value(&self, index: usize) -> bool;
    }

    impl ClickValue for ShoulderCameraSettings {
        fn click_value(&self, index: usize) -> bool {
            match index {
                super::camera::ENABLED_INDEX => self.enabled,
                super::camera::FENCE_INDEX => self.fence_camera,
                super::camera::LADDER_INDEX => self.ladder_camera,
                super::camera::PROXIMITY_INDEX => self.proximity_camera,
                super::camera::ELYTRA_INDEX => self.elytra_camera,
                super::camera::MAP_INDEX => self.map_camera,
                super::camera::BOW_AND_CROSSBOW_INDEX => self.bow_and_crossbow_camera_fix,
                super::camera::SPEAR_AND_TRIDENT_INDEX => self.spear_and_trident_camera_fix,
                super::camera::LADDER_BODY_LOCK_INDEX => self.ladder_body_lock,
                _ => false,
            }
        }
    }

    /// Steps a scroll by `delta` rows (positive moves down the list), clamped to
    /// `0..=max`; the Java wheel clamps the same way.
    fn clamp_scroll(current: usize, delta: i32, max: usize) -> usize {
        (current as i32 + delta).clamp(0, max as i32) as usize
    }

    /// A scrollbar's track and thumb top, the Java `Scrollbar` geometry.
    fn scrollbar_geometry(
        top: f32,
        bottom: f32,
        count: usize,
        visible: usize,
        scroll: usize,
    ) -> (i32, i32, i32) {
        let top = top as i32;
        let bottom = bottom as i32;
        let max = count.saturating_sub(visible).max(1);
        let track = (bottom - top).max(1);
        let thumb = thumb_height(count, visible, track);
        let movable = (track - thumb).max(1);
        let at = top + (movable as f32 * scroll as f32 / max as f32).round() as i32;
        (top, bottom, at)
    }

    /// A scrollbar's thumb height, the Java `max(18, track * visible / count)`.
    fn thumb_height(count: usize, visible: usize, track: i32) -> i32 {
        ((track as usize * visible / count.max(1)) as i32).max(18)
    }

    /// How wide a settings panel is, in GUI pixels.
    pub const PANEL_WIDTH: f32 = 300.0;
    /// How tall a settings panel is, in GUI pixels.
    pub const PANEL_HEIGHT: f32 = 170.0;

    // ---------------------------------------------------------------------------
    // The colour picker
    // ---------------------------------------------------------------------------

    /// The swatches the picker offers, as straight RGB bytes.
    pub const SWATCHES: [[u8; 3]; 8] = [
        [255, 255, 255],
        [30, 30, 34],
        [214, 64, 64],
        [236, 152, 60],
        [238, 214, 88],
        [92, 190, 108],
        [86, 148, 232],
        [178, 116, 214],
    ];

    /// The picker's channels, in focus order: hue, saturation, brightness.
    pub const COLOR_CHANNELS: usize = 3;

    /// The colour square's side, the Java `SQUARE`.
    pub const PICKER_SQUARE: i32 = 128;
    /// The hue bar's width, the Java `BAR`.
    pub const PICKER_BAR: i32 = 12;
    /// The gap between the square and the bar, the Java `GAP`.
    pub const PICKER_GAP: i32 = 6;
    /// The picker window's widest, the Java `min(width - 12, 334)`.
    pub const PICKER_MAX_W: f32 = 334.0;
    /// Its tallest, the Java `min(height - 12, 217)`.
    pub const PICKER_MAX_H: f32 = 217.0;

    /// A colour picker over one RGB colour, the Java `ColorPickerScreen`'s
    /// model: a hue bar, a saturation-brightness square, a hex field and
    /// presets.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ColorPickerState {
        open: bool,
        /// The hue, `0.0..=1.0` round the wheel.
        hue: f32,
        /// The saturation, `0.0..=1.0`.
        saturation: f32,
        /// The brightness, `0.0..=1.0`.
        brightness: f32,
        /// The colour the picker opened with, RGB bytes.
        original: [u8; 3],
        /// Which picker part the pointer drags: 1 the square, 2 the bar.
        dragging: i32,
        focus: usize,
    }

    impl Default for ColorPickerState {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ColorPickerState {
        pub fn new() -> Self {
            Self {
                open: false,
                hue: 0.0,
                saturation: 0.0,
                brightness: 0.0,
                original: [0, 0, 0],
                dragging: 0,
                focus: 0,
            }
        }

        pub fn is_open(&self) -> bool {
            self.open
        }

        /// Puts the picker up over `rgb`, remembering it as the colour the
        /// cancel returns to, the Java constructor.
        pub fn open_over(&mut self, rgb: [u8; 3]) {
            self.open = true;
            self.original = rgb;
            self.dragging = 0;
            let [h, s, v] = rgb_to_hsv([
                rgb[0] as f32 / 255.0,
                rgb[1] as f32 / 255.0,
                rgb[2] as f32 / 255.0,
            ]);
            self.hue = h / 360.0;
            self.saturation = s;
            self.brightness = v;
        }

        pub fn open(&mut self) {
            self.open_over(self.rgb());
        }

        /// Closes, handing back the colour it opened with (the Java cancel).
        pub fn close(&mut self) -> [u8; 3] {
            self.open = false;
            self.dragging = 0;
            self.original
        }

        pub fn toggle(&mut self) -> bool {
            self.open = !self.open;
            self.open
        }

        /// The colour as RGB bytes, the Java `rgb()`.
        pub fn rgb(&self) -> [u8; 3] {
            let [r, g, b] = hsv_to_rgb([self.hue * 360.0, self.saturation, self.brightness]);
            [
                (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                (b.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]
        }

        /// Sets the colour from RGB bytes, the Java `set`: the hue only moves
        /// when the new colour has some saturation, so greys keep their hue.
        pub fn set_rgb(&mut self, rgb: [u8; 3]) {
            let [h, s, v] = rgb_to_hsv([
                rgb[0] as f32 / 255.0,
                rgb[1] as f32 / 255.0,
                rgb[2] as f32 / 255.0,
            ]);
            if s > 0.0 {
                self.hue = h / 360.0;
            }
            self.saturation = s;
            self.brightness = v;
        }

        /// The hue, `0.0..=1.0` round the wheel.
        pub fn hue(&self) -> f32 {
            self.hue
        }

        /// The saturation, `0.0..=1.0`.
        pub fn saturation(&self) -> f32 {
            self.saturation
        }

        /// The brightness, `0.0..=1.0`.
        pub fn brightness(&self) -> f32 {
            self.brightness
        }

        /// The colour as hue (degrees), saturation and value.
        pub fn hsv(&self) -> [f32; 3] {
            [self.hue * 360.0, self.saturation, self.brightness]
        }

        /// Sets the colour from hue (degrees), saturation and value.
        pub fn set_hsv(&mut self, hsv: [f32; 3]) {
            self.hue = (hsv[0] / 360.0).rem_euclid(1.0);
            self.saturation = hsv[1].clamp(0.0, 1.0);
            self.brightness = hsv[2].clamp(0.0, 1.0);
        }

        /// The colour as `#RRGGBB`, the Java `hexOf`.
        pub fn hex(&self) -> String {
            let [r, g, b] = self.rgb();
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        }

        /// Sets the colour from `#RRGGBB` or `RRGGBB`; `false` when the text is
        /// not a colour (the colour is left alone), the Java hex responder.
        pub fn set_hex(&mut self, text: &str) -> bool {
            let text = text.trim().trim_start_matches('#');
            if text.len() != 6 || !text.chars().all(|c| c.is_ascii_hexdigit()) {
                return false;
            }
            let channel = |at: usize| u8::from_str_radix(&text[at..at + 2], 16).ok();
            match (channel(0), channel(2), channel(4)) {
                (Some(r), Some(g), Some(b)) => {
                    self.set_rgb([r, g, b]);
                    true
                }
                _ => false,
            }
        }

        /// The colour the picker opened with.
        pub fn original(&self) -> [u8; 3] {
            self.original
        }

        /// The picker part the pointer drags: 0 none, 1 the square, 2 the bar.
        pub fn dragging(&self) -> i32 {
            self.dragging
        }

        /// The focused channel (0 hue, 1 saturation, 2 brightness).
        pub fn focus(&self) -> usize {
            self.focus
        }

        /// Focuses a channel, wrapping.
        pub fn focus_at(&mut self, channel: usize) -> usize {
            self.focus = channel % COLOR_CHANNELS;
            self.focus
        }

        /// Steps the focus to the next or previous channel.
        pub fn nudge_focus(&mut self, delta: i32) -> usize {
            let channels = COLOR_CHANNELS as i32;
            self.focus = (self.focus as i32 + delta).rem_euclid(channels) as usize;
            self.focus
        }

        /// Moves the focused channel by `delta`: hue takes degrees, saturation
        /// and brightness take fractions.
        pub fn step(&mut self, delta: f32) {
            if !delta.is_finite() {
                return;
            }
            match self.focus {
                0 => self.hue = (self.hue + delta / 360.0).rem_euclid(1.0),
                1 => self.saturation = (self.saturation + delta).clamp(0.0, 1.0),
                _ => self.brightness = (self.brightness + delta).clamp(0.0, 1.0),
            }
        }

        /// Takes a swatch's colour, the Java preset clicks.
        pub fn select_swatch(&mut self, index: usize) {
            if let Some(swatch) = SWATCHES.get(index) {
                self.set_rgb(*swatch);
            }
        }

        /// The swatches.
        pub fn swatches(&self) -> [[u8; 3]; 8] {
            SWATCHES
        }

        /// The colour square's rect on the layout, the Java `squareX`/`squareY`.
        pub fn square(&self, layout: &UiLayout) -> UiRect {
            let panel = UiRect::new(
                (layout.width - PICKER_MAX_W) * 0.5,
                (layout.height - PICKER_MAX_H) * 0.5,
                PICKER_MAX_W,
                PICKER_MAX_H,
            );
            UiRect::new(
                panel.x + 10.0,
                panel.y + 29.0 + 8.0,
                PICKER_SQUARE as f32,
                PICKER_SQUARE as f32,
            )
        }

        /// The hue bar's rect, the Java `barX`.
        pub fn bar(&self, layout: &UiLayout) -> UiRect {
            let square = self.square(layout);
            UiRect::new(
                square.right() + PICKER_GAP as f32,
                square.y,
                PICKER_BAR as f32,
                PICKER_SQUARE as f32,
            )
        }

        /// A click or drag at `(x, y)`: picks from the square or the bar, the
        /// Java `pickAt`. `start` begins a drag; `true` when it landed somewhere.
        pub fn pick_at(&mut self, layout: &UiLayout, x: f32, y: f32, start: bool) -> bool {
            let square = self.square(layout);
            let bar = self.bar(layout);
            let on_square =
                x >= square.x && x < square.right() && y >= square.y && y < square.bottom();
            let on_bar =
                x >= bar.x - 2.0 && x < bar.right() + 2.0 && y >= square.y && y < square.bottom();
            if start {
                self.dragging = if on_square {
                    1
                } else if on_bar {
                    2
                } else {
                    0
                };
            }
            if self.dragging == 1 {
                self.saturation = clamp01((x - square.x) / (PICKER_SQUARE - 1) as f32);
                self.brightness = 1.0 - clamp01((y - square.y) / (PICKER_SQUARE - 1) as f32);
            } else if self.dragging == 2 {
                self.hue = clamp01((y - square.y) / (PICKER_SQUARE - 1) as f32);
                // A pure white picked off the corner gets some colour back.
                if self.saturation < 0.05 && self.brightness > 0.95 {
                    self.saturation = 0.6;
                }
            } else {
                return false;
            }
            true
        }

        /// Lets go of the drag.
        pub fn release(&mut self) {
            self.dragging = 0;
        }

        /// Draws the picker: the window, the hue bar's six segments, the square,
        /// the sample swatches and the hex line.
        pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
            if !self.open {
                return;
            }
            let panel = frame.panel(layout, PICKER_MAX_W, PICKER_MAX_H, theme);
            super::ui::gold::window(
                frame,
                panel.x as i32,
                panel.y as i32,
                panel.right() as i32,
                panel.bottom() as i32,
                29,
                super::ui::gold::PANE,
            );
            frame.text(
                "punchy_moves.ui.face_colour",
                panel.x + 12.0,
                panel.y + 10.0,
                super::ui::gold::TITLE_INK,
            );
            let square = self.square(layout);
            let bar = self.bar(layout);
            // The square, one column per x in the hue, darkening downward (the
            // HUD has no gradient, so the shade steps in eighths).
            let [r, g, b] = hsv_to_rgb([self.hue * 360.0, 1.0, 1.0]);
            let hue_color = rgba((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255);
            frame.rect(square, hue_color);
            for step in 0..8 {
                let shade = 1.0 - step as f32 / 8.0;
                frame.rect(
                    UiRect::new(
                        square.x,
                        square.y + step as f32 * (square.h / 8.0),
                        square.w,
                        square.h / 8.0,
                    ),
                    rgba(
                        (r * shade * 255.0) as u8,
                        (g * shade * 255.0) as u8,
                        (b * shade * 255.0) as u8,
                        255,
                    ),
                );
            }
            super::ui::gold::outline(
                frame,
                square.x as i32 - 1,
                square.y as i32 - 1,
                square.right() as i32 + 1,
                square.bottom() as i32 + 1,
                super::ui::gold::WINDOW_EDGE,
            );
            // The cursor on the square.
            let cursor_x = square.x + self.saturation * (PICKER_SQUARE - 1) as f32;
            let cursor_y = square.y + (1.0 - self.brightness) * (PICKER_SQUARE - 1) as f32;
            let cursor_ink = if self.brightness > 0.5 {
                super::ui::gold::TEXT_DIS
            } else {
                super::ui::gold::TEXT_1
            };
            super::ui::gold::outline(
                frame,
                cursor_x as i32 - 3,
                cursor_y as i32 - 3,
                cursor_x as i32 + 4,
                cursor_y as i32 + 4,
                cursor_ink,
            );
            // The hue bar, six segments, the Java loop.
            for segment in 0..6 {
                let [r, g, b] = hsv_to_rgb([(segment as f32 + 0.5) / 6.0 * 360.0, 1.0, 1.0]);
                let y1 = square.y + segment as f32 * (square.h / 6.0);
                let y2 = square.y + (segment + 1) as f32 * (square.h / 6.0);
                frame.rect(
                    UiRect::new(bar.x, y1, bar.w, y2 - y1),
                    rgba((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255),
                );
            }
            super::ui::gold::outline(
                frame,
                bar.x as i32 - 1,
                square.y as i32 - 1,
                bar.right() as i32 + 1,
                square.bottom() as i32 + 1,
                super::ui::gold::WINDOW_EDGE,
            );
            // The hue cursor.
            let hue_y = square.y + self.hue * (PICKER_SQUARE - 1) as f32;
            frame.rect(
                UiRect::new(bar.x - 2.0, hue_y - 1.0, bar.w + 4.0, 2.0),
                super::ui::gold::TEXT_1,
            );
            // The sample, the original and the hex well.
            let sample_x = bar.right() + PICKER_GAP as f32 + 4.0;
            let sample_w = panel.right() - 10.0 - sample_x;
            let [r, g, b] = self.rgb();
            frame.rect(
                UiRect::new(sample_x, square.y, sample_w, 18.0),
                rgba(r, g, b, 255),
            );
            let [r, g, b] = self.original;
            frame.rect(
                UiRect::new(sample_x, square.y + 18.0, sample_w, 16.0),
                rgba(r, g, b, 255),
            );
            super::ui::gold::outline(
                frame,
                sample_x as i32 - 1,
                square.y as i32 - 1,
                (sample_x + sample_w) as i32 + 1,
                square.y as i32 + 35,
                super::ui::gold::WINDOW_EDGE,
            );
            frame.rect(
                UiRect::new(sample_x, square.y + 40.0, sample_w, 18.0),
                super::ui::gold::WELL,
            );
            frame.text(
                self.hex(),
                sample_x + 4.0,
                square.y + 46.0,
                super::ui::gold::TEXT_1,
            );
            // The presets.
            for (index, swatch) in SWATCHES.iter().enumerate() {
                let x = sample_x + (index % 4) as f32 * 15.0;
                let y = square.y + 64.0 + (index / 4) as f32 * 15.0;
                frame.rect(
                    UiRect::new(x, y, 12.0, 12.0),
                    rgba(swatch[0], swatch[1], swatch[2], 255),
                );
                super::ui::gold::outline(
                    frame,
                    x as i32 - 1,
                    y as i32 - 1,
                    x as i32 + 13,
                    y as i32 + 13,
                    if *swatch == self.rgb() {
                        super::ui::gold::GOLD_200
                    } else {
                        super::ui::gold::WINDOW_EDGE
                    },
                );
            }
            let _ = theme;
        }
    }

    /// Clamps to `0.0..=1.0`, the Java `clamp`.
    fn clamp01(value: f32) -> f32 {
        value.clamp(0.0, 1.0)
    }

    /// RGB in `0.0..=1.0` to hue in degrees, saturation and value.
    pub fn rgb_to_hsv(rgb: [f32; 3]) -> [f32; 3] {
        let (r, g, b) = (
            rgb[0].clamp(0.0, 1.0),
            rgb[1].clamp(0.0, 1.0),
            rgb[2].clamp(0.0, 1.0),
        );
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let hue = if delta <= f32::EPSILON {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        [
            hue.rem_euclid(360.0),
            if max <= 0.0 { 0.0 } else { delta / max },
            max,
        ]
    }

    /// Hue in degrees, saturation and value to RGB in `0.0..=1.0`.
    pub fn hsv_to_rgb(hsv: [f32; 3]) -> [f32; 3] {
        let hue = hsv[0].rem_euclid(360.0);
        let s = hsv[1].clamp(0.0, 1.0);
        let v = hsv[2].clamp(0.0, 1.0);
        let c = v * s;
        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match (hue / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        [r + m, g + m, b + m]
    }

    // ---------------------------------------------------------------------------
    // The face canvas
    // ---------------------------------------------------------------------------

    /// The cue fields the canvas edits, in focus order.
    pub const CANVAS_FIELDS: usize = 5;

    /// The face canvas: a cue's fields and a live preview of the face.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FaceCanvasState {
        open: bool,
        expression: FaceExpression,
        cue: FaceCue,
        focus: usize,
    }

    impl Default for FaceCanvasState {
        fn default() -> Self {
            Self {
                open: false,
                expression: FaceExpression::Happy,
                cue: FaceExpression::Happy.cue(),
                focus: 0,
            }
        }
    }

    impl FaceCanvasState {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn is_open(&self) -> bool {
            self.open
        }

        pub fn open(&mut self) {
            self.open = true;
        }

        pub fn close(&mut self) {
            self.open = false;
        }

        pub fn toggle(&mut self) -> bool {
            self.open = !self.open;
            self.open
        }

        /// The expression being previewed.
        pub fn expression(&self) -> FaceExpression {
            self.expression
        }

        /// Shows `expression`'s built-in cue for editing.
        pub fn select_expression(&mut self, expression: FaceExpression) -> FaceExpression {
            self.expression = expression;
            self.cue = expression.cue();
            self.focus = 0;
            self.expression
        }

        /// The cue being edited.
        pub fn cue(&self) -> FaceCue {
            self.cue
        }

        /// Replaces the cue.
        pub fn set_cue(&mut self, cue: FaceCue) {
            self.expression = cue.expression;
            self.cue = cue;
        }

        /// The focused field.
        pub fn focus(&self) -> usize {
            self.focus
        }

        /// Focuses a field, wrapping.
        pub fn focus_at(&mut self, field: usize) -> usize {
            self.focus = field % CANVAS_FIELDS;
            self.focus
        }

        /// Steps the focus to the next or previous field.
        pub fn nudge_focus(&mut self, delta: i32) -> usize {
            let fields = CANVAS_FIELDS as i32;
            self.focus = (self.focus as i32 + delta).rem_euclid(fields) as usize;
            self.focus
        }

        /// Moves the focused field by `delta` and returns the cue.
        pub fn step(&mut self, delta: f32) -> FaceCue {
            if !delta.is_finite() {
                return self.cue;
            }
            match self.focus {
                0 => self.cue.lid = (self.cue.lid + delta).clamp(0.0, 1.0),
                1 => self.cue.brow = (self.cue.brow + delta).clamp(-0.6, 0.6),
                2 => self.cue.mouth = (self.cue.mouth + delta).clamp(0.0, 1.0),
                3 => self.cue.widen = (self.cue.widen + delta).clamp(0.0, 1.0),
                _ => self.cue.seconds = (self.cue.seconds + delta).clamp(0.0, 10.0),
            }
            self.cue
        }

        /// The face the cue draws right now.
        pub fn preview(&self, settings: &FaceSettings) -> FaceQuads {
            let mut rig = FaceRig::new(1);
            rig.play_cue(self.cue);
            rig.quads(settings)
        }

        /// Draws the canvas: the expression, its five fields, and the preview's
        /// quads as a flat sketch of the face.
        pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
            if !self.open {
                return;
            }
            let panel = frame.panel(layout, PANEL_WIDTH, PANEL_HEIGHT, theme);
            frame.title("Face canvas", panel, theme);
            let rows = [
                ("Lids", format!("{:.0}%", self.cue.lid * 100.0)),
                ("Brow", format!("{:.2} rad", self.cue.brow)),
                ("Mouth", format!("{:.0}%", self.cue.mouth * 100.0)),
                ("Eyes wide", format!("{:.0}%", self.cue.widen * 100.0)),
                ("Lasts", format!("{:.1} s", self.cue.seconds)),
            ];
            frame.row(
                panel,
                0,
                "Expression",
                &format!("{:?}", self.expression),
                false,
                theme,
            );
            for (index, (label, value)) in rows.iter().enumerate() {
                frame.row(panel, index + 1, label, value, index == self.focus, theme);
            }
            // A sketch of the preview in the panel's right half: each quad a box.
            let settings = FaceSettings::default();
            let quads = self.preview(&settings);
            let face = UiRect::new(panel.right() - 110.0, panel.bottom() - 96.0, 96.0, 84.0);
            frame.rect(face, theme.panel);
            for quad in quads.iter() {
                let center = quad.center;
                let rect = UiRect::new(
                    face.x + face.w * 0.5 + center[0] * face.w,
                    face.y + face.h * 0.5 - center[1] * face.h,
                    quad.size[0] * face.w,
                    quad.size[1] * face.h,
                );
                let color = rgba(
                    (quad.color[0] * 255.0) as u8,
                    (quad.color[1] * 255.0) as u8,
                    (quad.color[2] * 255.0) as u8,
                    (quad.color[3].clamp(0.4, 1.0) * 255.0) as u8,
                );
                frame.rect(rect, color);
            }
            frame.footer(
                "Left and right change the row; Up and down change it",
                panel,
                theme,
            );
        }
    }

    // ---------------------------------------------------------------------------
    // The tutorial
    // ---------------------------------------------------------------------------

    /// One key hint of a step: which key mapping's suffix to look up, and the
    /// label shown beside it, the Java `KeyHint`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KeyHint {
        /// The key mapping's name suffix, the part after `key.punchy_moves.`.
        pub mapping: &'static str,
        /// The label key beside the keycap.
        pub label: &'static str,
    }

    /// One step of the tutorial: its area, its key hints, and the tab its open
    /// button shows, the Java `Step` record.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TutorialStep {
        /// The area key, shown small and uppercase above the title.
        pub area: &'static str,
        /// The key hints, wrapped left to right.
        pub keys: &'static [KeyHint],
        /// The camera page's tab the open button shows, if any.
        pub tab: Option<&'static str>,
        /// The open button's label key, if the step has one.
        pub open_label: Option<&'static str>,
    }

    /// The tutorial's step count, the Java `STEPS`.
    pub const TUTORIAL_STEPS: usize = 7;

    /// The tutorial's steps, in order, the Java `GUIDE`.
    pub const GUIDE: [TutorialStep; TUTORIAL_STEPS] = [
        TutorialStep {
            area: "guide",
            keys: &[],
            tab: None,
            open_label: None,
        },
        TutorialStep {
            area: "camera",
            keys: &[
                KeyHint {
                    mapping: "camera_toggle_v",
                    label: "key.camera",
                },
                KeyHint {
                    mapping: "free_camera",
                    label: "key.orbit",
                },
            ],
            tab: None,
            open_label: None,
        },
        TutorialStep {
            area: "emotes",
            keys: &[KeyHint {
                mapping: "emote_menu",
                label: "key.emotes",
            }],
            tab: None,
            open_label: None,
        },
        TutorialStep {
            area: "settings",
            keys: &[KeyHint {
                mapping: "camera_editor",
                label: "key.settings",
            }],
            tab: Some("camera"),
            open_label: Some("open.settings"),
        },
        TutorialStep {
            area: "camera_settings",
            keys: &[],
            tab: Some("camera"),
            open_label: Some("open.camera"),
        },
        TutorialStep {
            area: "camera_settings",
            keys: &[],
            tab: Some("camera"),
            open_label: Some("open.camera"),
        },
        TutorialStep {
            area: "shortcuts",
            keys: &[],
            tab: Some("shortcuts"),
            open_label: Some("open.shortcuts"),
        },
    ];

    /// The tutorial window's widest, in GUI pixels, the Java `MAX_W`.
    pub const TUTORIAL_MAX_W: f32 = 440.0;
    /// Its shortest, the Java `MIN_H`.
    pub const TUTORIAL_MIN_H: f32 = 180.0;
    /// The title bar's height, the Java `TITLE_BAR`.
    pub const TUTORIAL_TITLE_BAR: i32 = 29;
    /// The padding round the body, the Java `PAD`.
    pub const TUTORIAL_PAD: i32 = 12;
    /// The footer's height, the Java `FOOT`.
    pub const TUTORIAL_FOOT: i32 = 38;
    /// A button's height, the Java `BUTTON_H`.
    pub const TUTORIAL_BUTTON_H: i32 = 22;
    /// A text line's height, the Java `LINE`.
    pub const TUTORIAL_LINE: i32 = 11;
    /// A keycap's height, the Java `KEY_H`.
    pub const TUTORIAL_KEY_H: i32 = 16;
    /// A progress segment's height, the Java `SEGMENT_H`.
    pub const TUTORIAL_SEGMENT_H: i32 = 6;

    /// The tutorial: which step it is on, and whether it is up.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct TutorialState {
        open: bool,
        step: usize,
    }

    impl TutorialState {
        pub fn new() -> Self {
            Self::default()
        }

        /// Puts the tutorial up at its first step.
        pub fn start(&mut self) {
            self.open = true;
            self.step = 0;
        }

        pub fn is_open(&self) -> bool {
            self.open
        }

        /// The step being shown.
        pub fn step(&self) -> usize {
            self.step
        }

        /// The step being shown, if the tutorial is up.
        pub fn current(&self) -> Option<TutorialStep> {
            if self.open {
                GUIDE.get(self.step).copied()
            } else {
                None
            }
        }

        /// Jumps to a step (the progress track's clicks); past the end closes.
        pub fn go(&mut self, step: usize) -> bool {
            if step >= TUTORIAL_STEPS {
                self.close();
                self.step = 0;
                return true;
            }
            self.step = step;
            false
        }

        /// Moves on; `true` when the last step is done and the tutorial closed.
        // A step forward, not an iterator's item: the name reads as the page does.
        #[allow(clippy::should_implement_trait)]
        pub fn next(&mut self) -> bool {
            self.go(self.step + 1)
        }

        /// Goes back; `false` at the first step.
        pub fn back(&mut self) -> bool {
            if self.step == 0 {
                false
            } else {
                self.step -= 1;
                true
            }
        }

        /// Skips the tutorial out (the Java `skip` button, and `onClose`).
        pub fn close(&mut self) {
            self.open = false;
        }

        /// The x of the left edge of progress segment `index`, the Java
        /// `segmentX`: the track runs the panel's inner width, each segment
        /// overlapping the next by two pixels.
        pub fn segment_x(&self, panel: UiRect, index: usize) -> f32 {
            panel.x
                + TUTORIAL_PAD as f32
                + index as f32
                    * ((panel.w - TUTORIAL_PAD as f32 * 2.0 + 2.0) / TUTORIAL_STEPS as f32)
        }

        /// The progress segment a click at `(x, y)` lands on, or `None`, the
        /// Java `segmentAt`.
        pub fn segment_at(&self, panel: UiRect, x: f32, y: f32) -> Option<usize> {
            let track_y = panel.y + TUTORIAL_TITLE_BAR as f32 + 8.0;
            if y < track_y - 3.0 || y >= track_y + TUTORIAL_SEGMENT_H as f32 + 3.0 {
                return None;
            }
            for index in 0..TUTORIAL_STEPS {
                let start = self.segment_x(panel, index);
                let end = self.segment_x(panel, index + 1) - 2.0;
                if x >= start && x < end {
                    return Some(index);
                }
            }
            None
        }

        /// Draws the step: the window, the step counter, the progress track, the
        /// area, the title, the keycaps and the body, and the footer groove.
        pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
            let Some(step) = self.current() else {
                return;
            };
            let _ = layout;
            let width = TUTORIAL_MAX_W.min(theme_width(panel_width_cap()));
            let height = TUTORIAL_MIN_H;
            let panel = frame.panel(layout, width, height, theme);
            super::ui::gold::window(
                frame,
                panel.x as i32,
                panel.y as i32,
                panel.right() as i32,
                panel.bottom() as i32,
                TUTORIAL_TITLE_BAR,
                super::ui::gold::PANE,
            );
            // The step counter, top right; the window title, top left.
            let counter = format!(
                "punchy_moves.tutorial.step {}/{}",
                self.step + 1,
                TUTORIAL_STEPS
            );
            let counter_width = counter.chars().count() as f32 * GLYPH_WIDTH;
            super::ui::gold::draw_ink(
                frame,
                &counter,
                (panel.right() - TUTORIAL_PAD as f32 - counter_width) as i32,
                panel.y as i32 + 10,
                super::ui::gold::TITLE_INK,
            );
            super::ui::gold::draw_ink(
                frame,
                "punchy_moves.tutorial.window",
                panel.x as i32 + TUTORIAL_PAD,
                panel.y as i32 + 10,
                super::ui::gold::TITLE_INK,
            );
            // The progress track: one segment per step, the passed ones gold.
            let track_y = panel.y + TUTORIAL_TITLE_BAR as f32 + 8.0;
            for index in 0..TUTORIAL_STEPS {
                let start = self.segment_x(panel, index);
                let end = self.segment_x(panel, index + 1) - 2.0;
                let passed = index <= self.step;
                super::ui::gold::outline(
                    frame,
                    start as i32,
                    track_y as i32,
                    end as i32,
                    (track_y + TUTORIAL_SEGMENT_H as f32) as i32,
                    super::ui::gold::WINDOW_EDGE,
                );
                frame.rect(
                    UiRect::new(start + 1.0, track_y + 1.0, end - start - 2.0, 1.0),
                    if passed {
                        super::ui::gold::GOLD_300
                    } else {
                        super::ui::gold::TRACK_LIGHT
                    },
                );
                frame.rect(
                    UiRect::new(
                        start + 1.0,
                        track_y + 2.0,
                        end - start - 2.0,
                        TUTORIAL_SEGMENT_H as f32 - 2.0,
                    ),
                    if passed {
                        super::ui::gold::GOLD_500
                    } else {
                        super::ui::gold::TRACK
                    },
                );
            }
            // The area, then the title.
            let mut body_y = track_y + TUTORIAL_SEGMENT_H as f32 + 12.0;
            frame.text(
                step.area.to_uppercase(),
                panel.x + TUTORIAL_PAD as f32,
                body_y,
                super::ui::gold::GOLD_300,
            );
            body_y += 14.0;
            frame.text(
                step_title(self.step),
                panel.x + TUTORIAL_PAD as f32,
                body_y,
                super::ui::gold::TEXT_1,
            );
            body_y += TUTORIAL_LINE as f32;
            // The keycaps, then the body.
            if !step.keys.is_empty() {
                body_y += 5.0;
                let mut x = panel.x + TUTORIAL_PAD as f32;
                for hint in step.keys {
                    let cap_w = 22.0f32
                        .max((hint.mapping.chars().count() as f32 * GLYPH_WIDTH + 12.0).min(120.0));
                    super::ui::gold::keycap(
                        frame,
                        x as i32,
                        body_y as i32,
                        cap_w as i32,
                        TUTORIAL_KEY_H,
                        hint.mapping,
                    );
                    frame.text(
                        hint.label,
                        x + cap_w + 6.0,
                        body_y + 4.0,
                        super::ui::gold::TEXT_2,
                    );
                    x += cap_w + 6.0 + hint.label.chars().count() as f32 * GLYPH_WIDTH + 16.0;
                }
                body_y += TUTORIAL_KEY_H as f32 + 8.0;
            }
            frame.text(
                step_body(self.step),
                panel.x + TUTORIAL_PAD as f32,
                body_y,
                super::ui::gold::TEXT_2,
            );
            // The footer groove and the step's buttons, as labels.
            super::ui::gold::groove(
                frame,
                panel.x as i32 + 1,
                panel.right() as i32 - 1,
                (panel.bottom() - TUTORIAL_FOOT as f32) as i32,
            );
            let finish = self.step + 1 >= TUTORIAL_STEPS;
            let next_label = if finish {
                "punchy_moves.tutorial.finish"
            } else {
                "punchy_moves.tutorial.next"
            };
            let next_w = 64.0f32.max(next_label.chars().count() as f32 * GLYPH_WIDTH + 24.0);
            super::ui::gold::button(
                frame,
                (panel.right() - TUTORIAL_PAD as f32 - next_w) as i32,
                (panel.bottom() - TUTORIAL_FOOT as f32 + 9.0) as i32,
                next_w as i32,
                TUTORIAL_BUTTON_H,
                next_label,
                true,
                false,
                true,
            );
            if self.step > 0 {
                let back_w = 64.0f32
                    .max("punchy_moves.tutorial.back".chars().count() as f32 * GLYPH_WIDTH + 24.0);
                super::ui::gold::button(
                    frame,
                    (panel.right() - TUTORIAL_PAD as f32 - next_w - 6.0 - back_w) as i32,
                    (panel.bottom() - TUTORIAL_FOOT as f32 + 9.0) as i32,
                    back_w as i32,
                    TUTORIAL_BUTTON_H,
                    "punchy_moves.tutorial.back",
                    false,
                    false,
                    true,
                );
            }
            if !finish {
                frame.text(
                    "punchy_moves.tutorial.skip",
                    panel.x + TUTORIAL_PAD as f32 - 4.0,
                    panel.bottom() - TUTORIAL_FOOT as f32 + 16.0,
                    super::ui::gold::TEXT_3,
                );
            }
        }
    }

    /// The step's title line, the Java `title.<step>` key.
    fn step_title(step: usize) -> String {
        let area = GUIDE.get(step).map_or("guide", |s| s.area);
        format!("punchy_moves.tutorial.title.{}", area)
    }

    /// The step's body line, the Java `body.<step>` key.
    fn step_body(step: usize) -> String {
        let area = GUIDE.get(step).map_or("guide", |s| s.area);
        format!("punchy_moves.tutorial.body.{}", area)
    }

    /// The widest a panel may be, in GUI pixels (the HUD caps its own width).
    fn panel_width_cap() -> f32 {
        480.0
    }

    /// The width a panel of `want` takes under the cap.
    fn theme_width(want: f32) -> f32 {
        want.min(panel_width_cap())
    }

    // ---------------------------------------------------------------------------
    // The near-camera dither
    // ---------------------------------------------------------------------------

    /// The 4x4 ordered-dither matrix the entity shader carries, in reading order:
    /// the Java `PUNCHY_ORDER`, each of `0..=15` once.
    pub const PUNCHY_ORDER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

    /// The threshold the entity shader discards at, the fixed `>= 6` of
    /// `entity_dither.fsh`.
    pub const PUNCHY_THRESHOLD: u8 = 6;

    /// The threshold of a screen pixel, `0..=15`, the shader's
    /// `PUNCHY_ORDER[punchyPixel.y * 4 + punchyPixel.x]`.
    pub fn punchy_order(x: u32, y: u32) -> u8 {
        PUNCHY_ORDER[((y & 3) * 4 + (x & 3)) as usize]
    }

    /// Whether the entity shader discards the pixel: the fixed
    /// `PUNCHY_ORDER[...] >= 6` of `entity_dither.fsh`, every entity, every frame.
    pub fn punchy_discard(x: u32, y: u32) -> bool {
        punchy_order(x, y) >= PUNCHY_THRESHOLD
    }

    /// Whether a screen pixel draws at `alpha`: a pixel whose threshold is below
    /// the alpha shows, so a dithered shape covers the share of the screen its
    /// alpha asks for. Reads the same 4x4 order the shader does.
    pub fn dither(x: u32, y: u32, alpha: f32) -> bool {
        if !alpha.is_finite() || alpha <= 0.0 {
            return false;
        }
        if alpha >= 1.0 {
            return true;
        }
        f32::from(punchy_order(x, y)) < alpha * 16.0
    }

    /// How the mod fades its own geometry out as it comes close to the camera.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct DitherVisibility {
        /// Inside this many blocks the geometry is gone.
        pub near: f32,
        /// Over this many blocks past `near` it comes back.
        pub fade: f32,
    }

    impl Default for DitherVisibility {
        fn default() -> Self {
            Self {
                near: 0.35,
                fade: 0.45,
            }
        }
    }

    impl DitherVisibility {
        pub fn new() -> Self {
            Self::default()
        }

        /// The alpha at `distance` blocks from the eye: `0.0` inside the near
        /// distance, rising to `1.0` at the far end of the fade.
        pub fn alpha(&self, distance: f32) -> f32 {
            if !distance.is_finite() {
                return 1.0;
            }
            if self.fade <= 0.0 {
                return if distance >= self.near { 1.0 } else { 0.0 };
            }
            ((distance - self.near) / self.fade).clamp(0.0, 1.0)
        }

        /// Whether a pixel at `distance` draws at its screen position.
        pub fn visible(&self, x: u32, y: u32, distance: f32) -> bool {
            dither(x, y, self.alpha(distance))
        }
    }

    // ---------------------------------------------------------------------------
    // Every screen together
    // ---------------------------------------------------------------------------

    /// A screen Options > Mods can open.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Screen {
        EmoteWheel,
        Camera,
        ColorPicker,
        FaceCanvas,
        Tutorial,
    }

    /// The buttons Options > Mods shows for the mod's own screens, in order.
    pub fn settings_buttons() -> [(&'static str, Screen); 5] {
        [
            ("Emote wheel", Screen::EmoteWheel),
            ("Camera settings", Screen::Camera),
            ("Face colour", Screen::ColorPicker),
            ("Face canvas", Screen::FaceCanvas),
            ("Tutorial", Screen::Tutorial),
        ]
    }

    /// Every screen's state, on one value so a hook opens one at a time.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PunchyUi {
        pub emote_wheel: EmoteWheelState,
        pub emote_keys: EmoteKeybindManager,
        pub camera: ShoulderCameraUiState,
        pub color: ColorPickerState,
        pub canvas: FaceCanvasState,
        pub tutorial: TutorialState,
        pub dither: DitherVisibility,
    }

    impl Default for PunchyUi {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PunchyUi {
        pub fn new() -> Self {
            Self {
                emote_wheel: EmoteWheelState::new(),
                emote_keys: EmoteKeybindManager::new(),
                camera: ShoulderCameraUiState::new(),
                color: ColorPickerState::new(),
                canvas: FaceCanvasState::new(),
                tutorial: TutorialState::new(),
                dither: DitherVisibility::new(),
            }
        }

        /// Whether any screen is up.
        pub fn any_open(&self) -> bool {
            self.emote_wheel.is_open()
                || self.camera.is_open()
                || self.color.is_open()
                || self.canvas.is_open()
                || self.tutorial.is_open()
        }

        /// Takes every screen down.
        pub fn close_all(&mut self) {
            self.emote_wheel.cancel();
            self.camera.close();
            self.color.close();
            self.canvas.close();
            self.tutorial.close();
        }

        /// Puts one screen up (the others stay as they are, the new one on top).
        pub fn open_screen(&mut self, screen: Screen) {
            match screen {
                Screen::EmoteWheel => self.emote_wheel.open(),
                Screen::Camera => self.camera.open(),
                Screen::ColorPicker => self.color.open(),
                Screen::FaceCanvas => self.canvas.open(),
                Screen::Tutorial => self.tutorial.start(),
            }
        }

        /// A click on the mod's own buttons on Options > Mods; `index` counts from
        /// the first of [`settings_buttons`].
        pub fn click_settings(&mut self, index: usize) -> UiRequest {
            let Some((_, screen)) = settings_buttons().get(index).copied() else {
                return UiRequest::None;
            };
            self.open_screen(screen);
            UiRequest::None
        }

        /// A key press while a screen is up. `None` when no screen wanted the key,
        /// so the game may still see it.
        pub fn key(&mut self, key: &str, pressed: bool) -> Option<UiRequest> {
            if !pressed {
                // Only the emote wheel's own key cares about a release.
                if key == super::camera::TOGGLE_ORBIT {
                    return None;
                }
                return None;
            }
            if self.emote_wheel.is_open() {
                return Some(self.wheel_key(key));
            }
            if self.tutorial.is_open() {
                return Some(match key {
                    "key.keyboard.enter" | "key.keyboard.space" | "key.keyboard.right" => {
                        if self.tutorial.next() {
                            UiRequest::CloseAll
                        } else {
                            UiRequest::None
                        }
                    }
                    "key.keyboard.backspace" | "key.keyboard.left" => {
                        self.tutorial.back();
                        UiRequest::None
                    }
                    "key.keyboard.escape" => {
                        self.tutorial.close();
                        UiRequest::None
                    }
                    _ => return None,
                });
            }
            if self.camera.is_open() {
                let rows = self
                    .camera
                    .steppers(&ShoulderCameraSettings::default())
                    .len();
                return Some(match key {
                    "key.keyboard.tab" => {
                        self.camera.next_tab();
                        UiRequest::None
                    }
                    "key.keyboard.escape" => {
                        self.camera.close();
                        UiRequest::None
                    }
                    "key.keyboard.up" => {
                        self.camera.nudge_focus(-1, rows);
                        UiRequest::None
                    }
                    "key.keyboard.down" => {
                        self.camera.nudge_focus(1, rows);
                        UiRequest::None
                    }
                    "key.keyboard.enter" => {
                        let mut settings = ShoulderCameraSettings::default();
                        self.camera.click(&mut settings, self.camera.focus())
                    }
                    _ => return None,
                });
            }
            if self.color.is_open() {
                return Some(match key {
                    "key.keyboard.escape" => {
                        self.color.close();
                        UiRequest::None
                    }
                    "key.keyboard.left" => {
                        self.color.nudge_focus(-1);
                        UiRequest::None
                    }
                    "key.keyboard.right" => {
                        self.color.nudge_focus(1);
                        UiRequest::None
                    }
                    "key.keyboard.up" => {
                        self.color.step(0.05);
                        UiRequest::None
                    }
                    "key.keyboard.down" => {
                        self.color.step(-0.05);
                        UiRequest::None
                    }
                    _ => return None,
                });
            }
            if self.canvas.is_open() {
                return Some(match key {
                    "key.keyboard.escape" => {
                        self.canvas.close();
                        UiRequest::None
                    }
                    "key.keyboard.left" => {
                        self.canvas.nudge_focus(-1);
                        UiRequest::None
                    }
                    "key.keyboard.right" => {
                        self.canvas.nudge_focus(1);
                        UiRequest::None
                    }
                    "key.keyboard.up" => {
                        self.canvas.step(0.05);
                        UiRequest::None
                    }
                    "key.keyboard.down" => {
                        self.canvas.step(-0.05);
                        UiRequest::None
                    }
                    _ => return None,
                });
            }
            None
        }

        /// A key while the wheel is up: a number picks a wedge, the arrows turn
        /// the wheel, Enter plays and Escape puts it away.
        fn wheel_key(&mut self, key: &str) -> UiRequest {
            if key == "key.keyboard.escape" {
                self.emote_wheel.cancel();
                return UiRequest::None;
            }
            if key == "key.keyboard.enter" || key == "key.keyboard.space" {
                let selected = self.emote_wheel.selected();
                if self.emote_wheel.close().is_some() {
                    return UiRequest::PlayEmote(selected);
                }
                return UiRequest::None;
            }
            if let Some(digit) = key.strip_prefix("key.keyboard.").filter(|k| k.len() == 1)
                && let Ok(number) = digit.parse::<usize>()
            {
                // 1 is the first wedge, 0 the tenth (there are at most eight).
                let index = if number == 0 { 9 } else { number - 1 };
                if index < MAX_EMOTES {
                    self.emote_wheel.select(index);
                }
                return UiRequest::None;
            }
            match key {
                "key.keyboard.right" | "key.keyboard.d" => {
                    self.emote_wheel.nudge(1);
                }
                "key.keyboard.left" | "key.keyboard.a" => {
                    self.emote_wheel.nudge(-1);
                }
                _ => {}
            }
            UiRequest::None
        }

        /// Draws every open screen, the tutorial last so it stays on top.
        pub fn draw(
            &self,
            layout: &UiLayout,
            settings: &ShoulderCameraSettings,
            theme: &UiTheme,
        ) -> HudFrame {
            let mut frame = HudFrame::new();
            self.emote_wheel.draw(layout, theme, &mut frame);
            self.camera.draw(layout, settings, theme, &mut frame);
            self.color.draw(layout, theme, &mut frame);
            self.canvas.draw(layout, theme, &mut frame);
            self.tutorial.draw(layout, theme, &mut frame);
            frame
        }
    }

    thread_local! {
        /// Every screen's state, per thread (the client draws on one).
        static UI: RefCell<PunchyUi> = RefCell::new(PunchyUi::new());
    }

    /// Runs `f` with this thread's screens.
    pub fn ui<R>(f: impl FnOnce(&mut PunchyUi) -> R) -> R {
        UI.with_borrow_mut(f)
    }

    /// Puts every screen and the emote bindings back to how they start.
    pub fn reset_ui() {
        UI.with_borrow_mut(|ui| *ui = PunchyUi::new());
    }

    // ---------------------------------------------------------------------------
    // Signals for the screens
    // ---------------------------------------------------------------------------

    /// The signals the screens need while they are up: a screen holds still, so
    /// the body and the face keep their resting pose.
    pub fn resting_signals() -> MotionSignals {
        MotionSignals::new()
    }

    #[cfg(test)]
    mod tests {
        use super::super::faces::FaceExpression;
        use super::*;

        fn theme() -> UiTheme {
            UiTheme::default()
        }

        fn layout() -> UiLayout {
            UiLayout::new(320.0, 240.0)
        }

        #[test]
        fn colours_pack_the_way_the_hud_packs_them() {
            assert_eq!(rgba(1, 2, 3, 255), 0xFF03_0201);
            assert_eq!(rgba(255, 0, 0, 255), 0xFF00_00FF);
            assert_eq!(rgba(0, 0, 0, 0), 0);
        }

        #[test]
        fn rectangles_and_layout_keep_to_the_screen() {
            let rect = UiRect::new(10.0, 20.0, 100.0, 50.0);
            assert_eq!(rect.center(), [60.0, 45.0]);
            assert_eq!((rect.right(), rect.bottom()), (110.0, 70.0));
            assert!(rect.contains(10.0, 20.0));
            assert!(!rect.contains(9.0, 20.0));
            assert_eq!(rect.inset(5.0), UiRect::new(15.0, 25.0, 90.0, 40.0));
            // An inset larger than the rectangle never inverts it.
            assert_eq!(rect.inset(100.0).w, 0.0);

            let layout = layout();
            assert_eq!(layout.center(), [160.0, 120.0]);
            let panel = layout.panel(200.0, 100.0);
            assert_eq!(panel, UiRect::new(60.0, 70.0, 200.0, 100.0));
            // A panel larger than the screen fills it.
            assert_eq!(
                layout.panel(1000.0, 1000.0),
                UiRect::new(0.0, 0.0, 320.0, 240.0)
            );

            // Rows stack inside the panel without leaving it.
            let panel = layout.panel(PANEL_WIDTH, PANEL_HEIGHT);
            for index in 0..4 {
                let row = layout.row(panel, index);
                assert!(row.y >= panel.y);
                assert!(row.bottom() <= panel.bottom());
                assert!(row.x >= panel.x && row.right() <= panel.right());
            }
        }

        #[test]
        fn a_frame_records_what_it_draws_in_order() {
            let mut frame = HudFrame::new();
            assert!(frame.is_empty());
            frame.rect(UiRect::new(0.0, 0.0, 1.0, 1.0), rgba(1, 2, 3, 255));
            frame.quad([[0.0; 2]; 4], 0);
            frame.text("hi", 0.0, 0.0, 0xFFFF_FFFF);
            frame.plain("plain", 0.0, 8.0, 0xFFFF_FFFF);
            assert_eq!(frame.len(), 4);
            assert!(matches!(frame.draws()[0], HudDraw::Rect(..)));
            assert!(matches!(frame.draws()[1], HudDraw::Quad(..)));
            assert!(matches!(
                &frame.draws()[2],
                HudDraw::Text { shadow: true, .. }
            ));
            assert!(matches!(
                &frame.draws()[3],
                HudDraw::Text { shadow: false, .. }
            ));
        }

        #[test]
        fn a_panel_draws_a_wash_a_fill_and_a_border() {
            let mut frame = HudFrame::new();
            let theme = theme();
            let panel = frame.panel(&layout(), 200.0, 100.0, &theme);
            // The wash, the fill and four border edges.
            assert_eq!(frame.len(), 6);
            assert!(panel.w > 0.0 && panel.h > 0.0);
            frame.title("Title", panel, &theme);
            frame.row(panel, 0, "Label", "Value", true, &theme);
            frame.footer("Hint", panel, &theme);
            // The wash, the panel, four edges, the title, the row's highlight and
            // its two texts, and the footer.
            assert_eq!(frame.len(), 11);
        }

        #[test]
        fn the_wheel_carries_eight_emotes_and_wraps() {
            let mut wheel = EmoteWheelState::new();
            assert_eq!(wheel.emotes().len(), MAX_EMOTES);
            assert!(!wheel.is_open());
            assert_eq!(wheel.selected(), 0);
            assert_eq!(wheel.emote().unwrap().id, "wave");

            wheel.open();
            assert!(wheel.is_open());
            assert_eq!(wheel.nudge(1), 1);
            assert_eq!(wheel.nudge(-2), MAX_EMOTES - 1);
            assert_eq!(wheel.select(3), 3);
            assert_eq!(wheel.select(MAX_EMOTES + 2), 2);

            // Closing hands back what a release would play.
            let played = wheel.close().expect("an emote");
            assert_eq!(played.id, wheel.emotes()[2].id);
            assert!(!wheel.is_open());
            assert_eq!(wheel.close(), None);

            // Cancelling plays nothing.
            wheel.open();
            wheel.cancel();
            assert!(!wheel.is_open());
            assert_eq!(wheel.close(), None);

            // An empty wheel has nothing to pick.
            let mut empty = EmoteWheelState::with_emotes(Vec::new());
            assert_eq!(empty.selected(), 0);
            assert_eq!(empty.emote(), None);
            assert_eq!(empty.nudge(3), 0);
        }

        #[test]
        fn pointing_picks_the_tile_under_the_offset() {
            let mut wheel = EmoteWheelState::new();
            // Four tiles a page: up, right, down, left.
            assert_eq!(wheel.pointing(0.0, -100.0), 0);
            assert_eq!(wheel.pointing(100.0, 0.0), 1);
            assert_eq!(wheel.pointing(0.0, 100.0), 2);
            assert_eq!(wheel.pointing(-100.0, 0.0), 3);
            // No offset keeps the selection, and a broken one does not move it.
            wheel.select(3);
            assert_eq!(wheel.pointing(0.0, 0.0), 3);
            assert_eq!(wheel.pointing(f32::NAN, 1.0), 3);
            // A diagonal keeps the nearest axis.
            assert_eq!(wheel.pointing(100.0, -100.0), 0);
        }

        #[test]
        fn the_wheel_pages_four_tiles_at_a_time() {
            let mut wheel = EmoteWheelState::new();
            assert_eq!(wheel.pages(), 2, "eight emotes, four a page");
            // The page's tiles sit up, right, down, left round the middle.
            let l = layout();
            let [cx, cy] = l.center();
            let top = wheel.tile_rect(&l, 0);
            assert!(top.center()[1] < cy);
            assert!((top.center()[0] - cx).abs() < 1e-3);
            let right = wheel.tile_rect(&l, 1);
            assert!(right.center()[0] > cx);
            let bottom = wheel.tile_rect(&l, 2);
            assert!(bottom.center()[1] > cy);
            let left = wheel.tile_rect(&l, 3);
            assert!(left.center()[0] < cx);
            // Selecting an emote turns to its page.
            wheel.select(5);
            assert_eq!(wheel.page(), 1);
            assert_eq!(wheel.selected(), 5);
            // Paging wraps and stays in range.
            assert_eq!(wheel.turn_page(1), 0);
            assert_eq!(wheel.turn_page(-1), 1);
            wheel.set_page(99);
            assert_eq!(wheel.page(), 1);
            // A pointer over a tile of the shown page picks it.
            let rect = wheel.tile_rect(&l, 2);
            let [x, y] = rect.center();
            assert_eq!(wheel.pointing_at(&l, x, y), Some(6));
            // A pointer off the tiles keeps the selection.
            assert_eq!(wheel.pointing_at(&l, 2.0, 2.0), None);
            // An empty wheel still has one page.
            let empty = EmoteWheelState::with_emotes(Vec::new());
            assert_eq!(empty.pages(), 1);
        }

        #[test]
        fn keybinds_find_their_emotes_and_round_trip() {
            let mut keys = EmoteKeybindManager::new();
            assert_eq!(keys.slot(0), None);
            assert!(keys.bind(0, "key.keyboard.1"));
            assert!(keys.bind(3, "key.keyboard.4"));
            assert_eq!(keys.slot(0), Some("key.keyboard.1"));
            assert_eq!(keys.index_of_key("key.keyboard.4"), Some(3));
            assert_eq!(keys.index_of_key("key.keyboard.9"), None);

            // A blank key unbinds; an out-of-range slot is refused.
            assert!(keys.bind(0, "  "));
            assert_eq!(keys.slot(0), None);
            assert!(!keys.bind(MAX_EMOTES, "key.keyboard.1"));
            keys.unbind(3);
            assert_eq!(keys.slot(3), None);
            assert!(keys.bind(MAX_EMOTES - 1, "key.keyboard.unknown"));
            assert_eq!(keys.slot(MAX_EMOTES - 1), None);

            // The labels name every slot, bound or not.
            assert_eq!(keys.labels().len(), MAX_EMOTES);
            assert!(keys.labels()[0].contains("unbound"));

            // What it stores reads back the same bindings.
            keys.bind(1, "key.keyboard.g");
            let stored = keys.stored();
            assert_eq!(stored.len(), MAX_EMOTES);
            let read = EmoteKeybindManager::read(|key| {
                stored
                    .iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| value.clone())
            });
            assert_eq!(read.slot(1), Some("key.keyboard.g"));
            assert_eq!(read.slot(0), None);
        }

        #[test]
        fn playing_an_emote_feeds_the_engine_and_the_face() {
            super::super::engine::with_engine(|engine| {
                engine.update(77, super::super::signals::MotionSignals::new(), 0.05);
            });
            let emote = emotes()[0];
            assert!(play_emote(Some(77), emote));
            super::super::engine::with_engine(|engine| {
                let state = engine.state(77).unwrap();
                assert_eq!(state.face.expression(), emote.expression);
                assert_eq!(state.items.emote(), Some(emote.action));
            });
            // An emote needs a player the engine knows.
            assert!(!play_emote(Some(4242), emote));
            assert!(!play_emote(None, emote));
        }

        #[test]
        fn the_camera_screen_has_three_pages() {
            let settings = ShoulderCameraSettings::default();
            let mut page = ShoulderCameraUiState::new();
            assert!(!page.is_open());
            assert_eq!(page.page(), CameraPage::Camera);

            // Every page names rows, and its label is unique.
            for tab in CameraPage::ALL {
                page.select(tab);
                let rows = page.steppers(&settings);
                assert!(!rows.is_empty(), "{:?} has no rows", tab);
                assert!(!tab.label().is_empty());
            }
            assert_eq!(CameraPage::Camera.next(), CameraPage::Misc);
            assert_eq!(CameraPage::Camera.previous(), CameraPage::Shortcuts);
            assert_eq!(CameraPage::Shortcuts.next(), CameraPage::Camera);
            assert_eq!(CameraPage::ALL.len(), 3);

            // The camera page carries the on/off row and the six sliders.
            page.select(CameraPage::Camera);
            assert_eq!(page.steppers(&settings).len(), 7);
            assert_eq!(CAMERA_SLIDERS.len(), 6);
            assert_eq!(CAMERA_SLIDERS[0].default, 1.17);
            assert_eq!(CAMERA_SLIDERS[1].default, -0.11);
            assert_eq!(CAMERA_SLIDERS[2].default, 2.5);
            assert_eq!(CAMERA_SLIDERS[3].default, 70.0);
            assert_eq!(
                (CAMERA_SLIDERS[3].min, CAMERA_SLIDERS[3].max),
                (30.0, 110.0)
            );

            // The misc page carries the ten features, four at a time.
            page.select(CameraPage::Misc);
            assert_eq!(page.steppers(&settings).len(), 10);
            assert_eq!(MAX_VISIBLE_FEATURES, 4);
            assert_eq!(page.feature_scroll_max(), 6);
            assert_eq!(
                FEATURES[8].setting, None,
                "better_combat has no Horizon setting"
            );
            assert_eq!(
                FEATURES[9].setting, None,
                "spatial_gui has no Horizon setting"
            );

            // The shortcuts page asks for the four keys.
            page.select(CameraPage::Shortcuts);
            assert_eq!(page.steppers(&settings).len(), 4);

            // The focus stays inside the rows it walks.
            page.focus_at(99, 3);
            assert_eq!(page.focus(), 2);
            assert_eq!(page.nudge_focus(1, 3), 0);
            assert_eq!(page.nudge_focus(-1, 3), 2);
            page.focus_at(0, 0);
            assert_eq!(page.focus(), 0);
        }

        #[test]
        fn clicking_the_camera_screen_changes_its_settings() {
            let mut settings = ShoulderCameraSettings::default();
            let mut page = ShoulderCameraUiState::new();
            page.select(CameraPage::Camera);

            // The first row is the camera's on/off switch.
            assert_eq!(page.click(&mut settings, 0), UiRequest::None);
            assert!(!settings.enabled);
            page.click(&mut settings, 0);
            assert!(settings.enabled);

            // A feature row toggles its own flag; the compat rows do nothing.
            page.select(CameraPage::Misc);
            page.click(&mut settings, 0);
            assert!(!settings.fence_camera);
            assert_eq!(page.click(&mut settings, 8), UiRequest::None);

            // The shortcuts page asks for the four keys.
            page.select(CameraPage::Shortcuts);
            assert_eq!(
                page.click(&mut settings, 0),
                UiRequest::RebindKey(super::super::camera::TOGGLE_CAMERA)
            );
            assert_eq!(
                page.click(&mut settings, 1),
                UiRequest::RebindKey(super::super::camera::TOGGLE_EDITOR)
            );
            assert_eq!(
                page.click(&mut settings, 2),
                UiRequest::RebindKey(super::super::camera::TOGGLE_ORBIT)
            );
            assert_eq!(
                page.click(&mut settings, 3),
                UiRequest::RebindKey(EMOTE_WHEEL_KEY)
            );
            // A row past the page does nothing.
            assert_eq!(page.click(&mut settings, 99), UiRequest::None);
        }

        #[test]
        fn the_camera_screen_scrolls_its_pages() {
            let mut page = ShoulderCameraUiState::new();
            page.select(CameraPage::Misc);
            assert_eq!(page.feature_scroll_max(), 6);
            page.scroll(1);
            assert_eq!(page.feature_scroll(), 1);
            for _ in 0..10 {
                page.scroll(1);
            }
            assert_eq!(page.feature_scroll(), 6);
            for _ in 0..10 {
                page.scroll(-1);
            }
            assert_eq!(page.feature_scroll(), 0);
            page.select(CameraPage::Camera);
            assert_eq!(page.slider_scroll_max(), 2);
            page.scroll(5);
            assert_eq!(page.slider_scroll(), 2);
            page.select(CameraPage::Shortcuts);
            // The four fixed keys plus one row per emote slot, ten at a time.
            assert_eq!(ShoulderCameraUiState::shortcut_count(), 4 + MAX_EMOTES);
            assert_eq!(page.shortcut_scroll_max(), 2);
            page.scroll(3);
            assert_eq!(page.shortcut_scroll(), 2);
        }

        #[test]
        fn the_camera_page_draws_its_tabs_and_rows() {
            let mut page = ShoulderCameraUiState::new();
            let mut frame = HudFrame::new();
            page.draw(
                &layout(),
                &ShoulderCameraSettings::default(),
                &theme(),
                &mut frame,
            );
            assert!(frame.is_empty());

            page.open();
            page.draw(
                &layout(),
                &ShoulderCameraSettings::default(),
                &theme(),
                &mut frame,
            );
            assert!(frame.len() > 6);
            // The tabs' labels are drawn on the header strip.
            let labels = frame
                .draws()
                .iter()
                .filter_map(|draw| match draw {
                    HudDraw::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for tab in CameraPage::ALL {
                assert!(labels.contains(&tab.label()), "{} missing", tab.label());
            }
        }

        #[test]
        fn the_colour_picker_moves_through_rgb_hsv_and_hex() {
            let mut picker = ColorPickerState::new();
            picker.set_hex("#336699");
            assert_eq!(picker.rgb(), [0x33, 0x66, 0x99]);
            assert_eq!(picker.hex(), "#336699");
            // Without the hash too.
            assert!(picker.set_hex("FF0000"));
            assert_eq!(picker.rgb(), [255, 0, 0]);
            // Bad text leaves the colour alone.
            assert!(!picker.set_hex("#12345"));
            assert!(!picker.set_hex("zzzzzz"));
            assert_eq!(picker.rgb(), [255, 0, 0]);

            // Saturation steps down from pure red toward white and stops there.
            picker.focus_at(1);
            picker.step(-0.75);
            assert_eq!(picker.rgb(), [255, 191, 191]);
            picker.step(-4.0);
            assert_eq!(picker.rgb(), [255, 255, 255]);

            // Hue focus turns the colour round.
            picker.set_hex("#FF0000");
            picker.focus_at(0);
            picker.step(120.0);
            assert_eq!(picker.rgb(), [0, 255, 0]);
            picker.step(-240.0);
            assert_eq!(picker.rgb(), [0, 0, 255]);

            // Brightness down to black.
            picker.set_hex("#FF0000");
            picker.focus_at(2);
            picker.step(-1.0);
            assert_eq!(picker.rgb(), [0, 0, 0]);

            // Focus wraps through three channels.
            picker.focus_at(0);
            assert_eq!(picker.nudge_focus(-1), COLOR_CHANNELS - 1);
            assert_eq!(picker.nudge_focus(1), 0);

            // A swatch is a colour.
            picker.select_swatch(4);
            assert_eq!(picker.swatches()[4], picker.rgb());
            picker.select_swatch(99);
            assert_eq!(picker.swatches()[4], picker.rgb());
        }

        #[test]
        fn hsv_and_rgb_round_trip() {
            for hex in [
                "#000000", "#FFFFFF", "#FF0000", "#00FF00", "#0000FF", "#336699", "#C08040",
            ] {
                let mut picker = ColorPickerState::new();
                assert!(picker.set_hex(hex));
                let hsv = picker.hsv();
                let rgb = hsv_to_rgb(hsv);
                let back = picker.rgb();
                for channel in 0..3 {
                    assert!(
                        (rgb[channel] * 255.0 - back[channel] as f32).abs() <= 1.0,
                        "{} channel {} drifted",
                        hex,
                        channel
                    );
                }
            }
            // Grey has no hue and no saturation.
            let grey = rgb_to_hsv([0.5, 0.5, 0.5]);
            assert_eq!(grey[0], 0.0);
            assert_eq!(grey[1], 0.0);
            assert!((grey[2] - 0.5).abs() < 1e-6);
            // Black is no value at all.
            assert_eq!(rgb_to_hsv([0.0, 0.0, 0.0])[2], 0.0);
        }

        #[test]
        fn the_picker_keeps_the_hue_of_greys_and_opens_over_a_colour() {
            let mut picker = ColorPickerState::new();
            // Opening remembers the colour the cancel returns to.
            picker.open_over([0x33, 0x66, 0x99]);
            assert_eq!(picker.original(), [0x33, 0x66, 0x99]);
            assert_eq!(picker.rgb(), [0x33, 0x66, 0x99]);
            // A grey keeps its hue: red to white to green stays green-hued.
            picker.set_hex("#FF0000");
            picker.set_hex("#808080");
            let hue = picker.hsv()[0];
            picker.set_hex("#00FF00");
            picker.set_hex("#FFFFFF");
            picker.set_hex("#FF0000");
            let _ = hue;
            // The Java rule: a set with saturation keeps the new hue, a grey
            // keeps the old one.
            picker.set_hex("#40FF40");
            let green_hue = picker.hsv()[0];
            picker.set_hex("#202020");
            assert!(
                (picker.hsv()[0] - green_hue).abs() < 1e-3,
                "grey kept the hue"
            );
            picker.set_hex("#FF0000");
            assert!(
                (picker.hsv()[0] - 0.0).abs() < 1e-3,
                "saturated set took the hue"
            );
        }

        #[test]
        fn the_picker_picks_from_the_square_and_the_bar() {
            let mut picker = ColorPickerState::new();
            picker.open_over([0, 0, 0]);
            let l = layout();
            let square = picker.square(&l);
            // Top right of the square: full saturation, full brightness.
            assert!(picker.pick_at(&l, square.right() - 1.0, square.y + 1.0, true));
            assert_eq!(picker.dragging(), 1);
            assert!(picker.saturation() > 0.95, "{}", picker.saturation());
            assert!(picker.brightness() > 0.95, "{}", picker.brightness());
            // Bottom left: no saturation, no brightness.
            picker.pick_at(&l, square.x, square.bottom() - 1.0, false);
            assert!(picker.saturation() < 0.05);
            assert!(picker.brightness() < 0.05);
            // The bar sets the hue; a near-white pick gets colour back.
            picker.set_hex("#FFFFFF");
            let bar = picker.bar(&l);
            picker.pick_at(
                &l,
                bar.center()[0],
                square.y + (square.h - 1.0) * 0.25,
                true,
            );
            assert_eq!(picker.dragging(), 2);
            assert!((picker.hue() - 0.25).abs() < 0.02, "{}", picker.hue());
            assert!(
                (picker.saturation() - 0.6).abs() < 1e-4,
                "the white got colour back"
            );
            // A click off both parts starts no drag.
            picker.release();
            assert!(!picker.pick_at(&l, 1.0, 1.0, true));
            assert_eq!(picker.dragging(), 0);
            // Closing hands back the original.
            let original = picker.close();
            assert_eq!(original, [0, 0, 0]);
            assert!(!picker.is_open());
        }

        #[test]
        fn the_colour_picker_draws_its_rows_and_swatches() {
            let mut picker = ColorPickerState::new();
            let mut frame = HudFrame::new();
            picker.draw(&layout(), &theme(), &mut frame);
            assert!(frame.is_empty());
            picker.open();
            picker.draw(&layout(), &theme(), &mut frame);
            // Rows, the hex line, the preview and eight swatches.
            assert!(frame.len() > 8);
        }

        #[test]
        fn the_canvas_edits_a_cue_and_previews_it() {
            let mut canvas = FaceCanvasState::new();
            assert_eq!(canvas.expression(), FaceExpression::Happy);
            canvas.open();

            // A new expression starts from its built-in cue.
            canvas.select_expression(FaceExpression::Angry);
            assert_eq!(canvas.cue(), FaceExpression::Angry.cue());

            // The focused field moves, and stops at its bounds.
            canvas.focus_at(0);
            canvas.step(0.5);
            assert!(canvas.cue().lid > FaceExpression::Angry.cue().lid);
            canvas.step(10.0);
            assert_eq!(canvas.cue().lid, 1.0);
            canvas.focus_at(2);
            canvas.step(-0.2);
            assert!(canvas.cue().mouth < 0.15 + 1e-6);
            canvas.focus_at(1);
            canvas.step(-10.0);
            assert_eq!(canvas.cue().brow, -0.6);

            // The last field is the cue's length, and the focus wraps.
            canvas.focus_at(4);
            assert_eq!(canvas.nudge_focus(1), 0);
            assert_eq!(canvas.nudge_focus(-1), CANVAS_FIELDS - 1);
            canvas.step(1.0);
            assert!(canvas.cue().seconds > 0.0);

            // The preview draws the eyes, brows and mouth at least.
            let quads = canvas.preview(&FaceSettings::default());
            assert!(quads.len() >= 5);
            assert!(quads.iter().any(|q| q.uv == super::super::faces::FX_IRIS));
        }

        #[test]
        fn the_canvas_draws_its_fields_and_the_face() {
            let mut canvas = FaceCanvasState::new();
            let mut frame = HudFrame::new();
            canvas.draw(&layout(), &theme(), &mut frame);
            assert!(frame.is_empty());
            canvas.open();
            canvas.draw(&layout(), &theme(), &mut frame);
            assert!(frame.len() > 8);
        }

        #[test]
        fn the_tutorial_walks_its_steps_and_ends() {
            let mut tutorial = TutorialState::new();
            assert!(!tutorial.is_open());
            assert!(tutorial.current().is_none());

            tutorial.start();
            assert_eq!(tutorial.step(), 0);
            assert_eq!(tutorial.current().unwrap().area, GUIDE[0].area);
            assert!(!tutorial.back());

            // Forward through every step; the last one closes it.
            for step in 1..TUTORIAL_STEPS {
                assert!(!tutorial.next());
                assert_eq!(tutorial.step(), step);
            }
            assert!(tutorial.next());
            assert!(!tutorial.is_open());
            assert_eq!(tutorial.step(), 0);

            // Backing up works from the second step on.
            tutorial.start();
            tutorial.next();
            assert!(tutorial.back());
            assert_eq!(tutorial.step(), 0);
            tutorial.close();
            assert!(!tutorial.is_open());
        }

        #[test]
        fn the_tutorial_draws_only_while_it_is_up() {
            let mut tutorial = TutorialState::new();
            let mut frame = HudFrame::new();
            tutorial.draw(&layout(), &theme(), &mut frame);
            assert!(frame.is_empty());
            tutorial.start();
            tutorial.draw(&layout(), &theme(), &mut frame);
            // The wash, the panel and its border, the title, the body and the hint.
            assert!(frame.len() >= 7);
        }

        #[test]
        fn gold_colours_read_the_java_ints() {
            // Opaque white is -1 in Java.
            assert_eq!(gold::TEXT_1, rgba(255, 255, 255, 255));
            // The wheel's wash carries its alpha in the top byte, 0x35.
            assert_eq!(gold::WHEEL_WASH, rgba(0, 0, 0, 0x35));
            // The backdrop is 0x8C alpha over black.
            assert_eq!(gold::BACKDROP, rgba(0, 0, 0, 0x8C));
            // The gold ramp's deepest step, 0xFF7A5207 read as ARGB.
            assert_eq!(gold::GOLD_700, rgba(0x7A, 0x52, 0x07, 0xFF));
            // The bevels keep their five colours and swap a face.
            assert_eq!(gold::GOLD.face, gold::GOLD_400);
            assert_eq!(gold::GOLD.with_face(gold::KNOB).face, gold::KNOB);
            assert_eq!(gold::GOLD.with_face(gold::KNOB).outline, gold::GOLD.outline);
        }

        #[test]
        fn the_punchy_order_is_a_threshold_tile() {
            // Every threshold appears exactly once, in the shader's order.
            let mut seen = [false; 16];
            for value in PUNCHY_ORDER {
                assert!(value < 16);
                assert!(!seen[value as usize], "{} repeats", value);
                seen[value as usize] = true;
            }
            assert!(seen.iter().all(|v| *v));
            assert_eq!(
                PUNCHY_ORDER,
                [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5]
            );
            // It tiles every 4 pixels, read the shader's way.
            assert_eq!(punchy_order(0, 0), punchy_order(4, 8));
            assert_eq!(punchy_order(1, 2), PUNCHY_ORDER[2 * 4 + 1]);
            // The shader's fixed cut: six of the sixteen cells survive.
            assert_eq!(PUNCHY_THRESHOLD, 6);
            let mut kept = 0;
            for y in 0..4 {
                for x in 0..4 {
                    kept += u32::from(!punchy_discard(x, y));
                }
            }
            assert_eq!(kept, 6);
            assert!(!punchy_discard(0, 0), "cell 0, order 0, survives");
        }

        #[test]
        fn dithering_covers_the_share_its_alpha_asks_for() {
            assert!(!dither(0, 0, 0.0));
            assert!(!dither(0, 0, f32::NAN));
            assert!(dither(0, 0, 1.0));
            assert!(dither(0, 0, 2.0));

            // Half opacity shows eight of the sixteen pixels.
            let mut shown = 0;
            for y in 0..4 {
                for x in 0..4 {
                    if dither(x, y, 0.5) {
                        shown += 1;
                    }
                }
            }
            assert_eq!(shown, 8);
            // A quarter shows four, and a full tile always shows.
            let mut quarter = 0;
            for y in 0..4 {
                for x in 0..4 {
                    quarter += u32::from(dither(x, y, 0.25));
                }
            }
            assert_eq!(quarter, 4);
        }

        #[test]
        fn the_near_fade_goes_from_none_to_full() {
            let dither = DitherVisibility::new();
            assert_eq!(dither.alpha(0.0), 0.0);
            assert_eq!(dither.alpha(dither.near), 0.0);
            let middle = dither.near + dither.fade * 0.5;
            assert!((dither.alpha(middle) - 0.5).abs() < 1e-6);
            assert!((dither.alpha(dither.near + dither.fade) - 1.0).abs() < 1e-6);
            assert_eq!(dither.alpha(100.0), 1.0);
            // A broken distance is far away, so fully drawn.
            assert_eq!(dither.alpha(f32::NAN), 1.0);

            // A body in front of the eye is dithered away; one far off is drawn.
            assert!(!dither.visible(0, 0, 0.0));
            assert!(dither.visible(0, 0, 4.0));
            // A zero fade never divides by zero.
            let sharp = DitherVisibility {
                near: 0.5,
                fade: 0.0,
            };
            assert_eq!(sharp.alpha(0.5), 1.0);
            assert_eq!(sharp.alpha(0.4), 0.0);
        }

        #[test]
        fn every_screen_opens_draws_and_closes() {
            let mut screens = PunchyUi::new();
            assert!(!screens.any_open());
            for (index, (label, screen)) in settings_buttons().iter().copied().enumerate() {
                assert!(!label.is_empty());
                assert_eq!(screens.click_settings(index), UiRequest::None);
                assert!(screens.any_open(), "{:?} did not open", screen);
                let frame = screens.draw(&layout(), &ShoulderCameraSettings::default(), &theme());
                assert!(!frame.is_empty(), "{:?} drew nothing", screen);
                screens.close_all();
                assert!(!screens.any_open());
            }
            // A button past the list does nothing.
            assert_eq!(screens.click_settings(99), UiRequest::None);
            assert!(!screens.any_open());
        }

        #[test]
        fn keys_drive_the_screens_that_are_up() {
            let mut screens = PunchyUi::new();
            // Nothing up: no screen wants the key.
            assert_eq!(screens.key("key.keyboard.escape", true), None);

            // The wheel: numbers pick a wedge, Enter plays, Escape puts it away.
            screens.emote_wheel.open();
            assert_eq!(screens.key("key.keyboard.3", true), Some(UiRequest::None));
            assert_eq!(screens.emote_wheel.selected(), 2);
            assert_eq!(
                screens.key("key.keyboard.right", true),
                Some(UiRequest::None)
            );
            assert_eq!(screens.emote_wheel.selected(), 3);
            screens.key("key.keyboard.3", true);
            assert_eq!(
                screens.key("key.keyboard.enter", true),
                Some(UiRequest::PlayEmote(2))
            );
            assert!(!screens.emote_wheel.is_open());

            screens.emote_wheel.open();
            assert_eq!(
                screens.key("key.keyboard.escape", true),
                Some(UiRequest::None)
            );
            assert!(!screens.emote_wheel.is_open());

            // The tutorial: Enter walks on, Escape closes it.
            screens.tutorial.start();
            let start = screens.tutorial.step();
            assert_eq!(
                screens.key("key.keyboard.enter", true),
                Some(UiRequest::None)
            );
            assert_eq!(screens.tutorial.step(), start + 1);
            assert_eq!(
                screens.key("key.keyboard.escape", true),
                Some(UiRequest::None)
            );
            assert!(!screens.tutorial.is_open());

            // The camera page: Tab changes page, Escape closes.
            screens.camera.open();
            let tab = screens.camera.page();
            screens.key("key.keyboard.tab", true);
            assert_ne!(screens.camera.page(), tab);
            screens.key("key.keyboard.escape", true);
            assert!(!screens.camera.is_open());

            // The colour picker and the canvas: arrows move their focus.
            screens.color.open();
            screens.color.focus_at(0);
            screens.key("key.keyboard.right", true);
            assert_eq!(screens.color.focus(), 1);
            screens.key("key.keyboard.escape", true);
            assert!(!screens.color.is_open());

            screens.canvas.open();
            screens.canvas.focus_at(0);
            screens.key("key.keyboard.right", true);
            assert_eq!(screens.canvas.focus(), 1);
            screens.key("key.keyboard.escape", true);
            assert!(!screens.canvas.is_open());
        }

        #[test]
        fn the_screens_thread_state_starts_fresh() {
            reset_ui();
            ui(|screens| screens.emote_wheel.open());
            assert!(ui(|screens| screens.emote_wheel.is_open()));
            reset_ui();
            assert!(!ui(|screens| screens.any_open()));
        }

        #[test]
        fn resting_signals_hold_still() {
            let signals = resting_signals();
            assert!(!signals.moving);
            assert!(!signals.mining);
            assert!(!signals.charging_crossbow);
            assert_eq!(signals.tick, super::super::signals::NEVER);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_id_is_the_mods_folder_name() {
        assert_eq!(ID, "punchy_moves");
        assert_eq!(VERSION.split('.').count(), 3);
        assert!(!DESCRIPTION.is_empty());
    }
}
