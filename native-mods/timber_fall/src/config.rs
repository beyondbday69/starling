//! Player-facing options and their defaults, as listed in the concept.

/// Timber Fall's options. Defaults match the concept's config table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimberConfig {
    pub sneak_disables_felling: bool,
    pub require_axe: bool,
    pub max_logs: usize,
    pub fall_speed: f32,
    pub falling_damage: bool,
    pub camera_shake: bool,
    pub auto_replant_sapling: bool,
}

impl Default for TimberConfig {
    fn default() -> Self {
        Self {
            sneak_disables_felling: true,
            require_axe: true,
            max_logs: 256,
            fall_speed: Self::DEFAULT_FALL_SPEED,
            falling_damage: true,
            camera_shake: true,
            auto_replant_sapling: false,
        }
    }
}

impl TimberConfig {
    /// The 1.5x speed the concept ships with.
    pub const DEFAULT_FALL_SPEED: f32 = 1.5;
    /// A tree is never allowed to fall without at least its cut log.
    pub const MIN_LOGS: usize = 1;

    /// Clamps values a config file could get wrong, keeping each one usable:
    /// at least one log, and a finite, positive fall speed.
    pub fn sanitized(mut self) -> Self {
        self.max_logs = self.max_logs.max(Self::MIN_LOGS);
        if !self.fall_speed.is_finite() || self.fall_speed <= 0.0 {
            self.fall_speed = Self::DEFAULT_FALL_SPEED;
        }
        self
    }

    /// Reads `key = value` lines, one option per line, `#` starting a comment.
    ///
    /// Missing keys keep their defaults, and so does a line whose value cannot be
    /// read; unknown keys are ignored. The result is sanitized, so a file can never
    /// make a tree unfellable or the fall speed unusable.
    pub fn parse(text: &str) -> Self {
        let mut config = Self::default();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            config.apply(key.trim(), value.trim());
        }
        config.sanitized()
    }

    /// Writes the options in the shape [`TimberConfig::parse`] reads back.
    pub fn to_config_string(&self) -> String {
        let mut text = String::new();
        text.push_str(&format!(
            "sneak_disables_felling = {}\n",
            self.sneak_disables_felling
        ));
        text.push_str(&format!("require_axe = {}\n", self.require_axe));
        text.push_str(&format!("max_logs = {}\n", self.max_logs));
        text.push_str(&format!("fall_speed = {}\n", self.fall_speed));
        text.push_str(&format!("falling_damage = {}\n", self.falling_damage));
        text.push_str(&format!("camera_shake = {}\n", self.camera_shake));
        text.push_str(&format!(
            "auto_replant_sapling = {}\n",
            self.auto_replant_sapling
        ));
        text
    }

    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "sneak_disables_felling" => {
                if let Some(parsed) = parse_bool(value) {
                    self.sneak_disables_felling = parsed;
                }
            }
            "require_axe" => {
                if let Some(parsed) = parse_bool(value) {
                    self.require_axe = parsed;
                }
            }
            "max_logs" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    self.max_logs = parsed;
                }
            }
            "fall_speed" => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.fall_speed = parsed;
                }
            }
            "falling_damage" => {
                if let Some(parsed) = parse_bool(value) {
                    self.falling_damage = parsed;
                }
            }
            "camera_shake" => {
                if let Some(parsed) = parse_bool(value) {
                    self.camera_shake = parsed;
                }
            }
            "auto_replant_sapling" => {
                if let Some(parsed) = parse_bool(value) {
                    self.auto_replant_sapling = parsed;
                }
            }
            _ => {}
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::TimberConfig;

    #[test]
    fn defaults_match_the_concept_table() {
        let config = TimberConfig::default();

        assert!(config.sneak_disables_felling);
        assert!(config.require_axe);
        assert_eq!(config.max_logs, 256);
        assert_eq!(config.fall_speed, 1.5);
        assert!(config.falling_damage);
        assert!(config.camera_shake);
        assert!(!config.auto_replant_sapling);
    }

    #[test]
    fn sanitize_raises_max_logs_to_one() {
        let config = TimberConfig {
            max_logs: 0,
            ..TimberConfig::default()
        }
        .sanitized();

        assert_eq!(config.max_logs, TimberConfig::MIN_LOGS);
    }

    #[test]
    fn sanitize_replaces_unusable_fall_speeds() {
        for fall_speed in [0.0, -2.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let config = TimberConfig {
                fall_speed,
                ..TimberConfig::default()
            }
            .sanitized();

            assert_eq!(config.fall_speed, TimberConfig::DEFAULT_FALL_SPEED);
        }
    }

    #[test]
    fn sanitize_keeps_values_that_are_already_valid() {
        let config = TimberConfig {
            sneak_disables_felling: false,
            require_axe: false,
            max_logs: 4,
            fall_speed: 2.5,
            falling_damage: false,
            camera_shake: false,
            auto_replant_sapling: true,
        }
        .sanitized();

        assert_eq!(
            config,
            TimberConfig {
                sneak_disables_felling: false,
                require_axe: false,
                max_logs: 4,
                fall_speed: 2.5,
                falling_damage: false,
                camera_shake: false,
                auto_replant_sapling: true,
            }
        );
    }

    #[test]
    fn config_round_trips_through_the_text_form() {
        let config = TimberConfig {
            sneak_disables_felling: false,
            require_axe: false,
            max_logs: 64,
            fall_speed: 2.5,
            falling_damage: false,
            camera_shake: false,
            auto_replant_sapling: true,
        };

        assert_eq!(TimberConfig::parse(&config.to_config_string()), config);
    }

    #[test]
    fn a_partial_file_keeps_defaults_for_missing_keys() {
        let config = TimberConfig::parse("max_logs = 4\nfall_speed = 2.5\n");

        assert_eq!(config.max_logs, 4);
        assert_eq!(config.fall_speed, 2.5);
        assert!(config.sneak_disables_felling);
        assert!(config.require_axe);
        assert!(config.falling_damage);
        assert!(config.camera_shake);
        assert!(!config.auto_replant_sapling);
    }

    #[test]
    fn invalid_lines_and_values_fall_back_to_defaults() {
        let text = "\
            # a comment\n\
            not a setting\n\
            sneak_disables_felling = maybe\n\
            require_axe =\n\
            max_logs = lots\n\
            fall_speed = fast\n\
            falling_damage = 1\n\
            unknown_key = 7\n\
        ";

        assert_eq!(TimberConfig::parse(text), TimberConfig::default());
    }

    #[test]
    fn comments_and_whitespace_are_ignored() {
        let text = "  # heading\n  max_logs   =   32   # inline\n\tfall_speed=3.0\n";

        let config = TimberConfig::parse(text);

        assert_eq!(config.max_logs, 32);
        assert_eq!(config.fall_speed, 3.0);
    }

    #[test]
    fn parsed_values_are_sanitized() {
        let config = TimberConfig::parse("max_logs = 0\nfall_speed = -1\n");

        assert_eq!(config.max_logs, TimberConfig::MIN_LOGS);
        assert_eq!(config.fall_speed, TimberConfig::DEFAULT_FALL_SPEED);
    }
}
