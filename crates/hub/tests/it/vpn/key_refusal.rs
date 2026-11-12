//! The VPN mutation resolvers check the token kind themselves, so an API key is refused even
//! on a schema without the `SignInRequired` backstop (D-23, VP-7).

use std::sync::Arc;

use anima_hub::{
    access::Identity,
    clock::{Clock, FixedClock},
    events::Operator,
    ids::OperatorId,
    vpn::graphql::{Mutation, RouteMutation},
};
use async_graphql::{EmptySubscription, MergedObject, Object, Request, Schema};
use chrono::TimeDelta;

use crate::hub::start;

#[derive(Default)]
struct Root;

#[Object]
impl Root {
    async fn ok(&self) -> bool {
        true
    }
}

#[derive(MergedObject, Default)]
struct VpnMutations(Mutation, RouteMutation);

const DOCUMENTS: [&str; 5] = [
    "mutation { createVpnInvite { expiresAt } }",
    "mutation { createRouterKey(siteId: \"0190a000-0000-7000-8000-0000000000aa\") { expiresAt } }",
    "mutation { registerVpnNode(authId: \"hskey-authreq-AAAAAAAAAAAA-_BBBBBBBBBB\") { name } }",
    "mutation { renewVpnKey(name: \"mac\") { name } }",
    "mutation { approveSiteRoute(siteId: \"0190a000-0000-7000-8000-0000000000aa\") }",
];

/// The code of the one error `identity` gets for `document` on a schema with the VPN
/// mutations, a clock and nothing else: no backstop, no broker, no database.
async fn code_for(identity: Identity, document: &str) -> String {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let schema = Schema::build(Root, VpnMutations::default(), EmptySubscription)
        .data(clock)
        .finish();
    let response = schema
        .execute(identity.attach(Request::new(document)))
        .await;
    let [error] = response.errors.as_slice() else {
        panic!("{document}: expected one error, got {:?}", response.errors);
    };
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    code.map(ToString::to_string).unwrap_or_default()
}

#[tokio::test]
async fn d_23_the_vpn_resolvers_refuse_an_api_key_without_the_backstop() {
    let operator = || Operator {
        id: OperatorId::from(uuid::Uuid::from_u128(1)),
        name: "operator".to_owned(),
    };
    let until = start() + TimeDelta::days(1);
    for document in DOCUMENTS {
        let key = code_for(Identity::api_key_for_tests(operator(), until), document).await;
        assert_eq!(key, "\"FORBIDDEN\"", "{document}");
        let login = code_for(Identity::for_tests(operator(), until), document).await;
        assert_ne!(login, "\"FORBIDDEN\"", "a login token passes: {document}");
    }
}
