//! The declaration offers the changes and says which permission each needs (SRC-010, OD-E14).

use source_exchange::{Body, ChangeReply, Limit, Receipt, Settled};

use super::every_change;
use crate::DECLARATION;

#[test]
fn od_e14_github_declares_its_changes() {
    let changes = DECLARATION.changes.expect("GitHub makes changes");
    let comment = &every_change()[1];
    assert_eq!(
        (changes.render)(comment).path(),
        "/repos/octo-org/hello.world/issues/1347/comments"
    );
    let reply = ChangeReply {
        status: 201,
        body: Body::Read(br#"{"id":9}"#),
        limit: Limit::default(),
        at: chrono::DateTime::UNIX_EPOCH,
    };
    assert_eq!(
        (changes.settle)(comment, &reply),
        Settled::Done(Receipt::Created { number: 9 })
    );
}

#[test]
fn src_010_the_declaration_names_each_write_permission_and_why() {
    let scopes = DECLARATION.scopes.join("\n");
    for scope in [
        "Issues: read and write",
        "Contents: read and write",
        "Actions: read and write",
        "Commit statuses: read and write",
    ] {
        assert!(scopes.contains(scope), "scopes lack {scope:?}");
    }
    let reason = DECLARATION.write_reason.expect("GitHub writes");
    for permission in [
        "Issues: write",
        "Contents: write",
        "Actions: write",
        "Commit statuses: write",
    ] {
        assert!(
            reason.contains(permission),
            "the reason lacks {permission:?}"
        );
    }
    assert!(!reason.contains("only reads"));
}
