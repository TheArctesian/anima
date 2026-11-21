//! The runner of `anima-daemon threads` with a made `claude` on the `PATH` of a made login
//! shell: a start, a request and its answer, a restart of the runner while the program stays in
//! the holder, and a stop that ends a child in a group of its own (TH11, TH12, THR-030,
//! THR-049, THR-113).

use std::path::{Path, PathBuf};
use std::process::{Child, Command as Process};
use std::time::Duration;

use agent_threads::control::{Action, Call, Change, Outcome, Reply, RunState, SOCKET};
use agent_threads::drive::{Answer, Mode, Program};
use anima_daemon::threads::link::HolderLink;
use anima_daemon::threads::wire;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use uuid::Uuid;

use super::gone;

/// A made Claude Code: it names its session, asks to run a command, and after the answer starts
/// a child in a process group of its own, writes the child's ID to `child.pid`, and ends its turn.
const CLAUDE: &str = r#"#!/usr/bin/perl
$| = 1;
my $sid = "none";
for my $i (0 .. $#ARGV) { $sid = $ARGV[$i + 1] if $ARGV[$i] eq "--session-id"; }
print qq({"type":"system","subtype":"init","session_id":"$sid","model":"made","permissionMode":"bypassPermissions"}\n);
while (my $line = <STDIN>) {
    if ($line =~ /"type":"user"/) {
        print qq({"type":"system","subtype":"session_state_changed","state":"running"}\n);
        print qq({"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"sleep 300"},"permission_suggestions":[],"tool_use_id":"t1"}}\n);
    } elsif ($line =~ /"control_response"/) {
        my $pid = fork();
        if ($pid == 0) { setpgrp(0, 0); exec("/bin/sleep", "300"); }
        open(my $file, ">", "child.pid"); print $file $pid; close($file);
        print qq({"type":"result","subtype":"success","is_error":false,"result":"done","terminal_reason":"completed"}\n);
        print qq({"type":"system","subtype":"session_state_changed","state":"idle"}\n);
    }
}
"#;

/// A made login shell: it runs the script it is given with the environment it got.
const SHELL: &str = "#!/bin/sh\nexec /bin/sh -c \"$4\"\n";

struct Kit {
    home: tempfile::TempDir,
    runner: Option<Child>,
}

impl Kit {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt as _;
        let home = tempfile::tempdir().expect("a folder");
        let bin = home.path().join("bin");
        std::fs::create_dir(&bin).expect("bin");
        for (name, text) in [("claude", CLAUDE), ("login-shell", SHELL)] {
            let path = bin.join(name);
            std::fs::write(&path, text).expect("written");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
        }
        std::fs::create_dir(home.path().join("work")).expect("work");
        let mut kit = Self { home, runner: None };
        kit.start();
        kit
    }

    fn path(&self, name: &str) -> PathBuf {
        self.home.path().join(name)
    }

    fn threads(&self) -> PathBuf {
        self.path("daemon").join("threads")
    }

    fn start(&mut self) {
        let bin = self.path("bin");
        let child = Process::new(env!("CARGO_BIN_EXE_anima-daemon"))
            .arg("threads")
            .env_clear()
            .env("HOME", self.home.path())
            .env("ANIMA_DAEMON_HOME", self.path("daemon"))
            .env("SHELL", bin.join("login-shell"))
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .spawn()
            .expect("the runner starts");
        self.runner = Some(child);
    }

    fn end_runner(&mut self) {
        if let Some(mut child) = self.runner.take() {
            let pid = rustix::process::Pid::from_raw(i32::try_from(child.id()).expect("a pid"));
            let _ =
                rustix::process::kill_process(pid.expect("a pid"), rustix::process::Signal::TERM);
            let _ = child.wait();
        }
    }

    async fn client(&self) -> Client {
        let socket = self.threads().join(SOCKET);
        for _ in 0..200 {
            if let Ok(stream) = UnixStream::connect(&socket).await {
                let (read, write) = stream.into_split();
                return Client {
                    lines: BufReader::new(read).lines(),
                    write,
                    next: 0,
                };
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the runner did not listen");
    }
}

impl Drop for Kit {
    fn drop(&mut self) {
        self.end_runner();
        let dir = self.threads();
        // The holder ends with the test: a thread of its own, as a drop runs inside the runtime.
        let _ = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime");
            runtime.block_on(async move {
                if let Ok(mut link) = HolderLink::connect(&dir).await {
                    let _ = link.send(wire::Action::Quit).await;
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            });
        })
        .join();
    }
}

struct Client {
    lines: Lines<BufReader<OwnedReadHalf>>,
    write: OwnedWriteHalf,
    next: u64,
}

impl Client {
    async fn call(&mut self, action: Action) -> u64 {
        self.next += 1;
        let mut text = serde_json::to_string(&Call {
            id: self.next,
            action,
        })
        .expect("a call");
        text.push('\n');
        self.write.write_all(text.as_bytes()).await.expect("sent");
        self.next
    }

    async fn until(&mut self, mut wanted: impl FnMut(&Reply) -> bool) -> Reply {
        let wait = async {
            loop {
                let line = self.lines.next_line().await.expect("reads").expect("open");
                let reply: Reply = serde_json::from_str(&line).expect("a reply");
                if wanted(&reply) {
                    return reply;
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(20), wait)
            .await
            .expect("in time")
    }

    async fn done(&mut self, id: u64) -> Outcome {
        match self
            .until(|reply| matches!(reply, Reply::Done { id: done, .. } if *done == id))
            .await
        {
            Reply::Done { result, .. } => result,
            Reply::Update(_) => unreachable!("only a done is wanted"),
        }
    }
}

fn change(reply: &Reply) -> Option<&Change> {
    match reply {
        Reply::Update(update) => Some(&update.change),
        Reply::Done { .. } => None,
    }
}

fn state_is(state: RunState) -> impl FnMut(&Reply) -> bool {
    move |reply| matches!(change(reply), Some(Change::State { state: now, .. }) if *now == state)
}

async fn child_pid(folder: &Path) -> u32 {
    for _ in 0..100 {
        if let Some(pid) = std::fs::read_to_string(folder.join("child.pid"))
            .ok()
            .and_then(|text| text.trim().parse().ok())
        {
            return pid;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the made program wrote no child");
}

/// Starts a thread, answers its request, and waits for its turn to end: the thread, the request
/// and the child that the made program left.
async fn start_and_answer(kit: &Kit) -> (Uuid, Uuid, u32) {
    let mut client = kit.client().await;
    let watch = client.call(Action::Watch).await;
    assert_eq!(client.done(watch).await, Outcome::Ok);
    let start = client
        .call(Action::Start {
            program: Program::ClaudeCode,
            folder: kit.path("work"),
            mode: Mode::FullAccess,
            model: None,
            effort: None,
            prompt: "Run the command".to_owned(),
        })
        .await;
    let Outcome::Started { thread } = client.done(start).await else {
        panic!("the thread started");
    };
    let asked = client
        .until(|reply| matches!(change(reply), Some(Change::Asked { .. })))
        .await;
    let Some(Change::Asked { request, detail }) = change(&asked) else {
        unreachable!()
    };
    assert_eq!(detail.subject, "sleep 300");
    let request: Uuid = *request;
    client.until(state_is(RunState::NeedsYou)).await;
    let answer = client
        .call(Action::Answer {
            thread,
            request,
            answer: Answer::Allow,
        })
        .await;
    assert_eq!(client.done(answer).await, Outcome::Ok);
    client.until(state_is(RunState::Idle)).await;
    let child = child_pid(&kit.path("work")).await;

    (thread, request, child)
}

#[tokio::test]
async fn th11_a_thread_starts_asks_is_answered_survives_a_restart_and_stops_with_its_children() {
    let mut kit = Kit::new();
    let (thread, request, child) = start_and_answer(&kit).await;

    kit.end_runner();
    kit.start();
    let mut client = kit.client().await;
    let list = client.call(Action::List).await;
    let Outcome::Threads { threads } = client.done(list).await else {
        panic!("threads")
    };
    assert_eq!(threads.len(), 1);
    assert_eq!(
        threads[0].state,
        RunState::Idle,
        "the program stayed in the holder"
    );
    assert_eq!(
        threads[0].session.as_deref(),
        Some(thread.to_string().as_str())
    );
    let again = client
        .call(Action::Answer {
            thread,
            request,
            answer: Answer::Allow,
        })
        .await;
    assert!(
        matches!(client.done(again).await, Outcome::Refused { .. }),
        "one answer for each request"
    );

    let stop = client.call(Action::Stop { thread }).await;
    assert_eq!(
        client.done(stop).await,
        Outcome::Stopped { all_ended: true }
    );
    assert!(
        gone(child).await,
        "the child in its own group ended with the program"
    );
    let log = std::fs::read_to_string(kit.threads().join("runner.log")).expect("a log");
    assert!(!log.contains("Run the command"), "the log holds no prompt");
    for kind in ["start", "answer", "stop", "refusal"] {
        assert!(
            log.contains(&format!(r#""kind":"{kind}""#)),
            "{kind} in {log}"
        );
    }
}
