use super::*;
use super::{host_error, new_device};

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
fn dev_010_host_is_kept_as_written_including_case() {
    let device = validate_new_device(&new_device("pve-01", " PVE-01.Home.LAN ")).unwrap();
    assert_eq!(device.host, "PVE-01.Home.LAN");
}
