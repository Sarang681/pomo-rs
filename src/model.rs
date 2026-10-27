use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;

pub const LONG_BREAK_LIMIT: u16 = 4;

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub enum Phase {
    #[default]
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Default, Debug, PartialEq)]
pub enum RunState {
    #[default]
    Idle,
    Running,
    Paused,
}

pub enum Message {
    Quit,
    ToggleTimer,
    Tick(Duration),
    Skip,
    Reset,
}

#[derive(Debug)]
pub struct Model {
    pub exit: bool,
    pub phase: Phase,
    pub run_state: RunState,
    pub remaining: Duration,
    pub sessions_completed: u16,
}

impl Default for Model {
    fn default() -> Self {
        let phase = Phase::Focus;
        Self {
            exit: false,
            phase,
            run_state: RunState::Idle,
            remaining: phase.map_phase_to_default_timer(),
            sessions_completed: 0,
        }
    }
}

impl Model {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Quit => self.exit = true,
            Message::ToggleTimer => match self.run_state {
                RunState::Idle => self.run_state = RunState::Running,
                RunState::Running => self.run_state = RunState::Paused,
                RunState::Paused => self.run_state = RunState::Running,
            },
            Message::Tick(elapsed_time) => self.handle_tick(elapsed_time),
            Message::Skip => {
                self.handle_phase_change(true);
                self.run_state = RunState::Idle;
            }
            Message::Reset => {
                self.remaining = self.phase.map_phase_to_default_timer();
                self.run_state = RunState::Idle;
            }
        }
    }

    fn handle_tick(&mut self, elapsed_time: Duration) {
        if self.run_state == RunState::Running {
            if self.remaining > elapsed_time {
                self.remaining -= elapsed_time;
            } else {
                self.remaining = Duration::ZERO;
                self.run_state = RunState::Idle;
                self.handle_phase_change(false);
            }
        }
    }

    fn handle_phase_change(&mut self, is_skipped: bool) {
        let new_phase;
        match self.phase {
            Phase::Focus => {
                if !is_skipped {
                    self.sessions_completed += 1;
                }
                if self.sessions_completed == LONG_BREAK_LIMIT {
                    new_phase = Phase::LongBreak;
                } else {
                    new_phase = Phase::ShortBreak;
                }
            }
            Phase::ShortBreak => new_phase = Phase::Focus,
            Phase::LongBreak => {
                new_phase = Phase::Focus;
                self.sessions_completed = 0;
            }
        }
        self.phase = new_phase;
        self.remaining = new_phase.map_phase_to_default_timer();
    }

    /// Fraction of the current phase that has elapsed, always within 0.0..=1.0.
    pub fn progress(&self) -> f64 {
        let total = self.phase.map_phase_to_default_timer().as_secs_f64();
        if total == 0.0 {
            return 0.0;
        }
        (1.0 - self.remaining.as_secs_f64() / total).clamp(0.0, 1.0)
    }

    pub fn format_time_remaining(&self) -> String {
        let total_seconds = self.remaining.as_secs_f64().ceil() as u64;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

impl Phase {
    fn map_phase_to_default_timer(&self) -> Duration {
        match self {
            Phase::Focus => {
                Duration::from_mins(25)
                // Duration::from_mins(1)
            }
            Phase::ShortBreak => {
                Duration::from_mins(5)
                // Duration::from_secs(10)
            }
            Phase::LongBreak => {
                Duration::from_mins(10)
                // Duration::from_secs(30)
            }
        }
    }
}

impl Display for Phase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Focus => write!(f, "FOCUS"),
            Phase::ShortBreak => write!(f, "SHORT BREAK"),
            Phase::LongBreak => write!(f, "LONG BREAK"),
        }
    }
}

impl Display for RunState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RunState::Idle => write!(f, "Ready"),
            RunState::Running => write!(f, "Running"),
            RunState::Paused => write!(f, "PAUSED"),
        }
    }
}

#[cfg(test)]
mod tests;
