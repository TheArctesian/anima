//! The sessions part of the hub declares exactly its K1 root fields (CON-002; the exact schema
//! comparison is `contract`). `acceptHostKey` and `acceptServerCertificate` are trust's
//! (`crate::trust::k1`).

use crate::contract::{Schema, k1, schema};
use anima_hub::fleet::sessions::{
    EndReason, Protocol,
    graphql::{Mutation, Query, Subscription},
};

/// The sessions part alone, for the root fields it declares. The SDL needs no schema data.
fn part() -> Schema {
    schema(&async_graphql::Schema::new(Query, Mutation, Subscription).sdl())
}

fn fields(schema: &Schema, root: &str) -> Vec<String> {
    schema[root].members.keys().cloned().collect()
}

#[tokio::test]
async fn k1_the_sessions_part_declares_exactly_its_root_fields_as_k1_does() {
    let (part, k1) = (part(), k1());
    assert_eq!(fields(&part, "SessionsQuery"), ["sessions"]);
    assert_eq!(
        fields(&part, "SessionsMutation"),
        ["endSession", "startSession", "testConnection"]
    );
    for (ours, theirs) in [("SessionsQuery", "Query"), ("SessionsMutation", "Mutation")] {
        for (field, signature) in &part[ours].members {
            assert_eq!(k1[theirs].members.get(field), Some(signature), "{field}");
        }
    }
}

#[test]
fn ra_008_every_k1_end_reason_is_stored_as_its_lower_snake_name() {
    let k1 = k1();
    let names: Vec<String> = k1["SessionEndReason"]
        .served_members()
        .map(|n| n.to_lowercase())
        .collect();
    let mut stored: Vec<String> = EndReason::ALL
        .iter()
        .map(|r| r.as_str().to_owned())
        .collect();
    stored.sort();
    assert_eq!(stored, names);
    for reason in EndReason::ALL {
        assert_eq!(reason.as_str().parse::<EndReason>(), Ok(reason));
    }
    assert!("crashed".parse::<EndReason>().is_err());
}

#[test]
fn ra_030_every_k1_protocol_is_stored_as_its_lower_case_name_and_has_its_tunnel() {
    let k1 = k1();
    let names: Vec<&String> = k1["Protocol"].served_members().collect();
    assert_eq!(names, ["RDP", "SSH", "VNC"]);
    for name in names {
        let stored = name.to_lowercase();
        let protocol = stored.parse::<Protocol>().unwrap();
        assert_eq!(protocol.as_str(), stored);
        let expected = if protocol == Protocol::Ssh {
            "/terminal"
        } else {
            "/tunnel"
        };
        assert_eq!(
            protocol.tunnel_path(),
            expected,
            "K4 §1 tunnelPath of {name}"
        );
    }
    for text in ["SSH", "telnet", "", "rdp "] {
        assert!(text.parse::<Protocol>().is_err(), "{text:?}");
    }
}
