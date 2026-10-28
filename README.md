# pomo-rs

A terminal Pomodoro timer written in Rust. It uses [ratatui](https://ratatui.rs) for drawing and [crossterm](https://github.com/crossterm-rs/crossterm) for terminal input.

This is a learning project, built in small phases. The aim is a keyboard-driven timer that stays out of the way: start a focus session, take a break, repeat.

## Status

A working timer with the full feature set below.

- A countdown shown as `MM:SS`, drawn as large block digits when the terminal has room
- Focus, short break and long break (after every fourth focus session), each waiting for you to start it
- Start, pause, resume, skip and reset, with accurate timing based on elapsed real time
- A task label for what you are working on, edited in place
- Quit confirmation, asked only when a focus session would be lost
- Change the length of the current phase with `+` and `-`
- A terminal bell and a desktop notification when a phase ends (each can be switched off)
- Help and settings screens
- Stats: today, streak, all time and a bar per day for the last week
- Settings and finished focus sessions are saved between runs
- A one-line compact view for tiny terminals such as a tmux pane
- Phase colors (coral, green, blue); the display goes gray and yellow when paused

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
| `+` / `-` | Lengthen or shorten the current phase by a minute (only while ready; saved) |
| `t` | Edit the task. `enter` saves, `esc` cancels |
| `Tab` | Stats |
| `,` | Settings: `↑`/`↓` select, `←`/`→` change, `esc` saves and closes |
| `?` | Help |
| `q` | Quit. Asks first during a focus session; `y` confirms, `n` or `esc` cancels |
| `Ctrl-C` | Quit immediately |

While you type a task, every key is text, so `q`, `s` and `space` can be part of the name.

## Files

| File | Location | Contents |
|---|---|---|
| Config | `$XDG_CONFIG_HOME/pomo-rs/config.toml`, or `~/.config/pomo-rs/config.toml` | Phase lengths and alerts |
| History | `$XDG_DATA_HOME/pomo-rs/history.json`, or `~/.local/share/pomo-rs/history.json` | One entry per finished focus session |

The config is plain TOML and you can edit it by hand. Every key is optional:

```toml
focus_minutes = 25
short_break_minutes = 5
long_break_minutes = 10
bell = true
notifications = true
```

Lengths are limited to 1-99 minutes. If a file exists but can't be parsed, the app stops with a message naming it, so a typo is never overwritten. Stats count days in UTC.

## tmux

When the terminal is three rows tall or less, the app draws one line, for example `FOCUS 24:13 ▶ 2/4 · write docs`. For a small pane at the bottom:

```sh
tmux split-window -v -l 1 pomo-rs
```

Keys go to whichever pane has focus, so select the pane to control the timer.

Notifications use `osascript` on macOS and `notify-send` on Linux. If the tool is missing, nothing is shown and the timer carries on.

## How it is built

The code follows the Elm architecture:

- **Model** (`src/model.rs`): all state, the `Message` enum, and a single `update` function. It has no terminal or file dependencies, so it is covered by plain unit tests (`src/model/`). It never reads the clock: the app passes in time as ticks and today's date. When something outside the model must happen (bell, notification, save), `update` queues an `Effect` and the app performs it.
- **Input** (`src/input.rs`): a pure function from the current mode and a key to a message. The mode is matched first, so a key can only do what the current screen lists.
- **App** (`src/app.rs`): owns the model, runs the event loop and carries out effects. Save failures are shown after the terminal is restored instead of crashing a running timer.
- **Settings** and **Stats** (`src/settings.rs`, `src/stats.rs`): plain serializable data and the numbers shown on the stats screen.
- **Storage** (`src/storage.rs`) and **Alerts** (`src/alerts.rs`): the only code that touches files, the bell and notification tools.
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
