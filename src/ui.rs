use crate::model::{LONG_BREAK_LIMIT, Model, Phase, RunState};
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Color, Stylize};
use ratatui::symbols::border;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

const PROGRESS_BAR_WIDTH: u16 = 40;

pub fn render(model: &Model, area: Rect, buf: &mut Buffer) {
    let title = Line::from("pomo-rs").fg(Color::Rgb(238, 129, 106)).bold();
    let block = Block::bordered()
        .title(title.left_aligned())
        .border_set(border::THICK);
    let inner = block.inner(area);
    block.render(area, buf);

    let [main_area, footer_area] = inner.layout(&Layout::vertical([Fill(1), Length(1)]));
    let [_, session_area, _, timer_area, progress_area, state_area, _] =
        main_area.layout(&Layout::vertical([
            Length(1),
            Length(2),
            Length(1),
            Fill(1),
            Length(1),
            Length(1),
            Length(1),
        ]));

    render_session_info(model, session_area, buf);
    render_timer(model, timer_area, buf);
    render_progress(model, progress_area, buf);
    render_state(model, state_area, buf);
    render_footer(model, footer_area, buf);
}

fn render_session_info(model: &Model, area: Rect, buf: &mut Buffer) {
    let label = match model.phase {
        Phase::Focus => format!(
            "{}  ·  Session {}/{}",
            model.phase,
            (model.sessions_completed + 1).min(LONG_BREAK_LIMIT),
            LONG_BREAK_LIMIT
        ),
        _ => format!(
            "{}  ·  {}/{} done",
            model.phase, model.sessions_completed, LONG_BREAK_LIMIT
        ),
    };
    let dots = (0..LONG_BREAK_LIMIT)
        .map(|i| {
            if i < model.sessions_completed {
                "●"
            } else {
                "○"
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    Paragraph::new(vec![Line::from(label).bold(), Line::from(dots)])
        .centered()
        .render(area, buf);
}

fn render_timer(model: &Model, area: Rect, buf: &mut Buffer) {
    let [_, time_area, _] = area.layout(&Layout::vertical([Fill(1), Length(1), Fill(1)]));

    Paragraph::new(Line::from(model.format_time_remaining()).bold())
        .centered()
        .render(time_area, buf);
}

fn render_progress(model: &Model, area: Rect, buf: &mut Buffer) {
    let ratio = model.progress();
    let width = usize::from(PROGRESS_BAR_WIDTH);
    let filled = (ratio * width as f64).round() as usize;
    let bar = format!(
        "{}{}  {:>3}%",
        "█".repeat(filled),
        "░".repeat(width - filled),
        (ratio * 100.0) as u16
    );

    Paragraph::new(Line::from(bar)).centered().render(area, buf);
}

fn render_state(model: &Model, area: Rect, buf: &mut Buffer) {
    Paragraph::new(Line::from(model.run_state.to_string()))
        .centered()
        .render(area, buf);
}

fn render_footer(model: &Model, area: Rect, buf: &mut Buffer) {
    let hints = format!(
        " [space] {}  [s] skip  [r] reset  [q] quit",
        space_action(&model.run_state)
    );

    Paragraph::new(Line::from(hints)).render(area, buf);
}

fn space_action(run_state: &RunState) -> &'static str {
    match run_state {
        RunState::Idle => "start",
        RunState::Running => "pause",
        RunState::Paused => "resume",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(model: &Model, width: u16, height: u16) -> String {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render(model, area, &mut buf);

        let mut text = String::new();
        for y in 0..height {
            for x in 0..width {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn test_render_shows_every_region() {
        let text = render_to_string(&Model::default(), 60, 20);

        assert!(text.contains("pomo-rs"));
        assert!(text.contains("FOCUS  ·  Session 1/4"));
        assert!(text.contains("○ ○ ○ ○"));
        assert!(text.contains("25:00"));
        assert!(text.contains("0%"));
        assert!(text.contains("Ready"));
        assert!(text.contains("[space] start"));
    }

    #[test]
    fn test_render_shows_break_progress_and_filled_dots() {
        let model = Model {
            phase: Phase::ShortBreak,
            sessions_completed: 2,
            ..Model::default()
        };

        let text = render_to_string(&model, 60, 20);

        assert!(text.contains("SHORT BREAK  ·  2/4 done"));
        assert!(text.contains("● ● ○ ○"));
    }

    #[test]
    fn test_render_draws_the_progress_bar_in_proportion() {
        let fresh = Model::default();
        let model = Model {
            remaining: fresh.remaining / 2,
            ..fresh
        };

        let text = render_to_string(&model, 60, 20);

        assert!(text.contains(&format!("{}{}   50%", "█".repeat(20), "░".repeat(20))));
    }

    #[test]
    fn test_render_does_not_panic_in_tiny_terminals() {
        for (width, height) in [(0, 0), (1, 1), (5, 3), (20, 6), (40, 8), (200, 60)] {
            render_to_string(&Model::default(), width, height);
        }
    }

    #[test]
    fn test_space_hint_follows_the_run_state() {
        assert_eq!(space_action(&RunState::Idle), "start");
        assert_eq!(space_action(&RunState::Running), "pause");
        assert_eq!(space_action(&RunState::Paused), "resume");
    }
}
