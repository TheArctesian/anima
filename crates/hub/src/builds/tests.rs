//! Tests of the pure build rules (BLD-002 to BLD-010, BLD-048, BLD-049, BLD-063, API-006).

use std::net::{IpAddr, Ipv4Addr};

use chrono::{DateTime, TimeDelta, Utc};

use super::{
    model::{Arch, BuildJob, Builder, Commit, JobState, Os, Platform, StateTimes, Target},
    version::Version,
};
use crate::ids::{BuildJobId, BuilderId, ReleaseId};

const JOB: &str = "0190a000-0000-7000-8009-000000000001";
const RELEASE: &str = "0190a000-0000-7000-8009-000000000002";
const BUILDER: &str = "0190a000-0000-7000-8009-000000000003";
const OTHER_BUILDER: &str = "0190a000-0000-7000-8009-000000000004";

fn start() -> DateTime<Utc> {
    "2030-05-05T12:00:00Z".parse().unwrap()
}

fn seconds(n: i64) -> DateTime<Utc> {
    start() + TimeDelta::seconds(n)
}

const LINUX: Platform = Platform {
    os: Os::Linux,
    arch: Arch::X86_64,
};

fn builder(id: &str) -> Builder {
    Builder {
        id: id.parse::<BuilderId>().unwrap(),
        name: "pop-os".to_owned(),
        platform: LINUX,
        client: IpAddr::V4(Ipv4Addr::new(100, 64, 0, 3)),
        enrolled_at: start(),
        last_seen_at: None,
    }
}

fn queued_job() -> BuildJob {
    BuildJob {
        id: JOB.parse::<BuildJobId>().unwrap(),
        release: RELEASE.parse::<ReleaseId>().unwrap(),
        target: Target::LinuxX86_64,
        version: Version::new(0, 1, 12),
        commit: Commit("a".repeat(40)),
        state: JobState::Queued,
        failure_reason: None,
        attempts: 0,
        builder: None,
        builder_name: None,
        lease_ends_at: None,
        log_sequence: 0,
        created_at: start(),
        times: StateTimes {
            queued: start(),
            claimed: None,
            building: None,
            uploading: None,
            published: None,
            failed: None,
            cancelled: None,
            changed: start(),
        },
    }
}

mod fields;
mod lines;
mod queue;
mod releases;
mod uploads;
mod versions;
