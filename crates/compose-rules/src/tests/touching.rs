//! Where a container touches the machine: its ports, its mounts, its networks, its configs.

use super::{ENVIRONMENT, Refusal, Rule, app, checked, refusal, with};
use crate::{Settings, check};

const OPEN: Settings = Settings {
    open_to_network: true,
    label_prefixes: Vec::new(),
};

#[test]
fn r_229_a_port_is_on_the_machine_itself_unless_open_to_the_network_is_set() {
    let not_at_machine = Refusal::NotAtMachine { service: Some(0) };
    for (old, new) in [
        ("        host_ip: 127.0.0.1\n", ""),
        ("host_ip: 127.0.0.1", "host_ip: 0.0.0.0"),
        ("host_ip: 127.0.0.1", "host_ip: 192.168.1.10"),
        ("host_ip: 127.0.0.1", "host_ip: \"::\""),
    ] {
        let file = with(old, new);
        assert_eq!(refusal(&file), not_at_machine, "{new}");
        assert!(check(&file, ENVIRONMENT, &OPEN).is_ok(), "{new}");
    }
    let not_an_address = with("host_ip: 127.0.0.1", "host_ip: example.com");
    assert_eq!(
        check(&not_an_address, ENVIRONMENT, &OPEN),
        Err(app(Rule::PortNotPermitted))
    );
}

#[test]
fn host_028_a_published_port_is_one_whole_number_from_1024() {
    for (old, new) in [
        ("published: 8080", "published: 80"),
        ("published: 8080", "published: 443"),
        ("published: 8080", "published: 1023"),
        ("published: 8080", "published: 65536"),
        ("published: 8080", "published: 8080-8090"),
        ("published: 8080", "published: \"127.0.0.1:8080\""),
        ("        published: 8080\n", ""),
        ("target: 3000", "target: 0"),
        ("protocol: tcp", "protocol: sctp"),
    ] {
        assert_eq!(
            refusal(&with(old, new)),
            app(Rule::PortNotPermitted),
            "{new}"
        );
    }
    assert!(checked(&with("published: 8080", "published: \"1024\"")).is_ok());
}

#[test]
fn host_028_a_mount_is_a_volume_of_the_file_or_a_folder_of_the_application() {
    for (old, new) in [
        ("source: data", "source: other"),
        ("source: data", "source: /var/lib/docker/volumes/x/_data"),
        ("        source: data\n", ""),
        ("source: binds/config", "source: /var/run/docker.sock"),
        ("source: binds/config", "source: /"),
        ("source: binds/config", "source: binds/config/../../files"),
        ("source: binds/config", "source: binds/a/b"),
        ("source: binds/config", "source: files"),
        (
            "source: binds/config",
            "source: ../other-application/binds/config",
        ),
        ("source: binds/config", "source: binds/Config"),
        ("      - type: bind", "      - type: tmpfs"),
        ("      - type: bind", "      - type: npipe"),
        ("target: /data", "target: /"),
        ("target: /data", "target: data"),
        ("target: /data", "target: /data/../etc"),
        ("target: /config", "target: /data"),
        ("target: /etc/tracks.yml", "target: /data"),
        ("read_only: true", "read_only: sometimes"),
        ("source: settings", "source: other"),
        ("mode: \"0444\"", "mode: \"rw\""),
    ] {
        assert_eq!(
            refusal(&with(old, new)),
            app(Rule::MountNotPermitted),
            "{new}"
        );
    }
    assert!(checked(&with("source: binds/config", "source: ./binds/config")).is_ok());
}

#[test]
fn host_028_a_tmpfs_is_a_path_with_no_option_and_a_network_is_of_this_file() {
    for tmpfs in [
        "tmpfs: [\"/run:exec,suid,dev\"]",
        "tmpfs: run",
        "tmpfs: \"/run,size=1g\"",
    ] {
        let file = with("restart: unless-stopped", tmpfs);
        assert_eq!(refusal(&file), app(Rule::MountNotPermitted), "{tmpfs}");
    }
    assert!(checked(&with("restart: unless-stopped", "tmpfs: [/run, /tmp]")).is_ok());
    let host = with("networks: [inner]", "networks: [host]");
    assert_eq!(refusal(&host), app(Rule::ValueNotPermitted));
    let alias = with("aliases: [database]", "aliases: [\"Bad Alias\"]");
    assert_eq!(
        refusal(&alias),
        Refusal::File {
            rule: Rule::ValueNotPermitted,
            service: Some(1)
        }
    );
}
