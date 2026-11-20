//! The ports, the mounts, the networks and the configs of a service: where a container
//! touches the machine and the other parts of the file (K10 Section 6.4.4).

use crate::{
    Node, Refusal, Rule, Settings,
    declared::Declared,
    forms::{is_name, is_target, truth, whole},
    project::Names,
};

/// The address of the machine itself.
const LOCAL: &str = "127.0.0.1";
/// The lowest port of the machine that a file may publish.
const LOWEST_PORT: u64 = 1024;

fn items<'a>(service: &Declared<'a>, key: &str) -> &'a [Node] {
    match service.node.get(key) {
        Some(Node::List(items)) => items,
        _ => &[],
    }
}

fn port(text: Option<&str>, lowest: u64) -> bool {
    text.and_then(whole)
        .is_some_and(|port| (lowest..=65535).contains(&port))
}

/// Each port has a target and a published port as one whole number each. With "Open to the
/// network" off, the address is the machine itself.
///
/// # Errors
/// `port_not_permitted` for a port that breaks the form; `not_at_machine` for an address
/// other than the machine itself while the setting is off.
pub(crate) fn ports(service: &Declared<'_>, settings: &Settings) -> Result<(), Refusal> {
    for entry in items(service, "ports") {
        let text = |key: &str| entry.get(key).and_then(Node::text);
        let protocol = matches!(text("protocol"), None | Some("tcp" | "udp"));
        if !port(text("target"), 1) || !port(text("published"), LOWEST_PORT) || !protocol {
            return Err(service.refusal(Rule::PortNotPermitted));
        }
        match text("host_ip") {
            Some(LOCAL) => {}
            Some(address)
                if settings.open_to_network && address.parse::<std::net::IpAddr>().is_ok() => {}
            None if settings.open_to_network => {}
            Some(_) if settings.open_to_network => {
                return Err(service.refusal(Rule::PortNotPermitted));
            }
            _ => {
                return Err(Refusal::NotAtMachine {
                    service: Some(service.position),
                });
            }
        }
    }
    Ok(())
}

/// A bind mount names exactly `binds/<name>` of the application.
fn is_bind_source(source: &str) -> bool {
    let relative = source.strip_prefix("./").unwrap_or(source);
    relative.strip_prefix("binds/").is_some_and(is_name)
}

/// One mount: its target, when the mount is permitted.
fn mount<'a>(entry: &'a Node, all: &Names<'_>) -> Option<&'a str> {
    let text = |key: &str| entry.get(key).and_then(Node::text);
    let source = text("source")?;
    let sound_source = match text("type")? {
        "volume" => all.volumes.contains(&source),
        "bind" => is_bind_source(source),
        _ => false,
    };
    let read_only = entry
        .get("read_only")
        .is_none_or(|value| value.text().and_then(truth).is_some());
    let target = text("target").filter(|target| is_target(target))?;
    (sound_source && read_only).then_some(target)
}

/// One config of a service: its target, when the config is permitted.
fn config<'a>(entry: &'a Node, all: &Names<'_>) -> Option<&'a str> {
    let text = |key: &str| entry.get(key).and_then(Node::text);
    let known = all.configs.contains(&text("source")?);
    let mode = text("mode").is_none_or(|mode| {
        (3..=4).contains(&mode.len()) && mode.bytes().all(|byte| (b'0'..=b'7').contains(&byte))
    });
    let target = text("target").filter(|target| is_target(target))?;
    (known && mode).then_some(target)
}

/// The networks of a service are networks of this file; an alias is a name.
fn networks(service: &Declared<'_>, all: &Names<'_>) -> bool {
    let known = |name: &str| all.networks.contains(&name);
    match service.node.get("networks") {
        None => true,
        Some(Node::List(names)) => names.iter().all(|name| name.text().is_some_and(known)),
        Some(node) => node
            .entries()
            .unwrap_or(&[])
            .iter()
            .all(|(name, attachment)| {
                let aliases = match attachment.get("aliases") {
                    None => true,
                    Some(Node::Text(alias)) => is_name(alias),
                    Some(Node::List(aliases)) => aliases
                        .iter()
                        .all(|alias| alias.text().is_some_and(is_name)),
                    Some(_) => false,
                };
                known(name) && aliases
            }),
    }
}

/// Checks the mounts, the configs and the networks of a service. Two mounts of one service
/// never have one target.
///
/// # Errors
/// `mount_not_permitted` for a mount or a config that breaks a rule; `value_not_permitted`
/// for a network that is not of this file.
pub(crate) fn checked(service: &Declared<'_>, all: &Names<'_>) -> Result<(), Refusal> {
    let refused = service.refusal(Rule::MountNotPermitted);
    let mut targets: Vec<&str> = Vec::new();
    let mounted = items(service, "volumes")
        .iter()
        .map(|entry| mount(entry, all));
    let configured = items(service, "configs")
        .iter()
        .map(|entry| config(entry, all));
    for target in mounted.chain(configured) {
        let target = target.ok_or(refused)?;
        if targets.contains(&target) {
            return Err(refused);
        }
        targets.push(target);
    }
    if networks(service, all) {
        Ok(())
    } else {
        Err(service.refusal(Rule::ValueNotPermitted))
    }
}
