//! TCP probe against local listeners (CHK-001).

use std::time::Duration;

use tokio::net::TcpListener;

use super::support::{LOOPBACK, env, now, target};
use crate::fleet::{
    checks::{
        config::CheckConfig,
        probe::{Failure, ProbeEnv, probe},
    },
    targets::tests::link_path::{COMPUTER, Link, link},
};

#[tokio::test]
async fn chk_001_tcp_open_port_is_up_with_connect_time_and_address() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let config = CheckConfig::Tcp {
        port,
        timeout_ms: 2000,
    };
    let result = probe(&target("127.0.0.1"), &config, None, &env()).await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(result.resolved_ip, Some(LOOPBACK));
    assert!(result.response_time.unwrap() < Duration::from_secs(2));
    assert_eq!(result.message, None);
}

#[tokio::test]
async fn chk_001_tcp_closed_port_fails_with_a_short_message() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let config = CheckConfig::Tcp {
        port,
        // Windows retries a refused loopback connection before reporting it.
        timeout_ms: 5000,
    };
    let result = probe(&target("127.0.0.1"), &config, None, &env()).await;
    assert!(!result.ok);
    assert_eq!(result.response_time, None);
    let message = result.message.unwrap();
    assert!(message.starts_with("the connection failed"), "{message}");
    assert!(message.len() <= 512);
}

#[tokio::test]
async fn chk_001_tcp_name_that_does_not_resolve_fails() {
    let mut env = env();
    env.overrides.hosts =
        std::collections::HashMap::from([("empty.test".to_owned(), vec![])]).into();
    let config = CheckConfig::Tcp {
        port: 80,
        timeout_ms: 1000,
    };
    let result = probe(&target("empty.test"), &config, None, &env).await;
    assert_eq!(
        result.message.as_deref(),
        Some(Failure::Unresolved.to_string().as_str())
    );
}

#[tokio::test]
async fn chk_001_tcp_uses_the_name_from_the_resolver_override() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let mut env = env();
    env.overrides.hosts =
        std::collections::HashMap::from([("nas.test".to_owned(), vec![LOOPBACK])]).into();
    let config = CheckConfig::Tcp {
        port,
        timeout_ms: 2000,
    };
    let result = probe(&target("NAS.test"), &config, None, &env).await;
    assert!(result.ok, "{:?}", result.message);
}

#[tokio::test]
async fn push_and_group_are_not_probed() {
    for config in [CheckConfig::Push, CheckConfig::Group] {
        let result = probe(&target("127.0.0.1"), &config, None, &env()).await;
        assert!(!result.ok);
        assert_eq!(
            result.message.as_deref(),
            Some("this check type is not probed by the hub")
        );
    }
}

#[tokio::test]
async fn an_invalid_stored_configuration_fails_without_a_network_call() {
    let config = CheckConfig::Tcp {
        port: 0,
        timeout_ms: 1000,
    };
    let result = probe(&target("127.0.0.1"), &config, None, &env()).await;
    assert_eq!(
        result.message.as_deref(),
        Some("the configuration is not valid")
    );
}

fn rust_sources(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn chk_012_no_check_source_starts_a_process() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fleet");
    let mut files = vec![root.join("checks.rs")];
    rust_sources(&root.join("checks"), &mut files);
    files.retain(|f| !f.components().any(|c| c.as_os_str() == "tests"));
    assert!(files.len() >= 15, "found only {} sources", files.len());
    for file in files {
        // A `FixedCommand` is text the SSH link sends; it starts nothing on the hub (CHK-012).
        let source = std::fs::read_to_string(&file)
            .unwrap()
            .replace("FixedCommand::new", "");
        for forbidden in ["std::process", "tokio::process", "Command::new"] {
            assert!(
                !source.contains(forbidden),
                "{forbidden} in {}",
                file.display()
            );
        }
    }
}

/// A TCP check on a computer of the link goes through the link: the hub has no route to a
/// link address, and loopback is not permitted here, so only the link can answer.
#[tokio::test]
async fn hmd_048_a_tcp_check_reaches_an_open_port_of_a_computer_on_the_link() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (_folder, policy) = link(Link::Carries);
    let config = CheckConfig::Tcp {
        port,
        timeout_ms: 2000,
    };
    let result = probe(
        &target(COMPUTER),
        &config,
        None,
        &ProbeEnv::new(policy, now()),
    )
    .await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(result.resolved_ip, Some(COMPUTER.parse().unwrap()));
    assert!(result.response_time.is_some());
}

#[tokio::test]
async fn hmd_049_a_tcp_check_of_a_port_that_is_not_turned_on_fails_and_says_so() {
    for (word, message) in [
        ("refused", "the connection failed: permission denied"),
        ("closed", "the connection failed: connection refused"),
        ("not_connected", "the connection failed: not connected"),
    ] {
        let (_folder, policy) = link(Link::Says(word));
        let config = CheckConfig::Tcp {
            port: 22,
            timeout_ms: 2000,
        };
        let result = probe(
            &target(COMPUTER),
            &config,
            None,
            &ProbeEnv::new(policy, now()),
        )
        .await;
        assert!(!result.ok, "{word}");
        assert_eq!(result.message.as_deref(), Some(message), "{word}");
    }
}
