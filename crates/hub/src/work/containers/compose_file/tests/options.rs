//! The options one at a time: the forms of an option, the options with a form of their own,
//! and the options that are not converted.

use super::super::Note;
use super::{converted, keys, list, text, tree};

/// The text at `path` of the one service of the file of `command`.
fn of_service(command: &str, path: &[&str]) -> String {
    let done = converted(command);
    let file = tree(&done.compose_file);
    let mut whole = vec!["services", done.service.as_str()];
    whole.extend(path);
    text(&file, &whole).to_owned()
}

/// A command, a path in its one service, and the text there.
const FORMS: [(&str, &[&str], &str); 31] = [
    ("docker run --name=a nginx", &["container_name"], "a"),
    ("docker run --name a nginx", &["container_name"], "a"),
    ("docker run -p 80:80 nginx", &["ports", "0"], "80:80"),
    ("docker run -p80:80 nginx", &["ports", "0"], "80:80"),
    ("docker run -p=80:80 nginx", &["ports", "0"], "80:80"),
    ("docker run -dp 80:80 nginx", &["ports", "0"], "80:80"),
    ("docker run -itp80:80 nginx", &["ports", "0"], "80:80"),
    ("docker run -eA=b=c nginx", &["environment", "0"], "A=b=c"),
    ("docker run -e A nginx", &["environment", "0"], "A"),
    ("docker run --env=A= nginx", &["environment", "0"], "A="),
    ("docker run -it nginx", &["tty"], "true"),
    ("docker run -ti nginx", &["stdin_open"], "true"),
    ("docker run --init nginx", &["init"], "true"),
    ("docker run --init=false nginx", &["init"], "false"),
    (
        "docker run --privileged=true nginx",
        &["privileged"],
        "true",
    ),
    ("docker run -u 1000:1000 nginx", &["user"], "1000:1000"),
    ("docker run -m 512m nginx", &["mem_limit"], "512m"),
    (
        "docker run --restart on-failure:3 nginx",
        &["restart"],
        "on-failure:3",
    ),
    ("docker run --restart no nginx", &["restart"], "no"),
    ("docker run --net none nginx", &["network_mode"], "none"),
    (
        "docker run --network container:vpn nginx",
        &["network_mode"],
        "container:vpn",
    ),
    (
        "docker run --stop-timeout 30 nginx",
        &["stop_grace_period"],
        "30s",
    ),
    (
        "docker run --entrypoint '/bin/sh -c' nginx",
        &["entrypoint", "0"],
        "/bin/sh -c",
    ),
    ("docker run --entrypoint '' nginx", &["entrypoint"], ""),
    (
        "docker run --ulimit nproc=64 nginx",
        &["ulimits", "nproc"],
        "64",
    ),
    (
        "docker run --ulimit nofile=1:2 nginx",
        &["ulimits", "nofile", "hard"],
        "2",
    ),
    (
        "docker run --no-healthcheck nginx",
        &["healthcheck", "disable"],
        "true",
    ),
    (
        "docker run --log-opt tag=a=b nginx",
        &["logging", "options", "tag"],
        "a=b",
    ),
    (
        "docker run --name a --name b nginx",
        &["container_name"],
        "b",
    ),
    ("docker run nginx -p 1:1 --name x", &["command", "0"], "-p"),
    (
        "docker run -- nginx -g 'daemon off;'",
        &["command", "1"],
        "daemon off;",
    ),
];

#[test]
fn host_022_each_form_of_an_option_is_read() {
    for (command, path, expected) in FORMS {
        assert_eq!(of_service(command, path), expected, "{command}");
    }
}

#[test]
fn host_022_a_mount_has_the_long_form_of_a_volume() {
    let done = converted(
        "docker run --mount type=bind,source=/srv/conf,target=/etc/conf,readonly \
         --mount type=volume,src=app-data,dst=/data,volume-nocopy \
         --mount source=cache,destination=/cache,ro=false \
         --mount type=tmpfs,target=/run,tmpfs-size=1048576 \
         --mount type=bind,src=/a,dst=/b,bind-propagation=rshared \
         -v app-data:/again -v /srv/plain:/plain -v /anonymous -v ./relative:/r nginx",
    );
    let file = tree(&done.compose_file);
    let volume = |index: &'static str, key: &'static str| {
        text(&file, &["services", "nginx", "volumes", index, key]).to_owned()
    };
    assert_eq!(
        [
            volume("0", "type"),
            volume("0", "source"),
            volume("0", "target")
        ],
        ["bind", "/srv/conf", "/etc/conf"]
    );
    assert_eq!(volume("0", "read_only"), "true");
    assert_eq!(
        keys(&file, &["services", "nginx", "volumes", "1"]),
        ["type", "source", "target", "volume"]
    );
    assert_eq!(
        text(
            &file,
            &["services", "nginx", "volumes", "1", "volume", "nocopy"]
        ),
        "true"
    );
    assert_eq!(
        keys(&file, &["services", "nginx", "volumes", "2"]),
        ["type", "source", "target"]
    );
    assert_eq!(volume("2", "type"), "volume");
    assert_eq!(
        text(
            &file,
            &["services", "nginx", "volumes", "3", "tmpfs", "size"]
        ),
        "1048576"
    );
    assert_eq!(
        text(
            &file,
            &["services", "nginx", "volumes", "4", "bind", "propagation"]
        ),
        "rshared"
    );
}

#[test]
fn host_022_only_a_named_volume_is_at_the_top_level() {
    let done = converted(
        "docker run --mount type=volume,src=app-data,dst=/data --mount source=cache,target=/c \
         -v app-data:/again -v /srv/plain:/plain -v /anonymous -v ./relative:/r \
         -v $HOME/x:/x -v C:/y nginx",
    );
    let file = tree(&done.compose_file);
    assert_eq!(keys(&file, &["volumes"]), ["app-data", "cache"]);
    assert_eq!(text(&file, &["volumes", "cache", "name"]), "cache");
    assert_eq!(text(&file, &["services", "nginx", "volumes", "7"]), "C:/y");
}

#[test]
fn host_022_a_named_network_is_external_and_a_mode_is_not() {
    let done = converted("docker run --network front --net back --network front nginx");
    let file = tree(&done.compose_file);
    assert_eq!(
        list(&file, &["services", "nginx", "networks"]),
        ["front", "back"]
    );
    assert_eq!(keys(&file, &["networks"]), ["front", "back"]);
    assert_eq!(text(&file, &["networks", "back", "external"]), "true");
    assert_eq!(done.notes, [Note::ExternalNetwork]);

    for mode in ["host", "none", "bridge", "container:vpn"] {
        let done = converted(&format!("docker run --network {mode} nginx"));
        let file = tree(&done.compose_file);
        assert_eq!(keys(&file, &[]), ["services"], "{mode}");
        assert!(done.notes.is_empty(), "{mode}");
    }
    // An alias with no network of a name has no place in the file.
    let alone = converted("docker run --network-alias web --network host nginx");
    assert_eq!(alone.not_converted, ["--network-alias"]);
}

#[test]
fn host_022_an_option_with_no_key_is_listed_and_is_not_an_error() {
    let done = converted(
        "docker run --cidfile /tmp/id --ip 10.0.0.2 -a stdout -c 512 -P --oom-kill-disable \
         --sig-proxy=false --cpu-shares=2 --cidfile /tmp/again --mount type=image,source=a \
         --mount 'type=bind,\"source=/a\",target=/b' --pull sometimes --gpus device=0 \
         --network name=front,alias=a --network default --stop-timeout -1 \
         --health-retries many --log-opt novalue --ulimit nofile=a:b nginx",
    );
    assert_eq!(
        done.not_converted,
        [
            "--cidfile",
            "--ip",
            "-a",
            "-c",
            "-P",
            "--oom-kill-disable",
            "--sig-proxy",
            "--cpu-shares",
            "--mount",
            "--pull",
            "--gpus",
            "--network",
            "--stop-timeout",
            "--health-retries",
            "--log-opt",
            "--ulimit",
        ]
    );
    let file = tree(&done.compose_file);
    assert_eq!(keys(&file, &["services", "nginx"]), ["image"]);
    assert_eq!(done.notes, []);
}

#[test]
fn host_022_an_option_that_docker_does_not_have_is_read_as_one_with_no_value() {
    let done = converted("docker run --frobnicate --level=9 -Z nginx echo");
    assert_eq!(done.not_converted, ["--frobnicate", "--level", "-Z"]);
    assert_eq!(done.notes, [Note::UnknownOptionReadAsFlag]);
    assert_eq!(
        of_service("docker run --frobnicate nginx echo", &["image"]),
        "nginx"
    );
    // With `=`, the converter knows where the value ends, and gives no note.
    assert_eq!(converted("docker run --level=9 nginx").notes, []);
}

#[test]
fn host_022_detach_and_rm_change_nothing_and_give_a_note() {
    let done = converted("docker run -d --rm --detach nginx");
    assert_eq!(done.notes, [Note::DetachNotNeeded, Note::RemoveNotKept]);
    assert_eq!(
        done.compose_file,
        "services:\n  nginx:\n    image: \"nginx\"\n"
    );
}
