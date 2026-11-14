//! Signs to proposed resources (PRO-003, PRO-004, PRO-006).

use provider_github::SignKind;
use uuid::Uuid;

use crate::{
    ids::ResourceId,
    work::projects::{
        model::{Held, ResourceKind, ResourceState, Sign},
        proposals::{Plan, kind_of, reconcile},
    },
};

fn held(kind: ResourceKind, state: ResourceState) -> Held {
    Held {
        id: ResourceId::from(Uuid::now_v7()),
        kind,
        state,
    }
}

#[test]
fn pro_003_each_sign_proposes_its_hosting() {
    let kinds = [
        (SignKind::WranglerConfig, ResourceKind::Cloudflare),
        (SignKind::VercelJson, ResourceKind::Vercel),
        (SignKind::VercelDeployment, ResourceKind::Vercel),
        (SignKind::Dockerfile, ResourceKind::Server),
        (SignKind::ComposeFile, ResourceKind::Server),
        (SignKind::TauriConf, ResourceKind::DesktopApplication),
    ];
    for (sign, kind) in kinds {
        assert_eq!(kind_of(Sign::of(sign)), kind, "{sign:?}");
        assert_eq!(Sign::parse(Sign::of(sign).as_str()), Some(Sign::of(sign)));
    }
}

#[test]
fn pro_006_only_hosting_kinds_exist() {
    let names: Vec<&str> = ResourceKind::ALL.iter().map(|k| k.as_str()).collect();
    assert_eq!(
        names,
        ["cloudflare", "vercel", "server", "desktop_application"]
    );
}

#[test]
fn pro_003_signs_with_no_resource_are_proposed_once_per_kind() {
    let plan = reconcile(&[Sign::Dockerfile, Sign::ComposeFile, Sign::TauriConf], &[]);
    assert_eq!(
        plan,
        Plan {
            add: vec![ResourceKind::Server, ResourceKind::DesktopApplication],
            drop: Vec::new(),
        }
    );
}

#[test]
fn pro_004_a_found_resource_whose_sign_went_is_dropped_and_a_confirmed_one_stays() {
    let found = held(ResourceKind::Vercel, ResourceState::FoundInRepository);
    let confirmed = held(ResourceKind::Server, ResourceState::Confirmed);
    let plan = reconcile(&[], &[found, confirmed]);
    assert_eq!(
        plan,
        Plan {
            add: Vec::new(),
            drop: vec![found.id],
        }
    );
}

#[test]
fn pro_004_a_held_kind_with_its_sign_is_left_alone() {
    let found = held(ResourceKind::Cloudflare, ResourceState::FoundInRepository);
    assert_eq!(
        reconcile(&[Sign::WranglerConfig], &[found]),
        Plan::default()
    );
}
