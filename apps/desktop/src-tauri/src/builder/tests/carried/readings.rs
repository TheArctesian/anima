//! The decision on what the checks read of one Mach-O file (UPD-011, S12 D-3): recorded output of
//! real files, each bad case alone. The names of the third-party files are changed, and hash lines
//! the checks do not read are left out.

use crate::builder::carried::mach_o::{self, Readings};

/// `codesign -d --verbose=4 --arch x86_64` of a bare universal program signed with a Developer ID,
/// the hardened runtime and a secure timestamp (recorded 2026-11-16, names changed).
const BARE_X86_64: &str = "Executable=/Applications/Other.app/Contents/MacOS/other_handler
Identifier=other_handler
Format=Mach-O thin (x86_64)
CodeDirectory v=20500 size=8124 flags=0x10000(runtime) hashes=248+2 location=embedded
Hash type=sha256 size=32
Hash choices=sha256
Authority=Developer ID Application: Other Ltd (24VZTF6M5V)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Timestamp=25 Sep 2026 at 1:45:42 AM
Info.plist=not bound
TeamIdentifier=24VZTF6M5V
Runtime Version=12.3.0
Sealed Resources=none";

/// The `arm64` slice of the same program.
const BARE_ARM64: &str = "Executable=/Applications/Other.app/Contents/MacOS/other_handler
Identifier=other_handler
Format=Mach-O thin (arm64)
CodeDirectory v=20500 size=7580 flags=0x10000(runtime) hashes=231+2 location=embedded
Hash type=sha256 size=32
Hash choices=sha256
Authority=Developer ID Application: Other Ltd (24VZTF6M5V)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Timestamp=25 Sep 2026 at 1:45:42 AM
Info.plist=not bound
TeamIdentifier=24VZTF6M5V
Runtime Version=12.3.0
Sealed Resources=none";

/// A program signed with two hash types (`Hash choices=sha1,sha256`) and no hardened runtime: it
/// still prints one `CodeDirectory` line (recorded 2026-11-16, names changed).
const TWO_HASHES: &str = "Executable=/Applications/Older.app/Contents/MacOS/older
Identifier=org.example.older
Format=app bundle with Mach-O thin (x86_64)
CodeDirectory v=20400 size=9942 flags=0x0(none) hashes=300+7 location=embedded
Hash type=sha256 size=32
Hash choices=sha1,sha256
Authority=Apple Mac OS Application Signing
Info.plist entries=30
TeamIdentifier=5433B4KXM8";

/// The same program signed ad hoc with `--options runtime`: no timestamp.
const AD_HOC: &str = "Executable=/tmp/adhoc
Format=Mach-O thin (x86_64)
CodeDirectory v=20500 size=271 flags=0x10002(adhoc,runtime) hashes=2+2 location=embedded
Signature=adhoc
TeamIdentifier=not set";

const ENTITLED: &str = r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>com.apple.security.get-task-allow</key><true/></dict></plist>"#;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

/// The readings of the recorded program: both architectures, verified, each display good and no
/// entitlement.
fn released() -> Readings {
    Readings {
        archs: lines("x86_64 arm64"),
        verified: true,
        displays: vec![("x86_64", lines(BARE_X86_64)), ("arm64", lines(BARE_ARM64))],
        entitlements: vec![("x86_64", Vec::new()), ("arm64", Vec::new())],
    }
}

#[test]
fn upd_011_a_bare_universal_developer_id_program_is_released() {
    assert!(mach_o::released(&released()));
    let mut either_order = released();
    either_order.displays.reverse();
    either_order.archs = lines("arm64 x86_64");
    assert!(mach_o::released(&either_order));
}

#[test]
fn upd_011_a_program_that_failed_the_requirement_is_not_released() {
    let mut readings = released();
    readings.verified = false;
    assert!(!mach_o::released(&readings));
}

#[test]
fn upd_011_a_program_with_one_architecture_or_a_third_is_not_released() {
    for archs in [
        "arm64",
        "x86_64",
        "x86_64 arm64 arm64e",
        "x86_64 x86_64",
        "",
    ] {
        let mut readings = released();
        readings.archs = lines(archs);
        assert!(!mach_o::released(&readings), "{archs:?}");
    }
}

#[test]
fn upd_011_a_display_of_only_one_architecture_is_not_released() {
    let mut one = released();
    one.displays.pop();
    assert!(!mach_o::released(&one));
    let mut twice = released();
    twice.displays[1] = ("x86_64", lines(BARE_X86_64));
    assert!(!mach_o::released(&twice), "the same architecture twice");
    let mut extra = released();
    extra.displays.push(("arm64", lines(BARE_ARM64)));
    assert!(!mach_o::released(&extra));
    let mut no_entitlement_read = released();
    no_entitlement_read.entitlements.pop();
    assert!(!mach_o::released(&no_entitlement_read));
}

#[test]
fn upd_011_one_bad_architecture_is_enough_to_refuse_the_program() {
    for (arch, display) in [
        ("x86_64", AD_HOC),
        ("arm64", AD_HOC),
        ("x86_64", TWO_HASHES),
    ] {
        let mut readings = released();
        let slot = readings
            .displays
            .iter_mut()
            .find(|(read, _)| *read == arch)
            .unwrap();
        slot.1 = lines(display);
        assert!(!mach_o::released(&readings), "{arch} {display:.40}");
    }
}

#[test]
fn s12_d_3_an_entitlement_in_one_architecture_refuses_the_program() {
    for index in 0..2 {
        let mut readings = released();
        readings.entitlements[index].1 = lines(ENTITLED);
        assert!(!mach_o::released(&readings));
    }
}

#[test]
fn upd_011_two_hash_types_print_one_code_directory() {
    let directories = TWO_HASHES
        .lines()
        .filter(|line| line.starts_with("CodeDirectory "))
        .count();
    assert_eq!(directories, 1);
    let hardened = TWO_HASHES
        .replace("flags=0x0(none)", "flags=0x10000(runtime)")
        .replace(
            "Info.plist entries=30",
            "Timestamp=25 Sep 2026 at 1:45:42 AM",
        );
    assert!(mach_o::hardened_and_timestamped(&lines(&hardened)));
    assert!(!mach_o::hardened_and_timestamped(&lines(TWO_HASHES)));
}
