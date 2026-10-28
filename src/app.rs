use crate::input::key_to_message;
use crate::model::{Effect, Message, Model};
use crate::stats::History;
use crate::{alerts, storage, ui};
use color_eyre::eyre::Context;
use crossterm::event;
use crossterm::event::{Event, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use ratatui::{DefaultTerminal, Frame};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

#[derive(Debug, Default)]
pub struct App {
    model: Model,
    config_path: Option<PathBuf>,
    history_path: Option<PathBuf>,
    /// Problems saving files. They are shown after the terminal is restored.
    pub warnings: Vec<String>,
}

impl App {
    /// Builds the app from the config and history files. Fails, before the screen is touched,
    /// if either file exists but can't be understood.
    pub fn load() -> color_eyre::Result<Self> {
        let config_path = storage::config_path();
        let history_path = storage::history_path();

        let settings = match &config_path {
            Some(path) => storage::load_settings(path)?,
            None => Default::default(),
        };
        let history = match &history_path {
            Some(path) => storage::load_history(path)?,
            None => History::default(),
        };

        Ok(Self {
            model: Model::new(settings, history),
            config_path,
            history_path,
            warnings: Vec::new(),
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        let mut prev_time = Instant::now();
        while !self.model.exit {
            self.model.today = current_day();
            //draw ui
            terminal.draw(|frame| self.draw(frame))?;
            //handle events
            let message = self
                .poll_and_convert_to_message()
                .wrap_err("handle events failed")?;
            //send a tick message every loop
            self.send_tick(&mut prev_time);
            //send update message if keystroke present
            if let Some(message) = message {
                self.model.update(message);
            }
            self.run_effects();
        }
        Ok(())
    }

    fn send_tick(&mut self, prev_time: &mut Instant) {
        let current_time = Instant::now();
        let time_passed = current_time - *prev_time;
        self.model.update(Message::Tick(time_passed));
        *prev_time = current_time;
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn poll_and_convert_to_message(&self) -> color_eyre::Result<Option<Message>> {
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key_event) = event::read()?
            && key_event.kind == KeyEventKind::Press
        {
            return Ok(key_to_message(self.model.mode, key_event));
        }
        Ok(None)
    }

    /// Performs the work the model queued. Save failures become warnings instead of crashes.
    fn run_effects(&mut self) {
        for effect in self.model.take_effects() {
            match effect {
                Effect::Bell => alerts::bell(),
                Effect::Notify { title, body } => alerts::notify(&title, &body),
                Effect::SaveHistory => {
                    if let Some(path) = &self.history_path
                        && let Err(error) = storage::save_history(path, &self.model.history)
                    {
                        self.warnings.push(format!("{error:#}"));
                    }
                }
                Effect::SaveSettings => {
                    if let Some(path) = &self.config_path
                        && let Err(error) = storage::save_settings(path, &self.model.settings)
                    {
                        self.warnings.push(format!("{error:#}"));
                    }
                }
            }
        }
    }
}

/// Whole days since the Unix epoch (UTC).
fn current_day() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() / SECONDS_PER_DAY)
        .unwrap_or(0)
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        ui::render(&self.model, area, buf);
    }
}
