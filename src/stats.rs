//! History of completed focus sessions and the numbers shown on the stats screen.
//!
//! Days are whole days since the Unix epoch (UTC). The model is told today's day by the app,
//! so nothing here reads the clock.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub day: u64,
    pub minutes: u64,
    #[serde(default)]
    pub task: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    pub sessions: Vec<SessionRecord>,
}

impl History {
    pub fn record(&mut self, record: SessionRecord) {
        self.sessions.push(record);
    }

    pub fn total_sessions(&self) -> usize {
        self.sessions.len()
    }

    pub fn sessions_on(&self, day: u64) -> usize {
        self.sessions.iter().filter(|s| s.day == day).count()
    }

    pub fn minutes_on(&self, day: u64) -> u64 {
        self.sessions
            .iter()
            .filter(|s| s.day == day)
            .map(|s| s.minutes)
            .sum()
    }

    /// Session counts for the `count` days ending at `today`, oldest first.
    pub fn last_days(&self, today: u64, count: u64) -> Vec<(u64, usize)> {
        (0..count)
            .rev()
            .filter_map(|offset| today.checked_sub(offset))
            .map(|day| (day, self.sessions_on(day)))
            .collect()
    }

    /// Consecutive days with at least one session. A streak is still alive if the last session
    /// was yesterday, so it doesn't read as broken before you have had a chance to focus today.
    pub fn streak(&self, today: u64) -> u64 {
        let mut day = if self.sessions_on(today) > 0 {
            today
        } else {
            match today.checked_sub(1) {
                Some(yesterday) => yesterday,
                None => return 0,
            }
        };
        let mut streak = 0;
        while self.sessions_on(day) > 0 {
            streak += 1;
            match day.checked_sub(1) {
                Some(previous) => day = previous,
                None => break,
            }
        }
        streak
    }
}

/// Three-letter weekday of a day number. Day 0 (1970-01-01) was a Thursday.
pub fn weekday_label(day: u64) -> &'static str {
    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][((day + 4) % 7) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history_on(days: &[u64]) -> History {
        let mut history = History::default();
        for &day in days {
            history.record(SessionRecord {
                day,
                minutes: 25,
                task: String::new(),
            });
        }
        history
    }

    #[test]
    fn test_counts_and_minutes_are_per_day() {
        let history = history_on(&[10, 10, 11]);

        assert_eq!(history.total_sessions(), 3);
        assert_eq!(history.sessions_on(10), 2);
        assert_eq!(history.minutes_on(10), 50);
        assert_eq!(history.sessions_on(12), 0);
    }

    #[test]
    fn test_last_days_is_oldest_first_and_includes_empty_days() {
        let history = history_on(&[8, 10, 10]);

        assert_eq!(
            history.last_days(10, 4),
            vec![(7, 0), (8, 1), (9, 0), (10, 2)]
        );
    }

    #[test]
    fn test_last_days_does_not_underflow_near_the_epoch() {
        assert_eq!(History::default().last_days(1, 7), vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn test_streak_counts_consecutive_days_up_to_today() {
        assert_eq!(history_on(&[8, 9, 10]).streak(10), 3);
    }

    #[test]
    fn test_streak_is_alive_until_a_day_is_missed() {
        assert_eq!(history_on(&[8, 9]).streak(10), 2);
        assert_eq!(history_on(&[8, 9]).streak(11), 0);
    }

    #[test]
    fn test_streak_stops_at_a_gap() {
        assert_eq!(history_on(&[5, 6, 8, 9, 10]).streak(10), 3);
    }

    #[test]
    fn test_streak_is_zero_for_empty_history_and_epoch_day() {
        assert_eq!(History::default().streak(0), 0);
        assert_eq!(history_on(&[0]).streak(0), 1);
    }

    #[test]
    fn test_weekday_labels_match_the_calendar() {
        // 1970-01-01 was a Thursday, 2000-01-01 (day 10957) a Saturday.
        assert_eq!(weekday_label(0), "Thu");
        assert_eq!(weekday_label(1), "Fri");
        assert_eq!(weekday_label(10957), "Sat");
    }

    #[test]
    fn test_history_round_trips_through_json() {
        let history = history_on(&[1, 2]);
        let text = serde_json::to_string(&history).unwrap();

        assert_eq!(serde_json::from_str::<History>(&text).unwrap(), history);
    }
}
