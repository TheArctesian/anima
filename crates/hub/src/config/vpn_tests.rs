//! The VPN endpoints: the Tailscale `LocalAPI` proxy and the broker socket (D-4, VP-7).

use super::tests::env;
use super::*;

#[test]
fn d_4_tailscale_localapi_parses_host_port_and_empty_means_none() {
    for good in ["proxy:41112", "[fd7a::1]:41112"] {
        let config = from_env(&env(&[(TAILSCALE_LOCALAPI, good)])).unwrap();
        assert_eq!(config.tailscale_localapi.as_deref(), Some(good));
    }
    let config = from_env(&env(&[(TAILSCALE_LOCALAPI, "")])).unwrap();
    assert_eq!(config.tailscale_localapi, None);
}

#[test]
fn d_4_malformed_tailscale_localapi_is_refused_naming_the_variable() {
    for bad in ["proxy", ":1", "proxy:x", "fd7a::1:41112"] {
        let error = from_env(&env(&[(TAILSCALE_LOCALAPI, bad)])).unwrap_err();
        assert!(
            matches!(
                error,
                Error::HostPort {
                    variable: TAILSCALE_LOCALAPI,
                    ..
                }
            ),
            "{bad}"
        );
        assert!(error.to_string().contains(TAILSCALE_LOCALAPI), "{bad}");
    }
}

#[test]
fn vp_7_vpn_broker_is_an_absolute_path_and_empty_means_none() {
    let socket = "/run/anima-broker/broker.sock";
    let config = from_env(&env(&[(VPN_BROKER, socket)])).unwrap();
    assert_eq!(config.vpn_broker, Some(PathBuf::from(socket)));
    assert_eq!(
        from_env(&env(&[(VPN_BROKER, "")])).unwrap().vpn_broker,
        None
    );
    assert_eq!(from_env(&env(&[])).unwrap().vpn_broker, None);
    let error = from_env(&env(&[(VPN_BROKER, "broker.sock")])).unwrap_err();
    assert!(matches!(
        error,
        Error::NotAbsolute {
            variable: VPN_BROKER,
            ..
        }
    ));
}
