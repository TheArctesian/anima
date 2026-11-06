//! Whether an ICMP check that never answered may repair its method, which of the device's
//! ports it tries, and what the repaired check and its log entries say (CHK-013, R-064).
//! Pure: the facts, the device and the names come in as parameters.

use serde_json::{Value, json};

use crate::{
    checks::{config::CheckConfig, store::repairs::Facts},
    devices::Device,
    status::Status,
};

/// The connect limit of each try and of the repaired check, in milliseconds.
pub(super) const TIMEOUT_MS: u32 = 5000;
/// The longest check name the database accepts, in characters.
const NAME_MAX: usize = 200;

/// Whether the check is a repair candidate: an ICMP check, not inverted, Down after its
/// retries, that never answered, that the operator never edited, that was never tried, not
/// paused and not in maintenance. An inverted check that is Down did get replies, so its
/// method works.
pub(super) fn is_candidate(facts: &Facts, in_maintenance: bool) -> bool {
    matches!(facts.config, Some(CheckConfig::Icmp { .. }))
        && !facts.settings.invert
        && facts.status == Status::Down
        && facts.failures > u32::from(facts.settings.retries)
        && !facts.answered
        && !facts.operator_edited
        && facts.tried_at.is_none()
        && !facts.paused
        && !in_maintenance
}

/// A connection protocol of a device, whose port a repaired check can watch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Protocol {
    Ssh,
}

impl Protocol {
    const fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
        }
    }

    const fn usual_port(self) -> u16 {
        match self {
            Self::Ssh => 22,
        }
    }
}

/// One enabled connection port of the device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Port {
    pub(super) protocol: Protocol,
    pub(super) number: u16,
}

impl Port {
    /// The TCP check that watches this port.
    pub(super) const fn config(self) -> CheckConfig {
        CheckConfig::Tcp {
            port: self.number,
            timeout_ms: TIMEOUT_MS,
        }
    }

    /// "SSH port" on the protocol's usual port, else "SSH port 2222".
    fn check_name(self) -> String {
        let label = self.protocol.label();
        if self.number == self.protocol.usual_port() {
            format!("{label} port")
        } else {
            format!("{label} port {}", self.number)
        }
    }
}

/// The device's enabled connection ports in the order a repair tries them: SSH first. RDP
/// and VNC join this list when devices have them.
pub(super) fn ports(device: &Device) -> Vec<Port> {
    [device.ssh.enabled.then_some(Port {
        protocol: Protocol::Ssh,
        number: device.ssh.port,
    })]
    .into_iter()
    .flatten()
    .collect()
}

/// `text` cut to its first `max` characters.
fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// The repaired check's name (D-5): the port's name, or "<that> (was <old>)" when another
/// check of the device has it, cut to 200 characters. When that is taken too, a number
/// follows, so the write never fails on the unique name and retries forever.
pub(super) fn repaired_name(port: Port, old: &str, taken: &[String]) -> String {
    let free = |name: &str| !taken.iter().any(|other| other == name);
    let base = port.check_name();
    if free(&base) {
        return base;
    }
    let was = cut(&format!("{base} (was {old})"), NAME_MAX);
    if free(&was) {
        return was;
    }
    let mut number = 2_usize;
    loop {
        let suffix = format!(" {number}");
        let name = format!("{}{suffix}", cut(&was, NAME_MAX - suffix.len()));
        if free(&name) {
            return name;
        }
        number += 1;
    }
}

/// The Info event of a repair: device, protocol and port only (LOG-007).
pub(super) fn repaired_message(device: &str, port: Port) -> String {
    format!(
        "Ping never answered on {device}; {} port {} does, so its check now uses TCP {}.",
        port.protocol.label(),
        port.number,
        port.number
    )
}

/// The Warning event when no port answered, or the device has none enabled.
pub(super) fn silent_message(device: &str) -> String {
    format!("Ping never answered on {device}, and no connection port answers either.")
}

/// A check's method for the audit entry: its type, configuration and name. ICMP and TCP
/// configurations hold no secret.
fn method(name: &str, config: &CheckConfig) -> Value {
    json!({
        "type": config.kind().as_sql(),
        "config": serde_json::to_value(config).unwrap_or(Value::Null),
        "name": name,
    })
}

/// The audit change of a repair (D-4): the hub did it, and the method before and after.
pub(super) fn audit_change(before: (&str, &CheckConfig), after: (&str, &CheckConfig)) -> Value {
    json!({
        "repairedBy": "hub",
        "before": method(before.0, before.1),
        "after": method(after.0, after.1),
    })
}
