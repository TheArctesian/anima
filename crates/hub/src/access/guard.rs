//! Refusing GraphQL requests without a signed-in operator (API-003): the field guard and the
//! schema-wide backstop for a resolver that forgets the guard.

use std::{
    future::{Future, ready},
    sync::Arc,
};

use async_graphql::{
    Context, Guard, Pos, ServerResult, Variables,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextParseQuery},
    parser::types::{
        DocumentOperations, ExecutableDocument, OperationDefinition, OperationType, Selection,
    },
};

use super::{Error, Identity, Scope};
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
    current(identity, clock).is_some()
}

/// The identity, if it is valid now by the clock.
fn current<'a>(
    identity: Option<&'a Identity>,
    clock: Option<&Arc<dyn Clock>>,
) -> Option<&'a Identity> {
    identity
        .zip(clock)
        .filter(|(identity, clock)| identity.valid_at(clock.now()))
        .map(|(identity, _)| identity)
}

/// Schema extension (API-003 backstop):
/// - without a valid identity, refuses with `UNAUTHENTICATED` any operation unless every
///   root selection is `__typename`, or `login` in a mutation. Introspection (`__schema`,
///   `__type`) is refused too;
/// - with a `read` token, refuses with `FORBIDDEN` any mutation that selects more than
///   `login`, `logout` and `__typename`.
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
        let refusal = match current(ctx.data_opt(), ctx.data_opt()) {
            None if !roots_only(&document, Roots::Only(&[]), Roots::Only(&["login"])) => {
                Some(Error::Unauthenticated)
            }
            Some(identity)
                if identity.scope() == Scope::Read
                    && !roots_only(&document, Roots::Any, Roots::Only(&["login", "logout"])) =>
            {
                Some(Error::ReadOnly)
            }
            _ => None,
        };
        match refusal {
            Some(error) => Err(error.to_graphql().into_server_error(Pos::default())),
            None => Ok(document),
        }
    }
}

/// The root fields an operation may select, besides `__typename`.
#[derive(Clone, Copy)]
enum Roots<'a> {
    Any,
    Only(&'a [&'a str]),
}

/// Whether every operation in the document selects at its root only `__typename` and the
/// listed fields: `queries` for query and subscription operations, `mutations` for mutations.
/// A fragment at the root is refused rather than followed.
fn roots_only(document: &ExecutableDocument, queries: Roots<'_>, mutations: Roots<'_>) -> bool {
    let allowed = |operation: &OperationDefinition| {
        let fields = if operation.ty == OperationType::Mutation {
            mutations
        } else {
            queries
        };
        operation
            .selection_set
            .node
            .items
            .iter()
            .all(|selection| match &selection.node {
                Selection::Field(field) => {
                    let name = field.node.name.node.as_str();
                    name == "__typename"
                        || match fields {
                            Roots::Any => true,
                            Roots::Only(names) => names.contains(&name),
                        }
                }
                Selection::FragmentSpread(_) | Selection::InlineFragment(_) => false,
            })
    };
    match &document.operations {
        DocumentOperations::Single(operation) => allowed(&operation.node),
        DocumentOperations::Multiple(operations) => operations
            .values()
            .all(|operation| allowed(&operation.node)),
    }
}
