//! What the checks domain adds to `Device` and `Query.devices`: the loaders behind
//! `status`, `uptime`, `checks` and `ip`, and the status filter and orders (API-005); and the
//! loaders the web interfaces domain adds (WEB-001, WEB-002) and the metrics domain adds
//! (MET-004, MET-006).

mod interfaces;
mod listing;
mod loaders;
mod metrics;
mod ranking;

pub use self::interfaces::{
    SuggestionsOf, WebInterfacesOf, web_interface_suggestions_loader, web_interfaces_loader,
};
pub use self::loaders::{ChecksOf, ResolvedIpOf, StatusOf, UptimeOf};
pub use self::metrics::{GpusOf, ReadingsOf, gpus_loader, readings_loader};
pub(super) use self::{
    listing::list,
    ranking::{Position, decode as decode_cursor},
};
