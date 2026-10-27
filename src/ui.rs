use crate::model::{LONG_BREAK_LIMIT, Model, Phase, RunState};
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Color, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

const PROGRESS_BAR_WIDTH: u16 = 40;

pub fn render(model: &Model, area: Rect, buf: &mut Buffer) {
    let phase_color = map_phase_to_color(&model.phase);
    let title = Line::from("pomo-rs").fg(phase_color).bold();
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

    render_session_info(model, session_area, buf, &phase_color);
    render_timer(model, timer_area, buf, &phase_color);
    render_progress(model, progress_area, buf, &phase_color);
    render_state(model, state_area, buf, &phase_color);
    render_footer(model, footer_area, buf);
}

fn render_session_info(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
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

    Paragraph::new(vec![
        Line::from(label).bold().fg(*phase_color),
        Line::from(dots),
    ])
    .centered()
    .render(area, buf);
}

fn render_timer(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
    let [_, time_area, _] = area.layout(&Layout::vertical([Fill(1), Length(1), Fill(1)]));

    let fg_color = if model.run_state == RunState::Paused {
        Color::Gray
    } else {
        *phase_color
    };

    Paragraph::new(Line::from(model.format_time_remaining()).bold())
        .centered()
        .fg(fg_color)
        .render(time_area, buf);
}

fn render_progress(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
    let ratio = model.progress();
    let width = usize::from(PROGRESS_BAR_WIDTH);
    let filled = (ratio * width as f64).round() as usize;
    let filled_color = if model.run_state == RunState::Running {
        *phase_color
    } else {
        Color::Gray
    };

    let bar = Line::from(vec![
        Span::styled("█".repeat(filled), filled_color),
        Span::styled("░".repeat(width - filled), Color::DarkGray),
        Span::raw(format!("  {:>3}%", (ratio * 100.0) as u16)),
    ]);

    Paragraph::new(bar).centered().render(area, buf);
}

fn render_state(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
    let fg_color = if model.run_state == RunState::Idle {
        Color::Gray
    } else if model.run_state == RunState::Paused {
        Color::Yellow
    } else {
        *phase_color
    };

    let state_text = if model.run_state == RunState::Paused {
        format!("▮▮ {} • space to resume", model.run_state)
    } else {
        model.run_state.to_string()
    };

    Paragraph::new(Line::from(state_text))
        .centered()
        .fg(fg_color)
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

/// map a run state to an RGB value
fn map_phase_to_color(phase: &Phase) -> Color {
    match phase {
        Phase::Focus => Color::Rgb(255, 122, 99),
        Phase::ShortBreak => Color::Rgb(123, 216, 143),
        Phase::LongBreak => Color::Rgb(108, 182, 255),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_buffer(model: &Model, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render(model, area, &mut buf);
        buf
    }

    fn render_to_string(model: &Model, width: u16, height: u16) -> String {
        let buf = render_to_buffer(model, width, height);

        let mut text = String::new();
        for y in 0..height {
            for x in 0..width {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    /// Foreground color of the first cell of the first occurrence of `needle`.
    fn fg_of(buf: &Buffer, needle: &str) -> Color {
        let area = buf.area;
        for y in area.top()..area.bottom() {
            let line: String = (area.left()..area.right())
                .map(|x| buf[(x, y)].symbol())
                .collect();
            if let Some(byte_index) = line.find(needle) {
                let x = area.left() + line[..byte_index].chars().count() as u16;
                return buf[(x, y)].fg;
            }
        }
        panic!("{needle:?} was not rendered");
    }

    fn model_with_state(run_state: RunState) -> Model {
        let fresh = Model::default();
        Model {
            run_state,
            remaining: fresh.remaining / 2,
            ..fresh
        }
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

    // ---- colors ----

    #[test]
    fn test_each_phase_has_its_own_color() {
        assert_eq!(map_phase_to_color(&Phase::Focus), Color::Rgb(255, 122, 99));
        assert_eq!(
            map_phase_to_color(&Phase::ShortBreak),
            Color::Rgb(123, 216, 143)
        );
        assert_eq!(
            map_phase_to_color(&Phase::LongBreak),
            Color::Rgb(108, 182, 255)
        );
    }

    #[test]
    fn test_phase_colors_are_distinct() {
        let focus = map_phase_to_color(&Phase::Focus);
        let short_break = map_phase_to_color(&Phase::ShortBreak);
        let long_break = map_phase_to_color(&Phase::LongBreak);

        assert_ne!(focus, short_break);
        assert_ne!(focus, long_break);
        assert_ne!(short_break, long_break);
    }

    #[test]
    fn test_title_and_phase_label_use_the_phase_color() {
        for phase in [Phase::Focus, Phase::ShortBreak, Phase::LongBreak] {
            let model = Model {
                phase,
                ..Model::default()
            };

            let buf = render_to_buffer(&model, 60, 20);

            let expected = map_phase_to_color(&phase);
            assert_eq!(fg_of(&buf, "pomo-rs"), expected);
            assert_eq!(fg_of(&buf, &phase.to_string()), expected);
        }
    }

    #[test]
    fn test_time_uses_the_phase_color_unless_paused() {
        let expected = map_phase_to_color(&Phase::Focus);

        for (run_state, color) in [
            (RunState::Idle, expected),
            (RunState::Running, expected),
            (RunState::Paused, Color::Gray),
        ] {
            let model = model_with_state(run_state);
            let time = model.format_time_remaining();

            let buf = render_to_buffer(&model, 60, 20);

            assert_eq!(fg_of(&buf, &time), color);
        }
    }

    #[test]
    fn test_progress_bar_filled_part_uses_the_phase_color_while_running() {
        let model = model_with_state(RunState::Running);

        let buf = render_to_buffer(&model, 60, 20);

        assert_eq!(fg_of(&buf, "█"), map_phase_to_color(&Phase::Focus));
    }

    #[test]
    fn test_progress_bar_filled_part_is_gray_when_not_running() {
        for run_state in [RunState::Idle, RunState::Paused] {
            let model = model_with_state(run_state);

            let buf = render_to_buffer(&model, 60, 20);

            assert_eq!(fg_of(&buf, "█"), Color::Gray);
        }
    }

    #[test]
    fn test_progress_bar_empty_part_is_never_the_phase_color() {
        for run_state in [RunState::Idle, RunState::Running, RunState::Paused] {
            let model = model_with_state(run_state);

            let buf = render_to_buffer(&model, 60, 20);

            assert_eq!(fg_of(&buf, "░"), Color::DarkGray);
        }
    }

    #[test]
    fn test_progress_percentage_keeps_the_default_color() {
        let model = model_with_state(RunState::Running);

        let buf = render_to_buffer(&model, 60, 20);

        assert_eq!(fg_of(&buf, "50%"), Color::Reset);
    }

    #[test]
    fn test_state_line_is_gray_when_ready() {
        let buf = render_to_buffer(&Model::default(), 60, 20);

        assert_eq!(fg_of(&buf, "Ready"), Color::Gray);
    }

    #[test]
    fn test_state_line_uses_the_phase_color_when_running() {
        let model = model_with_state(RunState::Running);

        let buf = render_to_buffer(&model, 60, 20);

        assert_eq!(fg_of(&buf, "Running"), map_phase_to_color(&Phase::Focus));
    }

    #[test]
    fn test_state_line_is_yellow_and_explains_how_to_resume_when_paused() {
        let model = model_with_state(RunState::Paused);

        let buf = render_to_buffer(&model, 60, 20);
        let text = render_to_string(&model, 60, 20);

        assert!(text.contains("▮▮ PAUSED • space to resume"));
        assert_eq!(fg_of(&buf, "▮▮ PAUSED"), Color::Yellow);
    }

    #[test]
    fn test_phase_color_follows_a_phase_transition() {
        let mut model = Model::default();
        let before = fg_of(&render_to_buffer(&model, 60, 20), "pomo-rs");

        model.update(crate::model::Message::Skip);
        let after = fg_of(&render_to_buffer(&model, 60, 20), "pomo-rs");

        assert_eq!(before, map_phase_to_color(&Phase::Focus));
        assert_eq!(after, map_phase_to_color(&Phase::ShortBreak));
    }

    #[test]
    fn test_space_hint_follows_the_run_state() {
        assert_eq!(space_action(&RunState::Idle), "start");
        assert_eq!(space_action(&RunState::Running), "pause");
        assert_eq!(space_action(&RunState::Paused), "resume");
    }
}
