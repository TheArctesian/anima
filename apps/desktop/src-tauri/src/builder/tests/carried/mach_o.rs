//! The checks of each Mach-O file of the signed app (UPD-011, S12 2.3, D-3 to D-5). The
//! decisions are tested on output recorded from real `codesign` runs on macOS 27 (2026-11-16): a
//! third-party Developer ID app, `/usr/bin/true` and copies of it signed ad hoc in a temporary
//! folder. `signed_files.rs` runs the planned commands against real files.

use std::path::Path;

use super::args;
use crate::builder::{
    carried::mach_o,
    error::BuilderError,
    plan::{Kept, Program},
};

const IDENTITY: &str = "Developer ID Application: Stephen Okita (7GDLG48PV6)";

/// `codesign -d --verbose=4` of a Developer ID program with the hardened runtime and a secure
/// timestamp (standard error).
const DEVELOPER_ID: &str = "Executable=/Applications/Other.app/Contents/MacOS/other
Identifier=com.example.other
Format=app bundle with Mach-O universal (x86_64 arm64)
CodeDirectory v=20500 size=43265 flags=0x10000(runtime) hashes=1341+7 location=embedded
Hash type=sha256 size=32
CDHash=c8a2e49324919708be3523282126862adc032703
Signature size=9057
Authority=Developer ID Application: Other Ltd (24VZTF6M5V)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Timestamp=27 Oct 2026 at 9:10:28 AM
Info.plist entries=50
TeamIdentifier=24VZTF6M5V
Runtime Version=27.0.0";

/// The same for a file signed ad hoc with `--options runtime`: no timestamp.
const AD_HOC: &str = "Executable=/tmp/adhoc
Identifier=adhoc-555549446fd45ef36f6030e48996526ff84e70f6
Format=Mach-O universal (x86_64 arm64e arm64e.x1)
CodeDirectory v=20500 size=271 flags=0x10002(adhoc,runtime) hashes=2+2 location=embedded
CDHash=3dad931afa599355ef7e7ff290e12cd9433f8fe8
Signature=adhoc
Info.plist=not bound
TeamIdentifier=not set
Runtime Version=27.0.0
Sealed Resources=none";

/// `/usr/bin/true`: Apple's own signature, no runtime flag and a signing time, not a timestamp.
const PLATFORM: &str = "Executable=/usr/bin/true
Identifier=com.apple.true
CodeDirectory v=20400 size=231 flags=0x0(none) hashes=2+2 location=embedded
Platform identifier=26
Authority=macOS Software Signing
Authority=Apple Code Signing Certification Authority
Authority=Apple Root CA
Signed Time=8 Aug 2026 at 3:25:59 PM
TeamIdentifier=not set";

/// `codesign -d --entitlements - --xml` (standard output) of a file with an entitlement, and of
/// one signed with an empty entitlement list.
const ENTITLED: &str = r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>com.apple.security.cs.disable-library-validation</key><true/></dict></plist>"#;
const EMPTY_LIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict></dict></plist>"#;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

/// `text` with the line that starts with `prefix` changed to `line`.
fn with_line(text: &str, prefix: &str, line: &str) -> Vec<String> {
    lines(text)
        .into_iter()
        .map(|old| {
            if old.starts_with(prefix) {
                line.to_owned()
            } else {
                old
            }
        })
        .collect()
}

#[test]
fn s12_d_4_a_file_is_mach_o_by_its_first_bytes_in_either_byte_order() {
    for magic in [
        [0xcf, 0xfa, 0xed, 0xfe],
        [0xfe, 0xed, 0xfa, 0xcf],
        [0xce, 0xfa, 0xed, 0xfe],
        [0xca, 0xfe, 0xba, 0xbe],
        [0xbe, 0xba, 0xfe, 0xca],
        [0xca, 0xfe, 0xba, 0xbf],
    ] {
        let mut header = magic.to_vec();
        header.extend([7, 0, 0, 1]);
        assert!(mach_o::is_mach_o(&header), "{magic:x?}");
        assert!(mach_o::is_mach_o(&magic));
    }
    for other in [
        &b"\x7fELF\x02\x01"[..],
        b"#!/bin/sh",
        b"bplist00",
        &[0xcf, 0xfa, 0xed],
        &[],
        &[0xcf, 0xfa, 0xed, 0xff],
    ] {
        assert!(!mach_o::is_mach_o(other), "{other:x?}");
    }
}

#[test]
fn s12_d_5_the_team_id_is_the_last_parentheses_of_the_identity_and_nothing_else() {
    let signer = mach_o::Signer::from_identity(IDENTITY).unwrap();
    assert_eq!(signer.identity, IDENTITY);
    assert_eq!(signer.team.as_str(), "7GDLG48PV6");
    let nested = "Developer ID Application: A (B) Ltd (L2GL58P3B6)";
    assert_eq!(
        mach_o::Signer::from_identity(nested).unwrap().team.as_str(),
        "L2GL58P3B6"
    );
    for bad in [
        "",
        "Developer ID Application: Stephen Okita",
        "Developer ID Application: Stephen Okita (7GDLG48PV6) ",
        "Developer ID Application: Stephen Okita (7gdlg48pv6)",
        "Developer ID Application: Stephen Okita (7GDLG48PV)",
        "Developer ID Application: Stephen Okita (7GDLG48PV66)",
        "Developer ID Application: X (7GDLG48PV\")",
        "Developer ID Application: X (\" or 1)",
        "Developer ID Application: X (7GDLG48PV6) (abc)",
        "(7GDLG48P\u{d7}6)",
    ] {
        assert_eq!(
            mach_o::Signer::from_identity(bad),
            Err(BuilderError::Bundle),
            "{bad:?}"
        );
    }
}

#[test]
fn upd_011_each_file_is_verified_against_the_developer_id_requirement_of_the_team() {
    let team = mach_o::Signer::from_identity(IDENTITY).unwrap().team;
    let file = Path::new("/c/Anima.ing.app/Contents/Resources/database/bin/postgres");
    let verify = mach_o::verify(file, &team);
    assert_eq!(verify.program, Program::Codesign);
    assert_eq!(
        verify.cwd,
        Path::new("/c/Anima.ing.app/Contents/Resources/database/bin")
    );
    assert_eq!(
        args(&verify),
        [
            "--verify",
            "--strict",
            "-R=anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists \
             and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate \
             leaf[subject.OU] = \"7GDLG48PV6\"",
            file.to_str().unwrap()
        ]
    );
    let other = mach_o::Signer::from_identity("Developer ID Application: Other Ltd (24VZTF6M5V)")
        .unwrap()
        .team;
    assert!(mach_o::requirement(&other).ends_with("leaf[subject.OU] = \"24VZTF6M5V\""));
    assert_eq!(
        (mach_o::archs(file).program, args(&mach_o::archs(file))),
        (Program::Lipo, vec!["-archs", file.to_str().unwrap()])
    );
}

#[test]
fn upd_011_the_display_keeps_codesigns_error_lines_for_each_architecture() {
    let file = Path::new("/c/app/Contents/MacOS/anima");
    let display = mach_o::display(file, "x86_64");
    assert_eq!(display.kept, Kept::Errors);
    assert_eq!(
        args(&display),
        [
            "-d",
            "--verbose=4",
            "--arch",
            "x86_64",
            "/c/app/Contents/MacOS/anima"
        ]
    );
    let entitlements = mach_o::entitlements(file, "arm64");
    assert_eq!(entitlements.kept, Kept::Output);
    assert_eq!(
        args(&entitlements),
        [
            "-d",
            "--entitlements",
            "-",
            "--xml",
            "--arch",
            "arm64",
            "/c/app/Contents/MacOS/anima"
        ]
    );
    assert_eq!(mach_o::ARCHS, ["x86_64", "arm64"]);
}

#[test]
fn upd_011_only_a_hardened_runtime_with_a_secure_timestamp_passes() {
    assert!(mach_o::hardened_and_timestamped(&lines(DEVELOPER_ID)));
    assert!(
        !mach_o::hardened_and_timestamped(&lines(AD_HOC)),
        "no timestamp"
    );
    assert!(!mach_o::hardened_and_timestamped(&lines(PLATFORM)));
    let no_runtime = with_line(
        DEVELOPER_ID,
        "CodeDirectory",
        "CodeDirectory v=20500 size=43265 flags=0x0(none) hashes=1341+7 location=embedded",
    );
    assert!(!mach_o::hardened_and_timestamped(&no_runtime));
    let no_timestamp = with_line(DEVELOPER_ID, "Timestamp=", "Signed Time=27 Oct 2026");
    assert!(!mach_o::hardened_and_timestamped(&no_timestamp));
    let empty_timestamp = with_line(DEVELOPER_ID, "Timestamp=", "Timestamp=none");
    assert!(!mach_o::hardened_and_timestamped(&empty_timestamp));
    let runtime_elsewhere = with_line(
        DEVELOPER_ID,
        "CodeDirectory",
        "CodeDirectory v=20500 flags=0x0(none) runtime location=embedded",
    );
    assert!(!mach_o::hardened_and_timestamped(&runtime_elsewhere));
    let mut two = lines(DEVELOPER_ID);
    two.push("CodeDirectory v=20500 flags=0x0(none) location=embedded".to_owned());
    assert!(!mach_o::hardened_and_timestamped(&two));
    assert!(!mach_o::hardened_and_timestamped(&[]));
}

#[test]
fn s12_d_3_a_file_with_any_entitlement_fails() {
    assert!(mach_o::no_entitlements(&[]));
    assert!(mach_o::no_entitlements(&lines(EMPTY_LIST)));
    assert!(mach_o::no_entitlements(&lines(
        &EMPTY_LIST.replace("<dict></dict>", "<dict/>")
    )));
    assert!(!mach_o::no_entitlements(&lines(ENTITLED)));
    let spread = ENTITLED.replace("><", ">\n<");
    assert!(!mach_o::no_entitlements(&lines(&spread)));
    for odd in [
        "not a plist",
        "<plist version=\"1.0\"><dict></dict></plist>",
        &EMPTY_LIST.replace(
            "<dict></dict>",
            "<dict></dict><dict><key>a</key><true/></dict>",
        ),
    ] {
        assert!(!mach_o::no_entitlements(&lines(odd)), "{odd}");
    }
}
