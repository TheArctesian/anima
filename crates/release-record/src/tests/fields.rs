use super::{TIME, daemon_file, key, record, version};
use crate::{Database, FORMAT, Program, Record, SignError, sign};

#[test]
fn rol_003_the_record_is_json_with_these_fields_and_names() {
    let (bytes, _) = sign(&record(), &key(), TIME).unwrap();
    let text: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        text,
        serde_json::json!({
            "format": 1,
            "version": "0.3.0",
            "commit": "0123456789abcdef0123456789abcdef01234567",
            "target": "linux-x86_64",
            "replaces_from": "0.2.0",
            "database": "compatible",
            "files": [{
                "name": "anima-daemon",
                "program": "anima-daemon",
                "size": daemon_file().size,
                "sha256": daemon_file().sha256,
            }],
        })
    );
    assert!(bytes.ends_with(b"}\n"));
}

#[test]
fn rol_036_a_release_that_needs_a_restore_says_so() {
    let restore = Record {
        database: Database::RestoreNeeded,
        ..record()
    };
    let text = serde_json::to_value(&restore).unwrap();
    assert_eq!(text["database"], "restore_needed");
}

#[test]
fn a_record_that_breaks_a_rule_is_not_signed() {
    let breaks: [fn(&mut Record); 9] = [
        |r| r.format = FORMAT + 1,
        |r| r.commit = "0123".to_owned(),
        |r| r.commit = r.commit.to_uppercase(),
        |r| r.replaces_from = super::version("0.3.1"),
        |r| r.files.clear(),
        |r| r.files[0].name = ".hidden".to_owned(),
        |r| r.files[0].sha256 = "00".to_owned(),
        |r| r.files[0].size = crate::MAX_FILE_BYTES + 1,
        |r| {
            let mut other = daemon_file();
            other.program = Program::Command;
            r.files.push(other);
        },
    ];
    for (index, change) in breaks.into_iter().enumerate() {
        let mut broken = record();
        change(&mut broken);
        assert_eq!(
            sign(&broken, &key(), TIME),
            Err(SignError::Invalid),
            "rule {index}"
        );
    }
    let mut two_programs = record();
    let mut command = daemon_file();
    command.name = "anima".to_owned();
    command.program = Program::Command;
    two_programs.files.push(command);
    assert!(sign(&two_programs, &key(), TIME).is_ok());
}

#[test]
fn a_version_has_one_text_and_its_order_is_by_number() {
    assert!(version("0.1.10") > version("0.1.9"));
    for wrong in [
        "0.1",
        "0.1.2.3",
        "01.1.2",
        "0.1.x",
        "",
        "0.1.-2",
        "1234567890.0.0",
    ] {
        assert!(wrong.parse::<crate::Version>().is_err(), "{wrong}");
    }
    assert_eq!(version("10.0.7").to_string(), "10.0.7");
}
