//! The names the hub's own device can take (HMD-045).

use crate::fleet::devices::hub_device::{free_name, names};

fn taken(names: &[&str]) -> Vec<String> {
    names.iter().map(|&name| name.to_owned()).collect()
}

#[test]
fn hmd_045_the_hub_takes_the_name_hub_when_it_is_free() {
    assert_eq!(free_name(names(), &[]).as_deref(), Some("Hub"));
    let other = taken(&["hub", "Hub (hosted)"]);
    assert_eq!(free_name(names(), &other).as_deref(), Some("Hub"));
}

#[test]
fn hmd_045_a_taken_name_gives_the_next_one() {
    let one = taken(&["Hub"]);
    assert_eq!(free_name(names(), &one).as_deref(), Some("Hub (hosted)"));
    let two = taken(&["Hub (hosted)", "Hub"]);
    assert_eq!(free_name(names(), &two).as_deref(), Some("Hub (hosted 2)"));
}

#[test]
fn hmd_045_no_name_is_given_when_each_one_is_taken() {
    assert_eq!(free_name(names(), &names()), None);
}

mod no_way_in {
    use crate::fleet::devices::{
        RdpSettings, SshSettings, VncSettings,
        hub_device::{is_hub_kind, refused_change},
        model::DeviceChange,
    };

    const NONE: [&str; 0] = [];

    fn fields(current: &str, change: &DeviceChange) -> Vec<&'static str> {
        refused_change(current, change)
            .into_iter()
            .map(|error| error.field)
            .collect()
    }

    fn ways_in(enabled: bool) -> DeviceChange {
        let mut change = DeviceChange {
            ssh: Some(SshSettings {
                enabled,
                ..SshSettings::default()
            }),
            ..DeviceChange::default()
        };
        change.remote.rdp = Some(RdpSettings {
            enabled,
            ..RdpSettings::default()
        });
        change.remote.vnc = Some(VncSettings {
            enabled,
            ..VncSettings::default()
        });
        change
    }

    #[test]
    fn hmd_045_the_hub_device_takes_no_ssh_rdp_or_vnc() {
        assert_eq!(fields("hub", &ways_in(true)), ["ssh", "rdp", "vnc"]);
        assert_eq!(fields("hub", &ways_in(false)), NONE);
        assert_eq!(fields("server", &ways_in(true)), NONE);
    }

    #[test]
    fn hmd_045_no_device_takes_or_leaves_the_kind_hub() {
        let to = |kind: &str| DeviceChange {
            kind: Some(kind.to_owned()),
            ..DeviceChange::default()
        };
        assert_eq!(fields("server", &to("Hub")), ["kind"]);
        assert_eq!(fields("hub", &to("server")), ["kind"]);
        assert_eq!(fields("hub", &to("hub")), NONE);
        assert_eq!(fields("server", &to("laptop")), NONE);
        assert!(is_hub_kind(" HUB "));
    }
}
