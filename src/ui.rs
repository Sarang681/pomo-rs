use crate::digits::{DIGIT_HEIGHT, big_digits, big_digits_width};
use crate::model::{LONG_BREAK_LIMIT, Mode, Model, Phase, RunState};
use crate::settings::SETTINGS_ROWS;
use crate::stats::weekday_label;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Color, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

const PROGRESS_BAR_WIDTH: u16 = 40;
/// Space the progress bar leaves for the percentage: two spaces, three digits and `%`.
const PERCENTAGE_WIDTH: u16 = 6;
/// Terminals this short (a tmux status pane, say) get a single line instead of the full screen.
const COMPACT_MAX_HEIGHT: u16 = 3;
const STATS_DAYS: u64 = 7;
const STATS_BAR_WIDTH: usize = 20;
/// Columns taken by the `Task: ` prefix and the cursor while editing.
const TASK_EDIT_CHROME: usize = 8;

pub fn render(model: &Model, area: Rect, buf: &mut Buffer) {
    if area.height <= COMPACT_MAX_HEIGHT {
        render_compact(model, area, buf);
        return;
    }

    let phase_color = map_phase_to_color(&model.phase);
    let title = Line::from("pomo-rs").fg(phase_color).bold();
    let block = Block::bordered()
        .title(title.left_aligned())
        .border_set(border::THICK);
    let inner = block.inner(area);
    block.render(area, buf);

    let [main_area, footer_area] = inner.layout(&Layout::vertical([Fill(1), Length(1)]));

    match model.mode {
        Mode::Help => render_help(main_area, buf),
        Mode::Settings => render_settings(model, main_area, buf, &phase_color),
        Mode::Stats => render_stats(model, main_area, buf, &phase_color),
        Mode::Normal | Mode::EditingTask | Mode::ConfirmingQuit => {
            let [
                task_area,
                session_area,
                _,
                timer_area,
                progress_area,
                state_area,
                _,
            ] = main_area.layout(&Layout::vertical([
                Length(1),
                Length(2),
                Length(1),
                Fill(1),
                Length(1),
                Length(1),
                Length(1),
            ]));

            render_task(model, task_area, buf);
            render_session_info(model, session_area, buf, &phase_color);
            render_timer(model, timer_area, buf, &phase_color);
            render_progress(model, progress_area, buf, &phase_color);
            render_state(model, state_area, buf, &phase_color);
        }
    }
    render_footer(model, footer_area, buf, &phase_color);
}

/// One line for very short terminals: phase, time, run state, session count and task.
fn render_compact(model: &Model, area: Rect, buf: &mut Buffer) {
    let color = if model.run_state == RunState::Paused {
        Color::Gray
    } else {
        map_phase_to_color(&model.phase)
    };
    let icon = match model.run_state {
        RunState::Idle => "○",
        RunState::Running => "▶",
        RunState::Paused => "▮▮",
    };
    let done = match model.phase {
        Phase::Focus => (model.sessions_completed + 1).min(LONG_BREAK_LIMIT),
        _ => model.sessions_completed,
    };
    let mut text = format!(
        "{} {} {icon} {done}/{LONG_BREAK_LIMIT}",
        model.phase,
        model.format_time_remaining()
    );
    if !model.task.is_empty() {
        text.push_str(&format!(" · {}", model.task));
    }

    Paragraph::new(Line::from(text).bold())
        .fg(color)
        .render(area, buf);
}

fn render_task(model: &Model, area: Rect, buf: &mut Buffer) {
    let line = match model.mode {
        Mode::EditingTask => {
            let room = usize::from(area.width).saturating_sub(TASK_EDIT_CHROME);
            Line::from(format!("Task: {}▏", tail(&model.draft, room))).fg(Color::Yellow)
        }
        _ if model.task.is_empty() => {
            Line::from("no task · press t to add one").fg(Color::DarkGray)
        }
        _ => Line::from(format!("▸ {}", model.task)).bold(),
    };

    Paragraph::new(line).centered().render(area, buf);
}

/// The last `max` characters of `text`, marked with `…` when something was cut off, so the
/// end you are typing at stays visible.
fn tail(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    let kept: String = text.chars().skip(count - max.saturating_sub(1)).collect();
    format!("…{kept}")
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
    let dots: Vec<Span> = (0..LONG_BREAK_LIMIT)
        .flat_map(|i| {
            let dot = if i < model.sessions_completed {
                Span::styled("●", *phase_color)
            } else {
                Span::styled("○", Color::DarkGray)
            };
            [dot, Span::raw(" ")]
        })
        .take(usize::from(LONG_BREAK_LIMIT) * 2 - 1)
        .collect();

    Paragraph::new(vec![
        Line::from(label).bold().fg(*phase_color),
        Line::from(dots),
    ])
    .centered()
    .render(area, buf);
}

fn render_timer(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
    let time = model.format_time_remaining();
    let fg_color = if model.run_state == RunState::Paused {
        Color::Gray
    } else {
        *phase_color
    };

    let digits_fit = usize::from(area.height) >= DIGIT_HEIGHT
        && usize::from(area.width) >= big_digits_width(&time);

    if digits_fit {
        let [_, digits_area, _] = area.layout(&Layout::vertical([
            Fill(1),
            Length(DIGIT_HEIGHT as u16),
            Fill(1),
        ]));
        let rows: Vec<Line> = big_digits(&time).into_iter().map(Line::from).collect();

        Paragraph::new(rows)
            .centered()
            .fg(fg_color)
            .render(digits_area, buf);
    } else {
        let [_, time_area, _] = area.layout(&Layout::vertical([Fill(1), Length(1), Fill(1)]));

        Paragraph::new(Line::from(time).bold())
            .centered()
            .fg(fg_color)
            .render(time_area, buf);
    }
}

fn render_progress(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
    let ratio = model.progress();
    let width = usize::from(PROGRESS_BAR_WIDTH.min(area.width.saturating_sub(PERCENTAGE_WIDTH)));
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
    if model.mode == Mode::ConfirmingQuit {
        Paragraph::new(Line::from("Quit and lose this focus session?  [y] yes  [n] no").bold())
            .centered()
            .fg(Color::LightRed)
            .render(area, buf);
        return;
    }

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

fn render_help(area: Rect, buf: &mut Buffer) {
    const HELP: [(&str, &str); 11] = [
        ("space", "start, pause or resume"),
        ("s", "skip to the next phase"),
        ("r", "reset the current phase"),
        ("+ / -", "change this phase's length (when ready)"),
        ("t", "set what you are working on"),
        ("Tab", "stats"),
        (",", "settings"),
        ("?", "this help"),
        ("q", "quit (asks during a focus session)"),
        ("Ctrl-C", "quit immediately"),
        ("esc", "close a screen"),
    ];

    let mut lines = vec![Line::from("Keys").bold(), Line::from("")];
    lines.extend(HELP.iter().map(|(key, what)| {
        Line::from(vec![
            Span::styled(format!("  {key:<8}"), Color::Yellow).bold(),
            Span::raw(*what),
        ])
    }));
    Paragraph::new(lines).render(area, buf);
}

fn render_settings(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
    let mut lines = vec![Line::from("Settings").bold(), Line::from("")];
    for (index, row) in SETTINGS_ROWS.into_iter().enumerate() {
        let selected = index == model.settings_cursor;
        let marker = if selected { "▸" } else { " " };
        let text = format!(
            "  {marker} {:<24}{}",
            row.label(),
            model.settings.value_text(row)
        );
        lines.push(if selected {
            Line::from(text).bold().fg(*phase_color)
        } else {
            Line::from(text)
        });
    }
    lines.push(Line::from(""));
    lines.push(
        Line::from("  Saved to the config file when you close this screen.").fg(Color::DarkGray),
    );
    Paragraph::new(lines).render(area, buf);
}

fn render_stats(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
    let history = &model.history;
    let today = model.today;
    let days = history.last_days(today, STATS_DAYS);
    let busiest = days
        .iter()
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(0)
        .max(1);

    let sessions_today = history.sessions_on(today);
    let streak = history.streak(today);
    let mut lines = vec![
        Line::from("Stats").bold(),
        Line::from(""),
        Line::from(format!(
            "  Today      {} {} · {} min",
            sessions_today,
            plural(sessions_today as u64, "session"),
            history.minutes_on(today)
        )),
        Line::from(format!("  Streak     {} {}", streak, plural(streak, "day"))),
        Line::from(format!(
            "  All time   {} sessions",
            history.total_sessions()
        )),
        Line::from(""),
        Line::from(format!("  Last {STATS_DAYS} days")).bold(),
    ];
    for (day, count) in days {
        let bar = "█".repeat(count * STATS_BAR_WIDTH / busiest);
        let marker = if day == today { " today" } else { "" };
        lines.push(Line::from(vec![
            Span::raw(format!("  {}  ", weekday_label(day))),
            Span::styled(
                format!("{bar:<width$}", width = STATS_BAR_WIDTH),
                *phase_color,
            ),
            Span::raw(format!(" {count}{marker}")),
        ]));
    }
    Paragraph::new(lines).render(area, buf);
}

fn plural(count: u64, word: &str) -> String {
    if count == 1 {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

fn render_footer(model: &Model, area: Rect, buf: &mut Buffer, phase_color: &Color) {
    let hints: Vec<(&str, &str)> = match model.mode {
        Mode::Normal => vec![
            ("space", space_action(&model.run_state)),
            ("s", "skip"),
            ("r", "reset"),
            ("q", "quit"),
            ("t", "task"),
            ("?", "help"),
        ],
        Mode::EditingTask => vec![("enter", "save"), ("esc", "cancel")],
        Mode::ConfirmingQuit => vec![("y", "quit"), ("n", "stay")],
        Mode::Help | Mode::Stats => vec![("esc", "close")],
        Mode::Settings => vec![
            ("↑/↓", "select"),
            ("←/→", "change"),
            ("esc", "save & close"),
        ],
    };

    let mut spans = vec![Span::raw(" ")];
    for (index, (key, action)) in hints.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(format!("[{key}]"), *phase_color).bold());
        spans.push(Span::raw(format!(" {action}")));
    }

    Paragraph::new(Line::from(spans)).render(area, buf);
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
mod tests;
