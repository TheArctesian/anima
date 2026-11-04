use std::str::FromStr;

use super::*;
use super::{fields, new_device, ssh};

#[test]
fn dev_010_name_is_trimmed_and_limited_to_64_characters() {
    assert_eq!(
        validate_new_device(&new_device("  pve-01 ", "10.10.0.2"))
            .unwrap()
            .name,
        "pve-01"
    );
    let long = "n".repeat(64);
    assert!(validate_new_device(&new_device(&long, "10.10.0.2")).is_ok());
    for bad in ["", "   ", &"n".repeat(65)] {
        let errors = validate_new_device(&new_device(bad, "10.10.0.2")).unwrap_err();
        assert_eq!(fields(&errors), ["name"], "{bad:?}");
    }
}

#[test]
fn dev_010_kind_is_trimmed_defaults_to_server_and_is_limited() {
    let mut input = new_device("pve-01", "10.10.0.2");
    input.kind = "  VM 101 on pve-01 ".into();
    assert_eq!(
        validate_new_device(&input).unwrap().kind,
        "VM 101 on pve-01"
    );
    input.kind = "  ".into();
    assert_eq!(validate_new_device(&input).unwrap().kind, "server");
    input.kind = "k".repeat(65);
    assert_eq!(fields(&validate_new_device(&input).unwrap_err()), ["kind"]);
}

#[test]
fn dev_010_tags_are_trimmed_and_deduplicated_in_first_seen_order() {
    let mut input = new_device("pve-01", "10.10.0.2");
    input.tags = ["proxmox", " infra ", "proxmox", "infra", "vm"]
        .map(String::from)
        .to_vec();
    assert_eq!(
        validate_new_device(&input).unwrap().tags,
        ["proxmox", "infra", "vm"]
    );
}

#[test]
fn dev_010_tags_reject_empty_and_over_32_characters() {
    let mut input = new_device("pve-01", "10.10.0.2");
    for bad in ["", "  ", &"t".repeat(33)] {
        input.tags = vec!["ok".into(), bad.into()];
        assert_eq!(
            fields(&validate_new_device(&input).unwrap_err()),
            ["tags"],
            "{bad:?}"
        );
    }
    input.tags = vec!["t".repeat(32)];
    assert!(validate_new_device(&input).is_ok());
}

#[test]
fn dev_010_ssh_defaults_to_off_port_22_private_key_when_not_given() {
    let device = validate_new_device(&new_device("pve-01", "10.10.0.2")).unwrap();
    assert_eq!(
        device.ssh,
        SshSettings {
            enabled: false,
            port: 22,
            username: String::new(),
            auth: SshAuth::PrivateKey
        }
    );
}

#[test]
fn dev_010_ssh_port_must_be_1_to_65535_and_defaults_to_22() {
    let mut input = new_device("pve-01", "10.10.0.2");
    for (port, ok) in [
        (Some(0), false),
        (Some(-1), false),
        (Some(65536), false),
        (Some(1), true),
        (Some(65535), true),
    ] {
        input.ssh = Some(ssh(true, port, "operator"));
        let result = validate_new_device(&input);
        assert_eq!(result.is_ok(), ok, "{port:?}");
        if let Err(errors) = result {
            assert_eq!(fields(&errors), ["ssh.port"]);
        }
    }
    input.ssh = Some(ssh(true, None, "operator"));
    assert_eq!(validate_new_device(&input).unwrap().ssh.port, 22);
}

#[test]
fn dev_010_ssh_username_is_required_only_when_enabled_and_has_no_spaces() {
    let mut input = new_device("pve-01", "10.10.0.2");
    input.ssh = Some(ssh(true, Some(22), "  "));
    assert_eq!(
        fields(&validate_new_device(&input).unwrap_err()),
        ["ssh.username"]
    );
    input.ssh = Some(ssh(true, Some(22), "two words"));
    assert_eq!(
        fields(&validate_new_device(&input).unwrap_err()),
        ["ssh.username"]
    );
    input.ssh = Some(ssh(false, Some(22), ""));
    assert!(validate_new_device(&input).is_ok());
    input.ssh = Some(ssh(true, Some(22), " operator "));
    assert_eq!(
        validate_new_device(&input).unwrap().ssh.username,
        "operator"
    );
}

#[test]
fn dev_010_all_field_errors_are_reported_together() {
    let mut input = new_device("", "http://x");
    input.tags = vec![String::new()];
    input.ssh = Some(ssh(true, Some(0), ""));
    let errors = validate_new_device(&input).unwrap_err();
    assert_eq!(
        fields(&errors),
        ["name", "host", "tags", "ssh.port", "ssh.username"]
    );
}

#[test]
fn dev_010_change_checks_only_the_fields_present() {
    let empty = validate_change(&DeviceChangeInput::default()).unwrap();
    assert_eq!(empty, DeviceChange::default());
    let change = validate_change(&DeviceChangeInput {
        name: Some(" docker-host ".into()),
        maintenance: Some(true),
        tags: Some(vec!["vm".into(), "vm".into()]),
        ..DeviceChangeInput::default()
    })
    .unwrap();
    assert_eq!(change.name.as_deref(), Some("docker-host"));
    assert_eq!(change.maintenance, Some(true));
    assert_eq!(change.tags, Some(vec!["vm".to_owned()]));
    assert_eq!(change.host, None);
}

#[test]
fn dev_010_change_reports_every_bad_present_field() {
    let errors = validate_change(&DeviceChangeInput {
        name: Some(String::new()),
        host: Some("x:22".into()),
        ssh: Some(ssh(true, Some(70000), "")),
        ..DeviceChangeInput::default()
    })
    .unwrap_err();
    assert_eq!(
        fields(&errors),
        ["name", "host", "ssh.port", "ssh.username"]
    );
}

#[test]
fn vpn_001_subnet_must_be_a_network_address() {
    let site = validate_new_site(&NewSiteInput {
        name: " home ".into(),
        subnet: "10.10.0.0/24".into(),
    })
    .unwrap();
    assert_eq!(site.name, "home");
    assert_eq!(site.subnet, ipnet::IpNet::from_str("10.10.0.0/24").unwrap());
    let errors = validate_new_site(&NewSiteInput {
        name: "home".into(),
        subnet: "10.10.0.5/24".into(),
    })
    .unwrap_err();
    assert_eq!(fields(&errors), ["subnet"]);
    assert!(errors[0].message.contains("10.10.0.0/24"));
}

#[test]
fn vpn_001_subnet_rejects_text_that_is_not_a_cidr() {
    for bad in ["", "10.10.0.0", "10.10.0.0/33", "home"] {
        let errors = validate_new_site(&NewSiteInput {
            name: "home".into(),
            subnet: bad.into(),
        })
        .unwrap_err();
        assert_eq!(fields(&errors), ["subnet"], "{bad:?}");
    }
    assert!(
        validate_new_site(&NewSiteInput {
            name: "v6".into(),
            subnet: "fd00::/64".into()
        })
        .is_ok()
    );
}

#[test]
fn vpn_001_site_name_is_1_to_64_characters_and_errors_come_together() {
    let errors = validate_new_site(&NewSiteInput {
        name: " ".into(),
        subnet: "x".into(),
    })
    .unwrap_err();
    assert_eq!(fields(&errors), ["name", "subnet"]);
    assert!(
        validate_new_site(&NewSiteInput {
            name: "s".repeat(65),
            subnet: "10.0.0.0/8".into()
        })
        .is_err()
    );
}
