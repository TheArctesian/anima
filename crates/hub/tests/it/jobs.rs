//! The work route through the hub's routes (K10, `contracts/work-route.md`; task W3): a daemon
//! claims a job with its token, reports it, and the hub ends what a machine lost.

mod flow;
mod refusals;

use anima_hub::{
    clock::Clock as _,
    fleet::events::{Bus, ClientAddr, Operator},
    ids::JobId,
    work::jobs::{self, Job, NewJob, Queue},
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use daemon_core::wire::{Limits, WorkArchitecture, WorkKind, WorkOs, WorkRepository, WorkType};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::daemon::harness::{Daemons, PERMITTED};

/// A hub with the fixture devices, and what a test needs to put a job in the queue.
struct Work {
    daemons: Daemons,
    pool: PgPool,
    bus: Bus,
}

fn build() -> NewJob {
    NewJob {
        work_type: WorkType::Build,
        kind: WorkKind::Container,
        os: Some(WorkOs::Linux),
        architecture: Some(WorkArchitecture::X86_64),
        project: None,
        repository: WorkRepository {
            forge: "github".to_owned(),
            owner: "example".to_owned(),
            name: "app".to_owned(),
        },
        parameters: json!({
            "recipe": "desktop_release", "commit": "0123456789abcdef0123456789abcdef01234567",
            "target": "linux-x86_64", "version": "0.3.0", "trigger": "operator", "origin": "default_branch"
        }),
        secrets: Vec::new(),
        limits: Limits {
            seconds: 600,
            idle_seconds: 60,
            memory_bytes: 1 << 30,
            disk_bytes: 1 << 30,
            orphan_seconds: 600,
        },
    }
}

fn runner() -> NewJob {
    NewJob {
        work_type: WorkType::Runner,
        parameters: json!({ "runnerVersion": "2.330.0", "runnerSha256": "9f".repeat(32), "labels": ["self-hosted", "anima"] }),
        secrets: vec!["jit_config".to_owned()],
        ..build()
    }
}

impl Work {
    async fn new(pool: &PgPool) -> Self {
        Self {
            daemons: Daemons::new(pool).await,
            pool: pool.clone(),
            bus: Bus::new(64),
        }
    }

    fn queue(&self) -> Queue<'_> {
        Queue {
            db: &self.pool,
            bus: &self.bus,
        }
    }

    async fn enqueue(&self, asked: NewJob) -> Job {
        let asker = (Operator::hub(), ClientAddr(PERMITTED.parse().unwrap()));
        jobs::enqueue(self.queue(), asked, asker, self.now())
            .await
            .unwrap()
    }

    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.daemons.clock.now()
    }

    async fn job(&self, id: JobId) -> (Job, String) {
        jobs::job_with_log(self.queue(), id).await.unwrap().unwrap()
    }

    /// `POST /work/v1/<route>` with `token` as the bearer; the status and the JSON of the body.
    async fn post(&self, route: &str, token: Option<&str>, body: &Value) -> (StatusCode, Value) {
        let mut request = Request::post(format!("/work/v1/{route}"))
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let request = request.body(Body::from(body.to_string())).unwrap();
        let (status, text) = self.daemons.send(request, PERMITTED).await;
        (status, serde_json::from_str(&text).unwrap_or(Value::Null))
    }

    async fn claim(&self, token: &str, claim: &Value) -> (StatusCode, Value) {
        self.post("claim", Some(token), claim).await
    }

    async fn report(&self, token: &str, report: &Value) -> (StatusCode, Value) {
        self.post("report", Some(token), report).await
    }
}

/// The claim of a Linux machine with one free slot that holds `holds`.
fn claim_of(device: &str, holds: &Value) -> Value {
    json!({
        "device": device, "switch": "on", "types": ["build", "runner"], "kinds": ["container"],
        "os": "linux", "architecture": "x86_64",
        "free": { "slots": 1, "memoryBytes": 8_589_934_592_u64, "diskBytes": 68_719_476_736_u64 },
        "holds": holds, "unclean": false
    })
}

fn report_of(job: &Value, state: &str) -> Value {
    let done = matches!(state, "done" | "failed" | "stopped" | "refused");
    let result = done.then(|| {
        json!({ "outcome": if state == "done" { "succeeded" } else { state }, "cause": null, "seconds": 7,
            "cleanup": { "isolation": "deleted", "checkout": "deleted", "secrets": "deleted", "keychain": "none" } })
    });
    json!({ "id": job["id"], "attempt": job["attempt"], "state": state, "result": result })
}
