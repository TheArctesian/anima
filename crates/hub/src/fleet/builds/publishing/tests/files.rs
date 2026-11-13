//! The file set of a job and each file read back (BLD-061, BLD-063, BLD-064, BLD-066, UPD-010).

use super::file;
use crate::fleet::builds::{
    model::Target,
    publishing::files::{Received, check_sums, content_type, stable_copy},
};

fn sums_line(name: &str, bytes: &[u8]) -> String {
    format!("{}  {name}\n", file(name, bytes).sha256)
}

#[test]
fn bld_066_the_stable_name_comes_from_the_one_file_of_the_target() {
    let linux = [
        file("Anima_0.1.12_amd64.AppImage", b"a"),
        file("SHA256SUMS", b"s"),
    ];
    assert_eq!(
        stable_copy(Target::LinuxX86_64, &linux).unwrap(),
        (
            "Anima_0.1.12_amd64.AppImage".to_owned(),
            "Anima-x86_64.AppImage"
        )
    );
    let mac = [
        file("Anima.app.tar.gz", b"a"),
        file("Anima_0.1.12_universal.dmg", b"d"),
        file("SHA256SUMS", b"s"),
    ];
    assert_eq!(
        stable_copy(Target::DarwinUniversal, &mac).unwrap().1,
        "Anima.dmg"
    );
    let two = [
        file("a.AppImage", b"a"),
        file("b.AppImage", b"b"),
        file("SHA256SUMS", b"s"),
    ];
    assert!(stable_copy(Target::LinuxX86_64, &two).is_err());
    let no_sums = [file("a.AppImage", b"a")];
    assert!(stable_copy(Target::LinuxX86_64, &no_sums).is_err());
}

#[test]
fn upd_010_the_sums_list_each_other_file_with_its_digest() {
    let app = file("Anima_0.1.12_amd64.AppImage", b"app");
    let text = sums_line(&app.name, b"app");
    let files = [app, file("SHA256SUMS", text.as_bytes())];
    assert!(check_sums(text.as_bytes(), &files).is_ok());
    let wrong = sums_line("Anima_0.1.12_amd64.AppImage", b"other");
    assert!(check_sums(wrong.as_bytes(), &files).is_err());
    let extra = format!("{text}{}", sums_line("evil.sh", b"x"));
    assert!(check_sums(extra.as_bytes(), &files).is_err());
    assert!(check_sums(b"", &files).is_err());
    assert!(check_sums(b"\xff", &files).is_err());
}

#[test]
fn bld_064_a_file_read_back_needs_its_reported_size_and_digest() {
    let reported = file("Anima_0.1.12_amd64.AppImage", b"exact bytes");

    let mut same = Received::new(&reported);
    same.push(b"exact ").unwrap();
    same.push(b"bytes").unwrap();
    assert!(same.finish().is_ok());

    let mut longer = Received::new(&reported);
    longer.push(b"exact bytes").unwrap();
    assert_eq!(
        longer.push(b"!").unwrap_err(),
        "Anima_0.1.12_amd64.AppImage is not the 11 bytes the builder reported."
    );

    let mut shorter = Received::new(&reported);
    shorter.push(b"exact").unwrap();
    assert!(shorter.finish().is_err());

    let mut tampered = Received::new(&reported);
    tampered.push(b"EXACT bytes").unwrap();
    assert_eq!(
        tampered.finish().unwrap_err(),
        "Anima_0.1.12_amd64.AppImage does not have the SHA-256 the builder reported."
    );
}

#[test]
fn bld_064_only_the_sums_bytes_are_kept() {
    let sums = file("SHA256SUMS", b"lines");
    let mut received = Received::new(&sums);
    received.push(b"lines").unwrap();
    assert_eq!(received.finish().unwrap(), b"lines");
    let app = file("a.AppImage", b"app");
    let mut received = Received::new(&app);
    received.push(b"app").unwrap();
    assert_eq!(received.finish().unwrap(), b"");
}

#[test]
fn bld_061_a_job_uploads_only_its_targets_release_files() {
    let version = "0.1.12".parse().unwrap();
    let granted = [
        (
            Target::LinuxX86_64,
            "Anima_0.1.12_amd64.AppImage",
            "application/vnd.appimage",
        ),
        (
            Target::LinuxX86_64,
            "SHA256SUMS",
            "text/plain; charset=utf-8",
        ),
        (
            Target::DarwinUniversal,
            "Anima.app.tar.gz",
            "application/gzip",
        ),
        (
            Target::DarwinUniversal,
            "Anima_0.1.12_universal.dmg",
            "application/x-apple-diskimage",
        ),
        (
            Target::DarwinUniversal,
            "SHA256SUMS",
            "text/plain; charset=utf-8",
        ),
    ];
    for (target, name, kind) in granted {
        assert_eq!(content_type(target, version, name), Some(kind), "{name}");
    }
    let refused = [
        (Target::LinuxX86_64, "Anima.app.tar.gz"),
        (Target::LinuxX86_64, "Anima_0.1.13_amd64.AppImage"),
        (Target::LinuxX86_64, "index.html"),
        (Target::LinuxX86_64, "sha256sums"),
        (Target::DarwinUniversal, "Anima_0.1.12_amd64.AppImage"),
        (Target::DarwinUniversal, "Anima_0.1.11_universal.dmg"),
    ];
    for (target, name) in refused {
        assert_eq!(content_type(target, version, name), None, "{name}");
    }
}
