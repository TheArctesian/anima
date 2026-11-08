//! The trust part of the hub declares exactly its K1 root fields (CON-002; the exact schema
//! comparison is `contract`).

use anima_hub::{sessions::graphql::Query, trust::graphql::Mutation};
use async_graphql::{EmptySubscription, Schema};

use crate::contract::{k1, schema};

#[test]
fn k1_the_trust_part_declares_exactly_its_root_fields_as_k1_does() {
    // A schema needs a query root; the sessions one stands in, and only TrustMutation is read.
    let part = schema(&Schema::new(Query, Mutation, EmptySubscription).sdl());
    let k1 = k1();
    let fields: Vec<String> = part["TrustMutation"].members.keys().cloned().collect();
    assert_eq!(fields, ["acceptHostKey", "acceptServerCertificate"]);
    for (field, signature) in &part["TrustMutation"].members {
        assert_eq!(
            k1["Mutation"].members.get(field),
            Some(signature),
            "{field}"
        );
    }
}
