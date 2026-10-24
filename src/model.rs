use std::fmt::Debug;
use std::time::Duration;

#[derive(Default, Debug, PartialEq)]
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
    TogglePhase,
    Tick(Duration),
}

#[derive(Debug)]
pub struct Model {
    pub exit: bool,
    pub phase: Phase,
    pub run_state: RunState,
    pub remaining: Duration,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            exit: false,
            phase: Phase::Focus,
            run_state: RunState::Idle,
            remaining: Duration::from_mins(25),
        }
    }
}

impl Model {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Quit => self.exit = true,
            Message::TogglePhase => match self.phase {
                Phase::Focus => self.phase = Phase::ShortBreak,
                Phase::ShortBreak => self.phase = Phase::LongBreak,
                Phase::LongBreak => self.phase = Phase::Focus,
            },
            Message::ToggleTimer => match self.run_state {
                RunState::Idle => self.run_state = RunState::Running,
                RunState::Running => self.run_state = RunState::Paused,
                RunState::Paused => self.run_state = RunState::Running,
            },
            Message::Tick(elapsed_time) => {
                if self.run_state == RunState::Running {
                    if self.remaining > elapsed_time {
                        self.remaining -= elapsed_time;
                    } else {
                        self.remaining = Duration::ZERO;
                        self.run_state = RunState::Idle;
                    }
                }
            }
        }
    }

    pub fn format_time_remaining(&self) -> String {
        let total_seconds = self.remaining.as_secs_f64().ceil() as u64;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Message, Model, Phase, RunState};
    use std::time::Duration;

    #[test]
    fn test_model_defaults() {
        let test_model = Model::default();
        assert_eq!(test_model.run_state, RunState::Idle);
        assert_eq!(test_model.phase, Phase::Focus);
        assert_eq!(test_model.remaining, Duration::from_mins(25));
    }

    #[test]
    fn test_toggle_idle_start_timer() {
        let mut test_model = Model::default();
        test_model.update(Message::ToggleTimer);

        assert_eq!(test_model.run_state, RunState::Running);
    }

    #[test]
    fn test_toggle_running_pause() {
        let mut test_model = Model::default();
        test_model.update(Message::ToggleTimer);
        test_model.update(Message::ToggleTimer);

        assert_eq!(test_model.run_state, RunState::Paused);
    }

    #[test]
    fn test_toggle_pause_running() {
        let mut test_model = Model::default();
        test_model.update(Message::ToggleTimer);
        test_model.update(Message::ToggleTimer);
        test_model.update(Message::ToggleTimer);

        assert_eq!(test_model.run_state, RunState::Running);
    }

    #[test]
    fn test_quit_set_exit() {
        let mut test_model = Model::default();
        test_model.update(Message::Quit);

        assert!(test_model.exit);
        assert_eq!(test_model.run_state, RunState::Idle);
        assert_eq!(test_model.phase, Phase::Focus);
    }

    #[test]
    fn test_format_remaining_for_default() {
        let test_model = Model::default();

        let formatted_string = test_model.format_time_remaining();
        assert_eq!("25:00", formatted_string);
    }

    #[test]
    fn test_format_remaining_for_partial_seconds() {
        let mut test_model = Model::default();

        test_model.update(Message::ToggleTimer);
        test_model.update(Message::Tick(Duration::from_millis(500)));

        let formatted_string = test_model.format_time_remaining();
        assert_eq!("25:00", formatted_string);
    }

    #[test]
    fn test_remaining_default() {
        let test_model = Model::default();

        assert_eq!(Duration::from_mins(25), test_model.remaining);
    }

    #[test]
    fn test_remaining_when_state_idle() {
        let mut test_model = Model::default();

        test_model.update(Message::Tick(Duration::from_mins(10)));

        assert_eq!(Duration::from_mins(25), test_model.remaining);
    }

    #[test]
    fn test_remaining_when_state_running() {
        let mut test_model = Model::default();

        test_model.update(Message::ToggleTimer);
        test_model.update(Message::Tick(Duration::from_mins(10)));

        assert_eq!(Duration::from_mins(15), test_model.remaining);
    }

    #[test]
    fn test_remaining_when_state_paused() {
        let mut test_model = Model::default();

        test_model.update(Message::ToggleTimer);
        test_model.update(Message::ToggleTimer);
        test_model.update(Message::Tick(Duration::from_mins(10)));

        assert_eq!(Duration::from_mins(25), test_model.remaining);
    }

    #[test]
    fn test_remaining_when_tick_larger_than_remaining() {
        let mut test_model = Model::default();

        test_model.update(Message::ToggleTimer);
        test_model.update(Message::Tick(Duration::from_mins(30)));

        assert_eq!(Duration::ZERO, test_model.remaining);
    }

    #[test]
    fn test_remaining_when_tick_equals_remaining() {
        let mut test_model = Model::default();

        test_model.update(Message::ToggleTimer);
        test_model.update(Message::Tick(Duration::from_mins(25)));

        assert_eq!(Duration::ZERO, test_model.remaining);
    }
}
