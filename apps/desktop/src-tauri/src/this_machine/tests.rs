//! The tests use fixed addresses, a loopback listener and harmless programs (`echo`, `false`,
//! `sleep`); none reads this computer's real interfaces or opens an app.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener},
    time::{Duration, Instant},
};

use super::{
    OperatingSystem, ThisMachine, ThisMachineError,
    address::{accepts_connection, is_vpn_address, vpn_address_among},
    name::{first_line, output_of, short_host_name},
};

const PATIENCE: Duration = Duration::from_millis(300);

#[test]
fn the_vpn_range_is_100_64_0_0_slash_10() {
    assert!(is_vpn_address(Ipv4Addr::new(100, 64, 0, 0)));
    assert!(is_vpn_address(Ipv4Addr::new(100, 64, 0, 7)));
    assert!(is_vpn_address(Ipv4Addr::new(100, 127, 255, 255)));
    assert!(!is_vpn_address(Ipv4Addr::new(100, 63, 255, 255)));
    assert!(!is_vpn_address(Ipv4Addr::new(100, 128, 0, 0)));
    assert!(!is_vpn_address(Ipv4Addr::new(192, 168, 1, 20)));
    assert!(!is_vpn_address(Ipv4Addr::new(10, 64, 0, 1)));
}

#[test]
fn the_vpn_range_matches_the_prefix_for_every_sampled_address() {
    // A spread of addresses across the whole IPv4 space, plus every edge of the range.
    let prefix = u32::from(Ipv4Addr::new(100, 64, 0, 0));
    let mask = u32::MAX << (32 - 10);
    let edges = [
        prefix - 1,
        prefix,
        prefix + (1 << 22) - 1,
        prefix + (1 << 22),
    ];
    let spread = (0..=u32::MAX).step_by(65_521);
    for raw in spread.chain(edges) {
        let ip = Ipv4Addr::from(raw);
        assert_eq!(is_vpn_address(ip), raw & mask == prefix, "{ip}");
    }
}

#[test]
fn the_vpn_address_is_the_lowest_one_in_the_range_whatever_the_order() {
    let ips = [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(100, 101, 0, 9)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
        IpAddr::V4(Ipv4Addr::new(100, 64, 0, 7)),
    ];
    let expected = Some(Ipv4Addr::new(100, 64, 0, 7));
    assert_eq!(vpn_address_among(ips), expected);
    assert_eq!(vpn_address_among(ips.into_iter().rev()), expected);
}

#[test]
fn a_computer_off_the_vpn_has_no_vpn_address() {
    let ips = [
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
        IpAddr::V4(Ipv4Addr::new(100, 128, 0, 1)),
        IpAddr::V6("fd7a:115c:a1e0::1".parse().unwrap()),
    ];
    assert_eq!(vpn_address_among(ips), None);
    assert_eq!(vpn_address_among([]), None);
}

#[test]
fn a_listening_port_accepts_a_connection() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    assert!(accepts_connection(listener.local_addr().unwrap(), PATIENCE));
}

#[test]
fn an_address_that_does_not_answer_is_given_up_on_after_its_patience() {
    // TEST-NET-1 (RFC 5737) is never routed: the connection fails at once or times out. A
    // closed loopback port is not used because a program another test spawns can inherit
    // the listener before it is marked close-on-exec, and keep the port open.
    let started = Instant::now();
    assert!(!accepts_connection(
        (Ipv4Addr::new(192, 0, 2, 1), 22).into(),
        PATIENCE
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn the_computer_name_is_the_first_trimmed_line() {
    assert_eq!(
        first_line("Stephen’s MacBook Pro\n".as_bytes()),
        Some("Stephen’s MacBook Pro".to_owned())
    );
    assert_eq!(
        first_line(b"  studio  \nextra\n"),
        Some("studio".to_owned())
    );
    assert_eq!(first_line(b""), None);
    assert_eq!(first_line(b"  \n"), None);
    assert_eq!(first_line(&[0xff, 0xfe, b'\n']), None);
}

#[test]
fn a_host_name_drops_its_domain() {
    assert_eq!(short_host_name("studio.local"), Some("studio".to_owned()));
    assert_eq!(short_host_name(" nas \n"), Some("nas".to_owned()));
    assert_eq!(short_host_name(".local"), None);
    assert_eq!(short_host_name(""), None);
}

#[test]
fn a_name_program_that_succeeds_gives_its_output() {
    assert_eq!(
        output_of("echo", &["studio"], PATIENCE).as_deref(),
        Some(b"studio\n".as_slice())
    );
}

#[test]
fn a_name_program_that_fails_or_is_missing_gives_nothing() {
    assert_eq!(output_of("false", &[], PATIENCE), None);
    assert_eq!(output_of("/nonexistent/anima-scutil", &[], PATIENCE), None);
}

#[test]
fn a_name_program_that_hangs_is_given_up_on_after_its_patience() {
    let started = Instant::now();
    assert_eq!(output_of("sleep", &["5"], Duration::from_millis(50)), None);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn this_machine_crosses_to_the_client_in_k1_terms() {
    let machine = ThisMachine {
        name: "studio".to_owned(),
        user: "stephen".to_owned(),
        os: OperatingSystem::Macos,
        vpn_address: Some(Ipv4Addr::new(100, 64, 0, 7)),
        ssh_listening: true,
    };
    let json = serde_json::to_value(&machine).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "name": "studio",
            "user": "stephen",
            "os": "MACOS",
            "vpnAddress": "100.64.0.7",
            "sshListening": true,
        })
    );
    let off = ThisMachine {
        os: OperatingSystem::Linux,
        vpn_address: None,
        ssh_listening: false,
        ..machine
    };
    let json = serde_json::to_value(&off).unwrap();
    assert_eq!(json["os"], "LINUX");
    assert_eq!(json["vpnAddress"], serde_json::Value::Null);
}

#[test]
fn the_error_crosses_to_the_client_as_a_code_and_fixed_text() {
    let json = serde_json::to_value(ThisMachineError::Failed).unwrap();
    assert_eq!(json["code"], "failed");
    assert_eq!(json["message"], "this computer could not be read");
    let json = serde_json::to_value(ThisMachineError::Unavailable).unwrap();
    assert_eq!(json["code"], "unavailable");
}
