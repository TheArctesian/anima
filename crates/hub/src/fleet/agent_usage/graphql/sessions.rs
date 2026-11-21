//! `Query.agentSessions`: the single sessions of the agent programs (AIS-025, AIS-026).

use std::{collections::HashMap, sync::Arc};

use async_graphql::{Context, ID, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};

use super::{AgentSource, AgentUsageFilter, ask, usage_error};
use crate::{
    api_errors::{FieldError, internal, invalid, pool, schema_dependency},
    clock::Clock,
    fleet::{
        access::Authenticated,
        agent_usage::{Session, sessions},
        devices::{self, Device},
    },
    ids::DeviceId,
};

/// The most sessions one answer may list.
const MAX_FIRST: i32 = 200;

/// K1 `AgentSession`: one session with its sums over the days asked for.
#[derive(SimpleObject)]
pub struct AgentSession {
    /// The hub's ID of the session, stable while the hub keeps it.
    id: ID,
    device: Device,
    source: AgentSource,
    model: Option<String>,
    repository: Option<String>,
    started_at: DateTime<Utc>,
    last_use_at: DateTime<Utc>,
    /// The last use is at most 30 minutes before the clock of the hub (AIS-026).
    recent: bool,
    prompts: f64,
    tool_calls: f64,
    input_tokens: f64,
    output_tokens: f64,
    cache_read_tokens: f64,
    cache_write_tokens: f64,
    estimated_cost_usd: Option<f64>,
    unpriced_tokens: f64,
}

/// K1 `AgentSessions`.
#[derive(SimpleObject)]
pub struct AgentSessions {
    first_day: String,
    last_day: String,
    sessions: Vec<AgentSession>,
    sessions_left_out: i32,
}

impl AgentSession {
    fn of(device: Device, session: Session, now: DateTime<Utc>) -> Self {
        let stored = session.stored;
        Self {
            id: ID(stored.id.to_string()),
            device,
            source: stored.source.into(),
            model: stored.model,
            repository: stored.repository,
            started_at: stored.started_at,
            last_use_at: stored.last_use_at,
            recent: sessions::is_recent(stored.last_use_at, now),
            prompts: stored.prompts,
            tool_calls: stored.tool_calls,
            input_tokens: session.tokens.input,
            output_tokens: session.tokens.output,
            cache_read_tokens: session.tokens.cache_read,
            cache_write_tokens: session.tokens.cache_write,
            estimated_cost_usd: session.estimated_cost_usd,
            unpriced_tokens: session.unpriced_tokens,
        }
    }
}

#[derive(Default)]
pub struct SessionsQuery;

#[Object(name = "AgentSessionsQuery")]
impl SessionsQuery {
    /// The single sessions that the filter admits, the newest use first, each with its sums
    /// over the days of the filter (AIS-025) and whether its use is recent (AIS-026).
    #[graphql(guard = "Authenticated")]
    async fn agent_sessions(
        &self,
        ctx: &Context<'_>,
        filter: Option<AgentUsageFilter>,
        #[graphql(default = 50)] first: i32,
    ) -> Result<AgentSessions> {
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let mut bad = Vec::new();
        if !(1..=MAX_FIRST).contains(&first) {
            bad.push(FieldError::new("first", "Ask for 1 to 200 sessions."));
        }
        let asked = ask(filter.unwrap_or_default(), &[], now.date_naive());
        let ask = match asked {
            Ok(ask) if bad.is_empty() => ask,
            Ok(_) => return Err(invalid(&bad)),
            Err(more) => {
                bad.extend(more);
                return Err(invalid(&bad));
            }
        };
        let limit = usize::try_from(first).unwrap_or(0);
        let db = pool(ctx)?;
        let list = super::super::sessions(db, &ask, limit)
            .await
            .map_err(usage_error)?;
        let devices: HashMap<DeviceId, Device> = devices::list(db)
            .await
            .map_err(internal)?
            .into_iter()
            .map(|device| (device.id, device))
            .collect();
        let shown: Vec<AgentSession> = list
            .sessions
            .into_iter()
            .filter_map(|session| {
                let device = devices.get(&session.stored.device)?.clone();
                Some(AgentSession::of(device, session, now))
            })
            .collect();
        let left_out = list.admitted.saturating_sub(i64::from(first)).max(0);
        Ok(AgentSessions {
            first_day: ask.first_day.to_string(),
            last_day: ask.last_day.to_string(),
            sessions: shown,
            sessions_left_out: i32::try_from(left_out).unwrap_or(i32::MAX),
        })
    }
}
