//! The reasons, the pins and the tie-break of the placement (task W3 part 3; R-259).

use std::collections::HashMap;

use daemon_core::wire::{Switch, WorkKind, WorkOs, WorkType};

use super::{
    MAC, PVE, at, claim_of, device,
    placement::{machine, offer, verdicts},
    queued,
};
use crate::work::jobs::{
    model::{Job, Machine},
    placement::{self, Around, Unfit, Verdict},
    reasons::{self, Decision},
};

const GB: u64 = 1 << 30;

type Change = fn(&mut daemon_core::wire::Claim);

#[test]
fn rou_004_each_requirement_that_fails_is_named_and_the_first_one_is_the_verdict() {
    let job = queued(WorkType::Build);
    let changes: [(Change, Verdict); 6] = [
        (|claim| claim.types = vec![WorkType::Runner], Verdict::Type),
        (|claim| claim.kinds = vec![WorkKind::Native], Verdict::Kind),
        (|claim| claim.os = WorkOs::Macos, Verdict::Os),
        (|claim| claim.free.memory_bytes = GB - 1, Verdict::Memory),
        (|claim| claim.free.disk_bytes = 0, Verdict::Disk),
        (|_| (), Verdict::Fits),
    ];
    for (change, expected) in changes {
        let mut claim = claim_of(PVE);
        change(&mut claim);
        assert_eq!(placement::alone(&job, None, &offer(&claim)), expected);
    }
    let refused = Job {
        refused_by: vec![device(PVE)],
        ..job
    };
    assert_eq!(
        placement::alone(&refused, None, &offer(&claim_of(PVE))),
        Verdict::RefusedBefore
    );
}

#[test]
fn rou_006_a_job_that_names_a_machine_goes_to_that_machine_and_to_no_other() {
    let mut job = queued(WorkType::Build);
    job.asked.machine = Some(device(MAC));
    let jobs = [job];
    let pve = verdicts(&jobs, &offer(&claim_of(PVE)), &[], at(3600));
    assert_eq!(pve, [Verdict::NamesOther]);
    assert_eq!(placement::choose(&jobs, &pve, device(PVE)), None);
    let mac = verdicts(&jobs, &offer(&claim_of(MAC)), &[], at(3600));
    assert_eq!(mac, [Verdict::Fits]);
}

#[test]
fn rou_006_the_pin_of_a_project_names_the_machine_and_the_name_of_a_job_wins() {
    let project = "0191c000-0000-7000-8000-000000000001".parse().unwrap();
    let mut job = queued(WorkType::Build);
    job.asked.project = Some(project);
    let pins = HashMap::from([(project, device(MAC))]);
    let around = Around {
        pins: &pins,
        others: &[],
        now: at(3600),
    };
    let pve = offer(&claim_of(PVE));
    assert_eq!(placement::verdict(&job, &pve, &around), Verdict::NamesOther);
    assert_eq!(
        placement::verdict(&job, &offer(&claim_of(MAC)), &around),
        Verdict::Fits
    );
    job.asked.machine = Some(device(PVE));
    assert_eq!(placement::verdict(&job, &pve, &around), Verdict::Fits);
}

#[test]
fn rou_006_an_excluded_machine_gets_only_work_that_names_it() {
    let mut excluded = offer(&claim_of(PVE));
    excluded.excluded = true;
    let job = queued(WorkType::Build);
    assert_eq!(placement::alone(&job, None, &excluded), Verdict::Excluded);
    assert_eq!(
        placement::alone(&job, Some(device(PVE)), &excluded),
        Verdict::Fits
    );
}

#[test]
fn r_259_inside_20_seconds_the_machine_with_more_free_memory_gets_the_job_and_after_them_the_first()
{
    let jobs = [queued(WorkType::Build)];
    let small = offer(&claim_of(PVE));
    let mut large = offer(&claim_of(MAC));
    large.memory_bytes = 32 * GB;

    // The small machine asks first, 5 seconds after the job came; the large one asked before.
    let early = verdicts(&jobs, &small, std::slice::from_ref(&large), at(5));
    assert_eq!(early, [Verdict::WaitsForBetter]);
    assert_eq!(placement::choose(&jobs, &early, small.device), None);
    let second = verdicts(&jobs, &large, std::slice::from_ref(&small), at(6));
    assert_eq!(
        placement::choose(&jobs, &second, large.device),
        Some(&jobs[0])
    );

    // After the 20 seconds the first machine that asks and fits gets it.
    let last = verdicts(&jobs, &small, std::slice::from_ref(&large), at(19));
    assert_eq!(last, [Verdict::WaitsForBetter]);
    let late = verdicts(&jobs, &small, std::slice::from_ref(&large), at(20));
    assert_eq!(
        placement::choose(&jobs, &late, small.device),
        Some(&jobs[0])
    );
}

#[test]
fn r_259_a_job_waits_only_for_a_machine_that_can_run_it() {
    let jobs = [queued(WorkType::Build)];
    let small = offer(&claim_of(PVE));
    let mut mac = offer(&claim_of(MAC));
    (mac.memory_bytes, mac.os) = (32 * GB, Some(WorkOs::Macos));
    assert_eq!(
        verdicts(&jobs, &small, std::slice::from_ref(&mac), at(5)),
        [Verdict::Fits],
        "the larger machine has a different system"
    );
    mac.os = Some(WorkOs::Linux);
    mac.excluded = true;
    assert_eq!(
        verdicts(&jobs, &small, &[mac], at(5)),
        [Verdict::Fits],
        "the larger machine is excluded"
    );
    let same = offer(&claim_of(MAC));
    assert_eq!(
        verdicts(&jobs, &small, &[same], at(5)),
        [Verdict::Fits],
        "the same free memory is not more"
    );
}

fn decision(name: &str, id: &str, verdict: Verdict, memory: u64) -> Decision {
    Decision {
        device: device(id),
        machine: name.to_owned(),
        verdict,
        memory_bytes: memory,
        disk_bytes: 180 * GB,
        at: at(0),
    }
}

#[test]
fn rou_004_the_reason_is_one_line_with_the_machine_that_fits_first() {
    let mut job = queued(WorkType::Build);
    job.asked.limits.memory_bytes = 8 * GB;
    assert_eq!(
        reasons::line(&job, &[]),
        "no machine has asked for this job"
    );
    let decisions = [
        decision("laptop", MAC, Verdict::Memory, 2 * GB),
        decision("studio", PVE, Verdict::Fits, 22 * GB),
    ];
    assert_eq!(
        reasons::line(&job, &decisions),
        "studio: fits, 22 GB of memory and 180 GB of disk free; \
         laptop refused: 2 GB of memory free, the job needs 8 GB"
    );
}

#[test]
fn rou_006_a_named_machine_that_can_never_run_the_job_is_refused_with_the_reason() {
    let job = queued(WorkType::Build);
    let known = machine(device(MAC));
    let mut mac = offer(&claim_of(MAC));
    assert_eq!(
        placement::never_fits(&job, None, at(0)),
        Some(Unfit::NeverAsked)
    );
    assert_eq!(
        placement::never_fits(&job, Some((&known, &mac)), at(0)),
        None
    );

    // Too little free memory can change: the job waits.
    mac.memory_bytes = 1;
    assert_eq!(
        placement::never_fits(&job, Some((&known, &mac)), at(0)),
        None
    );

    mac.os = Some(WorkOs::Macos);
    let wrong = placement::never_fits(&job, Some((&known, &mac)), at(0));
    assert_eq!(wrong, Some(Unfit::Requirement(Verdict::Os)));
    assert_eq!(
        reasons::unfit(&job, Unfit::Requirement(Verdict::Os)),
        "the machine has a different operating system"
    );

    let off = Machine {
        switch: Switch::Off,
        ..known
    };
    assert_eq!(
        placement::never_fits(&job, Some((&off, &offer(&claim_of(MAC)))), at(0)),
        Some(Unfit::TakesNoWork)
    );
}
