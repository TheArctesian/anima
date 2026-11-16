use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
    time::{Duration, Instant},
};

use super::super::{
    LocalHubError,
    account_form::{self, Account},
    carried, from_main, key_copy,
    recovery::{Cancellations, PAUSE_AFTER_CANCEL, may_create},
};
use crate::carried::DevFolders;

#[test]
fn only_the_main_web_view_may_ask() {
    assert_eq!(from_main("main"), Ok(()));
    for other in ["web-1", "builder", "", "Main"] {
        assert_eq!(from_main(other), Err(LocalHubError::Caller), "{other:?}");
    }
}

#[test]
fn a_debug_build_carries_the_hub_only_from_both_dev_folders() {
    let folder = tempfile::tempdir().unwrap();
    let programs = folder.path().join("programs");
    let tree = folder.path().join("tree");
    fs::create_dir_all(tree.join("bin")).unwrap();
    fs::create_dir_all(&programs).unwrap();
    fs::write(tree.join("bin/postgres"), "").unwrap();
    fs::write(programs.join("anima-local-hub"), "").unwrap();
    let executable = Path::new("/nowhere/anima-desktop");
    let dev = DevFolders {
        programs: Some(programs.clone()),
        tree: Some(tree.clone()),
    };

    let found = carried::beside(executable, true, false, &dev).unwrap();
    assert_eq!(found.program, programs.join("anima-local-hub"));
    assert_eq!(found.tree, tree);

    // A release build never looks at the dev folders.
    assert_eq!(
        carried::beside(executable, true, true, &dev),
        Err(LocalHubError::NotCarried)
    );
    let no_tree = DevFolders {
        programs: Some(programs),
        tree: None,
    };
    assert_eq!(
        carried::beside(executable, true, false, &no_tree),
        Err(LocalHubError::NotCarried)
    );
}

#[test]
fn a_name_that_could_be_an_option_or_holds_a_control_character_is_refused() {
    assert_eq!(account_form::name("ada").unwrap(), "ada");
    assert!(account_form::name("ada.lovelace_1-x").is_ok());
    assert!(account_form::name(&"a".repeat(64)).is_ok());
    for bad in [
        "",
        "--root=/tmp",
        "-x",
        ".hidden",
        "a b",
        "a\nb",
        "a\u{7}",
        "ädа",
        &"a".repeat(65),
    ] {
        assert_eq!(
            account_form::name(bad),
            Err(LocalHubError::Argument),
            "{bad:?}"
        );
    }
}

#[test]
fn create_gets_three_lines_and_no_value_can_add_a_line() {
    let input = account_form::create_input("pass word", "JBSWY3DPEHPK3PXP", "123456").unwrap();
    assert_eq!(&input[..], b"pass word\nJBSWY3DPEHPK3PXP\n123456\n");

    let refused = [
        ("", "JBSWY3DP", "123456"),
        ("pass\nword", "JBSWY3DP", "123456"),
        ("pass\rword", "JBSWY3DP", "123456"),
        (&*"p".repeat(1025), "JBSWY3DP", "123456"),
        ("password", "", "123456"),
        ("password", "JBSW\nY3DP", "123456"),
        ("password", "JBSWY3DP1", "123456"),
        ("password", &*"A".repeat(65), "123456"),
        ("password", "JBSWY3DP", "12345"),
        ("password", "JBSWY3DP", "1234567"),
        ("password", "JBSWY3DP", "12345a"),
        ("password", "JBSWY3DP", "١٢٣٤٥٦"),
    ];
    for (password, secret, code) in refused {
        assert_eq!(
            account_form::create_input(password, secret, code).err(),
            Some(LocalHubError::Argument),
            "{secret:?} {code:?}"
        );
    }
    assert!(account_form::create_input(&"p".repeat(1024), &"A".repeat(64), "000000").is_ok());
}

#[test]
fn the_account_status_is_one_account_or_null() {
    assert_eq!(account_form::parse_account(b"null\n"), Ok(None));
    assert_eq!(
        account_form::parse_account(br#"{"name":"ada","totp":true}"#),
        Ok(Some(Account {
            name: "ada".into(),
            totp: true
        }))
    );
    for bad in [
        &b""[..],
        b"{}",
        br#"{"name":"ada"}"#,
        br#"{"name":1,"totp":true}"#,
        b"[]",
    ] {
        assert_eq!(account_form::parse_account(bad), Err(LocalHubError::Failed));
    }
}

#[test]
fn a_new_totp_is_a_secret_and_an_otpauth_uri_and_its_debug_shows_neither() {
    let printed = br#"{"secret":"JBSWY3DPEHPK3PXP","uri":"otpauth://totp/Anima:ada?secret=JBSWY3DPEHPK3PXP"}"#;

    let totp = account_form::parse_new_totp(printed).unwrap();

    assert_eq!(totp.secret, "JBSWY3DPEHPK3PXP");
    assert!(!format!("{totp:?}").contains("JBSW"));
    let sent = serde_json::to_value(&totp).unwrap();
    assert_eq!(
        sent["uri"],
        "otpauth://totp/Anima:ada?secret=JBSWY3DPEHPK3PXP"
    );
    for bad in [
        &br#"{"secret":"","uri":"otpauth://totp/x"}"#[..],
        br#"{"secret":"A","uri":"https://evil"}"#,
        br#"{"secret":"A"}"#,
        b"null",
    ] {
        assert_eq!(
            account_form::parse_new_totp(bad),
            Err(LocalHubError::Failed)
        );
    }
}

#[test]
fn create_runs_at_once_on_the_first_run_and_a_replacement_only_when_confirmed() {
    let asked = &std::cell::Cell::new(0);
    let answer = |yes| {
        move || {
            asked.set(asked.get() + 1);
            yes
        }
    };
    let now = Instant::now();
    let mut cancellations = Cancellations::default();

    assert_eq!(
        may_create(false, &mut cancellations, now, answer(false)),
        Ok(())
    );
    assert_eq!(asked.get(), 0, "no dialog on the first run");
    assert_eq!(
        may_create(true, &mut cancellations, now, answer(true)),
        Ok(())
    );
    assert_eq!(
        may_create(true, &mut cancellations, now, answer(false)),
        Err(LocalHubError::Cancelled)
    );
    assert_eq!(asked.get(), 2, "one dialog for each replacement");
}

#[test]
fn after_a_cancel_no_dialog_is_shown_for_a_minute() {
    let asked = &std::cell::Cell::new(0);
    let answer = |yes| {
        move || {
            asked.set(asked.get() + 1);
            yes
        }
    };
    let cancelled_at = Instant::now();
    let mut cancellations = Cancellations::default();
    let _ = may_create(true, &mut cancellations, cancelled_at, answer(false));

    let soon = cancelled_at + Duration::from_secs(59);
    assert_eq!(
        may_create(true, &mut cancellations, soon, answer(true)),
        Err(LocalHubError::Busy)
    );
    assert_eq!(asked.get(), 1, "no second dialog in the pause");
    // The first run needs no dialog, so the pause does not stop it.
    assert_eq!(
        may_create(false, &mut cancellations, soon, answer(false)),
        Ok(())
    );

    let later = cancelled_at + PAUSE_AFTER_CANCEL;
    assert_eq!(
        may_create(true, &mut cancellations, later, answer(true)),
        Ok(())
    );
    assert_eq!(asked.get(), 2);
    assert_eq!(PAUSE_AFTER_CANCEL, Duration::from_mins(1));
}

#[test]
fn the_key_copy_is_owner_only_and_replaces_a_plain_file() {
    let folder = tempfile::tempdir().unwrap();
    let key = folder.path().join("vault.key");
    fs::write(&key, b"key bytes").unwrap();
    let destination = folder.path().join("anima-hub-key");
    fs::write(&destination, b"older").unwrap();
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o644)).unwrap();

    key_copy::copy(&key, &destination).unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"key bytes");
    let mode = fs::metadata(&destination).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let names: Vec<_> = fs::read_dir(folder.path()).unwrap().collect();
    assert_eq!(names.len(), 2, "no temporary file is left");
}

#[test]
fn the_key_copy_refuses_a_symbolic_link_and_a_missing_key() {
    let folder = tempfile::tempdir().unwrap();
    let key = folder.path().join("vault.key");
    let elsewhere = folder.path().join("elsewhere");
    let destination = folder.path().join("anima-hub-key");
    symlink(&elsewhere, &destination).unwrap();

    assert_eq!(
        key_copy::copy(&key, &destination),
        Err(LocalHubError::NoKey)
    );
    fs::write(&key, b"key bytes").unwrap();
    assert_eq!(
        key_copy::copy(&key, &destination),
        Err(LocalHubError::Destination)
    );
    assert!(!elsewhere.exists());
    assert_eq!(
        key_copy::copy(&key, folder.path()),
        Err(LocalHubError::Destination)
    );
}

#[test]
fn the_new_error_codes_reach_the_client() {
    for (error, code) in [
        (LocalHubError::Argument, "argument"),
        (LocalHubError::Cancelled, "cancelled"),
        (LocalHubError::Refused, "refused"),
        (LocalHubError::NoKey, "no-key"),
        (LocalHubError::Destination, "destination"),
    ] {
        assert_eq!(serde_json::to_value(error).unwrap()["code"], code);
    }
}
