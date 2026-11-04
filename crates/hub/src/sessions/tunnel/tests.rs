//! Unit tests of the pure tunnel rules: `connect` values, the client and gateway rules and the
//! end table (WP-10 T4). The tunnel I/O is tested in `tests/it/sessions`.

mod arguments;
mod client;
mod ending;
mod gateway;
mod size;

use chrono::DateTime;
use guac_protocol::{Decoder, Instruction};
use uuid::Uuid;

use super::relay::Relay;
use crate::devices::{Device, OperatingSystem, SshAuth, SshSettings};
use crate::ids::{DeviceId, SiteId};
use crate::sessions::{HostKey, PublicKey};

/// A device with SSH on `port` as user `ops`.
fn device(host: &str, port: u16) -> Device {
    Device {
        id: DeviceId::from(Uuid::nil()),
        site: SiteId::from(Uuid::nil()),
        name: "db".to_owned(),
        host: host.to_owned(),
        ip: None,
        kind: "server".to_owned(),
        os: OperatingSystem::Linux,
        tags: Vec::new(),
        maintenance: false,
        ssh: SshSettings {
            enabled: true,
            port,
            username: "ops".to_owned(),
            auth: SshAuth::Password,
        },
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    }
}

/// The accepted ed25519 host key from the sessions test keys.
fn host_key() -> HostKey {
    HostKey {
        public_key: PublicKey::from_base64(super::super::tests::ED25519).unwrap(),
        first_seen_at: DateTime::UNIX_EPOCH,
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
