//! `anima` at a pseudo-terminal, as a person runs it: the system `script` program gives the
//! command a terminal (standard input, output and `/dev/tty` with echo), and the test answers
//! each question only once it is on screen. No sleep: every wait has a deadline.

use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::process::{Child, ChildStdin};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::kit::Anima;

/// How long a question or the end of the command may take to appear.
const DEADLINE: Duration = Duration::from_secs(30);

/// A shell script that prints the terminal's modes (`stty -a`, on one line) before and after it
/// runs its arguments, and exits with the command's code.
const SHOWING_MODES: &str = r#"modes() { printf '%s %s\n' "$1" "$(stty -a | tr '\n\t' '  ')"; }; modes modes-before; "$@"; code=$?; modes modes-after; exit $code"#;

/// `script`'s own arguments that run `command` at a new terminal and give back its exit code:
/// the BSD form on macOS, the util-linux form elsewhere.
fn script_arguments(command: &[&str]) -> Vec<String> {
    if cfg!(target_os = "macos") {
        let mut all = vec!["-q".to_owned(), "/dev/null".to_owned()];
        all.extend(command.iter().map(|&arg| (*arg).to_owned()));
        return all;
    }
    let quoted: Vec<String> = command
        .iter()
        .map(|arg| format!("'{}'", arg.replace('\'', r"'\''")))
        .collect();
    vec![
        "-q".to_owned(),
        "-e".to_owned(),
        "-c".to_owned(),
        quoted.join(" "),
        "/dev/null".to_owned(),
    ]
}

/// One run of `anima` at a terminal.
pub struct Terminal {
    child: Child,
    /// Kept open until the command ends: `script` ends the terminal at end of input.
    keys: ChildStdin,
    screen: Arc<Mutex<String>>,
    changed: Arc<Notify>,
    reader: JoinHandle<()>,
    /// How much of the screen the test has read past.
    seen: usize,
}

impl Terminal {
    pub fn start(anima: &Anima, args: &[&str]) -> Self {
        let mut command = vec![env!("CARGO_BIN_EXE_anima")];
        command.extend(args);
        Self::running(anima, &command)
    }

    /// As [`Terminal::start`], with the terminal's modes shown before and after `anima` runs;
    /// read them back with [`assert_modes_restored`].
    pub fn start_showing_modes(anima: &Anima, args: &[&str]) -> Self {
        let mut command = vec![
            "/bin/sh",
            "-c",
            SHOWING_MODES,
            "sh",
            env!("CARGO_BIN_EXE_anima"),
        ];
        command.extend(args);
        Self::running(anima, &command)
    }

    fn running(anima: &Anima, command: &[&str]) -> Self {
        let mut script = anima.command("/usr/bin/script");
        script
            .args(script_arguments(command))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = script.spawn().expect("script starts");
        let keys = child.stdin.take().expect("a stdin pipe");
        let mut output = child.stdout.take().expect("a stdout pipe");
        let (screen, changed) = (Arc::new(Mutex::new(String::new())), Arc::new(Notify::new()));
        let (writer, notify) = (Arc::clone(&screen), Arc::clone(&changed));
        let reader = tokio::spawn(async move {
            let mut buffer = [0_u8; 4096];
            while let Ok(read @ 1..) = output.read(&mut buffer).await {
                let text = String::from_utf8_lossy(&buffer[..read]);
                writer.lock().expect("the screen").push_str(&text);
                notify.notify_waiters();
            }
        });
        Self {
            child,
            keys,
            screen,
            changed,
            reader,
            seen: 0,
        }
    }

    /// Everything the terminal showed so far.
    pub fn screen(&self) -> String {
        self.screen.lock().expect("the screen").clone()
    }

    /// Waits until `text` appears after what was already read, and reads past it.
    pub async fn expect(&mut self, text: &str) {
        let waited = tokio::time::timeout(DEADLINE, async {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                let screen = self.screen();
                if let Some(at) = screen[self.seen..].find(text) {
                    return self.seen + at + text.len();
                }
                changed.await;
            }
        })
        .await;
        self.seen = waited.unwrap_or_else(|_elapsed| {
            panic!("{text:?} never appeared; the screen: {:?}", self.screen())
        });
    }

    /// Types `line` and Return.
    pub async fn type_line(&mut self, line: &str) {
        self.press(&format!("{line}\r")).await;
    }

    /// Sends `signal` (such as `INT`) to `anima` as another program would, not from a key. For
    /// a [`Terminal::start_showing_modes`] run, where `anima` is the child of the shell that
    /// `script` runs.
    pub async fn signal(&self, signal: &str) {
        let script = self.child.id().expect("script is running").to_string();
        let shell = tokio::process::Command::new("pgrep")
            .args(["-P", &script])
            .output()
            .await
            .expect("pgrep runs");
        let shell = String::from_utf8(shell.stdout).expect("a process ID");
        let sent = tokio::process::Command::new("pkill")
            .args([&format!("-{signal}"), "-P", shell.trim()])
            .status()
            .await
            .expect("pkill runs");
        assert!(sent.success(), "no anima under the shell {shell:?}");
    }

    /// Types `keys` as they are, without Return: control keys such as Ctrl-C (`\x03`).
    pub async fn press(&mut self, keys: &str) {
        self.keys
            .write_all(keys.as_bytes())
            .await
            .expect("keys typed");
        self.keys.flush().await.expect("keys sent");
    }

    /// Waits for the command to end; its exit code and the whole screen.
    pub async fn finish(mut self) -> (i32, String) {
        let status = tokio::time::timeout(DEADLINE, self.child.wait())
            .await
            .unwrap_or_else(|_| panic!("anima did not end; the screen: {:?}", self.screen()))
            .expect("an exit status");
        drop(self.keys);
        // The reader ends at end of output; the screen is complete once it has.
        tokio::time::timeout(DEADLINE, self.reader)
            .await
            .expect("the terminal closed")
            .expect("the reader ended");
        let screen = self.screen.lock().expect("the screen").clone();
        (status.code().expect("an exit code"), screen)
    }
}

/// Asserts that the terminal of a [`Terminal::start_showing_modes`] run had the same modes after
/// `anima` as before it, with echo, line editing and the signal keys on. `pendin` is left out:
/// the kernel sets it itself when line editing comes back on, until the next read.
pub fn assert_modes_restored(screen: &str) {
    let modes = |label: &str| -> Vec<String> {
        screen
            .lines()
            .find_map(|line| line.trim_end_matches('\r').strip_prefix(label))
            .unwrap_or_else(|| panic!("no {label:?}; the screen: {screen:?}"))
            .split_whitespace()
            .filter(|flag| !flag.ends_with("pendin"))
            .map(str::to_owned)
            .collect()
    };
    let after = modes("modes-after ");
    assert_eq!(modes("modes-before "), after, "{screen:?}");
    for flag in ["echo", "icanon", "isig"] {
        assert!(
            after.iter().any(|shown| shown == flag),
            "{flag} is off: {screen:?}"
        );
    }
}
