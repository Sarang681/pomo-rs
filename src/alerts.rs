//! Ways to get the user's attention when a phase ends. Failures are ignored: a missing
//! notification tool must never stop the timer.

use std::io::{self, Write};
use std::process::{Command, Stdio};

pub fn bell() {
    let mut stdout = io::stdout();
    let _ = stdout.write_all(b"\x07").and_then(|()| stdout.flush());
}

/// Shows a desktop notification using the platform's own tool, on a background thread so a
/// slow tool never freezes the screen.
pub fn notify(title: &str, body: &str) {
    let Some(mut command) = notification_command(title, body) else {
        return;
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    std::thread::spawn(move || {
        let _ = command.status();
    });
}

/// The title and body are passed as arguments, never spliced into a script, so any text is safe.
fn notification_command(title: &str, body: &str) -> Option<Command> {
    if cfg!(target_os = "macos") {
        let mut command = Command::new("osascript");
        command
            .args([
                "-e",
                "on run argv",
                "-e",
                "display notification (item 2 of argv) with title (item 1 of argv)",
                "-e",
                "end run",
                "--",
            ])
            .args([title, body]);
        Some(command)
    } else if cfg!(target_os = "linux") {
        let mut command = Command::new("notify-send");
        command.args(["--", title, body]);
        Some(command)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_is_passed_as_separate_arguments() {
        let Some(command) = notification_command("a \"title\"", "body; rm -rf /") else {
            return;
        };
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();

        assert!(args.contains(&"a \"title\"".to_string()));
        assert!(args.contains(&"body; rm -rf /".to_string()));
    }
}
