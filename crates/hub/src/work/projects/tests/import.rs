//! The import rule over every case of a listed repository (PRO-001, PRO-002, PRJ-016, SRC-012).

use uuid::Uuid;

use crate::{
    ids::{ProjectId, RepositoryId, SourceAccountId},
    work::projects::{
        import::{Step, project_name, step, usable},
        model::{Canonical, Known},
    },
};

fn fields(full_name: &str) -> Canonical {
    Canonical {
        full_name: full_name.to_owned(),
        html_url: format!("https://github.com/{full_name}"),
        default_branch: Some("main".to_owned()),
        private: true,
        archived: false,
        fork: false,
        description: None,
        pushed_at: None,
    }
}

fn known(fields: Canonical) -> Known {
    Known {
        id: RepositoryId::from(Uuid::now_v7()),
        project_id: ProjectId::from(Uuid::now_v7()),
        account_id: Some(SourceAccountId::from(Uuid::now_v7())),
        fields,
        left_out: false,
        gone: false,
    }
}

#[test]
fn pro_001_an_unknown_repository_is_imported() {
    assert_eq!(step(None, &fields("me/site")), Step::Import);
}

#[test]
fn src_012_a_known_repository_with_the_same_fields_is_kept() {
    let listed = fields("me/site");
    assert_eq!(step(Some(&known(listed.clone())), &listed), Step::Keep);
}

#[test]
fn prj_016_a_rename_refreshes_the_same_repository() {
    let stored = known(fields("me/site"));
    assert_eq!(step(Some(&stored), &fields("me/new-site")), Step::Refresh);
    let archived = Canonical {
        archived: true,
        ..fields("me/site")
    };
    assert_eq!(step(Some(&stored), &archived), Step::Refresh);
}

#[test]
fn pro_002_a_left_out_repository_is_never_imported_again() {
    let stored = Known {
        left_out: true,
        ..known(fields("me/site"))
    };
    assert_eq!(step(Some(&stored), &fields("me/renamed")), Step::Keep);
    let gone = Known {
        gone: true,
        ..stored
    };
    assert_eq!(step(Some(&gone), &fields("me/renamed")), Step::Return);
}

#[test]
fn a_repository_back_in_the_listing_or_without_an_account_is_refreshed() {
    let listed = fields("me/site");
    let gone = Known {
        gone: true,
        ..known(listed.clone())
    };
    assert_eq!(step(Some(&gone), &listed), Step::Refresh);
    let orphan = Known {
        account_id: None,
        ..known(listed.clone())
    };
    assert_eq!(step(Some(&orphan), &listed), Step::Refresh);
}

#[test]
fn only_owner_slash_name_of_at_most_300_characters_is_usable() {
    assert!(usable("me/site"));
    assert!(!usable("site"));
    assert!(!usable("/site"));
    assert!(!usable("me/"));
    assert!(!usable(&format!("me/{}", "a".repeat(298))));
    assert_eq!(project_name("me/site"), "site");
}
