use crate::digits::big_digits;
use crate::model::{Mode, Model, Phase, RunState};
use crate::stats::{History, SessionRecord};
use crate::ui::{map_phase_to_color, render, space_action};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

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

/// Tallest terminal height that is still too short for big digits: the timer region gets
/// 4 rows (14 minus 2 border rows, 1 footer row and 7 fixed rows), and the digits need 5.
const SHORT_HEIGHT: u16 = 14;
/// Shortest terminal height that has room for big digits.
const TALL_HEIGHT: u16 = SHORT_HEIGHT + 1;

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
    let text = render_to_string(&Model::default(), 60, SHORT_HEIGHT);

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

// ---- big digits and small terminals ----

#[test]
fn test_big_digits_are_drawn_when_there_is_room() {
    let model = Model::default();
    let time = model.format_time_remaining();

    let text = render_to_string(&model, 60, 20);

    for row in big_digits(&time) {
        assert!(text.contains(&row), "missing row {row:?}");
    }
    assert!(!text.contains(&time));
}

#[test]
fn test_big_digits_appear_at_the_smallest_height_that_fits() {
    let model = Model::default();
    let time = model.format_time_remaining();

    let text = render_to_string(&model, 60, TALL_HEIGHT);

    assert!(text.contains(&big_digits(&time)[0]));
    assert!(!text.contains(&time));
}

#[test]
fn test_plain_time_is_drawn_when_the_terminal_is_too_short() {
    let model = Model::default();
    let time = model.format_time_remaining();

    let text = render_to_string(&model, 60, SHORT_HEIGHT);

    assert!(text.contains(&time));
    assert!(!text.contains(&big_digits(&time)[0]));
}

#[test]
fn test_plain_time_is_drawn_when_the_terminal_is_too_narrow() {
    let model = Model::default();
    let time = model.format_time_remaining();

    let text = render_to_string(&model, 18, 20);

    assert!(text.contains(&time));
}

#[test]
fn test_big_digits_follow_the_phase_color_and_gray_when_paused() {
    let running = render_to_buffer(&model_with_state(RunState::Running), 60, 20);
    let paused = render_to_buffer(&model_with_state(RunState::Paused), 60, 20);

    assert_eq!(fg_of(&running, "█"), map_phase_to_color(&Phase::Focus));
    assert_eq!(fg_of(&paused, "█"), Color::Gray);
}

#[test]
fn test_big_digits_use_the_color_of_the_current_phase() {
    for phase in [Phase::Focus, Phase::ShortBreak, Phase::LongBreak] {
        let model = Model {
            phase,
            run_state: RunState::Running,
            ..Model::default()
        };

        let buf = render_to_buffer(&model, 60, 20);

        assert_eq!(fg_of(&buf, "█"), map_phase_to_color(&phase));
    }
}

#[test]
fn test_progress_bar_is_full_width_in_a_wide_terminal() {
    let text = render_to_string(&Model::default(), 60, 20);

    assert_eq!(text.matches('░').count(), 40);
}

#[test]
fn test_progress_bar_shrinks_to_keep_the_percentage_visible() {
    // 30 columns minus 2 border columns leaves 28; the percentage needs 6.
    let text = render_to_string(&Model::default(), 30, 20);

    assert_eq!(text.matches('░').count(), 22);
    assert!(text.contains("  0%"));
}

#[test]
fn test_progress_bar_keeps_the_percentage_in_a_very_narrow_terminal() {
    // 10 columns minus 2 border columns leaves 8: a 2 column bar plus the 6 column percentage.
    let text = render_to_string(&Model::default(), 10, 20);

    assert_eq!(text.matches('░').count(), 2);
    assert!(text.contains("  0%"));
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

        let buf = render_to_buffer(&model, 60, SHORT_HEIGHT);

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

        let buf = render_to_buffer(&model, 60, SHORT_HEIGHT);

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

// ---- task row, modes, overlays and compact mode ----

fn model_in(mode: Mode) -> Model {
    Model {
        mode,
        ..Model::default()
    }
}

#[test]
fn test_task_row_invites_you_to_add_a_task() {
    let text = render_to_string(&Model::default(), 60, 20);

    assert!(text.contains("press t to add one"));
}

#[test]
fn test_task_row_shows_the_current_task() {
    let model = Model {
        task: "write docs".into(),
        ..Model::default()
    };
    let text = render_to_string(&model, 60, 20);

    assert!(text.contains("▸ write docs"));
    assert!(!text.contains("press t to add one"));
}

#[test]
fn test_task_row_shows_the_draft_with_a_cursor_while_editing() {
    let model = Model {
        mode: Mode::EditingTask,
        task: "old".into(),
        draft: "new tex".into(),
        ..Model::default()
    };
    let text = render_to_string(&model, 60, 20);

    assert!(text.contains("Task: new tex▏"));
    assert!(!text.contains("old"));
}

#[test]
fn test_a_long_draft_keeps_its_end_visible() {
    let model = Model {
        mode: Mode::EditingTask,
        draft: format!("{}END", "x".repeat(100)),
        ..Model::default()
    };
    let text = render_to_string(&model, 30, 20);

    assert!(text.contains("END▏"));
    assert!(text.contains('…'));
}

#[test]
fn test_tail_keeps_short_text_and_marks_cut_text() {
    use crate::ui::tail;

    assert_eq!(tail("abc", 5), "abc");
    assert_eq!(tail("abcdef", 4), "…def");
    assert_eq!(tail("abc", 0), "…");
}

#[test]
fn test_quit_prompt_replaces_the_state_line() {
    let text = render_to_string(&model_in(Mode::ConfirmingQuit), 70, 20);

    assert!(text.contains("Quit and lose this focus session?"));
    assert!(text.contains("[y] quit"));
    assert!(!text.contains("Ready"));
}

#[test]
fn test_footer_follows_the_mode() {
    let normal = render_to_string(&model_in(Mode::Normal), 70, 20);
    let editing = render_to_string(&model_in(Mode::EditingTask), 70, 20);
    let settings = render_to_string(&model_in(Mode::Settings), 70, 20);

    assert!(normal.contains("[space] start") && normal.contains("[t] task"));
    assert!(editing.contains("[enter] save") && editing.contains("[esc] cancel"));
    assert!(!editing.contains("[space]"));
    assert!(settings.contains("save & close"));
}

#[test]
fn test_help_lists_the_keys_and_hides_the_timer() {
    let text = render_to_string(&model_in(Mode::Help), 70, 20);

    for needle in ["Keys", "skip to the next phase", "settings", "Ctrl-C"] {
        assert!(text.contains(needle), "{needle:?} missing");
    }
    assert!(!text.contains("Ready"));
}

#[test]
fn test_settings_shows_values_and_marks_the_selected_row() {
    let model = Model {
        mode: Mode::Settings,
        settings_cursor: 1,
        ..Model::default()
    };
    let text = render_to_string(&model, 70, 20);

    assert!(text.contains("Focus length"));
    assert!(text.contains("25 min"));
    assert!(text.contains("▸ Short break length"));
    assert!(!text.contains("▸ Focus length"));
    assert!(text.contains("Desktop notifications"));
}

#[test]
fn test_stats_shows_totals_streak_and_a_bar_per_day() {
    let mut history = History::default();
    for day in [99, 100, 100] {
        history.record(SessionRecord {
            day,
            minutes: 25,
            task: String::new(),
        });
    }
    let model = Model {
        mode: Mode::Stats,
        today: 100,
        history,
        ..Model::default()
    };
    let text = render_to_string(&model, 70, 24);

    assert!(text.contains("Today      2 sessions · 50 min"));
    assert!(text.contains("Streak     2 days"));
    assert!(text.contains("All time   3 sessions"));
    assert!(text.contains(&format!("{} 2 today", "█".repeat(20))));
    assert!(text.contains(&format!("{}{} 1", "█".repeat(10), " ".repeat(10))));
}

#[test]
fn test_stats_uses_singular_words_and_copes_with_no_history() {
    let model = Model {
        mode: Mode::Stats,
        today: 100,
        ..Model::default()
    };
    let text = render_to_string(&model, 70, 24);

    assert!(text.contains("Today      0 sessions"));
    assert!(text.contains("Streak     0 days"));
    assert_eq!(crate::ui::plural(1, "day"), "day");
}

#[test]
fn test_stats_does_not_panic_near_the_epoch_or_in_small_terminals() {
    for (today, width, height) in [(0, 70, 24), (3, 10, 5), (100, 70, 8)] {
        let model = Model {
            mode: Mode::Stats,
            today,
            ..Model::default()
        };
        render_to_string(&model, width, height);
    }
}

#[test]
fn test_short_terminals_get_one_compact_line() {
    let text = render_to_string(&Model::default(), 40, 1);

    assert!(text.contains("FOCUS 25:00 ○ 1/4"));
    assert!(!text.contains("pomo-rs"));
}

#[test]
fn test_compact_line_shows_state_and_task_and_follows_the_phase_color() {
    let model = Model {
        phase: Phase::ShortBreak,
        run_state: RunState::Running,
        sessions_completed: 2,
        task: "email".into(),
        ..Model::default()
    };
    let buf = render_to_buffer(&model, 40, 2);
    let text = render_to_string(&model, 40, 2);

    assert!(text.contains("SHORT BREAK 25:00 ▶ 2/4 · email"));
    assert_eq!(
        fg_of(&buf, "SHORT BREAK"),
        map_phase_to_color(&Phase::ShortBreak)
    );
}

#[test]
fn test_compact_line_is_gray_when_paused() {
    let model = model_with_state(RunState::Paused);
    let buf = render_to_buffer(&model, 40, 1);

    assert_eq!(fg_of(&buf, "FOCUS"), Color::Gray);
    assert!(render_to_string(&model, 40, 1).contains("▮▮"));
}

#[test]
fn test_full_screen_returns_just_above_the_compact_height() {
    assert!(render_to_string(&Model::default(), 60, 4).contains("pomo-rs"));
}

#[test]
fn test_filled_session_dots_use_the_phase_color() {
    let model = Model {
        sessions_completed: 2,
        ..Model::default()
    };
    let buf = render_to_buffer(&model, 60, 20);

    assert_eq!(fg_of(&buf, "●"), map_phase_to_color(&Phase::Focus));
    assert_eq!(fg_of(&buf, "○"), Color::DarkGray);
}
