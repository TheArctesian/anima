//! The address that `install` was given stays through an update.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use super::rig;
use crate::units::install::{self, Request};

#[test]
fn an_update_keeps_the_address_that_install_was_given() {
    let rig = rig();
    let file = rig
        .folder
        .path()
        .join("home/Library/LaunchAgents/ing.anima.hub.plist");
    let listen = "127.0.0.1:9000".parse().unwrap();
    let request = Request {
        listen: Some(listen),
        ..rig.request()
    };
    install::install(&request, &rig.source, |_| Ok(true)).unwrap();
    rig.new_programs();

    assert!(rig.update().0);

    assert!(
        fs::read_to_string(&file)
            .unwrap()
            .contains("127.0.0.1:9000")
    );
    assert_eq!(
        fs::metadata(rig.folders.listen_file())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn an_install_without_an_address_and_a_remove_forget_the_recorded_one() {
    let rig = rig();
    let request = Request {
        listen: Some("127.0.0.1:9000".parse().unwrap()),
        ..rig.request()
    };
    install::install(&request, &rig.source, |_| Ok(true)).unwrap();

    rig.install();
    assert!(!rig.folders.listen_file().exists());

    install::install(&request, &rig.source, |_| Ok(true)).unwrap();
    install::remove(&rig.request(), |_| Ok(true)).unwrap();
    assert!(!rig.folders.listen_file().exists());
}

#[test]
fn a_recorded_address_that_is_not_loopback_is_not_used() {
    let rig = rig();
    rig.install();
    for text in ["0.0.0.0:8443", "10.0.0.1:8443", "garbage", ""] {
        fs::write(rig.folders.listen_file(), text).unwrap();
        assert_eq!(crate::units::listen::recorded(&rig.folders), None, "{text}");
    }
    fs::write(rig.folders.listen_file(), "[::1]:7000\n").unwrap();
    assert_eq!(
        crate::units::listen::recorded(&rig.folders),
        Some("[::1]:7000".parse().unwrap())
    );
}
