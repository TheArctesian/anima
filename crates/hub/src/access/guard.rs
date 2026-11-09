//! Refusing GraphQL requests without a signed-in operator (API-003): the field guard and the
//! schema-wide backstop for a resolver that forgets the guard.

use std::{
    future::{Future, ready},
    sync::{Arc, OnceLock},
};

use async_graphql::{
    Context, Guard, Positioned, Response, ServerError, ServerResult, ValidationResult, Variables,
    extensions::{
        Extension, ExtensionContext, ExtensionFactory, NextParseQuery, NextRequest, NextValidation,
    },
    parser::types::{
        DocumentOperations, ExecutableDocument, OperationDefinition, OperationType, Selection,
    },
};

use super::{Error, Identity, Scope};
use crate::clock::Clock;

/// The root mutations a builder token may call, and only it (BLD-022, D-1). `access` keeps the
/// list because the backstop below refuses every other root field to a builder; a `builds` test
/// keeps it equal to the fields of `builds::graphql::BuilderMutation`.
pub const BUILDER_FIELDS: [&str; 3] = ["claimBuildJob", "reportBuildJob", "reportBuildUploaded"];

/// The refusal a builder token gets for an operator field.
const BUILDER_ONLY: &str = "A builder token may only claim and report build jobs.";

/// The refusal an operator token gets for a builder field.
const NOT_BUILDER: &str = "Only a builder token may do this.";

/// GraphQL guard: requires a valid operator token; puts `Operator` in the context (API-003).
///
/// A guard cannot add context data (D-6): the HTTP and WebSocket edges resolve the bearer
/// token and attach an [`Identity`] with its `Operator`. This guard passes only when the
/// identity is there, has not expired by the context's `Arc<dyn Clock>` and is not a builder
/// token, and refuses with `UNAUTHENTICATED` otherwise (`FORBIDDEN` for a builder token,
/// BLD-022). An `Operator` alone does not pass. A schema without a clock refuses every identity
/// with `INTERNAL` and writes the fault (`missing_context`) to the process log: the hub is
/// miswired, not the operator signed out.
pub struct Authenticated;

impl Guard for Authenticated {
    fn check(&self, ctx: &Context<'_>) -> impl Future<Output = async_graphql::Result<()>> + Send {
        ready(holder_verdict(ctx, Holder::Operator))
    }
}

/// GraphQL guard of the builder fields: requires a valid `builder` token and refuses an
/// operator token with `FORBIDDEN` (BLD-022, D-1). The field still checks the builder record and
/// its address (BLD-024).
pub struct BuilderSignedIn;

impl Guard for BuilderSignedIn {
    fn check(&self, ctx: &Context<'_>) -> impl Future<Output = async_graphql::Result<()>> + Send {
        ready(holder_verdict(ctx, Holder::Builder))
    }
}

/// Who a guard lets through.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Holder {
    Operator,
    Builder,
}

/// A guard's answer: the identity must be valid now and its scope must belong to `wanted`.
fn holder_verdict(ctx: &Context<'_>, wanted: Holder) -> async_graphql::Result<()> {
    let identity = ctx.data_opt::<Identity>();
    let clock = ctx.data_opt::<Arc<dyn Clock>>();
    if identity.is_some() && clock.is_none() {
        let error = Error::Unavailable;
        error.log();
        return Err(error.into_graphql());
    }
    let Some(identity) = current(identity, clock) else {
        return Err(Error::Unauthenticated.into_graphql());
    };
    match (wanted, identity.scope() == Scope::Builder) {
        (Holder::Operator, true) => Err(Error::OutOfScope(BUILDER_ONLY).into_graphql()),
        (Holder::Builder, false) => Err(Error::OutOfScope(NOT_BUILDER).into_graphql()),
        _ => Ok(()),
    }
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
///   `__typename`, or is a mutation with exactly one root field, which is `login`,
///   `signInBrowser` or `joinShare` (D-4, WP-43 A-7). Anything else (other fields, two of
///   them, introspection, root fragments) gets one `UNAUTHENTICATED` error and nothing else;
/// - in `validation`, an allowed document that fails validation gets one `UNAUTHENTICATED`
///   error in place of all its validation errors. The root check does not look at fragment
///   definitions, variable types or directives, and their validation errors would name
///   schema types and fields, so a signed-out client cannot map the schema through them
///   (T3 security SHOULD 1). A sign-in whose argument cannot be coerced while resolving is
///   still answered with the fixed `VALIDATION` text of the API boundary. `joinShare` has no
///   resolver until WP-47, so today it always fails validation here;
/// - in `request`, a response holding an error with neither a path nor a code (a syntax
///   error, an `operationName` the document lacks) becomes the same one `UNAUTHENTICATED`
///   (T8 round 2).
///
/// Accepted (issue #1): signed out, a valid `login` or `signInBrowser` document runs, so its
/// sub-selection and input fields can be told apart from invalid ones, one throttled attempt per
/// probe.
///
/// With a `read` token, a mutation that selects more than `login`, `logout` and
/// `__typename` is refused with `FORBIDDEN`. With a `builder` token, a document that selects at
/// its root anything but the builder mutations and `__typename` is refused with `FORBIDDEN`;
/// with an operator token, a mutation that selects a builder mutation at its root is too
/// (BLD-022).
pub struct SignInRequired;

impl ExtensionFactory for SignInRequired {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(SignInCheck::default())
    }
}

/// One request's backstop. Whether it came without a valid identity is recorded when its
/// document is parsed: the `request` hook cannot read the request's data.
/// It covers `execute` only: `execute_stream` never calls the `request` hook. A subscription
/// needs a signed-in `connection_init` first, and `InternalErrors::subscribe` masks its texts.
#[derive(Default)]
struct SignInCheck {
    signed_out: OnceLock<bool>,
}

/// Whether the error came from neither a resolver nor a refusal: no path and no code.
fn unexplained(error: &ServerError) -> bool {
    error.path.is_empty()
        && error
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.get("code"))
            .is_none()
}

#[async_graphql::async_trait::async_trait]
impl Extension for SignInCheck {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let response = next.run(ctx).await;
        if self.signed_out.get() == Some(&true) && response.errors.iter().any(unexplained) {
            let mut refused =
                Response::from_errors(vec![Error::Unauthenticated.into_response_error()]);
            refused.http_headers = response.http_headers;
            return refused;
        }
        response
    }

    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let identity = current(ctx.data_opt(), ctx.data_opt());
        let _ = self.signed_out.set(identity.is_none());
        let document = next.run(ctx, query, variables).await?;
        let refusal = match identity.map(Identity::scope) {
            None => (!signed_out_allowed(&document)).then_some(Error::Unauthenticated),
            Some(Scope::Builder) => (!roots_only(&document, NO_ROOTS, BUILDER_ROOTS))
                .then_some(Error::OutOfScope(BUILDER_ONLY)),
            Some(_) if selects_any(&document, &BUILDER_FIELDS) => {
                Some(Error::OutOfScope(NOT_BUILDER))
            }
            Some(Scope::Read)
                if !roots_only(&document, Roots::Any, Roots::Only(&["login", "logout"])) =>
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

/// The mutations a signed-out client may run, one per operation (API-003, D-4, A-7).
const SIGNED_OUT_MUTATIONS: [&str; 3] = ["login", "signInBrowser", "joinShare"];

/// Whether a document may run without an identity: each operation selects only
/// `__typename`, or is a mutation whose one root selection is a field in
/// [`SIGNED_OUT_MUTATIONS`].
fn signed_out_allowed(document: &ExecutableDocument) -> bool {
    operations(document).all(|operation| {
        let items = &operation.selection_set.node.items;
        let typename_only = items
            .iter()
            .all(|item| field_name(item) == Some("__typename"));
        let one_sign_in = operation.ty == OperationType::Mutation
            && items.len() == 1
            && items.iter().all(|item| {
                field_name(item).is_some_and(|name| SIGNED_OUT_MUTATIONS.contains(&name))
            });
        typename_only || one_sign_in
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

/// Only `__typename`.
const NO_ROOTS: Roots<'static> = Roots::Only(&[]);
/// The builder mutations and `__typename`.
const BUILDER_ROOTS: Roots<'static> = Roots::Only(&BUILDER_FIELDS);

/// Whether any operation in the document names one of `names` as a root field. A root fragment
/// is not followed: the field guards refuse what it hides.
fn selects_any(document: &ExecutableDocument, names: &[&str]) -> bool {
    operations(document).any(|operation| {
        (operation.selection_set.node.items.iter())
            .any(|selection| field_name(selection).is_some_and(|name| names.contains(&name)))
    })
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
