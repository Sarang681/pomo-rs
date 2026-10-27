# pomo-rs

A terminal Pomodoro timer written in Rust. It uses [ratatui](https://ratatui.rs) for drawing and [crossterm](https://github.com/crossterm-rs/crossterm) for terminal input.

This is a learning project, built in small phases. The aim is a keyboard-driven timer that stays out of the way: start a focus session, take a break, repeat.

## Status

Work in progress. What works today:

- A countdown shown as `MM:SS`: 25 minutes for focus, 5 for a short break, 10 for a long break
- Start, pause and resume with one key
- Accurate timing based on elapsed real time, so pausing and slow redraws don't cause drift
- Automatic phase transitions: focus, short break, and a long break after every fourth focus session
- Skip the current phase, or reset it to its full length
- Each new phase waits for you to start it, so a break never begins while you're away
- A laid-out screen with the phase and session counter, cycle dots, the time, a progress bar, the run state, and key hints in a footer that change with the timer state
- Phase colors: coral for focus, green for a short break, blue for a long break
- Clear states: the time and bar go gray when paused, and the state line turns yellow and explains how to resume
- Large block digits for the timer when the terminal has room, with a plain `MM:SS` fallback in small terminals
- A progress bar that shrinks to fit narrow terminals and always keeps its percentage visible

Planned, roughly in this order:

- Colored session dots and footer keys
- Task label and quit confirmation
- Help and settings screens
- Daily stats and saved history
- Config file, desktop notifications, and a compact one-line mode for tmux

## Run it

You need a recent stable Rust toolchain (edition 2024). Install one with [rustup](https://rustup.rs).

```sh
git clone git@github.com:Sarang681/pomo-rs.git
cd pomo-rs
cargo run
```

## Keys

| Key | Action |
|---|---|
| `space` | Start, pause or resume |
| `s` | Skip to the next phase (a skipped focus session is not counted) |
| `r` | Reset the current phase to its full length |
| `q` | Quit |

More keys arrive with later phases (task, help, settings).

## How it is built

The code follows the Elm architecture:

- **Model** (`src/model.rs`): all state, the `Message` enum, and a single `update` function. It has no terminal dependencies, so it is covered by plain unit tests (`src/model/tests.rs`).
- **App** (`src/app.rs`): owns the model, runs the event loop and maps keys to messages.
- **UI** (`src/ui.rs`): draws the screen from the model, one small function per region. It only reads the model, and it is tested by rendering into an in-memory buffer, including the color of individual cells.
- **Digits** (`src/digits.rs`): turns a time string into five rows of large block digits. It has no terminal dependencies, so it is tested with plain strings.
- **Time**: each loop iteration measures real elapsed time with `Instant` and sends it to the model as a tick. The model only counts it while running.

## Development

```sh
cargo test      # run the unit tests
cargo clippy    # lint
cargo fmt       # format
```

## License

Not yet chosen.
