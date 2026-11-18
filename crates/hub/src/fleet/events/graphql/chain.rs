//! The K1 `AuditChainCheck` type that `Query.auditChain` returns (LOG-027, R-136).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, SimpleObject};
use chrono::{DateTime, Utc};

use crate::fleet::events::chain::{ChainCheck, Fault, FaultKind, hex};

/// K1 `AuditChainCheck`.
#[derive(SimpleObject)]
#[graphql(name = "AuditChainCheck")]
pub(in crate::fleet::events) struct ChainCheckNode {
    /// True when the check found no fault.
    intact: bool,
    links: i32,
    dropped: i32,
    head_seq: Option<i32>,
    head_hash: Option<String>,
    fault: Option<FaultNode>,
}

/// K1 `AuditChainFault`.
#[derive(SimpleObject)]
#[graphql(name = "AuditChainFault")]
struct FaultNode {
    kind: FaultKindNode,
    seq: Option<i32>,
    entry_id: ID,
    at: DateTime<Utc>,
}

/// K1 `AuditChainFaultKind`.
#[derive(Enum, Clone, Copy, PartialEq, Eq)]
#[graphql(name = "AuditChainFaultKind")]
enum FaultKindNode {
    Changed,
    Missing,
    Broken,
    NotLinked,
}

/// A count or a sequence number that does not fit a GraphQL `Int`.
#[derive(Debug, thiserror::Error)]
#[error("an audit chain count does not fit a GraphQL Int")]
pub(in crate::fleet::events) struct TooLarge;

fn int(value: impl TryInto<i32>) -> Result<i32, TooLarge> {
    value.try_into().map_err(|_| TooLarge)
}

impl TryFrom<ChainCheck> for ChainCheckNode {
    type Error = TooLarge;

    fn try_from(check: ChainCheck) -> Result<Self, Self::Error> {
        let fault = check.fault.map(FaultNode::try_from).transpose()?;
        Ok(Self {
            intact: fault.is_none(),
            links: int(check.links)?,
            dropped: int(check.dropped)?,
            head_seq: check.head.map(|(seq, _)| int(seq)).transpose()?,
            head_hash: check.head.map(|(_, hash)| hex(&hash)),
            fault,
        })
    }
}

impl TryFrom<Fault> for FaultNode {
    type Error = TooLarge;

    fn try_from(fault: Fault) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: match fault.kind {
                FaultKind::Changed => FaultKindNode::Changed,
                FaultKind::Missing => FaultKindNode::Missing,
                FaultKind::Broken => FaultKindNode::Broken,
                FaultKind::NotLinked => FaultKindNode::NotLinked,
            },
            seq: fault.seq.map(int).transpose()?,
            entry_id: ID(fault.entry_id.to_string()),
            at: fault.at,
        })
    }
}
