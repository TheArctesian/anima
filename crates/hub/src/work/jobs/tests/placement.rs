use daemon_core::wire::{Switch, WorkArchitecture, WorkKind, WorkOs, WorkType};

use super::{MAC, PVE, at, claim_of, device, queued};
use crate::{
    ids::JobId,
    work::jobs::{
        model::{Job, Machine},
        placement,
    },
};

fn numbered(job: Job, number: u128) -> Job {
    Job {
        id: JobId::from(uuid::Uuid::from_u128(number)),
        ..job
    }
}

#[test]
fn pro_051_a_machine_gets_the_oldest_job_that_it_can_run() {
    let mac_only = Job {
        asked: {
            let mut asked = queued(WorkType::Build).asked;
            asked.os = Some(WorkOs::Macos);
            asked.architecture = Some(WorkArchitecture::Aarch64);
            asked
        },
        ..numbered(queued(WorkType::Build), 1)
    };
    let jobs = [
        mac_only,
        numbered(queued(WorkType::Build), 2),
        numbered(queued(WorkType::Runner), 3),
    ];
    let chosen = placement::choose(&jobs, device(PVE), &claim_of(PVE)).unwrap();
    assert_eq!(chosen.id, jobs[1].id);
}

#[test]
fn pro_051_each_thing_that_a_job_asks_of_a_machine_can_refuse_it() {
    let job = queued(WorkType::Build);
    let pve = device(PVE);
    assert!(placement::fits(&job, pve, &claim_of(PVE)));
    let changes: [fn(&mut daemon_core::wire::Claim); 6] = [
        |claim| claim.types = vec![WorkType::Runner],
        |claim| claim.kinds = vec![WorkKind::Native],
        |claim| claim.os = WorkOs::Macos,
        |claim| claim.architecture = WorkArchitecture::Aarch64,
        |claim| claim.free.memory_bytes = (1 << 30) - 1,
        |claim| claim.free.disk_bytes = 0,
    ];
    for change in changes {
        let mut claim = claim_of(PVE);
        change(&mut claim);
        assert!(!placement::fits(&job, pve, &claim), "{claim:?}");
    }
    let any = Job {
        asked: {
            let mut asked = job.asked.clone();
            (asked.os, asked.architecture) = (None, None);
            asked
        },
        ..job.clone()
    };
    let mut mac = claim_of(MAC);
    (mac.os, mac.architecture) = (WorkOs::Macos, WorkArchitecture::Aarch64);
    assert!(
        placement::fits(&any, device(MAC), &mac),
        "no system asked takes each"
    );
    let refused = Job {
        refused_by: vec![pve],
        ..job
    };
    assert!(!placement::fits(&refused, pve, &claim_of(PVE)));
}

#[test]
fn k10_4_a_machine_that_is_off_not_clean_or_full_gets_nothing() {
    let jobs = [queued(WorkType::Build)];
    let changes: [fn(&mut daemon_core::wire::Claim); 3] = [
        |claim| claim.switch = Switch::Off,
        |claim| claim.unclean = true,
        |claim| claim.free.slots = 0,
    ];
    for change in changes {
        let mut claim = claim_of(PVE);
        change(&mut claim);
        assert_eq!(
            placement::choose(&jobs, device(PVE), &claim),
            None,
            "{claim:?}"
        );
    }
}

#[test]
fn use_007_a_job_that_a_machine_lost_goes_to_a_different_job_first_for_that_machine() {
    let lost_here = Job {
        last_device: Some(device(PVE)),
        ..numbered(queued(WorkType::Build), 1)
    };
    let other = numbered(queued(WorkType::Build), 2);
    let jobs = [lost_here.clone(), other.clone()];
    assert_eq!(
        placement::choose(&jobs, device(PVE), &claim_of(PVE))
            .unwrap()
            .id,
        other.id
    );
    assert_eq!(
        placement::choose(&jobs, device(MAC), &claim_of(MAC))
            .unwrap()
            .id,
        lost_here.id
    );
    let alone = [lost_here.clone()];
    assert_eq!(
        placement::choose(&alone, device(PVE), &claim_of(PVE))
            .unwrap()
            .id,
        lost_here.id,
        "with no other job the machine gets it again"
    );
}

#[test]
fn k10_11_five_jobs_refused_or_lost_in_one_hour_give_one_hour_with_no_work() {
    assert_eq!(placement::off_until(4, at(0)), None);
    assert_eq!(placement::off_until(5, at(0)), Some(at(3600)));
    let machine = Machine {
        device: device(PVE),
        switch: Switch::On,
        unclean: false,
        off_by_operator: false,
        off_until: Some(at(3600)),
        first_on_at: Some(at(0)),
    };
    assert!(placement::is_off(&machine, at(3599)));
    assert!(!placement::is_off(&machine, at(3600)));
    let by_operator = Machine {
        off_by_operator: true,
        off_until: None,
        ..machine
    };
    assert!(placement::is_off(&by_operator, at(10_000)));
}
