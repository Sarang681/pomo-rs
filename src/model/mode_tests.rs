//! Modes, task editing, quit confirmation, time adjustment, overlays, alerts and history.

use crate::model::{Effect, MAX_TASK_LEN, Message, Mode, Model, Phase, RunState};
use crate::settings::Settings;
use std::time::Duration;

fn type_text(model: &mut Model, text: &str) {
    for c in text.chars() {
        model.update(Message::TypeChar(c));
    }
}

fn running(phase: Phase) -> Model {
    Model {
        phase,
        remaining: Settings::default().duration(phase),
        run_state: RunState::Running,
        ..Model::default()
    }
}

/// Ticks past the end of the phase.
fn finish(model: &mut Model) {
    model.update(Message::Tick(Duration::from_hours(2)));
}

// ---- task editing ----

#[test]
fn test_editing_saves_the_draft_as_the_task() {
    let mut model = Model::default();
    model.update(Message::StartEditing);
    type_text(&mut model, "write docs");
    model.update(Message::FinishEditing);

    assert_eq!(model.task, "write docs");
    assert_eq!(model.mode, Mode::Normal);
    assert!(model.draft.is_empty());
}

#[test]
fn test_editing_starts_from_the_current_task() {
    let mut model = Model {
        task: "old".into(),
        ..Model::default()
    };
    model.update(Message::StartEditing);
    type_text(&mut model, "er");
    model.update(Message::FinishEditing);

    assert_eq!(model.task, "older");
}

#[test]
fn test_cancelling_keeps_the_old_task() {
    let mut model = Model {
        task: "keep".into(),
        ..Model::default()
    };
    model.update(Message::StartEditing);
    type_text(&mut model, " this?");
    model.update(Message::CancelEditing);

    assert_eq!(model.task, "keep");
    assert_eq!(model.mode, Mode::Normal);
}

#[test]
fn test_backspace_removes_the_last_character_and_is_safe_when_empty() {
    let mut model = Model::default();
    model.update(Message::StartEditing);
    type_text(&mut model, "ab");
    model.update(Message::Backspace);
    assert_eq!(model.draft, "a");

    model.update(Message::Backspace);
    model.update(Message::Backspace);
    assert_eq!(model.draft, "");
}

#[test]
fn test_saved_task_is_trimmed() {
    let mut model = Model::default();
    model.update(Message::StartEditing);
    type_text(&mut model, "  spaced  ");
    model.update(Message::FinishEditing);

    assert_eq!(model.task, "spaced");
}

#[test]
fn test_draft_length_is_capped_and_counts_characters_not_bytes() {
    let mut model = Model::default();
    model.update(Message::StartEditing);
    type_text(&mut model, &"é".repeat(MAX_TASK_LEN + 10));

    assert_eq!(model.draft.chars().count(), MAX_TASK_LEN);
}

#[test]
fn test_control_characters_are_not_typed() {
    let mut model = Model::default();
    model.update(Message::StartEditing);
    model.update(Message::TypeChar('\n'));
    model.update(Message::TypeChar('\t'));

    assert!(model.draft.is_empty());
}

#[test]
fn test_typing_is_ignored_outside_editing() {
    let mut model = Model::default();
    type_text(&mut model, "nope");
    model.update(Message::Backspace);
    model.update(Message::FinishEditing);

    assert!(model.draft.is_empty());
    assert!(model.task.is_empty());
}

#[test]
fn test_the_timer_keeps_running_while_editing() {
    let mut model = running(Phase::Focus);
    model.update(Message::StartEditing);
    model.update(Message::Tick(Duration::from_secs(60)));

    assert_eq!(model.remaining, Duration::from_mins(24));
}

// ---- quit confirmation ----

#[test]
fn test_quit_is_immediate_when_idle() {
    let mut model = Model::default();
    model.update(Message::AskToQuit);

    assert!(model.exit);
}

#[test]
fn test_quit_asks_while_a_focus_session_runs_or_is_paused() {
    for run_state in [RunState::Running, RunState::Paused] {
        let mut model = Model {
            run_state,
            ..Model::default()
        };
        model.update(Message::AskToQuit);

        assert!(!model.exit);
        assert_eq!(model.mode, Mode::ConfirmingQuit);
    }
}

#[test]
fn test_quit_is_immediate_during_a_break_even_when_running() {
    let mut model = running(Phase::ShortBreak);
    model.update(Message::AskToQuit);

    assert!(model.exit);
}

#[test]
fn test_cancelling_the_quit_returns_to_normal_and_keeps_the_timer() {
    let mut model = running(Phase::Focus);
    model.update(Message::AskToQuit);
    model.update(Message::CancelQuit);

    assert_eq!(model.mode, Mode::Normal);
    assert!(!model.exit);
    assert_eq!(model.run_state, RunState::Running);
}

#[test]
fn test_cancel_quit_does_not_close_other_screens() {
    let mut model = Model::default();
    model.update(Message::OpenHelp);
    model.update(Message::CancelQuit);

    assert_eq!(model.mode, Mode::Help);
}

// ---- adjusting time ----

#[test]
fn test_adjusting_time_changes_the_phase_length_and_remaining() {
    let mut model = Model::default();
    model.update(Message::AdjustTime(5));

    assert_eq!(model.remaining, Duration::from_mins(30));
    assert_eq!(model.settings.focus_minutes, 30);
    assert_eq!(model.progress(), 0.0);
}

#[test]
fn test_adjusting_only_affects_the_current_phase() {
    let mut model = Model {
        phase: Phase::ShortBreak,
        remaining: Duration::from_mins(5),
        ..Model::default()
    };
    model.update(Message::AdjustTime(-2));

    assert_eq!(model.settings.short_break_minutes, 3);
    assert_eq!(model.settings.focus_minutes, 25);
}

#[test]
fn test_adjusting_never_goes_below_one_minute() {
    let mut model = Model::default();
    model.update(Message::AdjustTime(-100));

    assert_eq!(model.remaining, Duration::from_mins(1));
}

#[test]
fn test_adjusting_is_ignored_while_started() {
    for run_state in [RunState::Running, RunState::Paused] {
        let mut model = Model {
            run_state,
            ..Model::default()
        };
        model.update(Message::AdjustTime(5));

        assert_eq!(model.settings.focus_minutes, 25);
        assert_eq!(model.remaining, Duration::from_mins(25));
    }
}

#[test]
fn test_adjusting_asks_for_the_settings_to_be_saved() {
    let mut model = Model::default();
    model.update(Message::AdjustTime(1));

    assert_eq!(model.take_effects(), vec![Effect::SaveSettings]);
}

// ---- overlays and settings ----

#[test]
fn test_overlays_open_and_close() {
    for (open, mode) in [
        (Message::OpenHelp, Mode::Help),
        (Message::OpenSettings, Mode::Settings),
        (Message::OpenStats, Mode::Stats),
    ] {
        let mut model = Model::default();
        model.update(open);
        assert_eq!(model.mode, mode);

        model.update(Message::CloseOverlay);
        assert_eq!(model.mode, Mode::Normal);
    }
}

#[test]
fn test_close_overlay_leaves_editing_and_confirmation_alone() {
    let mut model = Model::default();
    model.update(Message::StartEditing);
    model.update(Message::CloseOverlay);

    assert_eq!(model.mode, Mode::EditingTask);
}

#[test]
fn test_settings_cursor_wraps_around() {
    let mut model = Model::default();
    model.update(Message::OpenSettings);
    model.update(Message::SettingsMove(-1));
    assert_eq!(model.settings_cursor, 4);

    model.update(Message::SettingsMove(1));
    assert_eq!(model.settings_cursor, 0);
}

#[test]
fn test_settings_adjust_changes_the_selected_row_and_the_idle_timer() {
    let mut model = Model::default();
    model.update(Message::OpenSettings);
    model.update(Message::SettingsAdjust(10));

    assert_eq!(model.settings.focus_minutes, 35);
    assert_eq!(model.remaining, Duration::from_mins(35));
}

#[test]
fn test_settings_adjust_toggles_the_bell_row() {
    let mut model = Model::default();
    model.update(Message::OpenSettings);
    model.update(Message::SettingsMove(3));
    model.update(Message::SettingsAdjust(1));

    assert!(!model.settings.bell);
}

#[test]
fn test_shortening_the_phase_in_settings_caps_a_running_timer() {
    let mut model = running(Phase::Focus);
    model.update(Message::OpenSettings);
    model.update(Message::SettingsAdjust(-20));

    assert_eq!(model.remaining, Duration::from_mins(5));
    assert_eq!(model.run_state, RunState::Running);
}

#[test]
fn test_settings_changes_outside_the_settings_screen_are_ignored() {
    let mut model = Model::default();
    model.update(Message::SettingsAdjust(5));
    model.update(Message::SettingsMove(1));

    assert_eq!(model.settings, Settings::default());
    assert_eq!(model.settings_cursor, 0);
}

#[test]
fn test_closing_settings_asks_for_them_to_be_saved() {
    let mut model = Model::default();
    model.update(Message::OpenSettings);
    model.update(Message::CloseOverlay);

    assert_eq!(model.take_effects(), vec![Effect::SaveSettings]);
}

#[test]
fn test_custom_lengths_drive_the_next_phase() {
    let mut model = Model::new(
        Settings {
            short_break_minutes: 7,
            ..Settings::default()
        },
        Default::default(),
    );
    model.update(Message::ToggleTimer);
    finish(&mut model);

    assert_eq!(model.phase, Phase::ShortBreak);
    assert_eq!(model.remaining, Duration::from_mins(7));
}

// ---- alerts and history ----

#[test]
fn test_finishing_focus_rings_notifies_and_saves_history() {
    let mut model = running(Phase::Focus);
    finish(&mut model);

    assert_eq!(
        model.take_effects(),
        vec![
            Effect::SaveHistory,
            Effect::Bell,
            Effect::Notify {
                title: "FOCUS finished".into(),
                body: "Up next: SHORT BREAK".into(),
            },
        ]
    );
}

#[test]
fn test_finishing_a_break_alerts_but_does_not_touch_history() {
    let mut model = running(Phase::ShortBreak);
    finish(&mut model);
    let effects = model.take_effects();

    assert!(effects.contains(&Effect::Bell));
    assert!(!effects.contains(&Effect::SaveHistory));
    assert_eq!(model.history.total_sessions(), 0);
}

#[test]
fn test_alerts_can_be_switched_off() {
    let mut model = running(Phase::Focus);
    model.settings.bell = false;
    model.settings.notifications = false;
    finish(&mut model);

    assert_eq!(model.take_effects(), vec![Effect::SaveHistory]);
}

#[test]
fn test_skipping_raises_no_alerts_and_logs_nothing() {
    let mut model = running(Phase::Focus);
    model.update(Message::Skip);

    assert!(model.take_effects().is_empty());
    assert_eq!(model.history.total_sessions(), 0);
}

#[test]
fn test_a_finished_focus_session_is_logged_with_day_length_and_task() {
    let mut model = running(Phase::Focus);
    model.today = 20_000;
    model.task = "ship it".into();
    finish(&mut model);

    let record = &model.history.sessions[0];
    assert_eq!(record.day, 20_000);
    assert_eq!(record.minutes, 25);
    assert_eq!(record.task, "ship it");
}

#[test]
fn test_taking_effects_empties_the_queue() {
    let mut model = running(Phase::Focus);
    finish(&mut model);

    assert!(!model.take_effects().is_empty());
    assert!(model.take_effects().is_empty());
}
