//! Recording tests: the pure queue, the recording root's checks, the row, and the recorder
//! writing real files in a temporary folder (RA-040, D-13, D-14, K4 §11).

mod idle;
mod queue;
mod recorder;
mod space;
mod stops;
mod store;

use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use sqlx::PgPool;
use tempfile::TempDir;
use uuid::Uuid;

use super::{
    Origin, Recorder, Stored,
    format::{LENGTH_LEN, Reader, RecordingKey, Step},
    recorder::begin,
    space::{FreeSpace, MIN_FREE},
};
use crate::{
    clock::FixedClock,
    events::Bus,
    ids::{RecordingId, SessionId},
    sessions::{
        Live, Protocol, Recording, Serving,
        recording::recorder::key_context,
        store::{self as sessions_store, NewSession},
        tests::fixture::{add_operator, docker_host, now, operator, vault},
    },
};

/// Bytes a sealed part adds to its plaintext: its length and its tag.
const FRAMING: u64 = LENGTH_LEN as u64 + 16;

/// How long a test waits for the writer task: generous, as the machine running the gates is
/// often loaded.
const PATIENCE: Duration = Duration::from_secs(15);

/// Waits in real time until `done` holds, for at most [`PATIENCE`]; panics with `what`.
async fn eventually(what: &str, mut done: impl AsyncFnMut() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !done().await {
        assert!(Instant::now() < deadline, "never happened: {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A free-space reading far above 1 GiB.
const PLENTY: FreeSpace = |_| Ok(u64::MAX);

/// A free-space reading of exactly 1 GiB: still enough.
const EXACT: FreeSpace = |_| Ok(MIN_FREE);

/// A free-space reading one byte under 1 GiB.
const LOW: FreeSpace = |_| Ok(MIN_FREE - 1);

/// A free-space reading that fails, as a file system that stopped answering.
const FAILING: FreeSpace = |_| Err(io::Error::other("the disk stopped answering"));

/// A recordable session of `docker-host` over VNC, and a hub that records into a fresh folder.
pub(super) struct World {
    pub(super) serving: Serving,
    pub(super) origin: Origin,
    pub(super) root: TempDir,
}

impl World {
    /// The hub, the operator and a live session row; nothing recorded yet.
    pub(super) async fn new(pool: &PgPool, max_bytes: u64) -> Self {
        add_operator(pool).await;
        let session = new_session(pool).await;
        let root = tempfile::tempdir().unwrap();
        let serving = Serving {
            db: pool.clone(),
            vault: vault(),
            bus: Bus::new(64),
            clock: Arc::new(FixedClock(now())),
            live: Live::default(),
            guacd: String::new(),
            drive_root: String::new(),
            recording: Recording {
                root: Some(root.path().to_owned()),
                max_bytes,
            },
        };
        let origin = Origin {
            session,
            device: docker_host(),
            device_name: "docker-host".to_owned(),
            protocol: Protocol::Vnc,
        };
        Self {
            serving,
            origin,
            root,
        }
    }

    /// Starts recording the session with the free space read by `free`.
    pub(super) async fn start(&self, free: FreeSpace) -> Recorder {
        begin(&self.serving, self.origin.clone(), free)
            .await
            .unwrap()
    }

    pub(super) fn path(&self, id: RecordingId) -> PathBuf {
        self.root.path().join(format!("{id}.anrec"))
    }

    pub(super) async fn stored(&self, id: RecordingId) -> Stored {
        super::get(&self.serving.db, id).await.unwrap().unwrap()
    }

    /// The recording's plaintext, read with the key unsealed from its row, and how it ended.
    pub(super) async fn read_back(&self, id: RecordingId) -> (Vec<u8>, Ended) {
        let stored = self.stored(id).await;
        let secret = self
            .serving
            .vault
            .open(&stored.key_sealed, key_context(id).as_bytes())
            .unwrap();
        let key = RecordingKey::from_bytes(secret.expose().try_into().unwrap());
        read_file(&self.path(id), key, id)
    }
}

/// Inserts another live VNC session of `docker-host` by the operator.
pub(super) async fn new_session(pool: &PgPool) -> SessionId {
    let session = SessionId::from(Uuid::now_v7());
    let operator = operator();
    let new = NewSession {
        id: session,
        device: docker_host(),
        device_name: "docker-host",
        protocol: Protocol::Vnc,
        operator: &operator,
        client: "127.0.0.1".parse().unwrap(),
        started_at: now(),
    };
    sessions_store::insert(pool, &new).await.unwrap();
    session
}

/// How a read recording ended.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Ended {
    Done,
    Incomplete,
}

fn read_file(path: &Path, key: RecordingKey, id: RecordingId) -> (Vec<u8>, Ended) {
    let file = std::fs::File::open(path).unwrap();
    let mut reader = Reader::open(file, key, id).unwrap();
    let mut plaintext = Vec::new();
    loop {
        match reader.next_part().unwrap() {
            Step::Part(part) => plaintext.extend_from_slice(&part),
            Step::Done => return (plaintext, Ended::Done),
            Step::Incomplete => return (plaintext, Ended::Incomplete),
        }
    }
}

/// The length of the file at `path`.
pub(super) fn length(path: &Path) -> u64 {
    std::fs::metadata(path).unwrap().len()
}
