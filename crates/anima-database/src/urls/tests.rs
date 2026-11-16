use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::{UrlError, password, percent_encoded, url, write};
use crate::{Layout, Password, Role, owner_only};

fn layout_in(folder: &std::path::Path) -> Layout {
    let layout = Layout::new(folder.join("db root"), PathBuf::from("/run/app support/pg")).unwrap();
    owner_only::folder(&layout.urls_folder()).unwrap();
    layout
}

#[test]
fn db_002_a_url_names_the_login_its_database_and_the_socket_folder() {
    let layout = Layout::new(
        PathBuf::from("/var/db"),
        PathBuf::from("/run/app support/pg"),
    )
    .unwrap();
    let password = Password::from_bytes(&[0xab; 24]);
    let secret = "ab".repeat(24);

    assert_eq!(
        *url(&layout, Role::Hub, &password),
        format!(
            "postgres://anima_hub:{secret}@localhost:5432/anima?host=%2Frun%2Fapp%20support%2Fpg"
        )
    );
    assert_eq!(
        *url(&layout, Role::Owner, &password),
        format!(
            "postgres://anima_owner:{secret}@localhost:5432/anima?host=%2Frun%2Fapp%20support%2Fpg"
        )
    );
}

#[test]
fn a_folder_name_keeps_only_unreserved_characters() {
    assert_eq!(percent_encoded(b"/a-b_c.d~e"), "%2Fa-b_c.d~e");
    assert_eq!(
        percent_encoded("/é?&#%".as_bytes()),
        "%2F%C3%A9%3F%26%23%25"
    );
}

#[test]
fn db_002_a_written_file_is_owner_only_and_gives_its_password_back() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());

    write(&layout, Role::Hub, &Password::from_bytes(&[3; 24])).unwrap();

    let file = layout.url_file(Role::Hub);
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        password(&layout, Role::Hub).unwrap().unwrap().expose(),
        "03".repeat(24)
    );
    let left: Vec<_> = fs::read_dir(layout.urls_folder())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, ["hub.url"]);
}

#[test]
fn a_second_write_replaces_the_password() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    write(&layout, Role::Owner, &Password::from_bytes(&[3; 24])).unwrap();

    write(&layout, Role::Owner, &Password::from_bytes(&[4; 24])).unwrap();

    assert_eq!(
        password(&layout, Role::Owner).unwrap().unwrap().expose(),
        "04".repeat(24)
    );
}

#[test]
fn a_role_with_no_file_has_no_password_yet() {
    let folder = tempfile::tempdir().unwrap();

    assert!(
        password(&layout_in(folder.path()), Role::Hub)
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_file_that_is_not_this_roles_url_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    write(&layout, Role::Owner, &Password::from_bytes(&[3; 24])).unwrap();
    fs::copy(layout.url_file(Role::Owner), layout.url_file(Role::Hub)).unwrap();

    assert!(matches!(password(&layout, Role::Hub), Err(UrlError::Form)));

    fs::write(
        layout.url_file(Role::Hub),
        "postgres://anima_hub:not hex@x/anima",
    )
    .unwrap();
    assert!(matches!(password(&layout, Role::Hub), Err(UrlError::Form)));
}
