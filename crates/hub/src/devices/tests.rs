use std::str::FromStr;

use super::*;
use crate::ids::SiteId;

const HOME: &str = "0190a000-0000-7000-8001-000000000001";

fn home() -> SiteId {
    HOME.parse().unwrap()
}

fn new_device(name: &str, host: &str) -> NewDeviceInput {
    NewDeviceInput {
        site: home(),
        name: name.into(),
        host: host.into(),
        kind: "Proxmox VE node".into(),
        os: OperatingSystem::Linux,
        tags: vec![],
        ssh: None,
    }
}

fn ssh(enabled: bool, port: Option<i32>, username: &str) -> SshInput {
    SshInput {
        enabled,
        port,
        username: username.into(),
        auth: SshAuth::PrivateKey,
    }
}

fn fields(errors: &[FieldError]) -> Vec<&'static str> {
    errors.iter().map(|error| error.field).collect()
}

fn host_error(host: &str) -> Option<FieldError> {
    validate_new_device(&new_device("pve-01", host))
        .err()
        .and_then(|mut e| e.pop())
}

#[test]
fn dev_010_host_accepts_ip_literals_and_host_names() {
    let long_label = "a".repeat(63);
    let long_name = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61)
    );
    for host in [
        "10.10.0.21",
        "fd00::1",
        "pve-01.home.lan",
        "pve-01.home.lan.",
        "ha.home.lan",
        "localhost",
        "  pihole  ",
        long_label.as_str(),
        long_name.as_str(),
    ] {
        let device = validate_new_device(&new_device("pve-01", host)).unwrap();
        assert_eq!(device.host, host.trim(), "{host}");
    }
}

#[test]
fn dev_010_host_rejects_scheme() {
    let error = host_error("http://x").unwrap();
    assert_eq!(error.field, "host");
    assert!(error.message.contains("scheme"));
}

#[test]
fn dev_010_host_rejects_port_path_and_spaces() {
    assert!(host_error("x:22").unwrap().message.contains("port"));
    assert!(
        host_error("pve-01.home.lan/admin")
            .unwrap()
            .message
            .contains("path")
    );
    assert!(host_error("has space").unwrap().message.contains("spaces"));
}

#[test]
fn dev_010_host_rejects_malformed_names() {
    let label64 = format!("{}.lan", "a".repeat(64));
    let name254 = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(62)
    );
    for host in [
        "",
        "   ",
        "-bad.lan",
        "bad-.lan",
        "a..b",
        ".lan",
        "..",
        "a_b.lan",
        "ünï.lan",
        label64.as_str(),
        name254.as_str(),
    ] {
        assert_eq!(host_error(host).map(|e| e.field), Some("host"), "{host:?}");
    }
}

#[test]
fn dev_010_host_rejects_a_malformed_ip_address() {
    assert_eq!(host_error("10.10.0.256").map(|e| e.field), Some("host"));
    assert_eq!(host_error("10.10.0").map(|e| e.field), Some("host"));
}

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

#[test]
fn dev_010_enums_round_trip_through_database_text() {
    for os in [
        OperatingSystem::Linux,
        OperatingSystem::Macos,
        OperatingSystem::Windows,
        OperatingSystem::Other,
    ] {
        assert_eq!(os.as_str().parse::<OperatingSystem>().unwrap(), os);
    }
    for auth in [SshAuth::Password, SshAuth::PrivateKey] {
        assert_eq!(auth.as_str().parse::<SshAuth>().unwrap(), auth);
    }
    assert_eq!(SshAuth::PrivateKey.as_str(), "private_key");
    assert!("beos".parse::<OperatingSystem>().is_err());
    assert!("Linux".parse::<OperatingSystem>().is_err());
    assert!("".parse::<SshAuth>().is_err());
}

#[test]
fn dev_010_graphql_enum_names_follow_the_schema() {
    use async_graphql::EnumType;
    let os: Vec<_> = OperatingSystem::items()
        .iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(os, ["LINUX", "MACOS", "WINDOWS", "OTHER"]);
    let auth: Vec<_> = SshAuth::items().iter().map(|item| item.name).collect();
    assert_eq!(auth, ["PASSWORD", "PRIVATE_KEY"]);
}

#[test]
fn api_006_cursor_round_trips_any_name() {
    for name in ["pve-01", "", "docker host", "ünï-ζ", "a.b/c"] {
        assert_eq!(decode_cursor(&encode_cursor(name)).unwrap(), name);
    }
    assert_ne!(encode_cursor("pve-01"), "pve-01");
}

#[test]
fn api_006_bad_cursor_is_an_after_field_error() {
    for bad in [
        "",
        "pve-01",
        "n1.zz",
        "n1.abc",
        "n1.ff",
        "n2.7076652d3031",
        "n1.é",
    ] {
        let error = decode_cursor(bad).unwrap_err();
        assert_eq!(error.field, "after", "{bad:?}");
    }
}

#[test]
fn api_006_first_must_be_1_to_200() {
    assert_eq!(page_size(1).unwrap(), 1);
    assert_eq!(page_size(200).unwrap(), 200);
    for bad in [0, -5, 201, i32::MAX, i32::MIN] {
        assert_eq!(page_size(bad).unwrap_err().field, "first", "{bad}");
    }
}

#[test]
fn api_006_split_page_reports_whether_more_rows_follow() {
    let more = split_page(vec![1, 2, 3], 2);
    assert_eq!((more.rows, more.has_next_page), (vec![1, 2], true));
    let exact = split_page(vec![1, 2], 2);
    assert_eq!((exact.rows, exact.has_next_page), (vec![1, 2], false));
    let none = split_page(Vec::<i32>::new(), 2);
    assert_eq!((none.rows, none.has_next_page), (vec![], false));
}

#[test]
fn api_006_in_memory_pages_of_two_cover_every_device_once_in_name_order() {
    let names = [
        "docker-host",
        "home-assistant",
        "ollama",
        "pihole",
        "pve-01",
    ];
    let mut seen = Vec::new();
    let mut after: Option<String> = None;
    let mut pages = 0;
    loop {
        let page = page_after(&names, |name| name, after.as_deref(), 2);
        pages += 1;
        seen.extend(page.rows.iter().map(|name| (*name).to_owned()));
        if !page.has_next_page {
            break;
        }
        let last = page.rows.last().unwrap();
        after = Some(decode_cursor(&encode_cursor(last)).unwrap());
    }
    assert_eq!(seen, names);
    assert_eq!(pages, 3);
}

#[test]
fn api_006_in_memory_page_after_the_last_name_is_empty() {
    let names = ["a", "b"];
    let page = page_after(&names, |name| name, Some("b"), 5);
    assert!(page.rows.is_empty() && !page.has_next_page);
    let from_gap = page_after(&names, |name| name, Some("aa"), 5);
    assert_eq!(from_gap.rows, [&"b"]);
}

#[test]
fn error_names_what_was_not_found() {
    let error = Error::NotFound(Missing::Site(home()));
    assert_eq!(error.to_string(), format!("site {HOME} not found"));
}
