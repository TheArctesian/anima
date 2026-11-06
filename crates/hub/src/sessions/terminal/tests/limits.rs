//! The hub's own waits and SSH settings (K4 Sections 8.2 and 8.5): the values the K3
//! `terminal_router` uses, which the wire tests shorten.

use std::time::Duration;

use super::super::{opening::config, route::Timing};

#[test]
fn k4_8_2_keep_alive_every_30_seconds_and_three_unanswered_lose_the_connection() {
    let config = config();
    assert_eq!(config.keepalive_interval, Some(Duration::from_secs(30)));
    assert_eq!(config.keepalive_max, 3);
    assert_eq!(config.inactivity_timeout, None, "an idle shell stays open");
    assert_eq!(config.channel_buffer_size, 4, "K4 Section 8.5");
    assert!(config.nodelay, "keystrokes go out at once");
}

#[test]
fn k4_8_5_the_router_waits_are_the_contract_values() {
    let seconds = Duration::from_secs;
    let expected = Timing {
        opening: seconds(8),
        upgrade: seconds(10),
        first_size: seconds(10),
        hub_silence: seconds(5),
        client_idle: seconds(15),
        request: seconds(15),
    };
    assert_eq!(Timing::default(), expected);
}
