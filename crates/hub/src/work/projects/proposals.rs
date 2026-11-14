//! Which resources the signs of a project propose (PRO-003, PRO-004, PRO-006). Pure.

use std::collections::BTreeSet;

use super::model::{Held, ResourceKind, ResourceState, Sign};
use crate::ids::ResourceId;

/// The hosting a sign points at. Every sign names hosting; none names a service that holds
/// data about the operator (PRO-006).
#[must_use]
pub const fn kind_of(sign: Sign) -> ResourceKind {
    match sign {
        Sign::WranglerConfig => ResourceKind::Cloudflare,
        Sign::VercelJson | Sign::VercelDeployment => ResourceKind::Vercel,
        Sign::Dockerfile | Sign::ComposeFile => ResourceKind::Server,
        Sign::TauriConf => ResourceKind::DesktopApplication,
    }
}

/// What to change so the stored resources match the signs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Kinds with a sign and no resource: propose them as found in the repository.
    pub add: Vec<ResourceKind>,
    /// Found resources whose kind has no sign left. A confirmed one is never here.
    pub drop: Vec<ResourceId>,
}

/// The plan for a project whose repositories (those not left out) show `signs`, holding
/// `held`.
#[must_use]
pub fn reconcile(signs: &[Sign], held: &[Held]) -> Plan {
    let signed: BTreeSet<ResourceKind> = signs.iter().copied().map(kind_of).collect();
    let add = signed
        .iter()
        .copied()
        .filter(|kind| !held.iter().any(|resource| resource.kind == *kind))
        .collect();
    let drop = held
        .iter()
        .filter(|resource| {
            resource.state == ResourceState::FoundInRepository && !signed.contains(&resource.kind)
        })
        .map(|resource| resource.id)
        .collect();
    Plan { add, drop }
}
