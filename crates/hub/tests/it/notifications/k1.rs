//! The notifications part and K1 agree where they share a declaration with another part
//! (CON-002; the exact schema comparison is `contract`).

use anima_hub::fleet::{checks, notifications};
use async_graphql::{EmptySubscription, Schema};

use crate::contract::{k1, schema};

/// Both parts declare `HttpHeaderInput`; the hub schema keeps whichever registers first and
/// ignores the other (`ignore_name_conflicts`), which is safe only while the two declare the
/// same fields.
#[test]
fn k1_the_notifications_and_checks_header_inputs_declare_the_same_fields() {
    let ours = schema(
        &Schema::new(
            notifications::graphql::Query,
            notifications::graphql::Mutation,
            EmptySubscription,
        )
        .sdl(),
    );
    let theirs = schema(
        &Schema::new(
            checks::graphql::Query,
            checks::graphql::Mutation,
            checks::graphql::Subscription,
        )
        .sdl(),
    );
    let name = "HttpHeaderInput";
    assert_eq!(ours[name], theirs[name]);
    assert_eq!(ours[name].members, k1()[name].members);
}
