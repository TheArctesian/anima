//! Tests of the containers of a device: the reading of what `docker` writes (`docker`), the
//! two reads against the link's test SSH server on loopback (`reads`), and each error with
//! its code and sentence (`sentences`). The queries themselves are in the integration crate. In-crate, for the loopback policy.

mod detail;
mod docker;
mod project;
mod reads;
mod sentences;
mod storage;

use sqlx::PgPool;

use super::Reach;
use crate::fleet::ssh::tests::{Answer, Bench, Heard, bench, reaching};

/// A full container identifier.
const ID: &str = "4f5c1d2e3a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456789a";
/// Text only a container's log holds; nothing the hub writes besides the answer may hold it.
const SECRET: &str = "API_TOKEN=hunter2";

/// What `docker ps` writes for two containers in the hub's format: one of a Compose project,
/// healthy, with two ports; one stopped, with no project.
const LISTED: &str = concat!(
    r#"{"id":"4f5c1d2e3a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456789a","#,
    r#""names":"dawarich_app","image":"freikin/dawarich:latest","state":"running","#,
    r#""status":"Up 3 hours (healthy)","ports":"0.0.0.0:3000->3000/tcp, [::]:3000->3000/tcp","#,
    r#""created":"2026-11-18 10:00:00 -0800 PST","project":"dawarich","service":"app"}"#,
    "\n",
    r#"{"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","#,
    r#""names":"web/db,old-db","image":"postgres:17","state":"exited","#,
    r#""status":"Exited (0) 2 days ago","ports":"","created":"not a time","#,
    r#""project":"","service":""}"#,
    "\n",
);

/// An answer that writes `stdout`, and [`SECRET`] to standard error, and exits with `status`.
fn exiting(stdout: &str, status: u32) -> Answer {
    Answer::Exit {
        stdout: stdout.as_bytes().to_vec(),
        stderr: SECRET.as_bytes().to_vec(),
        status,
    }
}

/// A bench whose `docker-host` answers every command with `answer`.
async fn device(pool: &PgPool, answer: Answer) -> Bench {
    bench(pool, answer).await
}

fn reach<'a>(bench: &'a Bench, policy: &'a crate::fleet::targets::AddressPolicy) -> Reach<'a> {
    Reach {
        db: &bench.db,
        vault: &bench.vault,
        policy,
    }
}

/// The command lines the device heard.
fn execs(bench: &Bench) -> Vec<String> {
    bench
        .server
        .heard()
        .into_iter()
        .filter_map(|heard| match heard {
            Heard::Exec(line) => Some(line),
            _ => None,
        })
        .collect()
}

/// The policy that reaches the loopback test server.
fn policy() -> crate::fleet::targets::AddressPolicy {
    reaching()
}
