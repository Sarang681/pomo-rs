# pomo-rs

A terminal Pomodoro timer written in Rust. It uses [ratatui](https://ratatui.rs) for drawing and [crossterm](https://github.com/crossterm-rs/crossterm) for terminal input.

This is a learning project, built in small phases. The aim is a keyboard-driven timer that stays out of the way: start a focus session, take a break, repeat.

## Status

Work in progress. What works today:

- A 25-minute countdown shown as `MM:SS`
- Start, pause and resume with one key
- Accurate timing based on elapsed real time, so pausing and slow redraws don't cause drift
- Phase and run state shown on screen (focus, short break, long break; idle, running, paused)

Planned, roughly in this order:

- Automatic phase transitions (focus, short break, long break after four sessions), skip and reset
- Layout with a progress bar, large digits, phase colors and key hints
- Task label and quit confirmation
- Help and settings screens
- Daily stats and saved history
- Config file, desktop notifications, and a compact one-line mode for tmux

## Run it

You need a recent stable Rust toolchain (edition 2024). Install one with [rustup](https://rustup.rs).

```sh
git clone <repo-url>
cd pomo-rs
cargo run
```

## Keys

| Key | Action |
|---|---|
| `space` | Start, pause or resume |
| `q` | Quit |

More keys arrive with later phases (skip, reset, task, help, settings).

## How it is built

The code follows the Elm architecture:

- **Model** (`src/model.rs`): all state, the `Message` enum, and a single `update` function. It has no terminal dependencies, so it is covered by plain unit tests.
- **App** (`src/app.rs`): owns the model, runs the event loop, maps keys to messages and renders the screen.
- **Time**: each loop iteration measures real elapsed time with `Instant` and sends it to the model as a tick. The model only counts it while running.

## Development

```sh
cargo test      # run the unit tests
cargo clippy    # lint
cargo fmt       # format
```

## License

Not yet chosen.
