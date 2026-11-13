//! The pages that `Query.events` and `Query.audit` return, and the K1 `Event` and `AuditEntry`
//! types (API-006, LOG-004, LOG-005).

use async_graphql::{ID, Json, SimpleObject};
use chrono::{DateTime, Utc};

use crate::{
    fleet::events::{
        actor::{ActorKind, TokenKind},
        model::{AuditRecord, EventSummary, Severity, Source},
        page::{Page, Position, encode_cursor},
        retention::Log,
    },
    paging::PageEnd,
    scalars::IpAddr,
};

/// K1 `Event`.
#[derive(SimpleObject)]
#[graphql(name = "Event")]
pub(in crate::fleet::events) struct EventNode {
    id: ID,
    at: DateTime<Utc>,
    severity: Severity,
    source: Source,
    device_id: Option<ID>,
    /// The device name when the event happened; it stays after a rename or delete.
    device_name: Option<String>,
    message: String,
    /// Who caused the event; null for an event written before LOG-013.
    actor_kind: Option<ActorKind>,
}

#[cfg(test)]
impl EventNode {
    pub(in crate::fleet::events) fn text(&self) -> &str {
        &self.message
    }
}

impl From<EventSummary> for EventNode {
    fn from(event: EventSummary) -> Self {
        let (device_id, device_name) = event
            .device
            .map(|(id, name)| (ID(id.to_string()), name))
            .unzip();
        Self {
            id: ID(event.id.to_string()),
            at: event.at,
            severity: event.severity,
            source: event.source,
            device_id,
            device_name,
            message: event.message,
            actor_kind: event.actor,
        }
    }
}

#[derive(SimpleObject)]
struct EventEdge {
    cursor: String,
    node: EventNode,
}

/// K1 `EventConnection`.
#[derive(SimpleObject)]
pub(super) struct EventConnection {
    edges: Vec<EventEdge>,
    page_info: PageEnd,
}

impl From<Page<EventSummary>> for EventConnection {
    fn from(page: Page<EventSummary>) -> Self {
        let edges: Vec<_> = page
            .rows
            .into_iter()
            .map(|event| EventEdge {
                cursor: encode_cursor(
                    Log::Events,
                    Position {
                        at: event.at,
                        id: event.id.into(),
                    },
                ),
                node: event.into(),
            })
            .collect();
        let end_cursor = edges.last().map(|edge| edge.cursor.clone());
        Self {
            edges,
            page_info: PageEnd {
                has_next_page: page.has_next_page,
                end_cursor,
            },
        }
    }
}

/// K1 `AuditEntry`.
#[derive(SimpleObject)]
#[graphql(name = "AuditEntry")]
struct AuditNode {
    id: ID,
    at: DateTime<Utc>,
    operator_name: String,
    action: String,
    target_type: String,
    target_id: String,
    client_addr: IpAddr,
    /// Changed fields. Secret values are replaced with "[redacted]".
    change: Json<serde_json::Value>,
    /// Who did it; null for an entry written before LOG-012.
    actor_kind: Option<ActorKind>,
    /// The token the request signed in with; null for a Hub entry and an old entry (LOG-018).
    token_kind: Option<TokenKind>,
    /// The name of the API key, for an API key only; never the key itself (LOG-018).
    api_key_name: Option<String>,
}

#[derive(SimpleObject)]
struct AuditEdge {
    cursor: String,
    node: AuditNode,
}

/// K1 `AuditConnection`.
#[derive(SimpleObject)]
pub(super) struct AuditConnection {
    edges: Vec<AuditEdge>,
    page_info: PageEnd,
}

impl From<Page<AuditRecord>> for AuditConnection {
    fn from(page: Page<AuditRecord>) -> Self {
        let edges: Vec<_> = page
            .rows
            .into_iter()
            .map(|entry| AuditEdge {
                cursor: encode_cursor(
                    Log::Audit,
                    Position {
                        at: entry.at,
                        id: entry.id,
                    },
                ),
                node: AuditNode {
                    id: ID(entry.id.to_string()),
                    at: entry.at,
                    operator_name: entry.operator_name,
                    action: entry.action,
                    target_type: entry.target_type,
                    target_id: entry.target_id,
                    client_addr: IpAddr(entry.client),
                    change: Json(entry.change),
                    actor_kind: entry.actor_kind,
                    token_kind: entry.token_kind,
                    api_key_name: entry.api_key_name,
                },
            })
            .collect();
        let end_cursor = edges.last().map(|edge| edge.cursor.clone());
        Self {
            edges,
            page_info: PageEnd {
                has_next_page: page.has_next_page,
                end_cursor,
            },
        }
    }
}
