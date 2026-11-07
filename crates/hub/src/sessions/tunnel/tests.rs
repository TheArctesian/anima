//! Tests of the tunnel: the pure rules (`connect` values, the client and gateway rules and the
//! end table) and the tunnel over real sockets (`wire`).

mod arguments;
mod client;
mod ending;
mod gateway;
mod rdp_vnc_args;
mod size;
mod wire;

use chrono::DateTime;
use guac_protocol::{Decoder, Instruction};
use uuid::Uuid;

use super::relay::Relay;
use crate::devices::{
    Device, MetricsSource, OperatingSystem, RdpSecurity, RdpSettings, ResizeMethod, SshAuth,
    SshSettings, TransferPolicy, VncSettings,
};
use crate::ids::{DeviceId, SiteId};

/// The device's id: its shared drive's folder name.
const DEVICE_ID: &str = "0190a000-0000-7000-8002-0000000000aa";

/// A device at `host` with RDP on port 3390 as `ops` in `LAB` and VNC on port 5901 as
/// `viewer`; SSH is off. Every RDP switch is on, so each value K4 Section 3 sets is set.
fn device(host: &str) -> Device {
    Device {
        id: DEVICE_ID.parse::<DeviceId>().unwrap(),
        site: SiteId::from(Uuid::nil()),
        name: "desk".to_owned(),
        host: host.to_owned(),
        ip: None,
        kind: "workstation".to_owned(),
        os: OperatingSystem::Windows,
        tags: Vec::new(),
        maintenance: false,
        ssh: SshSettings {
            enabled: false,
            port: 22,
            username: String::new(),
            auth: SshAuth::Password,
        },
        rdp: RdpSettings {
            enabled: true,
            port: 3390,
            username: "ops".to_owned(),
            domain: Some("LAB".to_owned()),
            security: RdpSecurity::NlaExt,
            ignore_certificate: true,
            resize: ResizeMethod::Reconnect,
            audio_out: true,
            microphone: true,
            shared_drive: true,
        },
        vnc: VncSettings {
            enabled: true,
            port: 5901,
            username: Some("viewer".to_owned()),
        },
        mac_address: None,
        metrics_source: MetricsSource::default(),
        transfer: TransferPolicy::default(),
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    }
}

/// The one instruction `wire` holds, decoded as the tunnel task decodes it.
fn decoded(wire: &str) -> Instruction {
    let mut instructions = Decoder::new().feed(wire.as_bytes()).unwrap();
    assert_eq!(instructions.len(), 1, "one instruction in {wire:?}");
    instructions.remove(0)
}

/// A relay after guacd has asked for `names` with `required`.
fn prompted(names: &[&str]) -> Relay {
    let mut relay = Relay::default();
    relay.on_gateway(Instruction::new("required", names.iter().copied()));
    relay
}
