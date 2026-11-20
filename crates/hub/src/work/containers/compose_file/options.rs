//! The options of `docker run`: what each becomes in a Compose file, and whether it takes a
//! value. The converter must know each option of Docker, also those that it does not
//! convert, to find where the options end and the image starts.

use super::docker_run::Note;

/// What the converter does with one option.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reads {
    // Options with no value.
    /// A key whose value is true.
    Truth(&'static str),
    /// Nothing changes in the file; the result has a note.
    Noted(Note),
    NoHealthcheck,
    /// An option of Docker that has no key in the file.
    Flag,
    // Options with a value.
    /// A key with one text.
    Text(&'static str),
    /// A key with a list of texts.
    List(&'static str),
    Name,
    Volume,
    Mount,
    Network,
    NetworkAlias,
    Entrypoint,
    Pull,
    StopTimeout,
    Gpus,
    HealthCommand,
    /// A key of `healthcheck` with one text.
    Health(&'static str),
    HealthRetries,
    LogDriver,
    LogOption,
    Ulimit,
    /// An option of Docker with a value, that has no key in the file.
    Value,
}

impl Reads {
    pub(super) const fn takes_value(self) -> bool {
        !matches!(
            self,
            Self::Truth(_) | Self::Noted(_) | Self::NoHealthcheck | Self::Flag
        )
    }
}

const CONVERTED: [(&str, Reads); 71] = [
    ("name", Reads::Name),
    ("publish", Reads::List("ports")),
    ("volume", Reads::Volume),
    ("mount", Reads::Mount),
    ("env", Reads::List("environment")),
    ("env-file", Reads::List("env_file")),
    ("restart", Reads::Text("restart")),
    ("network", Reads::Network),
    ("net", Reads::Network),
    ("network-alias", Reads::NetworkAlias),
    ("net-alias", Reads::NetworkAlias),
    ("workdir", Reads::Text("working_dir")),
    ("user", Reads::Text("user")),
    ("hostname", Reads::Text("hostname")),
    ("entrypoint", Reads::Entrypoint),
    ("label", Reads::List("labels")),
    ("cap-add", Reads::List("cap_add")),
    ("cap-drop", Reads::List("cap_drop")),
    ("privileged", Reads::Truth("privileged")),
    ("memory", Reads::Text("mem_limit")),
    ("cpus", Reads::Text("cpus")),
    ("read-only", Reads::Truth("read_only")),
    ("init", Reads::Truth("init")),
    ("shm-size", Reads::Text("shm_size")),
    ("tmpfs", Reads::List("tmpfs")),
    ("add-host", Reads::List("extra_hosts")),
    ("dns", Reads::List("dns")),
    ("device", Reads::List("devices")),
    ("expose", Reads::List("expose")),
    ("security-opt", Reads::List("security_opt")),
    ("pid", Reads::Text("pid")),
    ("ipc", Reads::Text("ipc")),
    ("sysctl", Reads::List("sysctls")),
    ("ulimit", Reads::Ulimit),
    ("stop-signal", Reads::Text("stop_signal")),
    ("stop-timeout", Reads::StopTimeout),
    ("health-cmd", Reads::HealthCommand),
    ("health-interval", Reads::Health("interval")),
    ("health-timeout", Reads::Health("timeout")),
    ("health-retries", Reads::HealthRetries),
    ("health-start-period", Reads::Health("start_period")),
    ("health-start-interval", Reads::Health("start_interval")),
    ("no-healthcheck", Reads::NoHealthcheck),
    ("log-driver", Reads::LogDriver),
    ("log-opt", Reads::LogOption),
    ("platform", Reads::Text("platform")),
    ("pull", Reads::Pull),
    ("group-add", Reads::List("group_add")),
    ("interactive", Reads::Truth("stdin_open")),
    ("tty", Reads::Truth("tty")),
    ("link", Reads::List("links")),
    ("volumes-from", Reads::List("volumes_from")),
    ("gpus", Reads::Gpus),
    ("detach", Reads::Noted(Note::DetachNotNeeded)),
    ("rm", Reads::Noted(Note::RemoveNotKept)),
    ("domainname", Reads::Text("domainname")),
    ("mac-address", Reads::Text("mac_address")),
    ("runtime", Reads::Text("runtime")),
    ("dns-search", Reads::List("dns_search")),
    ("dns-option", Reads::List("dns_opt")),
    ("dns-opt", Reads::List("dns_opt")),
    ("memory-reservation", Reads::Text("mem_reservation")),
    ("memory-swap", Reads::Text("memswap_limit")),
    ("cpuset-cpus", Reads::Text("cpuset")),
    ("userns", Reads::Text("userns_mode")),
    ("uts", Reads::Text("uts")),
    ("cgroup-parent", Reads::Text("cgroup_parent")),
    ("cgroupns", Reads::Text("cgroup")),
    ("isolation", Reads::Text("isolation")),
    ("device-cgroup-rule", Reads::List("device_cgroup_rules")),
    ("label-file", Reads::List("label_file")),
];

/// The options of Docker with a value that the converter does not convert.
const WITH_VALUE: [&str; 29] = [
    "annotation",
    "attach",
    "blkio-weight",
    "blkio-weight-device",
    "cidfile",
    "cpu-count",
    "cpu-percent",
    "cpu-period",
    "cpu-quota",
    "cpu-rt-period",
    "cpu-rt-runtime",
    "cpu-shares",
    "cpuset-mems",
    "detach-keys",
    "device-read-bps",
    "device-read-iops",
    "device-write-bps",
    "device-write-iops",
    "io-maxbandwidth",
    "io-maxiops",
    "ip",
    "ip6",
    "kernel-memory",
    "link-local-ip",
    "memory-swappiness",
    "oom-score-adj",
    "pids-limit",
    "storage-opt",
    "volume-driver",
];

/// The options of Docker with no value that the converter does not convert.
const WITH_NO_VALUE: [&str; 7] = [
    "disable-content-trust",
    "oom-kill-disable",
    "publish-all",
    "quiet",
    "sig-proxy",
    "use-api-socket",
    "help",
];

/// What the converter does with the option `--name`; `None` for an option that Docker does
/// not have.
pub(super) fn long(name: &str) -> Option<Reads> {
    let converted = CONVERTED.iter().find(|(option, _)| *option == name);
    converted.map(|(_, reads)| *reads).or_else(|| {
        if WITH_VALUE.contains(&name) {
            Some(Reads::Value)
        } else {
            WITH_NO_VALUE.contains(&name).then_some(Reads::Flag)
        }
    })
}

/// What the converter does with the option of one letter.
pub(super) fn short(letter: char) -> Option<Reads> {
    let name = match letter {
        'a' => "attach",
        'c' => "cpu-shares",
        'd' => "detach",
        'e' => "env",
        'h' => "hostname",
        'i' => "interactive",
        'l' => "label",
        'm' => "memory",
        'p' => "publish",
        'P' => "publish-all",
        'q' => "quiet",
        't' => "tty",
        'u' => "user",
        'v' => "volume",
        'w' => "workdir",
        _ => return None,
    };
    long(name)
}
