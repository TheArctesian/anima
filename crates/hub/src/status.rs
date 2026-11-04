//! Health status of a check or device, and how statuses combine (CHK-004).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Enum)]
pub enum Status {
    Up,
    Down,
    Degraded,
    Maintenance,
    Unknown,
}

impl Status {
    /// Higher is worse: Down > Degraded > Unknown > Maintenance > Up.
    const fn severity(self) -> u8 {
        match self {
            Self::Up => 0,
            Self::Maintenance => 1,
            Self::Unknown => 2,
            Self::Degraded => 3,
            Self::Down => 4,
        }
    }
}

/// The more severe of two statuses; the device status is the worst of its checks (CHK-004).
#[must_use]
pub const fn worst(a: Status, b: Status) -> Status {
    if b.severity() > a.severity() { b } else { a }
}

#[cfg(test)]
mod tests {
    use super::Status::{self, Degraded, Down, Maintenance, Unknown, Up};
    use super::worst;

    const BEST_TO_WORST: [Status; 5] = [Up, Maintenance, Unknown, Degraded, Down];

    #[test]
    fn chk_004_worst_follows_severity_order_for_all_pairs() {
        for (i, a) in BEST_TO_WORST.into_iter().enumerate() {
            for (j, b) in BEST_TO_WORST.into_iter().enumerate() {
                let expected = BEST_TO_WORST[i.max(j)];
                assert_eq!(worst(a, b), expected, "{a:?} with {b:?}");
            }
        }
    }

    #[test]
    fn chk_006_maintenance_does_not_hide_a_down_check_elsewhere() {
        assert_eq!(worst(Maintenance, Down), Down);
    }

    #[test]
    fn graphql_names_are_screaming_snake_case() {
        use async_graphql::EnumType;
        let names: Vec<_> = Status::items().iter().map(|item| item.name).collect();
        assert_eq!(names, ["UP", "DOWN", "DEGRADED", "MAINTENANCE", "UNKNOWN"]);
    }
}
