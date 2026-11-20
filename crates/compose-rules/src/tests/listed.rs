//! A key that is not in the list is refused, at any level, with no rule of its own.

use super::{Refusal, Rule, SOUND, app, checked, refusal, top, with};

#[test]
fn host_028_each_key_that_gives_the_rights_of_the_machine_is_refused_because_it_is_not_listed() {
    for key in [
        "privileged: true",
        "cap_add: [SYS_ADMIN]",
        "devices: ['/dev/sda:/dev/sda']",
        "device_cgroup_rules: ['b *:* rwm']",
        "network_mode: host",
        "pid: host",
        "ipc: host",
        "uts: host",
        "userns_mode: host",
        "cgroup: host",
        "cgroup_parent: /",
        "volumes_from: ['container:traefik']",
        "runtime: nvidia",
        "sysctls: {net.ipv4.ip_forward: 1}",
        "oom_score_adj: -1000",
        "use_api_socket: true",
        "env_file: /etc/environment",
        "extends: {file: other.yml, service: app}",
        "build: .",
        "profiles: [debug]",
        "ulimits: {nofile: 65535}",
        "extra_hosts: ['a:1.2.3.4']",
        "post_start: [{command: 'id'}]",
        "secrets: [x]",
        "a_key_of_a_newer_compose: 1",
    ] {
        let file = with("restart: unless-stopped", key);
        assert_eq!(refusal(&file), app(Rule::KeyNotPermitted), "{key}");
    }
}

#[test]
fn host_028_a_key_of_the_top_level_that_is_not_listed_is_refused() {
    for addition in [
        "name: other-project\n",
        "include: [other.yml]\n",
        "secrets:\n  token:\n    file: /etc/shadow\n",
        "x-anything: 1\n",
        "version: '3'\n",
    ] {
        assert_eq!(
            refusal(&format!("{SOUND}{addition}")),
            top(Rule::KeyNotPermitted),
            "{addition}"
        );
    }
}

#[test]
fn host_028_a_volume_a_network_and_a_config_have_only_their_listed_members() {
    for (old, new) in [
        (
            "  data:\n",
            "  data:\n    driver_opts: {type: none, o: bind, device: /}\n",
        ),
        ("  data:\n", "  data:\n    external: true\n"),
        ("  data:\n", "  data:\n    name: other_data\n"),
        (
            "    internal: true\n",
            "    internal: true\n    external: true\n",
        ),
        (
            "    internal: true\n",
            "    ipam: {config: [{subnet: 10.0.0.0/8}]}\n",
        ),
        ("    internal: true\n", "    driver_opts: {parent: eth0}\n"),
        (
            "    content: |\n",
            "    file: ./files/compose.yaml\n    content: |\n",
        ),
        (
            "    content: |\n",
            "    environment: HOME\n    content: |\n",
        ),
    ] {
        assert_eq!(
            refusal(&with(old, new)),
            top(Rule::KeyNotPermitted),
            "{new}"
        );
    }
    let macvlan = with("    internal: true\n", "    driver: macvlan\n");
    assert_eq!(refusal(&macvlan), top(Rule::ValueNotPermitted));
}

#[test]
fn host_028_a_member_below_a_permitted_key_must_be_listed_too() {
    for (old, new) in [
        ("        protocol: tcp", "        mode: host"),
        (
            "        target: /data",
            "        target: /data\n        volume: {subpath: ../x}",
        ),
        (
            "        read_only: true",
            "        bind: {propagation: shared}",
        ),
        ("      retries: 3", "      retries: 3\n      output: x"),
        (
            "        condition: service_healthy",
            "        condition: service_healthy\n        x: 1",
        ),
        ("        mode: \"0444\"", "        uid: \"0\""),
        (
            "    mem_limit: 512m",
            "    logging: {driver: local, options: {max-size: 1g}}",
        ),
    ] {
        assert_eq!(
            refusal(&with(old, new)),
            app(Rule::KeyNotPermitted),
            "{new}"
        );
    }
    let alias = with(
        "        aliases: [database]",
        "        ipv4_address: 10.0.0.9",
    );
    assert_eq!(
        refusal(&alias),
        Refusal::File {
            rule: Rule::KeyNotPermitted,
            service: Some(1)
        }
    );
}

#[test]
fn host_028_a_short_form_of_a_port_a_mount_and_a_config_is_refused() {
    for (old, new) in [
        (
            "      - target: 3000\n        published: 8080\n        host_ip: 127.0.0.1\n        protocol: tcp",
            "      - \"8080:3000\"",
        ),
        (
            "      - type: volume\n        source: data\n        target: /data",
            "      - data:/data",
        ),
        (
            "      - source: settings\n        target: /etc/tracks.yml\n        mode: \"0444\"",
            "      - settings",
        ),
    ] {
        assert_eq!(
            refusal(&with(old, new)),
            app(Rule::ValueNotPermitted),
            "{new}"
        );
    }
    assert!(checked(SOUND).is_ok());
}
