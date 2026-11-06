//! Remote access settings: the MAC address, defaults and the rules (RA-030..033, D-2, D-6).

use super::*;
use super::{fields, new_device};
fn rdp(enabled: bool, username: &str) -> RdpInput {
    RdpInput {
        enabled,
        port: None,
        username: username.into(),
        domain: None,
        security: RdpSecurity::Nla,
        ignore_certificate: false,
        resize: ResizeMethod::DisplayUpdate,
        audio_out: true,
        microphone: false,
        shared_drive: false,
    }
}

fn vnc(port: Option<i32>, username: Option<&str>) -> VncInput {
    VncInput {
        enabled: true,
        port,
        username: username.map(Into::into),
    }
}

fn with_remote(remote: RemoteForm) -> NewDeviceInput {
    NewDeviceInput {
        remote,
        ..new_device("win-01", "10.10.0.9")
    }
}

#[test]
fn db_011_rust_defaults_are_the_k1_defaults() {
    let rdp = RdpSettings::default();
    assert!(!rdp.enabled && !rdp.ignore_certificate && !rdp.microphone && !rdp.shared_drive);
    assert!(rdp.audio_out);
    assert_eq!(
        (rdp.port, rdp.username.as_str(), rdp.domain),
        (3389, "", None)
    );
    assert_eq!(
        (rdp.security, rdp.resize),
        (RdpSecurity::Nla, ResizeMethod::DisplayUpdate)
    );
    let vnc = VncSettings::default();
    assert_eq!((vnc.enabled, vnc.port, vnc.username), (false, 5900, None));
    assert_eq!(MetricsSource::default(), MetricsSource::None);
    let transfer = TransferPolicy::default();
    assert!(transfer.upload && transfer.download && transfer.copy && transfer.paste);
}

#[test]
fn ra_030_enum_text_round_trips_and_unknown_text_is_refused() {
    for security in [
        RdpSecurity::Any,
        RdpSecurity::Nla,
        RdpSecurity::NlaExt,
        RdpSecurity::Tls,
        RdpSecurity::Vmconnect,
        RdpSecurity::Rdp,
    ] {
        assert_eq!(security.as_str().parse(), Ok(security));
    }
    assert_eq!("reconnect".parse(), Ok(ResizeMethod::Reconnect));
    assert_eq!("ssh".parse(), Ok(MetricsSource::Ssh));
    assert!("NLA".parse::<RdpSecurity>().is_err());
}

#[test]
fn ra_030_a_new_device_without_settings_gets_the_defaults() {
    let checked = validate_new_device(&new_device("win-01", "10.10.0.9")).unwrap();
    assert_eq!(checked.remote, RemoteAccess::default());
}

#[test]
fn ra_030_rdp_without_a_port_gets_3389_and_vnc_5900() {
    let form = RemoteForm {
        rdp: Some(rdp(true, " admin ")),
        vnc: Some(vnc(None, None)),
        ..RemoteForm::default()
    };
    let remote = validate_new_device(&with_remote(form)).unwrap().remote;
    assert_eq!(
        (remote.rdp.port, remote.rdp.username.as_str()),
        (3389, "admin")
    );
    assert_eq!(remote.vnc.port, 5900);
}

#[test]
fn ra_030_rdp_username_is_required_only_when_rdp_is_on() {
    let on = RemoteForm {
        rdp: Some(rdp(true, "  ")),
        ..RemoteForm::default()
    };
    let errors = validate_new_device(&with_remote(on)).unwrap_err();
    assert_eq!(fields(&errors), ["rdp.username"]);
    let off = RemoteForm {
        rdp: Some(rdp(false, "")),
        ..RemoteForm::default()
    };
    assert!(validate_new_device(&with_remote(off)).is_ok());
}

#[test]
fn ra_030_rdp_username_is_limited_plain_and_without_spaces() {
    for bad in [&"u".repeat(65), "two words", "line\nbreak"] {
        let form = RemoteForm {
            rdp: Some(rdp(true, bad)),
            ..RemoteForm::default()
        };
        let errors = validate_new_device(&with_remote(form)).unwrap_err();
        assert_eq!(fields(&errors), ["rdp.username"], "{bad:?}");
    }
}

#[test]
fn ra_030_ports_must_be_between_1_and_65535() {
    for bad in [0, -1, 65_536] {
        let form = RemoteForm {
            rdp: Some(RdpInput {
                port: Some(bad),
                ..rdp(false, "")
            }),
            vnc: Some(vnc(Some(bad), None)),
            ..RemoteForm::default()
        };
        let errors = validate_new_device(&with_remote(form)).unwrap_err();
        assert_eq!(fields(&errors), ["rdp.port", "vnc.port"], "{bad}");
    }
    let form = RemoteForm {
        vnc: Some(vnc(Some(65_535), None)),
        ..RemoteForm::default()
    };
    assert!(validate_new_device(&with_remote(form)).is_ok());
}

#[test]
fn ra_030_rdp_domain_is_trimmed_empty_means_none_and_it_is_limited() {
    let domain = |text: Option<&str>| {
        let form = RemoteForm {
            rdp: Some(RdpInput {
                domain: text.map(Into::into),
                ..rdp(false, "")
            }),
            ..RemoteForm::default()
        };
        validate_new_device(&with_remote(form)).map(|device| device.remote.rdp.domain)
    };
    assert_eq!(domain(Some(" CORP ")).unwrap().as_deref(), Some("CORP"));
    assert_eq!(domain(Some("  ")).unwrap(), None);
    assert_eq!(domain(None).unwrap(), None);
    assert!(domain(Some(&"d".repeat(255))).is_ok());
    for bad in [&"d".repeat(256), "corp lan", "corp\u{202e}"] {
        assert_eq!(
            fields(&domain(Some(bad)).unwrap_err()),
            ["rdp.domain"],
            "{bad:?}"
        );
    }
}

#[test]
fn ra_030_vnc_username_is_optional_trimmed_and_limited() {
    let username = |text: Option<&str>| {
        let form = RemoteForm {
            vnc: Some(vnc(None, text)),
            ..RemoteForm::default()
        };
        validate_new_device(&with_remote(form)).map(|device| device.remote.vnc.username)
    };
    assert_eq!(
        username(Some(" viewer ")).unwrap().as_deref(),
        Some("viewer")
    );
    assert_eq!(username(Some(" ")).unwrap(), None);
    assert_eq!(username(None).unwrap(), None);
    assert_eq!(
        fields(&username(Some(&"v".repeat(65))).unwrap_err()),
        ["vnc.username"]
    );
}

#[test]
fn ra_031_a_new_device_reads_a_blank_mac_as_no_address_and_refuses_a_bad_one() {
    let mac_form = |text: &str| RemoteForm {
        mac_address: Some(text.into()),
        ..RemoteForm::default()
    };
    let remote = |text: &str| validate_new_device(&with_remote(mac_form(text)));
    assert_eq!(remote("").unwrap().remote.mac_address, None);
    assert_eq!(
        remote(" AA-BB-0C-DD-EE-F2 ")
            .unwrap()
            .remote
            .mac_address
            .map(|m| m.to_string()),
        Some("aa:bb:0c:dd:ee:f2".to_owned())
    );
    assert_eq!(
        fields(&remote("ff:ff:ff:ff:ff:ff").unwrap_err()),
        ["macAddress"]
    );
}

#[test]
fn ra_031_an_update_leaves_clears_or_sets_the_mac_address() {
    let change = |mac_address: Option<&str>| {
        let input = DeviceChangeInput {
            remote: RemoteForm {
                mac_address: mac_address.map(Into::into),
                ..RemoteForm::default()
            },
            ..DeviceChangeInput::default()
        };
        validate_change(&input).map(|change| change.remote.mac_address)
    };
    assert_eq!(change(None).unwrap(), None);
    assert_eq!(change(Some("")).unwrap(), Some(None));
    assert_eq!(
        change(Some("02:00:5e:00:00:fb"))
            .unwrap()
            .flatten()
            .map(|m| m.to_string()),
        Some("02:00:5e:00:00:fb".to_owned())
    );
    assert_eq!(fields(&change(Some("nope")).unwrap_err()), ["macAddress"]);
}

#[test]
fn ra_030_an_update_with_no_remote_part_changes_nothing_and_a_present_part_is_replaced() {
    assert!(
        validate_change(&DeviceChangeInput::default())
            .unwrap()
            .is_empty()
    );
    let input = DeviceChangeInput {
        remote: RemoteForm {
            vnc: Some(vnc(Some(5901), None)),
            ..RemoteForm::default()
        },
        ..DeviceChangeInput::default()
    };
    let change = validate_change(&input).unwrap();
    assert_eq!(change.remote.rdp, None);
    assert_eq!(change.remote.vnc.map(|vnc| vnc.port), Some(5901));
}

#[test]
fn ra_030_every_remote_error_is_reported_with_the_other_fields_at_once() {
    let form = RemoteForm {
        rdp: Some(RdpInput {
            port: Some(0),
            domain: Some("a b".into()),
            ..rdp(true, "")
        }),
        vnc: Some(vnc(Some(70_000), Some("x y"))),
        mac_address: Some("zz".into()),
        ..RemoteForm::default()
    };
    let mut input = with_remote(form);
    input.name = String::new();
    let errors = validate_new_device(&input).unwrap_err();
    assert_eq!(
        fields(&errors),
        [
            "name",
            "rdp.port",
            "rdp.username",
            "rdp.domain",
            "vnc.port",
            "vnc.username",
            "macAddress"
        ]
    );
}
