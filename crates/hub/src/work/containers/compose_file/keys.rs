//! The keys of the Compose specification: those of the top level, and those of a service with
//! the forms that the value of each may have.

/// The forms that the value of a key may have. A value that is empty, or a reference to an
/// anchor, passes each of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Accepts {
    Any,
    Text,
    List,
    Map,
    TextOrList,
    TextOrMap,
    ListOrMap,
    /// A text that is one of the restart policies of Compose.
    Restart,
}

const TOP_LEVEL: [&str; 9] = [
    "name", "version", "services", "networks", "volumes", "configs", "secrets", "include", "models",
];

/// Whether `key` is a key of the top level of a Compose file. Each key that starts with `x-`
/// is an extension, which Compose ignores.
pub(super) fn top_level(key: &str) -> bool {
    key.starts_with("x-") || TOP_LEVEL.contains(&key)
}

const TEXT: [&str; 19] = [
    "image",
    "container_name",
    "hostname",
    "domainname",
    "user",
    "working_dir",
    "network_mode",
    "pid",
    "ipc",
    "platform",
    "stop_signal",
    "stop_grace_period",
    "mac_address",
    "runtime",
    "pull_policy",
    "cgroup_parent",
    "isolation",
    "userns_mode",
    "uts",
];

const LIST: [&str; 16] = [
    "ports",
    "volumes",
    "expose",
    "cap_add",
    "cap_drop",
    "devices",
    "security_opt",
    "volumes_from",
    "links",
    "external_links",
    "group_add",
    "profiles",
    "device_cgroup_rules",
    "dns_opt",
    "configs",
    "secrets",
];

const MAP: [&str; 10] = [
    "healthcheck",
    "logging",
    "deploy",
    "ulimits",
    "develop",
    "blkio_config",
    "credential_spec",
    "storage_opt",
    "driver_opts",
    "provider",
];

const TEXT_OR_LIST: [&str; 7] = [
    "command",
    "entrypoint",
    "env_file",
    "dns",
    "dns_search",
    "tmpfs",
    "label_file",
];

const TEXT_OR_MAP: [&str; 2] = ["build", "extends"];

const LIST_OR_MAP: [&str; 7] = [
    "environment",
    "depends_on",
    "labels",
    "networks",
    "extra_hosts",
    "sysctls",
    "annotations",
];

/// The keys of a service whose value the check does not look at.
const ANY: [&str; 31] = [
    "attach",
    "cgroup",
    "cpu_count",
    "cpu_percent",
    "cpu_shares",
    "cpu_period",
    "cpu_quota",
    "cpu_rt_runtime",
    "cpu_rt_period",
    "cpus",
    "cpuset",
    "gpus",
    "init",
    "mem_limit",
    "mem_reservation",
    "mem_swappiness",
    "memswap_limit",
    "models",
    "oom_kill_disable",
    "oom_score_adj",
    "pids_limit",
    "post_start",
    "pre_stop",
    "privileged",
    "pull_refresh_after",
    "read_only",
    "scale",
    "shm_size",
    "stdin_open",
    "tty",
    "use_api_socket",
];

/// The forms of the value of `key` in a service; `None` for a key that Compose does not know.
pub(super) fn service(key: &str) -> Option<Accepts> {
    let lists: [(&[&str], Accepts); 7] = [
        (&TEXT, Accepts::Text),
        (&LIST, Accepts::List),
        (&MAP, Accepts::Map),
        (&TEXT_OR_LIST, Accepts::TextOrList),
        (&TEXT_OR_MAP, Accepts::TextOrMap),
        (&LIST_OR_MAP, Accepts::ListOrMap),
        (&ANY, Accepts::Any),
    ];
    if key == "restart" {
        return Some(Accepts::Restart);
    }
    lists
        .iter()
        .find(|(keys, _)| keys.contains(&key))
        .map(|(_, accepts)| *accepts)
}

/// Whether `policy` is a restart policy of Compose. A text with a variable passes: Compose
/// fills it later.
pub(super) fn restart_policy(policy: &str) -> bool {
    let retries = |count: &str| !count.is_empty() && count.bytes().all(|b| b.is_ascii_digit());
    policy.contains('$')
        || matches!(policy, "no" | "always" | "on-failure" | "unless-stopped")
        || policy.strip_prefix("on-failure:").is_some_and(retries)
}
