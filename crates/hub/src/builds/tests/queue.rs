//! The job rules: state changes, the lease and its end, platform match (BLD-003 to BLD-010).

use super::{BUILDER, OTHER_BUILDER, builder, queued_job, seconds, start};
use crate::builds::{
    model::{Arch, JobState, Os, Platform, Target},
    queue::{self, Refusal, STOPPED_REASON},
};

fn claimed() -> crate::builds::BuildJob {
    queue::claim(&queued_job(), &builder(BUILDER), start())
}

#[test]
fn bld_005_a_claim_leases_the_job_for_90_seconds_to_the_builder() {
    let job = claimed();
    assert_eq!(job.state, JobState::Claimed);
    assert_eq!(job.builder, Some(builder(BUILDER).id));
    assert_eq!(job.builder_name.as_deref(), Some("pop-os"));
    assert_eq!(job.lease_ends_at, Some(seconds(90)));
    assert_eq!(job.times.claimed, Some(start()));
    assert_eq!(job.times.changed, start());
}

#[test]
fn bld_005_each_report_extends_the_lease_to_90_seconds_from_the_report() {
    let job = queue::report(&claimed(), builder(BUILDER).id, None, "", seconds(15)).unwrap();
    assert_eq!(job.state, JobState::Claimed);
    assert_eq!(job.lease_ends_at, Some(seconds(105)));
    let job = queue::report(
        &job,
        builder(BUILDER).id,
        Some(JobState::Building),
        "",
        seconds(30),
    )
    .unwrap();
    assert_eq!(job.state, JobState::Building);
    assert_eq!(job.times.building, Some(seconds(30)));
    assert_eq!(job.lease_ends_at, Some(seconds(120)));
}

#[test]
fn bld_010_a_report_from_a_builder_without_the_lease_is_refused() {
    let refused = queue::report(&claimed(), builder(OTHER_BUILDER).id, None, "", seconds(5));
    assert_eq!(refused, Err(Refusal::NotHolder));
    let queued = queue::report(&queued_job(), builder(BUILDER).id, None, "", seconds(5));
    assert_eq!(queued, Err(Refusal::NotHolder));
}

#[test]
fn bld_006_a_report_after_the_lease_ended_is_refused() {
    let late = queue::report(&claimed(), builder(BUILDER).id, None, "", seconds(90));
    assert_eq!(late, Err(Refusal::LeaseEnded));
}

#[test]
fn bld_010_a_builder_may_report_only_the_changes_the_diagram_permits() {
    let holder = builder(BUILDER).id;
    for to in [
        JobState::Uploading,
        JobState::Published,
        JobState::Queued,
        JobState::Cancelled,
    ] {
        let refused = queue::report(&claimed(), holder, Some(to), "", seconds(1));
        assert_eq!(
            refused,
            Err(Refusal::Change {
                from: JobState::Claimed,
                to
            }),
            "{to:?}"
        );
    }
    let uploading = [JobState::Building, JobState::Uploading]
        .into_iter()
        .fold(claimed(), |job, to| {
            queue::report(&job, holder, Some(to), "", seconds(1)).unwrap()
        });
    let published = queue::report(&uploading, holder, Some(JobState::Published), "", seconds(2));
    assert!(matches!(published, Err(Refusal::Change { .. })));
}

#[test]
fn bld_010_a_failed_report_ends_the_lease_with_its_reason() {
    let job = queue::report(
        &claimed(),
        builder(BUILDER).id,
        Some(JobState::Failed),
        "gates failed",
        seconds(10),
    )
    .unwrap();
    assert_eq!(job.state, JobState::Failed);
    assert_eq!(job.failure_reason.as_deref(), Some("gates failed"));
    assert_eq!(job.lease_ends_at, None);
    assert_eq!(job.times.failed, Some(seconds(10)));
}

#[test]
fn bld_006_an_ended_lease_requeues_the_job_twice_then_fails_it() {
    let first = queue::expire(&claimed(), seconds(90));
    assert_eq!(first.state, JobState::Queued);
    assert_eq!(first.attempts, 1);
    assert_eq!((first.builder, first.lease_ends_at), (None, None));
    assert_eq!(first.times.queued, seconds(90));

    let again = queue::claim(&first, &builder(BUILDER), seconds(100));
    let second = queue::expire(&again, seconds(190));
    assert_eq!((second.state, second.attempts), (JobState::Queued, 2));

    let third = queue::expire(
        &queue::claim(&second, &builder(BUILDER), seconds(200)),
        seconds(290),
    );
    assert_eq!((third.state, third.attempts), (JobState::Failed, 3));
    assert_eq!(third.failure_reason.as_deref(), Some(STOPPED_REASON));
    assert_eq!(third.times.failed, Some(seconds(290)));
}

#[test]
fn bld_005_a_lease_ends_at_its_end_and_not_before() {
    let job = claimed();
    assert!(!queue::lease_ended(&job, seconds(89)));
    assert!(queue::lease_ended(&job, seconds(90)));
    assert!(!queue::lease_ended(&queued_job(), seconds(1000)));
}

#[test]
fn bld_008_only_a_queued_job_can_be_cancelled() {
    let cancelled = queue::cancel(&queued_job(), seconds(5)).unwrap();
    assert_eq!(cancelled.state, JobState::Cancelled);
    assert_eq!(cancelled.times.cancelled, Some(seconds(5)));
    assert!(queue::cancel(&claimed(), seconds(5)).is_err());
    assert!(queue::cancel(&cancelled, seconds(6)).is_err());
}

#[test]
fn bld_003_a_platform_builds_only_its_own_target() {
    let platform = |os, arch| Platform { os, arch };
    let cases = [
        (platform(Os::Linux, Arch::X86_64), vec![Target::LinuxX86_64]),
        (platform(Os::Macos, Arch::Aarch64), vec![Target::DarwinUniversal]),
        (platform(Os::Linux, Arch::Aarch64), vec![]),
        (platform(Os::Macos, Arch::X86_64), vec![]),
    ];
    for (platform, targets) in cases {
        assert_eq!(queue::buildable(platform), targets, "{platform:?}");
    }
}

#[test]
fn bld_007_a_builder_is_online_for_60_seconds_after_it_called() {
    assert!(!queue::online(None, start()));
    assert!(queue::online(Some(start()), seconds(60)));
    assert!(!queue::online(Some(start()), seconds(61)));
}

#[test]
fn bld_010_the_permitted_changes_are_those_of_the_diagram() {
    use JobState::{Building, Cancelled, Claimed, Failed, Published, Queued, Uploading};
    let all = [
        Queued, Claimed, Building, Uploading, Published, Failed, Cancelled,
    ];
    let allowed = [
        (Queued, Claimed),
        (Queued, Cancelled),
        (Claimed, Building),
        (Claimed, Failed),
        (Claimed, Queued),
        (Building, Uploading),
        (Building, Failed),
        (Building, Queued),
        (Uploading, Published),
        (Uploading, Failed),
        (Uploading, Queued),
    ];
    for from in all {
        for to in all {
            let expected = allowed.contains(&(from, to));
            assert_eq!(queue::permitted(from, to), expected, "{from:?} -> {to:?}");
        }
    }
}
