//! Where the power comes from (CAP-004): the parse is in the crate `power`, which the desktop's
//! stay-awake control also uses; this module gives its result as the batch's `PowerSource`.

pub use ::power::{Power, Supply, from_pmset, from_supplies};

use super::PowerSource;

impl From<Power> for PowerSource {
    fn from(power: Power) -> Self {
        match power {
            Power::Mains => Self::Mains,
            Power::Battery { .. } => Self::Battery,
        }
    }
}
