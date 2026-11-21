//! One agent program in the holder: its start, the readers of its output and the wait for its
//! end (THR-040, THR-113).

use std::io;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustix::process::{Pid, Signal, kill_process_group};
use tokio::io::{AsyncBufRead, AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::process::{Child, ChildStdin};

use super::holder::{Shared, locked};
use super::outbox::{MAX_LINE, TOO_LONG};
use super::tree;
use super::wire::{Event, Exit, Line, Stream, ThreadKey};

/// What the daemon names for a start.
pub struct Start<'a> {
    pub program: &'a str,
    pub args: &'a [String],
    pub env: &'a [(String, String)],
    pub folder: &'a str,
}

impl Start<'_> {
    /// An absolute program path and no NUL byte in any value: a value with NUL cannot reach the
    /// program as it was given.
    #[must_use]
    pub fn fits(&self) -> bool {
        let clean = |text: &str| !text.contains('\0');
        Path::new(self.program).is_absolute()
            && Path::new(self.folder).is_absolute()
            && clean(self.program)
            && clean(self.folder)
            && self.args.iter().all(|arg| clean(arg))
            && self.env.iter().all(|(name, value)| {
                clean(name) && clean(value) && !name.contains('=') && !name.is_empty()
            })
    }

    /// Starts the program: by its path, with no shell, with only this environment, in its
    /// folder, in a process group of its own, with three descriptors (THR-040). The standard
    /// library opens each other descriptor of the holder with close-on-exec.
    ///
    /// # Errors
    /// The error of the system when the program cannot start.
    pub fn spawn(&self) -> io::Result<Child> {
        tokio::process::Command::new(self.program)
            .args(self.args)
            .env_clear()
            .envs(self.env.iter().map(|(name, value)| (name, value)))
            .current_dir(self.folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .kill_on_drop(false)
            .spawn()
    }
}

/// The next line of `reader` without its line end, at most [`MAX_LINE`] bytes in memory: a
/// longer line is read to its end and given as [`TOO_LONG`]. `None` at the end of the stream.
async fn next_line<R: AsyncBufRead + Unpin>(reader: &mut R) -> io::Result<Option<String>> {
    let mut line = Vec::new();
    let mut over = false;
    loop {
        let buffer = reader.fill_buf().await?;
        if buffer.is_empty() {
            return Ok((over || !line.is_empty()).then(|| text(&line, over)));
        }
        let (part, used, ends) = match buffer.iter().position(|byte| *byte == b'\n') {
            Some(at) => (&buffer[..at], at + 1, true),
            None => (buffer, buffer.len(), false),
        };
        if line.len() + part.len() > MAX_LINE {
            over = true;
            line.clear();
        } else if !over {
            line.extend_from_slice(part);
        }
        reader.consume(used);
        if ends {
            return Ok(Some(text(&line, over)));
        }
    }
}

fn text(line: &[u8], over: bool) -> String {
    if over {
        TOO_LONG.to_owned()
    } else {
        String::from_utf8_lossy(line)
            .trim_end_matches('\r')
            .to_owned()
    }
}

/// Reads the lines of one stream into the outbox of the thread and gives each to the daemon if
/// it is there. While the outbox is full it reads nothing, so the program waits (H10).
pub async fn read(
    shared: Arc<Mutex<Shared>>,
    thread: ThreadKey,
    stream: Stream,
    pipe: impl tokio::io::AsyncRead + Unpin,
) {
    let mut reader = BufReader::new(pipe);
    loop {
        while waits(&shared, &thread) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let Ok(Some(text)) = next_line(&mut reader).await else {
            break;
        };
        let Ok(mut state) = shared.lock() else { break };
        let Some(slot) = state.threads.get_mut(&thread) else {
            break;
        };
        let line = Line { stream, text };
        let first = slot.outbox.push(line.stream, line.text.clone());
        state.send(&Event::Lines {
            thread: thread.clone(),
            first,
            lines: vec![line],
        });
    }
}

/// Whether the outbox of the thread is full; the first time, the daemon is told.
fn waits(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey) -> bool {
    let Ok(mut state) = shared.lock() else {
        return false;
    };
    let Some(slot) = state.threads.get_mut(thread) else {
        return false;
    };
    if !slot.outbox.full() {
        slot.waiting = false;
        return false;
    }
    if !slot.waiting {
        slot.waiting = true;
        state.send(&Event::Waiting {
            thread: thread.clone(),
        });
    }
    true
}

/// How often the holder looks at the processes below a running program.
const LOOK: std::time::Duration = std::time::Duration::from_secs(2);

/// Waits for the end of the program and looks at the processes below it while it runs; then
/// ends what is left in its process group and what it left in other groups, waits a short time
/// for the last lines, and tells the daemon.
pub async fn wait(
    shared: Arc<Mutex<Shared>>,
    thread: ThreadKey,
    mut child: Child,
    readers: [tokio::task::JoinHandle<()>; 2],
) {
    let pid = child.id();
    let status = loop {
        tokio::select! {
            status = child.wait() => break status,
            () = tokio::time::sleep(LOOK) => look_below(&shared, &thread, pid).await,
        }
    };
    let seen = {
        let mut state = locked(&shared);
        state
            .threads
            .get_mut(&thread)
            .map_or_else(tree::Known::new, |slot| {
                slot.reaped = true;
                std::mem::take(&mut slot.seen)
            })
    };
    // The group keeps the ID of the program while one of its members lives, so this signal
    // reaches only what the program left in its group; an empty group gives an error.
    if let Some(group) = pid
        .and_then(|pid| i32::try_from(pid).ok())
        .and_then(Pid::from_raw)
    {
        let _ = kill_process_group(group, Signal::KILL);
    }
    let swept = tree::sweep(&seen).await;
    if let Some(slot) = locked(&shared).threads.get_mut(&thread) {
        slot.left_behind = !swept.all;
    }
    finish(&shared, &thread, readers).await;
    let exit = status.map_or(
        Exit {
            code: None,
            signal: None,
        },
        |status| {
            use std::os::unix::process::ExitStatusExt as _;
            Exit {
                code: status.code(),
                signal: status.signal(),
            }
        },
    );
    let mut state = locked(&shared);
    if let Some(slot) = state.threads.get_mut(&thread) {
        slot.exit = Some(exit);
        slot.input = None;
    }
    state.send(&Event::Exited { thread, exit });
    state.write_record();
}

/// Waits for the readers of an ended program. A reader that waits for the daemon (a full
/// outbox) is never cut, so no line is lost (H10). A reader that still reads two seconds after
/// the end reads a pipe that a process which left the tree holds open: it is cut.
async fn finish(
    shared: &Arc<Mutex<Shared>>,
    thread: &ThreadKey,
    readers: [tokio::task::JoinHandle<()>; 2],
) {
    let mut quiet = Duration::ZERO;
    let step = Duration::from_millis(50);
    while readers.iter().any(|reader| !reader.is_finished()) {
        let waiting = shared
            .lock()
            .is_ok_and(|state| state.threads.get(thread).is_some_and(|slot| slot.waiting));
        if !waiting {
            quiet += step;
        }
        if quiet >= Duration::from_secs(2) {
            readers.iter().for_each(tokio::task::JoinHandle::abort);
            break;
        }
        tokio::time::sleep(step).await;
    }
}

/// The most lines that wait in front of the input of one program.
pub const INPUT_LINES: usize = 16;

/// The input of a program: a queue in front of a task that writes each line to the pipe, so that
/// no command of the holder waits for a program that does not read (THR-113). `None` when the
/// input is closed.
pub type Input = Option<tokio::sync::mpsc::Sender<String>>;

/// Starts the writer of the input of a program and gives its queue. The writer ends when the
/// queue closes or the pipe fails.
#[must_use]
pub fn input(mut pipe: ChildStdin) -> tokio::sync::mpsc::Sender<String> {
    let (sender, mut lines) = tokio::sync::mpsc::channel::<String>(INPUT_LINES);
    tokio::spawn(async move {
        while let Some(line) = lines.recv().await {
            if pipe.write_all(line.as_bytes()).await.is_err() {
                break;
            }
        }
    });
    sender
}

/// Keeps the processes below the program now, with those seen before that still run, so that
/// the set holds no process that ended (H9).
async fn look_below(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey, pid: Option<u32>) {
    let (Some(pid), Some(table)) = (pid, tree::table().await) else {
        return;
    };
    let now = table.below(pid);
    if let Some(slot) = locked(shared).threads.get_mut(thread) {
        let still = table.still(&slot.seen);
        slot.seen.retain(|pid, _| still.contains(pid));
        slot.seen.extend(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn lines(bytes: &[u8]) -> Vec<String> {
        let mut reader = BufReader::with_capacity(16, bytes);
        let mut found = Vec::new();
        while let Some(line) = next_line(&mut reader).await.expect("reads") {
            found.push(line);
        }
        found
    }

    #[tokio::test]
    async fn thr_113_whole_lines_come_out_also_when_they_cross_the_buffer() {
        let found = lines(b"first line that is long\r\nsecond\nlast with no end").await;
        assert_eq!(
            found,
            ["first line that is long", "second", "last with no end"]
        );
    }

    #[tokio::test]
    async fn thr_046_a_line_over_the_limit_is_read_to_its_end_and_marked() {
        let mut bytes = vec![b'x'; MAX_LINE + 10];
        bytes.extend_from_slice(b"\nnext\n");
        assert_eq!(lines(&bytes).await, [TOO_LONG, "next"]);
    }

    #[test]
    fn thr_040_a_start_needs_absolute_paths_and_no_nul_byte() {
        let args = vec!["-p".to_owned()];
        let env = vec![("HOME".to_owned(), "/home/a".to_owned())];
        let good = Start {
            program: "/bin/cat",
            args: &args,
            env: &env,
            folder: "/tmp",
        };
        assert!(good.fits());
        assert!(
            !Start {
                program: "cat",
                ..good
            }
            .fits()
        );
        assert!(
            !Start {
                folder: "work",
                ..good
            }
            .fits()
        );
        let bad_env = vec![("A=B".to_owned(), "x".to_owned())];
        assert!(
            !Start {
                env: &bad_env,
                ..good
            }
            .fits()
        );
        let nul = vec!["a\0b".to_owned()];
        assert!(!Start { args: &nul, ..good }.fits());
    }
}
