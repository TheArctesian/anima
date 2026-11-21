//! The frames between the daemon and the holder (THR-113; H8 to H10 of the third security read):
//! one line of JSON for each frame, a version, an ID for each command, and a limit of size.

use serde::{Deserialize, Serialize};

/// The version of the protocol. A holder and a daemon of different versions say so and do not
/// guess: the holder then takes no new start (THR-113).
pub const VERSION: u32 = 1;

/// The most bytes of one frame. A longer frame, or one that does not read, closes the
/// connection and harms no agent (H8). A line of a program is at most 4 MiB, and JSON writes a
/// control byte in six bytes, so one line in a frame fits with room.
pub const MAX_FRAME: usize = 32 * 1024 * 1024;

/// The ID of a thread, as the runner named it: a UUID in text form.
pub type ThreadKey = String;

/// A command of the daemon to the holder. Each has an ID, and the holder does it one time: a
/// second command with the same ID gets the first answer again (H10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub id: u64,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum Action {
    /// The first frame of a connection; the holder answers with [`Event::Hello`] and gives again
    /// each line that the daemon did not confirm.
    Hello { version: u32 },
    /// Start a program for a thread: an absolute path, no shell, this environment and folder.
    Start {
        thread: ThreadKey,
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        folder: String,
    },
    /// Write a line to the standard input of the program of the thread.
    Write { thread: ThreadKey, line: String },
    /// Close the standard input of the program.
    CloseInput { thread: ThreadKey },
    /// The daemon read each line of the thread up to and with `seq`; the holder drops them.
    Ack { thread: ThreadKey, seq: u64 },
    /// End each process below the program, not the program (the interrupt of Codex, THR-049).
    EndChildren { thread: ThreadKey },
    /// End the program and each process below it (THR-049).
    Stop { thread: ThreadKey },
    /// The daemon knows that the thread ended; the holder forgets it.
    Forget { thread: ThreadKey },
    /// End the holder; refused while a program of a thread runs.
    Quit,
}

/// A frame of the holder to the daemon.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Hello {
        version: u32,
        threads: Vec<Held>,
    },
    /// The answer to the command `id`.
    Done {
        id: u64,
        outcome: Outcome,
    },
    /// Whole lines of output, in order; `first` is the sequence number of the first line.
    Lines {
        thread: ThreadKey,
        first: u64,
        lines: Vec<Line>,
    },
    /// The holder stopped reading the output of the thread: it holds its limit of bytes that the
    /// daemon did not confirm, so the program waits (H10).
    Waiting {
        thread: ThreadKey,
    },
    Exited {
        thread: ThreadKey,
        exit: Exit,
    },
}

/// A thread that the holder holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub thread: ThreadKey,
    pub pid: u32,
    pub exit: Option<Exit>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    Ok,
    Started {
        pid: u32,
    },
    /// What a stop found: `all_ended` is false when a process of the tree was still there after
    /// the stop (a process that left the tree on purpose is not found at all, THR-049).
    Stopped {
        ended: u32,
        all_ended: bool,
    },
    Refused {
        why: Refusal,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// The protocol versions differ: no new start (THR-113).
    Version,
    UnknownThread,
    ThreadExists,
    /// The program path is not absolute, or a value holds a NUL byte.
    BadStart,
    /// The program could not be started; the reason is the kind of the system error.
    StartFailed,
    /// The input of the program is closed, or the program ended.
    InputClosed,
    /// A program of a thread still runs.
    Busy,
}

/// The stream a line comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    Out,
    Err,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub stream: Stream,
    pub text: String,
}

/// How a program ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exit {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

/// One frame as a line, with its line end.
#[must_use]
pub fn frame<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string(value).unwrap_or_default();
    text.push('\n');
    text
}

/// A frame from one line, if it is not too long and reads.
#[must_use]
pub fn parse<T: for<'de> Deserialize<'de>>(line: &[u8]) -> Option<T> {
    if line.len() > MAX_FRAME {
        return None;
    }
    serde_json::from_slice(line).ok()
}
