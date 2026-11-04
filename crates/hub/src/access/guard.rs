//! Refusing GraphQL requests without a signed-in operator (API-003): the field guard and the
//! schema-wide backstop for a resolver that forgets the guard.

use std::{
    future::{Future, ready},
    sync::Arc,
};

use async_graphql::{
    Context, Guard, Positioned, ServerError, ServerResult, ValidationResult, Variables,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextParseQuery, NextValidation},
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
/// `UNAUTHENTICATED` otherwise. An `Operator` alone does not pass. A schema without a clock
/// refuses every identity with `INTERNAL` and writes the fault (`missing_context`) to the
/// process log: the hub is miswired, not the operator signed out.
pub struct Authenticated;

impl Guard for Authenticated {
    fn check(&self, ctx: &Context<'_>) -> impl Future<Output = async_graphql::Result<()>> + Send {
        let identity = ctx.data_opt::<Identity>();
        let clock = ctx.data_opt::<Arc<dyn Clock>>();
        let verdict = if identity.is_some() && clock.is_none() {
            let error = Error::Unavailable;
            error.log();
            Err(error.into_graphql())
        } else if signed_in(identity, clock) {
            Ok(())
        } else {
            Err(Error::Unauthenticated.into_graphql())
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

/// Schema extension (API-003 backstop, ruling H-6(a)). Without a valid identity:
/// - in `parse_query`, after the document is parsed and before validation, variable
///   coercion or any resolver, a document is allowed only when every operation selects just
///   `__typename`, or is a mutation with exactly one root field, `login`. Anything else
///   (other fields, several `login`s, introspection, root fragments) gets one
///   `UNAUTHENTICATED` error and nothing else;
/// - in `validation`, an allowed document that fails validation gets one `UNAUTHENTICATED`
///   error in place of all its validation errors. The root check does not look at fragment
///   definitions, variable types or directives, and their validation errors would name
///   schema types and fields, so a signed-out client cannot map the schema through them
///   (T3 security SHOULD 1). A `login` whose argument cannot be coerced while resolving is
///   still answered with the fixed `VALIDATION` text of the API boundary.
///
/// With a `read` token, a mutation that selects more than `login`, `logout` and
/// `__typename` is refused with `FORBIDDEN`.
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
            None => (!signed_out_allowed(&document)).then_some(Error::Unauthenticated),
            Some(identity)
                if identity.scope() == Scope::Read
                    && !roots_only(&document, Roots::Any, Roots::Only(&["login", "logout"])) =>
            {
                Some(Error::ReadOnly)
            }
            Some(_) => None,
        };
        match refusal {
            Some(error) => Err(error.into_response_error()),
            None => Ok(document),
        }
    }

    async fn validation(
        &self,
        ctx: &ExtensionContext<'_>,
        next: NextValidation<'_>,
    ) -> Result<ValidationResult, Vec<ServerError>> {
        let result = next.run(ctx).await;
        if result.is_err() && current(ctx.data_opt(), ctx.data_opt()).is_none() {
            return Err(vec![Error::Unauthenticated.into_response_error()]);
        }
        result
    }
}

/// Whether a document may run without an identity: each operation selects only
/// `__typename`, or is a mutation whose one root selection is a `login` field.
fn signed_out_allowed(document: &ExecutableDocument) -> bool {
    operations(document).all(|operation| {
        let items = &operation.selection_set.node.items;
        let typename_only = items
            .iter()
            .all(|item| field_name(item) == Some("__typename"));
        let one_login = operation.ty == OperationType::Mutation
            && items.len() == 1
            && items.iter().all(|item| field_name(item) == Some("login"));
        typename_only || one_login
    })
}

/// The field a selection names; `None` for a fragment.
fn field_name(selection: &Positioned<Selection>) -> Option<&str> {
    match &selection.node {
        Selection::Field(field) => Some(field.node.name.node.as_str()),
        Selection::FragmentSpread(_) | Selection::InlineFragment(_) => None,
    }
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
