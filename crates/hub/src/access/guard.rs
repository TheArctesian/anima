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
/// - without a valid identity, an operation may select exactly one root field, `login` in a
///   mutation; anything else (`__typename`, introspection, fragments) is refused with
///   `UNAUTHENTICATED`, and several `login` fields with `VALIDATION`, before any resolver runs;
/// - with a `read` token, a mutation that selects more than `login`, `logout` and
///   `__typename` is refused with `FORBIDDEN`.
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
            None => signed_out_refusal(&document),
            Some(identity)
                if identity.scope() == Scope::Read
                    && !roots_only(&document, Roots::Any, Roots::Only(&["login", "logout"])) =>
            {
                Some(Error::ReadOnly)
            }
            Some(_) => None,
        };
        match refusal {
            Some(error) => Err(error.to_graphql().into_server_error(Pos::default())),
            None => Ok(document),
        }
    }
}

/// Why a document without an identity is refused, if it is: each operation must be a
/// mutation whose only root selection is one `login` field.
fn signed_out_refusal(document: &ExecutableDocument) -> Option<Error> {
    let mut logins = Vec::new();
    for operation in operations(document) {
        let only_logins = operation.ty == OperationType::Mutation
            && operation.selection_set.node.items.iter().all(|selection| {
                matches!(&selection.node, Selection::Field(field) if field.node.name.node == "login")
            });
        if !only_logins {
            return Some(Error::Unauthenticated);
        }
        logins.push(operation.selection_set.node.items.len());
    }
    logins
        .iter()
        .any(|&count| count != 1)
        .then_some(Error::Validation {
            field: "login",
            message: "Send one login at a time.",
        })
}

/// The root fields an operation may select, besides `__typename`.
#[derive(Clone, Copy)]
enum Roots<'a> {
    /// Any field, and fragments too.
    Any,
    /// Only these fields; a fragment is refused rather than followed.
    Only(&'a [&'a str]),
}

/// Whether every operation in the document selects at its root only `__typename` and the
/// allowed fields: `queries` for query and subscription operations, `mutations` for mutations.
fn roots_only(document: &ExecutableDocument, queries: Roots<'_>, mutations: Roots<'_>) -> bool {
    operations(document).all(|operation| {
        let roots = if operation.ty == OperationType::Mutation {
            mutations
        } else {
            queries
        };
        operation
            .selection_set
            .node
            .items
            .iter()
            .all(|selection| match (&selection.node, roots) {
                (_, Roots::Any) => true,
                (Selection::Field(field), Roots::Only(names)) => {
                    let name = field.node.name.node.as_str();
                    name == "__typename" || names.contains(&name)
                }
                (Selection::FragmentSpread(_) | Selection::InlineFragment(_), Roots::Only(_)) => {
                    false
                }
            })
    })
}

/// Every operation in the document, named or not.
fn operations(document: &ExecutableDocument) -> impl Iterator<Item = &OperationDefinition> {
    let (single, multiple) = match &document.operations {
        DocumentOperations::Single(operation) => (Some(&operation.node), None),
        DocumentOperations::Multiple(operations) => (None, Some(operations.values())),
    };
    single.into_iter().chain(
        multiple
            .into_iter()
            .flatten()
            .map(|operation| &operation.node),
    )
}
