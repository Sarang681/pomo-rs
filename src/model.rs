use crate::settings::{SETTINGS_ROWS, Settings};
use crate::stats::{History, SessionRecord};
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;

pub const LONG_BREAK_LIMIT: u16 = 4;
pub const MAX_TASK_LEN: usize = 60;

/// Which screen is showing, and therefore which keys mean what.
#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub enum Mode {
    #[default]
    Normal,
    EditingTask,
    ConfirmingQuit,
    Help,
    Settings,
    Stats,
}

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

#[derive(Debug, PartialEq, Clone)]
pub enum Message {
    Quit,
    ToggleTimer,
    Tick(Duration),
    Skip,
    Reset,
    /// Change the length of the current phase by this many minutes (only while idle).
    AdjustTime(i32),
    /// Quit now, or ask first if a focus session would be lost.
    AskToQuit,
    CancelQuit,
    StartEditing,
    TypeChar(char),
    Backspace,
    FinishEditing,
    CancelEditing,
    OpenHelp,
    OpenSettings,
    OpenStats,
    CloseOverlay,
    SettingsMove(i32),
    SettingsAdjust(i32),
}

/// Work the model asks the app to do. The model stays pure; the app performs these.
#[derive(Debug, PartialEq, Clone)]
pub enum Effect {
    Bell,
    Notify { title: String, body: String },
    SaveHistory,
    SaveSettings,
}

#[derive(Debug)]
pub struct Model {
    pub exit: bool,
    pub phase: Phase,
    pub run_state: RunState,
    pub remaining: Duration,
    pub sessions_completed: u16,
    pub mode: Mode,
    pub task: String,
    /// Text being typed while editing the task; it replaces `task` only on save.
    pub draft: String,
    pub settings: Settings,
    pub settings_cursor: usize,
    pub history: History,
    /// Today as a day number, supplied by the app so the model never reads the clock.
    pub today: u64,
    /// Queued by `update`, drained with `take_effects`.
    pub(crate) effects: Vec<Effect>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new(Settings::default(), History::default())
    }
}

impl Model {
    pub fn new(settings: Settings, history: History) -> Self {
        let phase = Phase::Focus;
        Self {
            exit: false,
            phase,
            run_state: RunState::Idle,
            remaining: settings.duration(phase),
            sessions_completed: 0,
            mode: Mode::Normal,
            task: String::new(),
            draft: String::new(),
            settings,
            settings_cursor: 0,
            history,
            today: 0,
            effects: Vec::new(),
        }
    }

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
                self.remaining = self.settings.duration(self.phase);
                self.run_state = RunState::Idle;
            }
            Message::AdjustTime(minutes) => {
                if self.run_state == RunState::Idle {
                    self.settings.adjust_phase(self.phase, minutes);
                    self.remaining = self.settings.duration(self.phase);
                    self.effects.push(Effect::SaveSettings);
                }
            }
            Message::AskToQuit => {
                if self.needs_quit_confirmation() {
                    self.mode = Mode::ConfirmingQuit;
                } else {
                    self.exit = true;
                }
            }
            Message::CancelQuit => self.leave(Mode::ConfirmingQuit),
            Message::StartEditing => {
                self.draft = self.task.clone();
                self.mode = Mode::EditingTask;
            }
            Message::TypeChar(c) => {
                if self.mode == Mode::EditingTask
                    && !c.is_control()
                    && self.draft.chars().count() < MAX_TASK_LEN
                {
                    self.draft.push(c);
                }
            }
            Message::Backspace => {
                if self.mode == Mode::EditingTask {
                    self.draft.pop();
                }
            }
            Message::FinishEditing => {
                if self.mode == Mode::EditingTask {
                    self.task = self.draft.trim().to_string();
                    self.draft.clear();
                    self.mode = Mode::Normal;
                }
            }
            Message::CancelEditing => {
                if self.mode == Mode::EditingTask {
                    self.draft.clear();
                    self.mode = Mode::Normal;
                }
            }
            Message::OpenHelp => self.mode = Mode::Help,
            Message::OpenSettings => self.mode = Mode::Settings,
            Message::OpenStats => self.mode = Mode::Stats,
            Message::CloseOverlay => {
                if self.mode == Mode::Settings {
                    self.effects.push(Effect::SaveSettings);
                }
                if matches!(self.mode, Mode::Help | Mode::Settings | Mode::Stats) {
                    self.mode = Mode::Normal;
                }
            }
            Message::SettingsMove(delta) => {
                if self.mode == Mode::Settings {
                    let rows = SETTINGS_ROWS.len() as i32;
                    self.settings_cursor =
                        (self.settings_cursor as i32 + delta).rem_euclid(rows) as usize;
                }
            }
            Message::SettingsAdjust(delta) => {
                if self.mode == Mode::Settings {
                    self.settings
                        .adjust(SETTINGS_ROWS[self.settings_cursor], delta);
                    self.sync_remaining_to_settings();
                }
            }
        }
    }

    /// Hands over the work queued by `update`, leaving the queue empty.
    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    /// Quitting only costs something while a focus session is under way.
    fn needs_quit_confirmation(&self) -> bool {
        self.phase == Phase::Focus && self.run_state != RunState::Idle
    }

    fn leave(&mut self, mode: Mode) {
        if self.mode == mode {
            self.mode = Mode::Normal;
        }
    }

    /// After a length changed: an untouched timer shows the new length, a started one is capped.
    fn sync_remaining_to_settings(&mut self) {
        let full = self.settings.duration(self.phase);
        self.remaining = if self.run_state == RunState::Idle {
            full
        } else {
            self.remaining.min(full)
        };
    }

    fn handle_tick(&mut self, elapsed_time: Duration) {
        if self.run_state == RunState::Running {
            if self.remaining > elapsed_time {
                self.remaining -= elapsed_time;
            } else {
                self.remaining = Duration::ZERO;
                self.run_state = RunState::Idle;
                self.complete_phase();
            }
        }
    }

    /// The timer ran out: log a finished focus session, move on and raise the alerts.
    fn complete_phase(&mut self) {
        let finished = self.phase;
        if finished == Phase::Focus {
            self.history.record(SessionRecord {
                day: self.today,
                minutes: self.settings.focus_minutes,
                task: self.task.clone(),
            });
            self.effects.push(Effect::SaveHistory);
        }
        self.handle_phase_change(false);

        if self.settings.bell {
            self.effects.push(Effect::Bell);
        }
        if self.settings.notifications {
            self.effects.push(Effect::Notify {
                title: format!("{finished} finished"),
                body: format!("Up next: {}", self.phase),
            });
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
        self.remaining = self.settings.duration(new_phase);
    }

    /// Fraction of the current phase that has elapsed, always within 0.0..=1.0.
    pub fn progress(&self) -> f64 {
        let total = self.settings.duration(self.phase).as_secs_f64();
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

#[cfg(test)]
impl Phase {
    /// The length a phase has before any settings change it.
    fn map_phase_to_default_timer(&self) -> Duration {
        Settings::default().duration(*self)
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

#[cfg(test)]
mod mode_tests;
