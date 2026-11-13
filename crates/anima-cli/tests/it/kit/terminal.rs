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

/// `script`'s own arguments that run `anima args` at a new terminal and give back its exit
/// code: the BSD form on macOS, the util-linux form elsewhere.
fn script_arguments(args: &[&str]) -> Vec<String> {
    let anima = env!("CARGO_BIN_EXE_anima");
    if cfg!(target_os = "macos") {
        let mut all = vec!["-q".to_owned(), "/dev/null".to_owned(), anima.to_owned()];
        all.extend(args.iter().map(|&arg| arg.to_owned()));
        return all;
    }
    let quoted: Vec<String> = std::iter::once(anima)
        .chain(args.iter().copied())
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
        let mut command = anima.command("/usr/bin/script");
        command
            .args(script_arguments(args))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn().expect("script starts");
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
        let keys = format!("{line}\r");
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
