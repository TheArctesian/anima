//! What the checks domain adds to `Device` and `Query.devices`: the loaders behind
//! `status`, `uptime`, `checks` and `ip`, and the status filter and orders (R-011, API-005).

mod listing;
mod loaders;
mod ranking;

pub use self::loaders::{ChecksOf, ResolvedIpOf, StatusOf, UptimeOf};
pub(super) use self::{
    listing::list,
    ranking::{Position, decode as decode_cursor},
};
