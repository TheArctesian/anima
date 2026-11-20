//! Native `ConPTY` fixtures: the command sees real Windows console handles.
use super::Anima;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read as _, Write},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;
const DEADLINE: Duration = Duration::from_secs(30);

pub struct Terminal {
    child: Box<dyn Child + Send + Sync>,
    master: Option<Box<dyn MasterPty + Send>>,
    keys: Option<Box<dyn Write + Send>>,
    screen: Arc<Mutex<Vec<u8>>>,
    changed: Arc<Notify>,
    seen: usize,
    answered_queries: usize,
    finished: Option<tokio::sync::oneshot::Receiver<()>>,
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
impl Terminal {
    pub fn start(anima: &Anima, args: &[&str]) -> Self {
        Self::running(anima, env!("CARGO_BIN_EXE_anima"), args)
    }
    pub fn start_showing_modes(anima: &Anima, args: &[&str]) -> Self {
        let path = anima.home.path().join("modes.ps1");
        std::fs::write(&path, include_str!("modes.ps1")).unwrap();
        let mut all = vec![
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            path.to_str().unwrap(),
            env!("CARGO_BIN_EXE_anima"),
        ];
        all.extend_from_slice(args);
        let root = std::env::var("SystemRoot").unwrap();
        Self::running(
            anima,
            &format!("{root}/System32/WindowsPowerShell/v1.0/powershell.exe"),
            &all,
        )
    }
    fn running(anima: &Anima, program: &str, args: &[&str]) -> Self {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 60,
                cols: 240,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let env = anima.command(program);
        let mut command = CommandBuilder::new(program);
        command.env_clear();
        for (name, value) in env.as_std().get_envs() {
            if let Some(value) = value {
                command.env(name, value);
            }
        }
        command.args(args);
        let child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        let keys = pair.master.take_writer().unwrap();
        let mut output = pair.master.try_clone_reader().unwrap();
        let screen = Arc::new(Mutex::new(Vec::new()));
        let changed = Arc::new(Notify::new());
        let (writer, notify) = (screen.clone(), changed.clone());
        let (done, finished) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || {
            let mut buffer = [0; 4096];
            while let Ok(read @ 1..) = output.read(&mut buffer) {
                writer.lock().unwrap().extend_from_slice(&buffer[..read]);
                notify.notify_waiters();
            }
            notify.notify_waiters();
            let _ = done.send(());
        });
        Self {
            child,
            master: Some(pair.master),
            keys: Some(keys),
            screen,
            changed,
            seen: 0,
            answered_queries: 0,
            finished: Some(finished),
        }
    }
    pub fn screen(&self) -> String {
        String::from_utf8_lossy(&self.screen.lock().unwrap()).into_owned()
    }
    fn answer_cursor_queries(&mut self, screen: &str) {
        // ConPTY inherits the cursor by asking its terminal for a position before launching
        // the child. This fixture is that terminal; leaving the query unanswered blocks it.
        let queries = screen.matches("\x1b[6n").count();
        let writer = self.keys.as_mut().unwrap();
        for _ in self.answered_queries..queries {
            writer.write_all(b"\x1b[1;1R").unwrap();
            writer.flush().unwrap();
        }
        self.answered_queries = queries;
    }
    pub async fn expect(&mut self, text: &str) {
        let changed = Arc::clone(&self.changed);
        self.seen = tokio::time::timeout(DEADLINE, async {
            loop {
                let change = changed.notified();
                tokio::pin!(change);
                change.as_mut().enable();
                let screen = self.screen();
                self.answer_cursor_queries(&screen);
                if let Some(at) = screen[self.seen..].find(text) {
                    return self.seen + at + text.len();
                }
                change.await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{text:?} never appeared: {:?}", self.screen()));
    }
    pub async fn type_line(&mut self, line: &str) {
        self.press(&format!("{line}\r")).await;
    }
    #[expect(
        clippy::unused_async,
        reason = "the shared terminal fixture API is async; ConPTY accepts these bounded test keystrokes synchronously"
    )]
    pub async fn press(&mut self, keys: &str) {
        let writer = self.keys.as_mut().unwrap();
        writer.write_all(keys.as_bytes()).unwrap();
        writer.flush().unwrap();
    }
    pub async fn finish(mut self) -> (i32, String) {
        let status = tokio::time::timeout(DEADLINE, async {
            loop {
                self.answer_cursor_queries(&self.screen());
                if let Some(status) = self.child.try_wait().unwrap() {
                    return status;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("anima did not end: {:?}", self.screen()));
        self.keys.take();
        self.master.take();
        tokio::time::timeout(DEADLINE, self.finished.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        (
            i32::from_ne_bytes(status.exit_code().to_ne_bytes()),
            self.screen(),
        )
    }
}

pub fn assert_modes_restored(screen: &str) {
    let mode = |label| {
        screen
            .split_once(label)
            .unwrap_or_else(|| panic!("no {label}: {screen:?}"))
            .1
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap()
    };
    let before = mode("modes-before ");
    assert_eq!(mode("modes-after "), before, "{screen:?}");
    assert_eq!(
        before & 7,
        7,
        "echo, line processing and signal keys must start enabled"
    );
}
