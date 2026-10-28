//! User-adjustable settings: phase lengths and alerts. Pure data, no terminal or file access.

use crate::model::Phase;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MIN_MINUTES: u64 = 1;
pub const MAX_MINUTES: u64 = 99;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub focus_minutes: u64,
    pub short_break_minutes: u64,
    pub long_break_minutes: u64,
    pub bell: bool,
    pub notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 10,
            bell: true,
            notifications: true,
        }
    }
}

/// One line of the settings screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsRow {
    Focus,
    ShortBreak,
    LongBreak,
    Bell,
    Notifications,
}

pub const SETTINGS_ROWS: [SettingsRow; 5] = [
    SettingsRow::Focus,
    SettingsRow::ShortBreak,
    SettingsRow::LongBreak,
    SettingsRow::Bell,
    SettingsRow::Notifications,
];

impl SettingsRow {
    pub fn label(self) -> &'static str {
        match self {
            SettingsRow::Focus => "Focus length",
            SettingsRow::ShortBreak => "Short break length",
            SettingsRow::LongBreak => "Long break length",
            SettingsRow::Bell => "Terminal bell",
            SettingsRow::Notifications => "Desktop notifications",
        }
    }
}

fn step_minutes(current: u64, delta: i32) -> u64 {
    (current as i64 + i64::from(delta)).clamp(MIN_MINUTES as i64, MAX_MINUTES as i64) as u64
}

fn on_off(value: bool) -> String {
    if value { "on" } else { "off" }.to_string()
}

impl Settings {
    pub fn duration(&self, phase: Phase) -> Duration {
        let minutes = match phase {
            Phase::Focus => self.focus_minutes,
            Phase::ShortBreak => self.short_break_minutes,
            Phase::LongBreak => self.long_break_minutes,
        };
        Duration::from_mins(minutes)
    }

    /// Moves the length of `phase` by `delta` minutes, staying within the allowed range.
    pub fn adjust_phase(&mut self, phase: Phase, delta: i32) {
        let row = match phase {
            Phase::Focus => SettingsRow::Focus,
            Phase::ShortBreak => SettingsRow::ShortBreak,
            Phase::LongBreak => SettingsRow::LongBreak,
        };
        self.adjust(row, delta);
    }

    /// Changes one row: lengths move by `delta` minutes, switches flip.
    pub fn adjust(&mut self, row: SettingsRow, delta: i32) {
        match row {
            SettingsRow::Focus => self.focus_minutes = step_minutes(self.focus_minutes, delta),
            SettingsRow::ShortBreak => {
                self.short_break_minutes = step_minutes(self.short_break_minutes, delta)
            }
            SettingsRow::LongBreak => {
                self.long_break_minutes = step_minutes(self.long_break_minutes, delta)
            }
            SettingsRow::Bell => self.bell = !self.bell,
            SettingsRow::Notifications => self.notifications = !self.notifications,
        }
    }

    pub fn value_text(&self, row: SettingsRow) -> String {
        match row {
            SettingsRow::Focus => format!("{} min", self.focus_minutes),
            SettingsRow::ShortBreak => format!("{} min", self.short_break_minutes),
            SettingsRow::LongBreak => format!("{} min", self.long_break_minutes),
            SettingsRow::Bell => on_off(self.bell),
            SettingsRow::Notifications => on_off(self.notifications),
        }
    }

    /// Clamps hand-edited lengths into the supported range.
    pub fn sanitized(mut self) -> Self {
        for minutes in [
            &mut self.focus_minutes,
            &mut self.short_break_minutes,
            &mut self.long_break_minutes,
        ] {
            *minutes = (*minutes).clamp(MIN_MINUTES, MAX_MINUTES);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_match_the_classic_cycle() {
        let settings = Settings::default();

        assert_eq!(settings.duration(Phase::Focus), Duration::from_mins(25));
        assert_eq!(settings.duration(Phase::ShortBreak), Duration::from_mins(5));
        assert_eq!(settings.duration(Phase::LongBreak), Duration::from_mins(10));
        assert!(settings.bell && settings.notifications);
    }

    #[test]
    fn test_adjusting_a_phase_moves_only_that_phase() {
        let mut settings = Settings::default();
        settings.adjust_phase(Phase::ShortBreak, 2);

        assert_eq!(settings.short_break_minutes, 7);
        assert_eq!(settings.focus_minutes, 25);
        assert_eq!(settings.long_break_minutes, 10);
    }

    #[test]
    fn test_lengths_stay_within_the_allowed_range() {
        let mut settings = Settings::default();
        settings.adjust(SettingsRow::Focus, -1000);
        assert_eq!(settings.focus_minutes, MIN_MINUTES);

        settings.adjust(SettingsRow::Focus, 1000);
        assert_eq!(settings.focus_minutes, MAX_MINUTES);
    }

    #[test]
    fn test_switch_rows_flip() {
        let mut settings = Settings::default();
        settings.adjust(SettingsRow::Bell, 1);
        settings.adjust(SettingsRow::Notifications, -1);

        assert!(!settings.bell);
        assert!(!settings.notifications);

        settings.adjust(SettingsRow::Bell, -1);
        assert!(settings.bell);
    }

    #[test]
    fn test_value_text_reads_naturally() {
        let settings = Settings::default();

        assert_eq!(settings.value_text(SettingsRow::Focus), "25 min");
        assert_eq!(settings.value_text(SettingsRow::Bell), "on");
    }

    #[test]
    fn test_sanitized_clamps_out_of_range_lengths() {
        let settings = Settings {
            focus_minutes: 0,
            short_break_minutes: 500,
            ..Settings::default()
        }
        .sanitized();

        assert_eq!(settings.focus_minutes, MIN_MINUTES);
        assert_eq!(settings.short_break_minutes, MAX_MINUTES);
    }

    #[test]
    fn test_a_partial_config_falls_back_to_defaults() {
        let settings: Settings = toml::from_str("focus_minutes = 50").unwrap();

        assert_eq!(settings.focus_minutes, 50);
        assert_eq!(settings.short_break_minutes, 5);
        assert!(settings.bell);
    }

    #[test]
    fn test_settings_round_trip_through_toml() {
        let settings = Settings {
            focus_minutes: 40,
            bell: false,
            ..Settings::default()
        };
        let text = toml::to_string_pretty(&settings).unwrap();

        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), settings);
    }
}
