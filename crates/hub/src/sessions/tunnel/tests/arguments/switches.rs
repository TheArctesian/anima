//! Each device switch reaches the one guacd argument K4 Section 3 maps it to, with guacd's
//! text: security and resize names, and booleans as `true` or empty (RA-021, RA-033).

use super::super::super::arguments::pinned_security;
use super::{DRIVE_ROOT, Desktop, Sources, TARGET, connect_for, connect_values, device};
use crate::devices::{RdpSecurity, ResizeMethod, TransferPolicy};

#[test]
fn k4_3_each_rdp_security_maps_to_guacd_s_name() {
    let mut device = device("desk");
    for (security, name) in [
        (RdpSecurity::Any, "any"),
        (RdpSecurity::Nla, "nla"),
        (RdpSecurity::NlaExt, "nla-ext"),
        (RdpSecurity::Tls, "tls"),
        (RdpSecurity::Vmconnect, "vmconnect"),
        (RdpSecurity::Rdp, "rdp"),
    ] {
        device.rdp.security = security;
        assert_eq!(connect_for(Desktop::Rdp, &device)["security"], name);
    }
}

#[test]
fn k4_3_each_resize_method_maps_to_guacd_s_name() {
    let mut device = device("desk");
    for (resize, name) in [
        (ResizeMethod::DisplayUpdate, "display-update"),
        (ResizeMethod::Reconnect, "reconnect"),
    ] {
        device.rdp.resize = resize;
        assert_eq!(connect_for(Desktop::Rdp, &device)["resize-method"], name);
    }
}

#[test]
fn sec_007_the_certificate_is_checked_unless_the_operator_ignores_it() {
    let mut device = device("desk");
    device.rdp.ignore_certificate = false;
    assert_eq!(connect_for(Desktop::Rdp, &device)["ignore-cert"], "");
    device.rdp.ignore_certificate = true;
    assert_eq!(connect_for(Desktop::Rdp, &device)["ignore-cert"], "true");
}

#[test]
fn k4_3_sound_off_disables_audio_and_the_microphone_is_off_by_default() {
    let mut device = device("desk");
    device.rdp.audio_out = false;
    device.rdp.microphone = false;
    let values = connect_for(Desktop::Rdp, &device);
    assert_eq!(values["disable-audio"], "true");
    assert_eq!(values["enable-audio-input"], "");
}

#[test]
fn ra_021_no_shared_drive_sends_no_drive_values() {
    let mut device = device("desk");
    device.rdp.shared_drive = false;
    let values = connect_for(Desktop::Rdp, &device);
    for name in ["enable-drive", "drive-path", "create-drive-path"] {
        assert_eq!(values[name], "", "{name}");
    }
}

#[test]
fn ra_033_each_transfer_switch_off_disables_its_guacd_side() {
    let cases = [
        (
            "disable-copy",
            TransferPolicy {
                copy: false,
                ..TransferPolicy::default()
            },
        ),
        (
            "disable-paste",
            TransferPolicy {
                paste: false,
                ..TransferPolicy::default()
            },
        ),
        (
            "disable-download",
            TransferPolicy {
                download: false,
                ..TransferPolicy::default()
            },
        ),
        (
            "disable-upload",
            TransferPolicy {
                upload: false,
                ..TransferPolicy::default()
            },
        ),
    ];
    let names = cases.map(|(name, _)| name);
    for (off, policy) in cases {
        let mut device = device("desk");
        device.transfer = policy;
        let values = connect_for(Desktop::Rdp, &device);
        for name in names {
            let want = if name == off { "true" } else { "" };
            assert_eq!(values[name], want, "{name} with {off} off");
        }
    }
}

#[test]
fn ra_033_vnc_copy_and_paste_follow_the_policy() {
    let mut device = device("desk");
    device.transfer = TransferPolicy {
        copy: false,
        paste: false,
        ..TransferPolicy::default()
    };
    let values = connect_for(Desktop::Vnc, &device);
    assert_eq!(values["disable-copy"], "true");
    assert_eq!(values["disable-paste"], "true");
}

#[test]
fn k4_3_an_unset_domain_or_vnc_user_stays_empty() {
    let mut device = device("desk");
    device.rdp.domain = None;
    device.vnc.username = None;
    assert_eq!(connect_for(Desktop::Rdp, &device)["domain"], "");
    assert_eq!(connect_for(Desktop::Vnc, &device)["username"], "");
}

#[test]
fn gw_015_without_a_stored_password_guacd_gets_none_and_may_ask() {
    let device = device("desk");
    for desktop in [Desktop::Rdp, Desktop::Vnc] {
        let sources = Sources {
            device: &device,
            target: TARGET,
            password: None,
            certificate: None,
            drive_root: DRIVE_ROOT,
        };
        let values = connect_values(desktop, &sources).unwrap();
        let sent = super::connect_by_name_of(desktop, &device, values);
        assert_eq!(sent["password"], "", "{desktop:?}");
    }
}

#[test]
fn sec_007_any_becomes_the_tls_security_the_server_chose_and_the_rest_is_kept() {
    for chosen in [RdpSecurity::Tls, RdpSecurity::Nla, RdpSecurity::NlaExt] {
        assert_eq!(pinned_security(RdpSecurity::Any, chosen), chosen);
        for set in [
            RdpSecurity::Tls,
            RdpSecurity::Nla,
            RdpSecurity::NlaExt,
            RdpSecurity::Vmconnect,
        ] {
            assert_eq!(pinned_security(set, chosen), set, "{set:?} is kept");
        }
    }
}
