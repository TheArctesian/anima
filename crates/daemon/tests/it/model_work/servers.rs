use std::time::Duration;

use anima_daemon::model::supervise::ServerState;
use daemon_core::model_work::cause::Cause;
use daemon_core::model_work::names::Actor;
use daemon_core::model_work::order::{Order, Parameters};
use daemon_core::model_work::settings::ServerSettings;
use daemon_core::wire::WorkType;
use uuid::Uuid;

use super::kit::{FAKE_RUNTIME, PROGRAM, archive, entry, node_on, release};
use anima_daemon::model::cycle::Node;
use anima_daemon::model::install::place_archive;

const SERVER: Uuid = Uuid::from_u128(0x0192_f6aa_0000_7000_8000_0000_0000_0042);

fn order(work_type: WorkType, parameters: Parameters) -> Order {
    Order {
        id: Uuid::nil(),
        attempt: 1,
        work_type,
        actor: Actor::User,
        parameters,
    }
}

fn start() -> Order {
    order(
        WorkType::ServerStart,
        Parameters::Start {
            server: SERVER,
            release: release(),
            settings: ServerSettings::default(),
        },
    )
}

fn installed_node(root: &std::path::Path, mode: &str, first_port: u16) -> Node {
    let node = node_on(root, first_port..=first_port + 49);
    place_archive(
        archive(&[(PROGRAM, 0o755, Some(FAKE_RUNTIME.as_bytes()))]),
        &node.folders,
        &release(),
        &entry(),
    )
    .expect("installed");
    set_mode(&node, mode);
    node
}

fn set_mode(node: &Node, mode: &str) {
    std::fs::write(node.folders.release(&release()).join("bin/mode"), mode).expect("the mode");
}

#[tokio::test]
async fn a_server_listens_on_loopback_with_the_fixed_environment() {
    let root = tempfile::tempdir().expect("a folder");
    let mut node = installed_node(root.path(), "loopback", 42050);
    assert_eq!(node.run(&start()).await, Ok(None));
    assert_eq!(node.servers.map[&SERVER].state, ServerState::Ready);
    let env =
        std::fs::read_to_string(node.folders.release(&release()).join("bin/env")).expect("env");
    let mut names: Vec<&str> = env.lines().collect();
    names.retain(|name| !["PWD", "SHLVL", "_", "__CF_USER_TEXT_ENCODING"].contains(name));
    assert_eq!(
        names,
        [
            "DO_NOT_TRACK",
            "HF_HUB_OFFLINE",
            "HOME",
            "LLAMA_API_KEY",
            "PATH",
            "TMPDIR",
            "TRANSFORMERS_OFFLINE",
            "VLLM_NO_USAGE_STATS"
        ]
    );
    let stop = order(WorkType::ServerStop, Parameters::Stop { server: SERVER });
    assert_eq!(node.run(&stop).await, Ok(None));
    assert_eq!(node.run(&stop).await, Err(Cause::CopyUnknown));
}

#[tokio::test]
async fn a_listener_on_each_address_is_refused() {
    let root = tempfile::tempdir().expect("a folder");
    let mut node = installed_node(root.path(), "any", 42100);
    assert_eq!(node.run(&start()).await, Err(Cause::Listener));
    assert!(node.servers.map.is_empty());
}

#[tokio::test]
async fn the_sandbox_setting_with_no_isolation_refuses_the_start() {
    let root = tempfile::tempdir().expect("a folder");
    let mut node = installed_node(root.path(), "loopback", 42150);
    node.settings.sandbox = true;
    assert_eq!(node.run(&start()).await, Err(Cause::IsolationFailed));
    node.settings.sandbox = false;
    std::fs::remove_dir_all(node.folders.release(&release())).expect("removed");
    assert_eq!(node.run(&start()).await, Err(Cause::RuntimeNotInstalled));
}

#[tokio::test]
async fn a_runtime_that_stops_is_started_again_then_failed_after_the_limit() {
    let root = tempfile::tempdir().expect("a folder");
    let mut node = installed_node(root.path(), "loopback", 42200);
    node.run(&start()).await.expect("started");
    set_mode(&node, "exit");
    let group = node.servers.map[&SERVER]
        .running
        .as_ref()
        .expect("running")
        .0
        .group;
    let pid = rustix::process::Pid::from_raw(group).expect("a pid");
    rustix::process::kill_process_group(pid, rustix::process::Signal::KILL).expect("killed");
    let temporary = node.folders.server(SERVER).join("tmp");
    let mut now = 1_000;
    let mut states = Vec::new();
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(150)).await;
        node.servers.cycle(now, |_| temporary.clone()).await;
        let server = &node.servers.map[&SERVER];
        states.push((server.state, server.record.restarts()));
        if matches!(server.state, ServerState::Failed(_)) {
            break;
        }
        now += 300;
    }
    let restarts: Vec<usize> = states.iter().map(|(_, restarts)| *restarts).collect();
    assert_eq!(
        states.last().map(|(state, _)| *state),
        Some(ServerState::Failed(Cause::StartFailed)),
        "{states:?}"
    );
    assert_eq!(restarts.last(), Some(&5));
    assert!(
        states
            .iter()
            .any(|(state, _)| *state == ServerState::Restarting)
    );
}
