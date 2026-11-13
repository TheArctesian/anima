//! The K1 custom scalars as the generated document types see them. A document module imports
//! the ones its documents use (`use crate::hub::scalars::DateTime;`). They stay text, as the
//! hub wrote them, so `--json` prints exactly the hub's value.

/// RFC 3339 in UTC (K7).
pub type DateTime = String;
/// An IPv4 or IPv6 address.
pub type IpAddr = String;
/// A network such as `10.10.0.0/24`.
pub type Cidr = String;
// `JSON` (audit change records) joins when a document first reads one; graphql_client looks
// the alias up by the scalar's own name.
