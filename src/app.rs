use crate::model::Message::Tick;
use crate::model::{LONG_BREAK_LIMIT, Message, Model};
use color_eyre::eyre::Context;
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct App {
    model: Model,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        let mut prev_time = Instant::now();
        while !self.model.exit {
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
        }
        Ok(())
    }

    fn send_tick(&mut self, prev_time: &mut Instant) {
        let current_time = Instant::now();
        let time_passed = current_time - *prev_time;
        self.model.update(Tick(time_passed));
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
            let message = self
                .convert_key_press_to_message(key_event)
                .wrap_err_with(|| format!("handling key event failed\n{key_event:?}"))?;
            return Ok(message);
        }
        Ok(None)
    }

    fn convert_key_press_to_message(
        &self,
        key_event: KeyEvent,
    ) -> color_eyre::Result<Option<Message>> {
        let message = match key_event.code {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char(' ') => Some(Message::ToggleTimer),
            KeyCode::Char('r') => Some(Message::Reset),
            KeyCode::Char('s') => Some(Message::Skip),
            _ => None,
        };
        Ok(message)
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let title = Line::from("pomo-rs")
            .style(Style::default())
            .fg(Color::Rgb(238, 129, 106))
            .bold();
        let current_phase = &self.model.phase;
        let current_state = &self.model.run_state;
        let current_remaining = self.model.format_time_remaining();
        let current_sessions = self.model.sessions_completed;
        let phase = Line::from(format!("{current_phase}")).centered();
        let state = Line::from(format!("{current_state}")).centered();
        let remaining = Line::from(format!("{current_remaining} "))
            .centered()
            .bold();
        let sessions =
            Line::from(format!("{}/{} done", current_sessions, LONG_BREAK_LIMIT)).centered();
        let final_text = Text::from(vec![phase, state, remaining, sessions]);
        let block = Block::bordered()
            .title(title.left_aligned())
            .border_set(border::THICK);

        Paragraph::new(final_text).block(block).render(area, buf);
    }
}
