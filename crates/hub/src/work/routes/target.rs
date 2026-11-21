//! The target of a route: the port of an application on a device, fixed when the name is added
//! (SF-NEW-routetarget). It is never a port of a session or of Anima itself, and never an
//! address that the hub must not touch. Pure.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::fleet::targets::{AddressPolicy, on_link};

/// The ports of the sessions (SSH, VNC, RDP): never the port of an application (Section 12.3).
const SESSION_PORTS: [u16; 3] = [22, 5900, 3389];
/// The ports of Anima on a machine: the local hub (8443), `guacd` (4822) and the link (the
/// tunnel on 41641, STUN on 3478). The daemon listens on no port: only on its Unix socket.
const ANIMA_PORTS: [u16; 4] = [8443, 4822, 41641, 3478];
/// The ports that control the engine of the containers: the Docker API (2375, 2376) and the
/// kubelet (10250). Whoever reaches one controls the machine (SF-NEW-revroutetarget).
const ENGINE_PORTS: [u16; 3] = [2375, 2376, 10250];
/// The ports of databases (PostgreSQL, MySQL, Redis, MongoDB) and of Prometheus (9090): never a
/// page of an application for the public.
const SERVICE_PORTS: [u16; 5] = [5432, 3306, 6379, 27017, 9090];
/// The address of the metadata service of the clouds.
const METADATA: IpAddr = IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254));

/// Why a target is refused. Holds no address (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TargetRefusal {
    #[error("the port of an application is 1 to 65535")]
    Port,
    #[error("the port is a port of a session (22, 5900, 3389), never of an application")]
    SessionPort,
    #[error("the port is a port of Anima on the machine")]
    AnimaPort,
    #[error("the port controls the engine of the containers (2375, 2376, 10250)")]
    EnginePort,
    #[error("the port is a port of a database or of metrics, never of a public page")]
    ServicePort,
    #[error("the device resolves to an address that the hub may not reach for an application")]
    Address,
    #[error("the target is a listener of the hub itself")]
    Hub,
}

/// Checks the port of an application.
///
/// # Errors
/// [`TargetRefusal`] for port 0, a session port, a port of Anima, of the engine, of a database
/// or of metrics.
pub fn check_port(port: i64) -> Result<u16, TargetRefusal> {
    let port = u16::try_from(port)
        .ok()
        .filter(|port| *port > 0)
        .ok_or(TargetRefusal::Port)?;
    if SESSION_PORTS.contains(&port) {
        return Err(TargetRefusal::SessionPort);
    }
    if ANIMA_PORTS.contains(&port) {
        return Err(TargetRefusal::AnimaPort);
    }
    if ENGINE_PORTS.contains(&port) {
        return Err(TargetRefusal::EnginePort);
    }
    if SERVICE_PORTS.contains(&port) {
        return Err(TargetRefusal::ServicePort);
    }
    Ok(port)
}

/// Checks the address that the route will connect to, as resolved once when the name is added.
/// Its port must pass [`check_port`], also on a link address. A link address is reached through
/// the link only. Each other address must pass the hub's address rule (no loopback, link-local
/// or metadata address) and must not be one of `hub_listeners`.
///
/// # Errors
/// The refusal of [`check_port`], [`TargetRefusal::Address`] or [`TargetRefusal::Hub`].
pub fn check_address(
    target: SocketAddr,
    policy: &AddressPolicy,
    hub_listeners: &[SocketAddr],
) -> Result<SocketAddr, TargetRefusal> {
    check_port(i64::from(target.port()))?;
    let ip = target.ip().to_canonical();
    if on_link(ip) {
        return Ok(target);
    }
    if ip.is_loopback() || ip == METADATA || !policy.permits(ip) {
        return Err(TargetRefusal::Address);
    }
    let hub = hub_listeners.iter().any(|listener| {
        listener.port() == target.port()
            && (listener.ip().is_unspecified() || listener.ip().to_canonical() == ip)
    });
    if hub {
        return Err(TargetRefusal::Hub);
    }
    Ok(target)
}
