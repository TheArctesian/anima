//! Tests of the pure rules of a job and of the placement.

mod lifecycle;
mod placement;
mod reasons;

use chrono::{DateTime, TimeZone as _, Utc};
use daemon_core::wire::{
    Claim, Free, Limits, Switch, WorkArchitecture, WorkKind, WorkOs, WorkRepository, WorkType,
};
use serde_json::json;

use super::model::{Job, NewJob, State};
use crate::ids::{DeviceId, JobId};

const JOB: &str = "0192f6aa-0000-7000-8000-000000000001";
const PVE: &str = "0190a000-0000-7000-8001-000000000001";
const MAC: &str = "0190a000-0000-7000-8001-000000000002";

fn at(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_800_000_000 + seconds, 0).unwrap()
}

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

fn asked(work_type: WorkType) -> NewJob {
    NewJob {
        work_type,
        kind: WorkKind::Container,
        os: Some(WorkOs::Linux),
        architecture: Some(WorkArchitecture::X86_64),
        project: None,
        repository: WorkRepository {
            forge: "github".to_owned(),
            owner: "example".to_owned(),
            name: "app".to_owned(),
        },
        parameters: json!({}),
        secrets: Vec::new(),
        limits: Limits {
            seconds: 600,
            idle_seconds: 60,
            memory_bytes: 1 << 30,
            disk_bytes: 1 << 30,
            orphan_seconds: 600,
        },
        machine: None,
    }
}

fn queued(work_type: WorkType) -> Job {
    Job {
        id: JOB.parse::<JobId>().unwrap(),
        asked: asked(work_type),
        state: State::Queued,
        attempt: 0,
        lost: 0,
        holder: None,
        last_device: None,
        refused_by: Vec::new(),
        lease_ends_at: None,
        claimed_at: None,
        stop: None,
        reason: None,
        result: None,
        log_end: 0,
        created_at: at(0),
        changed_at: at(0),
        ended_at: None,
    }
}

fn claim_of(text: &str) -> Claim {
    Claim {
        device: device(text).into(),
        switch: Switch::On,
        types: vec![WorkType::Build, WorkType::Runner],
        kinds: vec![WorkKind::Container],
        os: WorkOs::Linux,
        architecture: WorkArchitecture::X86_64,
        free: Free {
            slots: 1,
            memory_bytes: 8 << 30,
            disk_bytes: 64 << 30,
        },
        holds: Vec::new(),
        unclean: false,
    }
}
