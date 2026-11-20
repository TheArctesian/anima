//! `anima-gateway listen <address:port> [--other-gateway <host:port> [--pass <protocol>]]`
//! runs the gateway. `anima-gateway session` is one worker; the listener starts it. Wiring
//! only.

use std::process::ExitCode;

use anima_gateway::listening::{self, MOST_SESSIONS, OTHER_GATEWAY, PASS, WORKER, Workers};
use anima_gateway::session::{self, Settings};
use tokio::net::TcpListener;

const USAGE: &str =
    "usage: anima-gateway listen <address:port> [--other-gateway <host:port> [--pass rdp|vnc]]";

/// The options of a command: another gateway, and the protocols that go to it though this
/// gateway serves them.
struct Options {
    other_gateway: Option<String>,
    passed: Vec<String>,
}

impl Options {
    /// Reads `rest`: nothing, or `--other-gateway` with its value and then any number of
    /// `--pass` with a protocol.
    fn read(rest: &[String]) -> std::io::Result<Self> {
        let usage = || std::io::Error::other(USAGE);
        let Some((other, pairs)) = rest.split_first_chunk::<2>() else {
            return if rest.is_empty() {
                Ok(Self {
                    other_gateway: None,
                    passed: Vec::new(),
                })
            } else {
                Err(usage())
            };
        };
        let [option, other_gateway] = other;
        if option != OTHER_GATEWAY || pairs.len() % 2 != 0 {
            return Err(usage());
        }
        let mut passed = Vec::new();
        for pass in pairs.as_chunks::<2>().0 {
            let served = ["rdp", "vnc"].contains(&pass[1].as_str());
            if pass[0] != PASS || !served {
                return Err(usage());
            }
            passed.push(pass[1].clone());
        }
        Ok(Self {
            other_gateway: Some(other_gateway.clone()),
            passed,
        })
    }
}

async fn listen(address: &str, options: Options) -> std::io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    let workers = Workers {
        program: std::env::current_exe()?,
        other_gateway: options.other_gateway,
        passed: options.passed,
        most_sessions: MOST_SESSIONS,
    };
    eprintln!("anima-gateway: listening on {}", listener.local_addr()?);
    #[cfg(unix)]
    return listening::listen(listener, workers).await;
    #[cfg(windows)]
    {
        let mut stop = anima_windows::shutdown::Requested::new()?;
        tokio::select! {
            result = listening::listen(listener, workers) => result,
            () = stop.wait() => Ok(()),
        }
    }
}

async fn work(options: Options) -> std::io::Result<()> {
    let hub = listening::hub_connection()?;
    let settings = Settings {
        other_gateway: options.other_gateway,
        passed: options.passed,
        ..Settings::default()
    };
    let connection_id = format!("${}", uuid::Uuid::now_v7());
    let ended = session::serve(hub, &connection_id, &settings).await;
    eprintln!("anima-gateway: session ended: {ended}");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    else {
        eprintln!("anima-gateway: the runtime did not start");
        return ExitCode::FAILURE;
    };
    let ran = match args.split_first() {
        Some((command, [address, rest @ ..])) if command == "listen" => {
            Options::read(rest).and_then(|options| runtime.block_on(listen(address, options)))
        }
        Some((command, rest)) if command == WORKER => {
            Options::read(rest).and_then(|options| runtime.block_on(work(options)))
        }
        _ => Err(std::io::Error::other(USAGE)),
    };
    match ran {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("anima-gateway: {error}");
            ExitCode::FAILURE
        }
    }
}
