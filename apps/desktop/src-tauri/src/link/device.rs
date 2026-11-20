//! The `anima-link device` child that runs while the app runs (2.6): started with the saved
//! port and `-stop-with-stdin`, so it ends with the app even when the app is killed, and started
//! again a pause after it ends. A port that another program holds is reported, never changed
//! (F-2). The child's log is read for its state only; no line of it is logged (LOG-007).

use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Read},
    net::{Ipv4Addr, TcpListener},
    path::PathBuf,
    process::{Child, ChildStderr, Command, Stdio},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use super::{
    printed::{self, DeviceLine},
    state::{DeviceState, Event},
};

const POLL: Duration = Duration::from_millis(50);
/// How often a link that a unit keeps is asked for its port.
pub const WATCH_EVERY: Duration = Duration::from_secs(2);
/// How long a child that was asked to stop (its standard input closed) has before it is killed.
pub const STOP_PATIENCE: Duration = Duration::from_secs(3);
/// The longest log line that is read whole; a longer one is cut.
const MOST_LINE: u64 = 4096;

/// How to run the device link.
#[derive(Debug, Clone)]
pub struct Launch {
    pub program: PathBuf,
    pub folder: PathBuf,
    pub port: u16,
    pub env: Vec<(&'static str, OsString)>,
    /// The pause between an end and the next start.
    pub restart_after: Duration,
}

/// The device link child of this app and its state.
#[derive(Debug, Default)]
pub struct DeviceLink {
    state: Arc<Mutex<DeviceState>>,
    running: Mutex<Option<Running>>,
    /// Set once the hub of this run is a hosted one: a unit of the user then keeps the link
    /// running with the app closed (HMD-058).
    background: AtomicBool,
}

#[derive(Debug)]
struct Running {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl DeviceLink {
    /// The state now.
    pub fn state(&self) -> DeviceState {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// From now, a link that this device joins is handed to a unit of the user.
    pub fn keep_in_background(&self) {
        self.background.store(true, Ordering::Release);
    }

    /// Whether a unit of the user must keep the link of this device.
    pub fn in_background(&self) -> bool {
        self.background.load(Ordering::Acquire)
    }

    /// Records a state that no child gives, such as a link that cannot start at all.
    pub fn record(&self, event: Event) {
        record(&self.state, event);
    }

    /// Runs the device link as `launch` says, after the one before has stopped.
    pub fn start(&self, launch: Launch) {
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(before) = running.take() {
            before.end();
        }
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::clone(&self.state);
        let stop_seen = Arc::clone(&stop);
        let thread = thread::spawn(move || supervise(&launch, &stop_seen, &state));
        *running = Some(Running { stop, thread });
    }

    /// Watches a device link that a unit of the user keeps running (HMD-058): `serving` asks
    /// the link for the port it listens on. No child is started, and [`Self::stop`] ends only
    /// the watch: the link runs on with the app closed.
    pub fn watch(&self, port: u16, serving: impl Fn() -> Option<u16> + Send + 'static) {
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(before) = running.take() {
            before.end();
        }
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::clone(&self.state);
        let stop_seen = Arc::clone(&stop);
        let thread = thread::spawn(move || observe(port, &serving, &stop_seen, &state));
        *running = Some(Running { stop, thread });
    }

    /// Stops the child and waits until it has ended. Blocks: call it off the async runtime.
    pub fn stop(&self) {
        let before = self
            .running
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(before) = before {
            before.end();
        }
        record(&self.state, Event::Stopped);
    }
}

impl Running {
    fn end(self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.thread.join();
    }
}

fn record(state: &Mutex<DeviceState>, event: Event) {
    let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
    *state = state.after(event);
}

/// Starts the child, waits for its end, pauses and starts it again, until `stop`.
fn supervise(launch: &Launch, stop: &AtomicBool, state: &Arc<Mutex<DeviceState>>) {
    while !stop.load(Ordering::Acquire) {
        record(state, Event::Starting { port: launch.port });
        run_once(launch, stop, state);
        let resume = Instant::now() + launch.restart_after;
        while !stop.load(Ordering::Acquire) && Instant::now() < resume {
            thread::sleep(POLL);
        }
    }
    record(state, Event::Stopped);
}

/// Asks the kept link for its port until `stop`, and records when it comes up and goes down.
fn observe(
    port: u16,
    serving: &dyn Fn() -> Option<u16>,
    stop: &AtomicBool,
    state: &Mutex<DeviceState>,
) {
    record(state, Event::Starting { port });
    let mut up = false;
    while !stop.load(Ordering::Acquire) {
        match serving() {
            Some(reported) if !up => {
                record(state, Event::Starting { port });
                record(state, Event::Ready { port: reported });
                up = true;
            }
            None if up => {
                record(state, Event::Ended);
                up = false;
            }
            _ => {}
        }
        let next = Instant::now() + WATCH_EVERY;
        while !stop.load(Ordering::Acquire) && Instant::now() < next {
            thread::sleep(POLL);
        }
    }
    record(state, Event::Stopped);
}

/// One run of the child, from its start to its end.
fn run_once(launch: &Launch, stop: &AtomicBool, state: &Arc<Mutex<DeviceState>>) {
    // A port another program holds is seen before the child would fail on it.
    if TcpListener::bind((Ipv4Addr::LOCALHOST, launch.port)).is_err() {
        record(state, Event::PortTaken);
        return;
    }
    let Ok(mut child) = start(launch) else {
        record(state, Event::Ended);
        return;
    };
    let reader = child.stderr.take().map(|stderr| {
        let state = Arc::clone(state);
        thread::spawn(move || read_log(stderr, &state))
    });
    wait(&mut child, stop);
    if let Some(reader) = reader {
        let _ = reader.join();
    }
    record(state, Event::Ended);
}

fn start(launch: &Launch) -> std::io::Result<Child> {
    let listen = format!("127.0.0.1:{}", launch.port);
    Command::new(&launch.program)
        .arg("device")
        .arg("-state")
        .arg(&launch.folder)
        .args(["-listen", &listen, "-stop-with-stdin"])
        .env_clear()
        .envs(launch.env.iter().map(|(name, value)| (name, value)))
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
}

/// Waits for the child's end. On `stop` its standard input is closed, which ends it; one that
/// does not end in time is killed.
fn wait(child: &mut Child, stop: &AtomicBool) {
    let mut stopping: Option<Instant> = None;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(_) => break,
        }
        match stopping {
            None if stop.load(Ordering::Acquire) => {
                drop(child.stdin.take());
                stopping = Some(Instant::now());
            }
            Some(since) if since.elapsed() > STOP_PATIENCE => break,
            _ => {}
        }
        thread::sleep(POLL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Reads the child's log until it ends, and records what a line tells.
fn read_log(stderr: ChildStderr, state: &Mutex<DeviceState>) {
    let mut log = BufReader::new(stderr);
    let mut line = Vec::new();
    loop {
        line.clear();
        match (&mut log).take(MOST_LINE).read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let event = match printed::device_line(&String::from_utf8_lossy(&line)) {
            Some(DeviceLine::Ready(port)) => Event::Ready { port },
            Some(DeviceLine::PortTaken) => Event::PortTaken,
            None => continue,
        };
        record(state, event);
    }
}
