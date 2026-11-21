use super::{DAEMON, daemon_file, expected, key, raw_signature, record, signed, version};
use crate::{Admitted, Expected, FileCheck, Program, Refusal, Target, admits, check};

#[test]
fn rol_004_rol_010_a_signed_record_passes_its_check_and_gives_the_file_of_the_program() {
    let key = key();
    let (bytes, signature, public) = signed(&key);
    let checked = check(&bytes, &signature, &public, expected()).unwrap();
    assert_eq!(checked.record, record());
    assert_eq!(checked.file, daemon_file());
}

#[test]
fn rol_010_the_signature_is_checked_first_with_the_public_key_of_the_program() {
    let key = key();
    let (bytes, signature, public) = signed(&key);
    let mut changed = bytes.clone();
    changed[10] ^= 1;
    assert_eq!(
        check(&changed, &signature, &public, expected()),
        Err(Refusal::RecordSignature)
    );
    let other = super::key().public_key();
    assert_eq!(
        check(&bytes, &signature, &other, expected()),
        Err(Refusal::RecordSignature)
    );
    assert_eq!(
        check(&bytes, "not a signature", &public, expected()),
        Err(Refusal::RecordSignature)
    );
}

#[test]
fn rol_004_the_signature_holds_only_for_the_version_that_it_names() {
    let key = key();
    let (bytes, signature, public) = signed(&key);
    let other = Expected {
        version: version("0.3.1"),
        ..expected()
    };
    assert_eq!(
        check(&bytes, &signature, &public, other),
        Err(Refusal::RecordSignature)
    );
}

#[test]
fn rol_013_the_signature_of_a_file_of_a_different_name_is_not_the_signature_of_a_record() {
    let key = key();
    let (bytes, _, public) = signed(&key);
    let as_daemon = raw_signature(&key, &bytes, "anima-daemon", "0.3.0");
    assert_eq!(
        check(&bytes, &as_daemon, &public, expected()),
        Err(Refusal::RecordSignature)
    );
}

#[test]
fn rol_010_rol_013_a_good_signature_over_a_record_of_a_different_target_or_program_is_refused() {
    let key = key();
    let (bytes, signature, public) = signed(&key);
    let mac = Expected {
        target: Target::DarwinUniversal,
        ..expected()
    };
    assert_eq!(
        check(&bytes, &signature, &public, mac),
        Err(Refusal::RecordMismatch)
    );
    let hub = Expected {
        program: Program::Hub,
        ..expected()
    };
    assert_eq!(
        check(&bytes, &signature, &public, hub),
        Err(Refusal::RecordMismatch)
    );
}

#[test]
fn rol_010_signed_bytes_that_are_no_valid_record_are_refused() {
    let key = key();
    let public = key.public_key();
    let refused = |bytes: &[u8]| {
        let signature = raw_signature(&key, bytes, crate::RECORD_FILE, "0.3.0");
        check(bytes, &signature, &public, expected())
    };
    assert_eq!(refused(b"an AppImage"), Err(Refusal::RecordMismatch));
    let mut two = record();
    two.files.push(daemon_file());
    assert_eq!(
        refused(&serde_json::to_vec(&two).unwrap()),
        Err(Refusal::RecordMismatch)
    );
    let mut text = serde_json::to_value(record()).unwrap();
    text["signed_by"] = "anyone".into();
    assert_eq!(
        refused(&serde_json::to_vec(&text).unwrap()),
        Err(Refusal::RecordMismatch)
    );
    let mut wrong = record();
    wrong.version = version("0.3.1");
    wrong.replaces_from = version("0.2.0");
    assert_eq!(
        refused(&serde_json::to_vec(&wrong).unwrap()),
        Err(Refusal::RecordMismatch)
    );
    assert_eq!(
        refused(&vec![b' '; crate::MAX_RECORD_BYTES + 1]),
        Err(Refusal::RecordMismatch)
    );
}

#[test]
fn rol_010_the_file_must_have_the_size_and_the_sha_256_of_the_record() {
    let whole = |chunks: &[&[u8]]| {
        let mut file = FileCheck::new(&daemon_file());
        for chunk in chunks {
            file.update(chunk)?;
        }
        file.finish()
    };
    assert_eq!(whole(&[&DAEMON[..9], &DAEMON[9..]]), Ok(()));
    assert_eq!(
        whole(&[&DAEMON[..DAEMON.len() - 1]]),
        Err(Refusal::FileMismatch)
    );
    assert_eq!(whole(&[DAEMON, b"!"]), Err(Refusal::FileMismatch));
    let mut changed = DAEMON.to_vec();
    changed[0] ^= 1;
    assert_eq!(whole(&[&changed]), Err(Refusal::FileMismatch));
}

#[test]
fn rol_010_a_file_that_is_longer_than_the_record_says_is_refused_at_once() {
    let mut file = FileCheck::new(&daemon_file());
    file.update(DAEMON).unwrap();
    assert_eq!(file.update(b"x"), Err(Refusal::FileMismatch));
}

#[test]
fn rol_012_rol_041_a_machine_takes_a_higher_version_that_can_replace_its_own_or_its_previous_version()
 {
    let record = record();
    assert_eq!(
        admits(&record, version("0.2.0"), None),
        Ok(Admitted::Update)
    );
    assert_eq!(
        admits(&record, version("0.2.9"), None),
        Ok(Admitted::Update)
    );
    assert_eq!(
        admits(&record, version("0.1.9"), None),
        Err(Refusal::NotReplaceable)
    );
    assert_eq!(
        admits(&record, version("0.3.0"), None),
        Err(Refusal::LowerVersion)
    );
    assert_eq!(
        admits(&record, version("0.3.1"), None),
        Err(Refusal::LowerVersion)
    );
    assert_eq!(
        admits(&record, version("0.3.1"), Some(version("0.3.0"))),
        Ok(Admitted::Rollback)
    );
    assert_eq!(
        admits(&record, version("0.3.2"), Some(version("0.3.1"))),
        Err(Refusal::LowerVersion)
    );
}

#[test]
fn each_refusal_has_the_code_of_a_refused_row_of_k8() {
    let contract = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/daemon-push.md"
    ))
    .unwrap();
    for refusal in [
        Refusal::RecordSignature,
        Refusal::RecordMismatch,
        Refusal::FileMismatch,
        Refusal::LowerVersion,
        Refusal::NotReplaceable,
    ] {
        assert!(
            contract.contains(&format!("| `refused` | `{}` |", refusal.code())),
            "{refusal:?}"
        );
    }
}
