//! The link mode (one-click plan, item S5): the hub is reached through `anima-link`, which
//! permits devices by key, and not through a VPN. `ANIMA_LINK_SOCKET` turns it on. The hub then
//! listens on loopback and on that socket only (NET-003), and its permitted clients are this
//! computer and the devices of the link (NET-001).

use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
};

use ipnet::{IpNet, Ipv4Net, Ipv6Net};

use super::{Error, PERMITTED_NETWORKS, optional_socket};

pub(super) const LINK_SOCKET: &str = "ANIMA_LINK_SOCKET";
pub(super) const LINK_CONTROL: &str = "ANIMA_LINK_CONTROL";
pub(super) const HOSTED_CLOUD_KEY_FILE: &str = "ANIMA_HOSTED_CLOUD_KEY_FILE";

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

/// The control socket of `anima-link hub`, or `None` when unset or empty. It is refused outside
/// the link mode: a hub that no link reaches has no devices of a link to manage (FRN-009).
pub(super) fn control(
    env: &BTreeMap<String, String>,
    link_mode: bool,
) -> Result<Option<PathBuf>, Error> {
    let control = optional_socket(env, LINK_CONTROL)?;
    if control.is_some() && !link_mode {
        return Err(Error::LinkControlAlone);
    }
    Ok(control)
}

/// The dial socket of `anima-link hub`, through which the hub reaches a local port of a
/// computer on its link (HMD-048): `dial.sock` in the folder of the control socket `control`,
/// where the link makes both (`link/cmd/anima-link/hub.go`).
#[must_use]
pub fn dial_socket(control: &Path) -> Option<PathBuf> {
    Some(control.parent()?.join("dial.sock"))
}

/// The cloud key file of the hosted mode, or `None` when unset or empty. It is refused without
/// the control socket: the hub makes the invites that the cloud asks for through it (R-129).
pub(super) fn hosted(
    env: &BTreeMap<String, String>,
    control_set: bool,
) -> Result<Option<PathBuf>, Error> {
    let file = optional_socket(env, HOSTED_CLOUD_KEY_FILE)?;
    if file.is_some() && !control_set {
        return Err(Error::HostedWithoutLink);
    }
    Ok(file)
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
pub(super) const HOSTED_RULE: &str = "must be set only with ANIMA_LINK_CONTROL: a hosted hub \
                                       makes its device invites through the link (R-129)";
pub(super) const CONTROL_RULE: &str = "must be set only with ANIMA_LINK_SOCKET: the control \
                                       socket belongs to the link mode (FRN-009)";

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
    fn frn_009_the_control_socket_is_read_in_the_link_mode_only() {
        const CONTROL: &str = "/run/anima/link/control.sock";
        let both = from_env(&env(&[(LINK_SOCKET, SOCKET), (LINK_CONTROL, CONTROL)])).unwrap();
        assert_eq!(both.link_control, Some(PathBuf::from(CONTROL)));
        let socket_only = from_env(&env(&[(LINK_SOCKET, SOCKET), (LINK_CONTROL, "")])).unwrap();
        assert_eq!(socket_only.link_control, None);
        let alone = from_env(&env(&[(LINK_CONTROL, CONTROL)])).unwrap_err();
        assert!(matches!(alone, Error::LinkControlAlone), "{alone}");
        assert!(alone.to_string().contains("ANIMA_LINK_SOCKET"));
        let relative = from_env(&env(&[
            (LINK_SOCKET, SOCKET),
            (LINK_CONTROL, "control.sock"),
        ]));
        assert!(matches!(
            relative.unwrap_err(),
            Error::NotAbsolute {
                variable: LINK_CONTROL,
                ..
            }
        ));
    }

    #[test]
    fn hmd_048_the_dial_socket_is_beside_the_control_socket() {
        assert_eq!(
            dial_socket(Path::new("/run/anima/link/control.sock")),
            Some(PathBuf::from("/run/anima/link/dial.sock"))
        );
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

    #[test]
    fn r_129_the_hosted_mode_needs_the_control_socket_of_the_link() {
        const KEY: &str = "/run/secrets/anima/cloud_key";
        let hosted = env(&[
            (LINK_SOCKET, SOCKET),
            (LINK_CONTROL, "/run/anima/link-control.sock"),
            (HOSTED_CLOUD_KEY_FILE, KEY),
        ]);
        assert_eq!(
            from_env(&hosted).unwrap().hosted_cloud_key_file,
            Some(PathBuf::from(KEY))
        );
        let alone = env(&[(LINK_SOCKET, SOCKET), (HOSTED_CLOUD_KEY_FILE, KEY)]);
        assert!(matches!(from_env(&alone), Err(Error::HostedWithoutLink)));
        assert_eq!(from_env(&env(&[])).unwrap().hosted_cloud_key_file, None);
    }
}
