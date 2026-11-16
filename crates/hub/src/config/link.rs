//! The link mode (one-click plan, item S5): the hub is reached through `anima-link`, which
//! permits devices by key, and not through a VPN. `ANIMA_LINK_SOCKET` turns it on. The hub then
//! listens on loopback and on that socket only (NET-003), and its permitted clients are this
//! computer and the devices of the link (NET-001).

use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    path::PathBuf,
};

use ipnet::{IpNet, Ipv4Net, Ipv6Net};

use super::{Error, PERMITTED_NETWORKS, optional_socket};

pub(super) const LINK_SOCKET: &str = "ANIMA_LINK_SOCKET";

/// Every link address: the name `anima-link` gives a permitted device key in place of a
/// network address (`link/devices/address.go` holds the same range). No packet goes to one.
pub const LINK_DEVICES: Ipv6Net =
    Ipv6Net::new_assert(Ipv6Addr::new(0xfd41, 0x4e49, 0x4d41, 0, 0, 0, 0, 0), 48);

/// The socket of the link mode, or `None` when the hub is not in it. In the link mode a
/// `listen` address off loopback is refused, and so is a list of permitted networks: the link
/// permits devices by key.
pub(super) fn socket(
    env: &BTreeMap<String, String>,
    listen: SocketAddr,
) -> Result<Option<PathBuf>, Error> {
    let Some(socket) = optional_socket(env, LINK_SOCKET)? else {
        return Ok(None);
    };
    if !listen.ip().to_canonical().is_loopback() {
        return Err(Error::LinkListen { listen });
    }
    if env
        .get(PERMITTED_NETWORKS)
        .is_some_and(|set| !set.is_empty())
    {
        return Err(Error::LinkNetworks);
    }
    Ok(Some(socket))
}

/// The permitted networks of the link mode: this computer and the devices of the link.
pub(super) fn permitted_networks() -> Vec<IpNet> {
    vec![
        IpNet::V4(Ipv4Net::new_assert(Ipv4Addr::new(127, 0, 0, 0), 8)),
        IpNet::V6(Ipv6Net::new_assert(Ipv6Addr::LOCALHOST, 128)),
        IpNet::V6(LINK_DEVICES),
    ]
}

/// The messages of the link mode's two refusals, for [`Error`].
pub(super) const LISTEN_RULE: &str = "with ANIMA_LINK_SOCKET the hub listens on loopback and \
                                      the link only (NET-003)";
pub(super) const NETWORKS_RULE: &str = "must not be set with ANIMA_LINK_SOCKET: the link \
                                        permits devices by key (NET-001)";

#[cfg(test)]
mod tests {
    use super::super::{Error, LISTEN, from_env, tests::env};
    use super::*;

    const SOCKET: &str = "/run/anima/link.sock";

    #[test]
    fn without_the_socket_the_hub_is_not_in_the_link_mode_and_nothing_changes() {
        for unset in [env(&[]), env(&[(LINK_SOCKET, "")])] {
            let config = from_env(&unset).unwrap();
            assert_eq!(config.link_socket, None);
            assert_eq!(
                config.permitted_networks,
                vec!["100.64.0.0/10".parse().unwrap()]
            );
        }
        let vpn = env(&[
            (LISTEN, "100.64.0.1:443"),
            (PERMITTED_NETWORKS, "10.0.0.0/8"),
        ]);
        let config = from_env(&vpn).unwrap();
        assert_eq!(config.listen, "100.64.0.1:443".parse().unwrap());
        assert_eq!(
            config.permitted_networks,
            vec!["10.0.0.0/8".parse().unwrap()]
        );
    }

    #[test]
    fn net_001_in_the_link_mode_the_clients_are_this_computer_and_the_links_devices() {
        let config = from_env(&env(&[(LINK_SOCKET, SOCKET)])).unwrap();
        assert_eq!(config.link_socket, Some(PathBuf::from(SOCKET)));
        let permitted: Vec<String> = config
            .permitted_networks
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(permitted, ["127.0.0.0/8", "::1/128", "fd41:4e49:4d41::/48"]);
    }

    #[test]
    fn net_003_in_the_link_mode_the_hub_listens_on_loopback_only() {
        for loopback in ["127.0.0.1:8443", "[::1]:8443", "[::ffff:127.0.0.1]:8443"] {
            let config = from_env(&env(&[(LINK_SOCKET, SOCKET), (LISTEN, loopback)]));
            assert!(config.is_ok(), "{loopback}");
        }
        for other in [
            "100.64.0.1:443",
            "10.0.0.5:8443",
            "192.168.1.9:8443",
            "[fd00::1]:8443",
        ] {
            let error = from_env(&env(&[(LINK_SOCKET, SOCKET), (LISTEN, other)])).unwrap_err();
            assert!(matches!(error, Error::LinkListen { .. }), "{other}");
            assert!(error.to_string().contains("NET-003"), "{other}");
        }
    }

    #[test]
    fn net_001_in_the_link_mode_a_list_of_permitted_networks_is_refused() {
        let both = env(&[(LINK_SOCKET, SOCKET), (PERMITTED_NETWORKS, "0.0.0.0/0")]);
        assert!(matches!(from_env(&both).unwrap_err(), Error::LinkNetworks));
        // Compose passes an unset variable as empty.
        let empty = env(&[(LINK_SOCKET, SOCKET), (PERMITTED_NETWORKS, "")]);
        assert!(from_env(&empty).is_ok());
    }

    #[test]
    fn the_socket_must_be_an_absolute_path() {
        let error = from_env(&env(&[(LINK_SOCKET, "link.sock")])).unwrap_err();
        assert!(matches!(
            error,
            Error::NotAbsolute {
                variable: LINK_SOCKET,
                ..
            }
        ));
    }
}
