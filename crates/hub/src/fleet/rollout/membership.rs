//! Which machines a rollout includes (ROL-028, ROL-043, ROL-044). A machine that it leaves out
//! has a reason that the client shows.

use super::model::{Machine, Reach, Release};
use crate::fleet::builds::Version;

/// Why a rollout leaves a machine out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftOut {
    /// Its daemon token ended (ROL-043).
    EnrolAgain,
    /// Its daemon has no update function (ROL-044).
    ManualInstallation,
    /// Updates are set off at the machine (ROL-028).
    SetOff,
    /// The operator holds it (ROL-028).
    Held,
    /// The hub holds no release of that version for its target, or does not know its version.
    NoRelease,
    /// It has that version, or a higher one.
    UpToDate,
}

/// Whether a rollout to `version` includes `machine`, in the order of the states of K8
/// Section 8.4. A machine that is not connected is included: it updates when it connects
/// (ROL-039, ROL-040). So is a busy machine: it starts when it is no longer busy.
///
/// # Errors
///
/// The reason when the rollout leaves the machine out.
pub fn place(machine: &Machine, version: Version, held: &[Release]) -> Result<(), LeftOut> {
    if machine.reach == Reach::TokenEnded {
        return Err(LeftOut::EnrolAgain);
    }
    let Some(said) = machine.said else {
        return Err(LeftOut::ManualInstallation);
    };
    if !said.switch_on {
        return Err(LeftOut::SetOff);
    }
    if machine.held {
        return Err(LeftOut::Held);
    }
    let released = held
        .iter()
        .any(|release| release.version == version && Some(release.target) == said.target);
    let Some(installed) = machine.installed.filter(|_| released) else {
        return Err(LeftOut::NoRelease);
    };
    if installed >= version {
        return Err(LeftOut::UpToDate);
    }
    Ok(())
}
