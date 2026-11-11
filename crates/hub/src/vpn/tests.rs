//! Tests of the vpn domain: the status parser, the picture, the warnings. Fixture expiries are
//! written relative to the test clock, `2030-05-05T12:00:00Z`.

mod peers;
mod picture;
mod proxy;
mod reader;
mod store;
mod warnings;

use std::{collections::HashSet, net::IpAddr};

use chrono::{DateTime, TimeZone as _, Utc};
use ipnet::IpNet;

use super::{
    model::Picture,
    peers::{Status, parse},
    picture::{Inputs, build},
};
use crate::{
    devices::{
        Device, MetricsSource, OperatingSystem, RdpSettings, SshSettings, TransferPolicy,
        VncSettings,
    },
    ids::{DeviceId, SiteId},
};

/// The test clock, as `tests/it/hub.rs` `start()` has it.
fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

/// The text of the fixture `name`.
fn fixture(name: &str) -> String {
    let path = format!(
        "{}/src/vpn/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

/// The parsed fixture `name`.
fn status(name: &str) -> Status {
    parse(fixture(name).as_bytes()).unwrap()
}

/// The fixture as JSON, to change one field before parsing.
fn json(name: &str) -> serde_json::Value {
    serde_json::from_str(&fixture(name)).unwrap()
}

/// The peer of `json` whose host name is `host`.
fn peer_mut<'a>(json: &'a mut serde_json::Value, host: &str) -> &'a mut serde_json::Value {
    json["Peer"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .find(|peer| peer["HostName"] == host)
        .unwrap()
}

fn reparse(json: &serde_json::Value) -> Status {
    parse(json.to_string().as_bytes()).unwrap()
}

fn site(n: u128, subnet: &str) -> (SiteId, IpNet) {
    (uuid::Uuid::from_u128(n).into(), subnet.parse().unwrap())
}

/// The sites `home` (10.10.0.0/24, id 1) and `office1` (10.20.0.0/24, id 2).
fn sites() -> Vec<(SiteId, IpNet)> {
    vec![site(1, "10.10.0.0/24"), site(2, "10.20.0.0/24")]
}

/// A device with id `n`, name `name`, host `host` and, when it is an address, that IP.
fn device(n: u128, name: &str, host: &str) -> Device {
    Device {
        id: device_id(n),
        site: uuid::Uuid::from_u128(1).into(),
        name: name.to_owned(),
        host: host.to_owned(),
        ip: host.parse::<IpAddr>().ok(),
        kind: "Server".to_owned(),
        os: OperatingSystem::Linux,
        tags: Vec::new(),
        maintenance: false,
        ssh: SshSettings::default(),
        rdp: RdpSettings::default(),
        vnc: VncSettings::default(),
        mac_address: None,
        metrics_source: MetricsSource::default(),
        session_limit: None,
        record_sessions: false,
        transfer: TransferPolicy::default(),
        distribution: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn device_id(n: u128) -> DeviceId {
    uuid::Uuid::from_u128(n).into()
}

/// The picture of `status` with `devices`, the two sites and the ping-checked devices.
fn picture_with(status: &Status, devices: &[Device], ping_checked: &[DeviceId]) -> Picture {
    let ping_checked: HashSet<DeviceId> = ping_checked.iter().copied().collect();
    build(&Inputs {
        status,
        sites: &sites(),
        devices,
        ping_checked: &ping_checked,
        now: now(),
    })
    .unwrap()
}

fn picture(status: &Status) -> Picture {
    picture_with(status, &[], &[])
}

/// The node of `picture` named `name`.
fn node<'a>(picture: &'a Picture, name: &str) -> &'a super::VpnNode {
    picture
        .nodes
        .iter()
        .find(|node| node.name == name)
        .unwrap_or_else(|| panic!("no node {name}"))
}
