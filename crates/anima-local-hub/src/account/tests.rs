use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Stdio;

use tempfile::TempDir;

use super::{AccountError, Action, Name, run};
use crate::Folders;
use crate::run::{Lock, lock};
use crate::state::{self, State};

/// The installed hub: it writes its arguments, its environment and its input beside itself,
/// and answers one JSON line.
const HUB: &str = r#"#!/bin/sh
here=$(dirname "$0")
printf '%s\n' "$*" > "$here/args"
/usr/bin/env > "$here/env"
cat > "$here/input"
echo '{"name":"ada","totp":true}'
[ -f "$here/refuse" ] && { echo "a secret that must not be shown" >&2; exit 3; }
exit 0
"#;

struct Rig {
    folder: TempDir,
    folders: Folders,
}

fn rig() -> Rig {
    let folder = tempfile::tempdir().unwrap();
    let folders = Folders::new(folder.path().join("hub root"), folder.path().join("s")).unwrap();
    let bin = folders.programs().join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("anima-hub"), HUB).unwrap();
    fs::set_permissions(bin.join("anima-hub"), fs::Permissions::from_mode(0o700)).unwrap();
    // As `install` leaves them: each folder this user's alone.
    for private in [
        folders.root().to_path_buf(),
        folders.all_programs(),
        folders.programs(),
        bin,
    ] {
        fs::set_permissions(private, fs::Permissions::from_mode(0o700)).unwrap();
    }
    Rig { folder, folders }
}

impl Rig {
    fn bin(&self, file: &str) -> PathBuf {
        self.folders.programs().join("bin").join(file)
    }

    /// A `run` holds the root and its state is `ready`.
    fn ready(&self) -> Lock {
        let held = lock(&self.folders).unwrap();
        let ready = State::Ready {
            hub: "127.0.0.1:8443".parse().unwrap(),
            recovered: false,
            link: None,
        };
        state::write(&self.folders.state_file(), &ready).unwrap();
        held
    }

    /// Runs `action` with `input` on standard input; returns the exit code and the output.
    fn run(&self, action: &Action, input: &str) -> Result<(i32, String), AccountError> {
        let given = self.folder.path().join("given");
        let taken = self.folder.path().join("taken");
        fs::write(&given, input).unwrap();
        let status = run(
            &self.folders,
            action,
            Stdio::from(File::open(&given).unwrap()),
            Stdio::from(File::create(&taken).unwrap()),
        )?;
        Ok((status.code().unwrap(), fs::read_to_string(taken).unwrap()))
    }

    fn written(&self, file: &str) -> String {
        fs::read_to_string(self.bin(file)).unwrap_or_default()
    }
}

fn name(text: &str) -> Name {
    Name::parse(text).unwrap()
}

#[test]
fn frn_003_each_account_command_runs_the_installed_hubs_operator_command() {
    let rig = rig();
    let _ready = rig.ready();

    for (action, args) in [
        (Action::Status, "operator show"),
        (Action::TotpNew(name("ada")), "operator totp ada --new"),
        (
            Action::Create(name("ada.l_1-x")),
            "operator create ada.l_1-x",
        ),
    ] {
        let (code, output) = rig.run(&action, "").unwrap();

        assert_eq!(code, 0);
        assert_eq!(output, "{\"name\":\"ada\",\"totp\":true}\n");
        assert_eq!(rig.written("args"), format!("{args}\n"));
    }
}

#[test]
fn frn_004_the_secrets_pass_on_standard_input_and_the_hub_sees_only_its_two_files() {
    let rig = rig();
    let _ready = rig.ready();
    let input = "a password\nJBSWY3DPEHPK3PXP\n123456\n";

    rig.run(&Action::Create(name("ada")), input).unwrap();

    assert_eq!(rig.written("input"), input);
    assert!(!rig.written("args").contains("password"));
    // The shell adds its own PWD, SHLVL and `_`.
    let mut env: Vec<String> = rig.written("env").lines().map(str::to_owned).collect();
    env.retain(|line| {
        !line.starts_with("PWD=") && !line.starts_with("SHLVL=") && !line.starts_with("_=")
    });
    env.sort();
    let database = rig.folders.database().root().join("urls/hub.url");
    assert_eq!(
        env,
        [
            format!("ANIMA_DATABASE_URL_FILE={}", database.display()),
            format!("ANIMA_KEY_FILE={}", rig.folders.vault_key().display()),
        ]
    );
}

#[test]
fn a_refusal_of_the_hub_gives_its_exit_code_and_none_of_its_text() {
    let rig = rig();
    let _ready = rig.ready();
    fs::write(rig.bin("refuse"), "").unwrap();

    let (code, output) = rig.run(&Action::Status, "").unwrap();

    assert_eq!(code, 3);
    assert!(!output.contains("secret"), "{output}");
}

#[test]
fn frn_007_a_ready_state_that_no_run_holds_is_not_ready() {
    let rig = rig();
    // What a run that was killed leaves: the state says ready, and no lock is held.
    drop(rig.ready());

    assert!(matches!(
        rig.run(&Action::Status, ""),
        Err(AccountError::NotReady)
    ));
    assert!(!rig.bin("args").exists(), "the program ran");
}

#[test]
fn sec_005_the_secrets_never_go_to_a_program_that_another_user_could_have_put_there() {
    let rig = rig();
    let _ready = rig.ready();
    let bin = rig.folders.programs().join("bin");
    let open = fs::Permissions::from_mode(0o777);
    let private = fs::Permissions::from_mode(0o700);
    let program = rig.bin("anima-hub");

    for (path, mode, restore) in [
        (rig.folders.root().to_path_buf(), &open, &private),
        (rig.folders.programs(), &open, &private),
        (bin.clone(), &fs::Permissions::from_mode(0o750), &private),
        (
            program.clone(),
            &fs::Permissions::from_mode(0o722),
            &private,
        ),
    ] {
        fs::set_permissions(&path, mode.clone()).unwrap();

        let refused = rig.run(&Action::Create(name("ada")), "a password\nS\n1\n");

        assert!(
            matches!(&refused, Err(AccountError::Untrusted(found)) if *found == path),
            "{}: {refused:?}",
            path.display()
        );
        fs::set_permissions(&path, restore.clone()).unwrap();
    }
    // A program that is a link to another file is refused too.
    fs::rename(&program, bin.join("real")).unwrap();
    std::os::unix::fs::symlink(bin.join("real"), &program).unwrap();
    assert!(matches!(
        rig.run(&Action::Status, ""),
        Err(AccountError::Untrusted(_))
    ));

    assert!(!rig.bin("args").exists(), "the program ran");
    assert!(!rig.bin("input").exists(), "the program got the secrets");
}

#[test]
fn frn_007_nothing_runs_while_the_hub_is_not_ready() {
    let rig = rig();
    // No run holds the root, and no state.
    assert!(matches!(
        rig.run(&Action::Status, ""),
        Err(AccountError::NotReady)
    ));

    // A run holds it, and it is still starting.
    let _held = lock(&rig.folders).unwrap();
    state::write(&rig.folders.state_file(), &State::Starting).unwrap();
    assert!(matches!(
        rig.run(&Action::Status, ""),
        Err(AccountError::NotReady)
    ));

    assert_eq!(rig.written("args"), "");
}

#[test]
fn a_name_is_1_to_64_plain_characters_and_never_an_option() {
    for good in ["ada", "A", "ada.lovelace_1-x", &"a".repeat(64)] {
        assert!(Name::parse(good).is_ok(), "{good}");
    }
    for bad in [
        "",
        "-x",
        "--new",
        ".ada",
        "_ada",
        "ada lovelace",
        "ada/x",
        "adä",
        &"a".repeat(65),
    ] {
        assert!(Name::parse(bad).is_err(), "{bad}");
    }
}
