use crate::model::{LONG_BREAK_LIMIT, Message, Model, Phase, RunState};
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

// ---- phase transitions ----

fn model_with(phase: Phase, run_state: RunState, sessions_completed: u16) -> Model {
    Model {
        phase,
        run_state,
        remaining: phase.map_phase_to_default_timer(),
        sessions_completed,
        ..Model::default()
    }
}

/// Starts the timer if needed, then ticks exactly the time that is left.
fn finish_phase(model: &mut Model) {
    if model.run_state != RunState::Running {
        model.update(Message::ToggleTimer);
    }
    model.update(Message::Tick(model.remaining));
}

#[test]
fn test_focus_completion_moves_to_short_break() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, 0);

    test_model.update(Message::Tick(test_model.remaining));

    assert_eq!(test_model.phase, Phase::ShortBreak);
    assert_eq!(test_model.sessions_completed, 1);
    assert_eq!(test_model.run_state, RunState::Idle);
    assert_eq!(
        test_model.remaining,
        Phase::ShortBreak.map_phase_to_default_timer()
    );
}

#[test]
fn test_short_break_completion_returns_to_focus() {
    let mut test_model = model_with(Phase::ShortBreak, RunState::Running, 1);

    test_model.update(Message::Tick(test_model.remaining));

    assert_eq!(test_model.phase, Phase::Focus);
    assert_eq!(test_model.sessions_completed, 1);
    assert_eq!(test_model.run_state, RunState::Idle);
    assert_eq!(
        test_model.remaining,
        Phase::Focus.map_phase_to_default_timer()
    );
}

#[test]
fn test_final_focus_session_leads_to_long_break() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, LONG_BREAK_LIMIT - 1);

    test_model.update(Message::Tick(test_model.remaining));

    assert_eq!(test_model.phase, Phase::LongBreak);
    assert_eq!(test_model.sessions_completed, LONG_BREAK_LIMIT);
    assert_eq!(test_model.run_state, RunState::Idle);
    assert_eq!(
        test_model.remaining,
        Phase::LongBreak.map_phase_to_default_timer()
    );
}

#[test]
fn test_focus_before_the_final_session_leads_to_short_break() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, LONG_BREAK_LIMIT - 2);

    test_model.update(Message::Tick(test_model.remaining));

    assert_eq!(test_model.phase, Phase::ShortBreak);
}

#[test]
fn test_long_break_completion_resets_the_cycle() {
    let mut test_model = model_with(Phase::LongBreak, RunState::Running, LONG_BREAK_LIMIT);

    test_model.update(Message::Tick(test_model.remaining));

    assert_eq!(test_model.phase, Phase::Focus);
    assert_eq!(test_model.sessions_completed, 0);
    assert_eq!(test_model.run_state, RunState::Idle);
}

#[test]
fn test_tick_larger_than_remaining_drops_the_overshoot() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, 0);

    test_model.update(Message::Tick(Duration::from_hours(1)));

    assert_eq!(test_model.phase, Phase::ShortBreak);
    assert_eq!(
        test_model.remaining,
        Phase::ShortBreak.map_phase_to_default_timer()
    );
}

#[test]
fn test_tick_after_transition_does_not_start_the_next_phase() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, 0);
    test_model.update(Message::Tick(test_model.remaining));

    test_model.update(Message::Tick(Duration::from_secs(30)));

    assert_eq!(
        test_model.remaining,
        Phase::ShortBreak.map_phase_to_default_timer()
    );
}

#[test]
fn test_paused_timer_does_not_complete_the_phase() {
    let mut test_model = model_with(Phase::Focus, RunState::Paused, 0);

    test_model.update(Message::Tick(Duration::from_hours(1)));

    assert_eq!(test_model.phase, Phase::Focus);
    assert_eq!(test_model.sessions_completed, 0);
    assert_eq!(
        test_model.remaining,
        Phase::Focus.map_phase_to_default_timer()
    );
}

// ---- skip ----

#[test]
fn test_skip_focus_moves_to_short_break_without_counting() {
    let mut test_model = model_with(Phase::Focus, RunState::Idle, 1);

    test_model.update(Message::Skip);

    assert_eq!(test_model.phase, Phase::ShortBreak);
    assert_eq!(test_model.sessions_completed, 1);
    assert_eq!(
        test_model.remaining,
        Phase::ShortBreak.map_phase_to_default_timer()
    );
}

#[test]
fn test_skip_final_focus_session_gives_short_break_not_long() {
    let mut test_model = model_with(Phase::Focus, RunState::Idle, LONG_BREAK_LIMIT - 1);

    test_model.update(Message::Skip);

    assert_eq!(test_model.phase, Phase::ShortBreak);
    assert_eq!(test_model.sessions_completed, LONG_BREAK_LIMIT - 1);
}

#[test]
fn test_skip_short_break_moves_to_focus() {
    let mut test_model = model_with(Phase::ShortBreak, RunState::Idle, 2);

    test_model.update(Message::Skip);

    assert_eq!(test_model.phase, Phase::Focus);
    assert_eq!(test_model.sessions_completed, 2);
    assert_eq!(
        test_model.remaining,
        Phase::Focus.map_phase_to_default_timer()
    );
}

#[test]
fn test_skip_long_break_resets_the_cycle() {
    let mut test_model = model_with(Phase::LongBreak, RunState::Idle, LONG_BREAK_LIMIT);

    test_model.update(Message::Skip);

    assert_eq!(test_model.phase, Phase::Focus);
    assert_eq!(test_model.sessions_completed, 0);
}

#[test]
fn test_skip_while_running_leaves_the_next_phase_idle() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, 0);

    test_model.update(Message::Skip);

    assert_eq!(test_model.run_state, RunState::Idle);
}

#[test]
fn test_skip_discards_progress_in_the_current_phase() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, 0);
    test_model.update(Message::Tick(Duration::from_mins(10)));

    test_model.update(Message::Skip);

    assert_eq!(
        test_model.remaining,
        Phase::ShortBreak.map_phase_to_default_timer()
    );
}

// ---- reset ----

#[test]
fn test_reset_restores_full_focus_time_and_goes_idle() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, 2);
    test_model.update(Message::Tick(Duration::from_mins(10)));

    test_model.update(Message::Reset);

    assert_eq!(
        test_model.remaining,
        Phase::Focus.map_phase_to_default_timer()
    );
    assert_eq!(test_model.run_state, RunState::Idle);
    assert_eq!(test_model.phase, Phase::Focus);
    assert_eq!(test_model.sessions_completed, 2);
}

#[test]
fn test_reset_in_short_break_uses_the_break_duration() {
    let mut test_model = model_with(Phase::ShortBreak, RunState::Running, 1);
    test_model.update(Message::Tick(Duration::from_secs(90)));

    test_model.update(Message::Reset);

    assert_eq!(test_model.phase, Phase::ShortBreak);
    assert_eq!(
        test_model.remaining,
        Phase::ShortBreak.map_phase_to_default_timer()
    );
}

#[test]
fn test_reset_in_long_break_uses_the_break_duration() {
    let mut test_model = model_with(Phase::LongBreak, RunState::Running, LONG_BREAK_LIMIT);
    test_model.update(Message::Tick(Duration::from_secs(90)));

    test_model.update(Message::Reset);

    assert_eq!(test_model.phase, Phase::LongBreak);
    assert_eq!(
        test_model.remaining,
        Phase::LongBreak.map_phase_to_default_timer()
    );
    assert_eq!(test_model.sessions_completed, LONG_BREAK_LIMIT);
}

#[test]
fn test_reset_while_paused_goes_idle() {
    let mut test_model = model_with(Phase::Focus, RunState::Paused, 0);

    test_model.update(Message::Reset);

    assert_eq!(test_model.run_state, RunState::Idle);
}

// ---- full cycle ----

#[test]
fn test_full_cycle_runs_focus_and_breaks_in_order() {
    let mut test_model = Model::default();
    let mut phases = Vec::new();

    for _ in 0..(LONG_BREAK_LIMIT * 2) {
        finish_phase(&mut test_model);
        phases.push(test_model.phase);
    }

    assert_eq!(
        phases,
        vec![
            Phase::ShortBreak,
            Phase::Focus,
            Phase::ShortBreak,
            Phase::Focus,
            Phase::ShortBreak,
            Phase::Focus,
            Phase::LongBreak,
            Phase::Focus,
        ]
    );
    assert_eq!(test_model.sessions_completed, 0);
    assert_eq!(test_model.run_state, RunState::Idle);
}

// ---- progress ----

#[test]
fn test_progress_is_zero_for_a_fresh_phase() {
    let test_model = Model::default();

    assert_eq!(test_model.progress(), 0.0);
}

#[test]
fn test_progress_is_half_when_half_the_phase_has_elapsed() {
    let mut test_model = model_with(Phase::Focus, RunState::Running, 0);

    test_model.update(Message::Tick(Phase::Focus.map_phase_to_default_timer() / 2));

    assert!((test_model.progress() - 0.5).abs() < 1e-9);
}

#[test]
fn test_progress_uses_the_length_of_the_current_phase() {
    let mut test_model = model_with(Phase::ShortBreak, RunState::Running, 1);

    test_model.update(Message::Tick(
        Phase::ShortBreak.map_phase_to_default_timer() / 2,
    ));

    assert!((test_model.progress() - 0.5).abs() < 1e-9);
}

#[test]
fn test_progress_is_one_when_nothing_remains() {
    let test_model = Model {
        remaining: Duration::ZERO,
        ..Model::default()
    };

    assert_eq!(test_model.progress(), 1.0);
}

#[test]
fn test_progress_stays_in_range_when_remaining_exceeds_the_phase_length() {
    let test_model = Model {
        remaining: Duration::from_hours(2),
        ..Model::default()
    };

    assert_eq!(test_model.progress(), 0.0);
}
