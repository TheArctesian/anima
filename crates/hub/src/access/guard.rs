//! Refusing GraphQL requests without a signed-in operator (API-003): the field guard and the
//! schema-wide backstop for a resolver that forgets the guard.

use std::{
    future::{Future, ready},
    sync::Arc,
};

use async_graphql::{
    Context, Guard, Pos, ServerResult, Variables,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextParseQuery},
    parser::types::{DocumentOperations, ExecutableDocument, OperationDefinition, OperationType, Selection},
};

use super::{Error, Identity};
use crate::clock::Clock;

/// GraphQL guard: requires a valid token; puts `Operator` in the context (API-003).
///
/// A guard cannot add context data (D-6): the HTTP and WebSocket edges resolve the bearer
/// token and attach an [`Identity`] with its `Operator`. This guard passes only when the
/// identity is there and has not expired by the context's `Arc<dyn Clock>`, and refuses with
/// `UNAUTHENTICATED` otherwise. An `Operator` alone does not pass.
pub struct Authenticated;

impl Guard for Authenticated {
    fn check(&self, ctx: &Context<'_>) -> impl Future<Output = async_graphql::Result<()>> + Send {
        let verdict = if signed_in(ctx.data_opt(), ctx.data_opt()) {
            Ok(())
        } else {
            Err(Error::Unauthenticated.to_graphql())
        };
        ready(verdict)
    }
}

/// Whether the request data holds an identity that is valid now. Without a clock nothing is
/// valid: the check fails closed.
fn signed_in(identity: Option<&Identity>, clock: Option<&Arc<dyn Clock>>) -> bool {
    identity.zip(clock).is_some_and(|(identity, clock)| identity.valid_at(clock.now()))
}

/// Schema extension: refuses, with `UNAUTHENTICATED`, any operation without a valid identity
/// unless every root selection is `login` or `__typename` (API-003 backstop). Introspection
/// (`__schema`, `__type`) is refused without an identity too.
pub struct SignInRequired;

impl ExtensionFactory for SignInRequired {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(SignInRequired)
    }
}

#[async_graphql::async_trait::async_trait]
impl Extension for SignInRequired {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let document = next.run(ctx, query, variables).await?;
        if signed_in(ctx.data_opt(), ctx.data_opt()) || anonymous_allowed(&document) {
            Ok(document)
        } else {
            Err(Error::Unauthenticated
                .to_graphql()
                .into_server_error(Pos::default()))
        }
    }
}

/// Whether every operation in the document selects only `__typename`, or `login` in a
/// mutation, at its root. A fragment at the root is refused rather than followed.
fn anonymous_allowed(document: &ExecutableDocument) -> bool {
    match &document.operations {
        DocumentOperations::Single(operation) => only_login(&operation.node),
        DocumentOperations::Multiple(operations) => operations
            .values()
            .all(|operation| only_login(&operation.node)),
    }
}

fn only_login(operation: &OperationDefinition) -> bool {
    let mutation = operation.ty == OperationType::Mutation;
    operation
        .selection_set
        .node
        .items
        .iter()
        .all(|selection| match &selection.node {
            Selection::Field(field) => {
                let name = field.node.name.node.as_str();
                name == "__typename" || (mutation && name == "login")
            }
            Selection::FragmentSpread(_) | Selection::InlineFragment(_) => false,
        })
}
