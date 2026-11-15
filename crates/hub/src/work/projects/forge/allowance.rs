//! How many times one GraphQL request may ask GitHub (K-E1 Section 13.1, "Request budget"):
//! a document can alias a list with its per-row reads many times over, so each request gets
//! its own count, and every read past it is refused with fixed words before anything is sent.
//!
//! Every read counts, a cache hit included: the same document always costs the same, whatever
//! the cache holds.

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use async_graphql::{
    Context, Request, Result, ServerResult,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextPrepareRequest},
};

use crate::{
    api_errors::{ErrorCode, coded, schema_dependency},
    fleet::access::{Identity, TokenKind},
};

/// An API key's caller is a script or an agent, which can page: less than a screen.
const KEY_READS: u32 = 60;
/// A login or browser token: the client's fullest screen, 50 pull requests with checks.
const SIGNED_IN_READS: u32 = 200;

/// The words of a read past the budget. No count: fixed words only.
const SPENT: &str = "This request asked GitHub too many times. Ask for less at once.";

/// Gives each GraphQL request, over HTTP or a WebSocket, its own count of reads.
pub struct RequestReads;

impl ExtensionFactory for RequestReads {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(Counting)
    }
}

struct Counting;

/// The reads one request has asked so far.
#[derive(Default)]
struct Asked(AtomicU32);

#[async_graphql::async_trait::async_trait]
impl Extension for Counting {
    async fn prepare_request(
        &self,
        ctx: &ExtensionContext<'_>,
        request: Request,
        next: NextPrepareRequest<'_>,
    ) -> ServerResult<Request> {
        next.run(ctx, request.data(Asked::default())).await
    }
}

/// The most reads a request with `kind` may ask; an unknown kind gets the smaller budget.
const fn most(kind: Option<TokenKind>) -> u32 {
    match kind {
        Some(TokenKind::Login | TokenKind::Browser) => SIGNED_IN_READS,
        _ => KEY_READS,
    }
}

/// Counts one read of this request.
///
/// # Errors
/// `UNAVAILABLE` with fixed words once the request has asked its budget; `INTERNAL` when the
/// request has no count (the schema lacks [`RequestReads`]).
pub(super) fn spend(ctx: &Context<'_>) -> Result<()> {
    let asked = schema_dependency::<Asked>(ctx)?;
    let most = most(ctx.data_opt::<Identity>().map(Identity::kind));
    // One count per read a resolver asks: the body, depth and complexity limits and the page
    // sizes keep a request far below `u32::MAX` of them, so the count cannot wrap.
    if asked.0.fetch_add(1, Ordering::Relaxed) < most {
        Ok(())
    } else {
        Err(coded(SPENT, ErrorCode::Unavailable))
    }
}
