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

use super::{Error, Identity, Scope, TokenKind};
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
/// With an identity, also in `parse_query` and before any resolver, each with `FORBIDDEN`:
/// - a root selection its token kind may not make ([`kind_roots`]): for an API key, any field
///   in [`API_KEY_REFUSED_ROOTS`] or any root fragment, in any operation of the document, by
///   field name whatever the alias (D-23, WP-43 A-6);
/// - with a `read` token, any mutation field other than `__typename` (D-23).
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
        match refusal(identity, &document) {
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

/// Why the document may not run for this identity, if it may not: signed out, a root its
/// token kind may not select, or a mutation with a `read` token.
fn refusal(identity: Option<&Identity>, document: &ExecutableDocument) -> Option<Error> {
    let Some(identity) = identity else {
        return (!signed_out_allowed(document)).then_some(Error::Unauthenticated);
    };
    let by_kind = kind_roots(identity.kind());
    if !roots_admitted(document, by_kind, by_kind) {
        return Some(Error::KindRefused);
    }
    let read_only = identity.scope() == Scope::Read;
    (read_only && !roots_admitted(document, Roots::Any, Roots::Only(&[])))
        .then_some(Error::ReadOnly)
}

/// The root fields an API key may never select, in any operation (D-23, WP-43 A-6): managing
/// keys, signing a browser in or out, sessions, files, shares, recordings and device
/// credentials. `logout` would delete the key, which is managing it. `enrolAgent` and
/// `revokeAgentToken` are refused by name before their resolvers exist (R-084, CR-8).
const API_KEY_REFUSED_ROOTS: [&str; 18] = [
    "createApiKey",
    "revokeApiKey",
    "apiKeys",
    "signInBrowser",
    "logout",
    "startSession",
    "openFiles",
    "shareSession",
    "joinShare",
    "openRecording",
    "setSshCredential",
    "clearSshCredential",
    "setRdpCredential",
    "clearRdpCredential",
    "setVncCredential",
    "clearVncCredential",
    "enrolAgent",
    "revokeAgentToken",
];

/// The root selections a token of `kind` may make, in every operation type. A new kind of
/// token must state its own rule here.
const fn kind_roots(kind: TokenKind) -> Roots<'static> {
    match kind {
        TokenKind::Login | TokenKind::Browser => Roots::Any,
        TokenKind::ApiKey => Roots::AllBut(&API_KEY_REFUSED_ROOTS),
    }
}

/// Whether a token of `kind` may select the root field `name`: the rule of [`kind_roots`], for
/// a resolver that checks it again behind the backstop.
pub(super) fn kind_may_select(kind: TokenKind, name: &str) -> bool {
    kind_roots(kind).admits(name)
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
    /// Any field but these; a fragment is refused rather than followed, so it cannot hide one.
    AllBut(&'a [&'a str]),
}

impl Roots<'_> {
    /// Whether a root field of this name may be selected.
    fn admits(self, name: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Only(names) => name == "__typename" || names.contains(&name),
            Self::AllBut(names) => !names.contains(&name),
        }
    }

    /// Whether a root fragment may be selected: only where any field may be.
    const fn admits_fragment(self) -> bool {
        matches!(self, Self::Any)
    }
}

/// Whether every operation in the document selects at its root only what its rule admits:
/// `queries` for query and subscription operations, `mutations` for mutations.
fn roots_admitted(document: &ExecutableDocument, queries: Roots<'_>, mutations: Roots<'_>) -> bool {
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
            .all(|selection| match &selection.node {
                Selection::Field(field) => roots.admits(field.node.name.node.as_str()),
                Selection::FragmentSpread(_) | Selection::InlineFragment(_) => {
                    roots.admits_fragment()
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
