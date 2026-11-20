//! The service that the converter builds while it reads the options of `docker run`.

use super::{
    docker_run::Note,
    mounts::{self, is_name, truth},
    options::Reads,
    writer::Value,
};

/// The most characters of the name of an option in the list of what was not converted.
const LONGEST_OPTION: usize = 40;
/// The most characters of the name of the service.
const LONGEST_SERVICE: usize = 63;

/// The keys of a service that come first in the file, in this order.
const FIRST_KEYS: [&str; 11] = [
    "image",
    "container_name",
    "entrypoint",
    "command",
    "restart",
    "ports",
    "environment",
    "env_file",
    "volumes",
    "networks",
    "network_mode",
];

pub(super) struct Draft {
    service: Value,
    name: Option<String>,
    networks: Vec<String>,
    aliases: Vec<Value>,
    volumes: Vec<String>,
    pub notes: Vec<Note>,
    pub not_converted: Vec<String>,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            service: Value::map(),
            name: None,
            networks: Vec::new(),
            aliases: Vec::new(),
            volumes: Vec::new(),
            notes: Vec::new(),
            not_converted: Vec::new(),
        }
    }
}

fn add<T: PartialEq>(list: &mut Vec<T>, value: T) {
    if !list.contains(&value) {
        list.push(value);
    }
}

fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// `name` as the name of a service: lower-case letters, digits, `.`, `_` and `-`, with a
/// letter or a digit first.
fn service_name(name: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    let kept: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| if safe(c) { c } else { '-' })
        .collect();
    let trimmed = kept.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if trimmed.is_empty() {
        "app".to_owned()
    } else {
        trimmed.chars().take(LONGEST_SERVICE).collect()
    }
}

/// The last part of the name of an image, with no tag and no digest.
fn image_name(image: &str) -> &str {
    let no_digest = image.split('@').next().unwrap_or(image);
    let last = no_digest.rsplit('/').next().unwrap_or(no_digest);
    last.split(':').next().unwrap_or(last)
}

impl Draft {
    pub(super) fn note(&mut self, note: Note) {
        add(&mut self.notes, note);
    }

    /// Records an option that is not in the file, by its name only: its value can be a secret.
    pub(super) fn skip(&mut self, option: &str) {
        let safe = |c: char| c.is_ascii_alphanumeric() || c == '-';
        let name = if option.chars().all(safe) {
            option.chars().take(LONGEST_OPTION).collect()
        } else {
            "(an option)".to_owned()
        };
        add(&mut self.not_converted, name);
    }

    /// An option that Docker does not have. With no `=`, the converter cannot know whether
    /// the next word is its value: it reads the option as one with no value, and says so.
    pub(super) fn unknown(&mut self, option: &str, alone: bool) {
        self.skip(option);
        if alone {
            self.note(Note::UnknownOptionReadAsFlag);
        }
    }

    /// An option with no value. `given` is the text after `=`, as in `--init=false`.
    pub(super) fn flag(&mut self, reads: Reads, option: &str, given: Option<&str>) {
        let Some(on) = given.map_or(Some(true), truth) else {
            return self.skip(option);
        };
        match reads {
            Reads::Truth(key) => self.service.set(key, Value::Truth(on)),
            Reads::Noted(note) => self.note(note),
            Reads::NoHealthcheck if on => {
                self.service
                    .child("healthcheck")
                    .set("disable", Value::Truth(true));
            }
            _ => self.skip(option),
        }
    }

    /// An option with its value.
    pub(super) fn apply(&mut self, reads: Reads, option: &str, value: &str) {
        let health = |draft: &mut Self, key: &str, value: Value| {
            draft.service.child("healthcheck").set(key, value);
            true
        };
        let converted = match reads {
            Reads::Text(key) => {
                self.service.set(key, Value::text(value));
                true
            }
            Reads::List(key) => {
                self.service.push(key, Value::text(value));
                true
            }
            Reads::Name => self.named(value),
            Reads::Volume => self.volume(value),
            Reads::Mount => self.mount(value),
            Reads::Network => self.network(value),
            Reads::NetworkAlias => {
                self.aliases.push(Value::text(value));
                true
            }
            Reads::Entrypoint => self.entrypoint(value),
            Reads::Pull => self.only("pull_policy", value, &["always", "missing", "never"]),
            Reads::Gpus => self.only("gpus", value, &["all"]),
            Reads::StopTimeout => self.stop_timeout(value),
            Reads::HealthCommand => {
                let test = vec![Value::text("CMD-SHELL"), Value::text(value)];
                health(self, "test", Value::List(test))
            }
            Reads::Health(key) => health(self, key, Value::text(value)),
            Reads::HealthRetries => value
                .parse::<u32>()
                .is_ok_and(|retries| health(self, "retries", Value::Number(retries.into()))),
            Reads::LogDriver => {
                self.service
                    .child("logging")
                    .set("driver", Value::text(value));
                true
            }
            Reads::LogOption => self.log_option(value),
            Reads::Ulimit => self.ulimit(value),
            _ => false,
        };
        if !converted {
            self.skip(option);
        }
    }

    fn named(&mut self, value: &str) -> bool {
        self.service.set("container_name", Value::text(value));
        self.name = Some(value.to_owned());
        true
    }

    /// Sets `key` when `value` is one of the values that Compose also has.
    fn only(&mut self, key: &str, value: &str, known: &[&str]) -> bool {
        let fits = known.contains(&value);
        if fits {
            self.service.set(key, Value::text(value));
        }
        fits
    }

    fn volume(&mut self, value: &str) -> bool {
        if let Some((source, _)) = value.split_once(':')
            && is_name(source)
        {
            add(&mut self.volumes, source.to_owned());
        }
        self.service.push("volumes", Value::text(value));
        true
    }

    fn mount(&mut self, value: &str) -> bool {
        let Some(mount) = mounts::mount(value) else {
            return false;
        };
        if let Some(name) = mount.named {
            add(&mut self.volumes, name);
        }
        self.service.push("volumes", mount.volume);
        true
    }

    fn network(&mut self, value: &str) -> bool {
        if matches!(value, "host" | "none" | "bridge") || value.starts_with("container:") {
            self.service.set("network_mode", Value::text(value));
        } else if is_name(value) && value != "default" {
            add(&mut self.networks, value.to_owned());
        } else {
            return false;
        }
        true
    }

    /// Docker takes the value as the one program to run, so the file has a list of one.
    fn entrypoint(&mut self, value: &str) -> bool {
        let entrypoint = if value.is_empty() {
            Value::text("")
        } else {
            Value::List(vec![Value::text(value)])
        };
        self.service.set("entrypoint", entrypoint);
        true
    }

    fn stop_timeout(&mut self, value: &str) -> bool {
        let seconds = digits(value);
        if seconds {
            let period = Value::Text(format!("{value}s"));
            self.service.set("stop_grace_period", period);
        }
        seconds
    }

    fn log_option(&mut self, value: &str) -> bool {
        let Some((key, value)) = value.split_once('=').filter(|(key, _)| is_name(key)) else {
            return false;
        };
        let logging = self.service.child("logging");
        logging.child("options").set(key, Value::text(value));
        true
    }

    /// `name=soft` or `name=soft:hard`, each a whole number.
    fn ulimit(&mut self, value: &str) -> bool {
        let Some((name, limits)) = value.split_once('=').filter(|(name, _)| is_name(name)) else {
            return false;
        };
        let (soft, hard) = limits.split_once(':').unwrap_or((limits, limits));
        let (Ok(soft), Ok(hard)) = (soft.parse::<i64>(), hard.parse::<i64>()) else {
            return false;
        };
        let limit = if soft == hard {
            Value::Number(soft)
        } else {
            let mut both = Value::map();
            both.set("soft", Value::Number(soft));
            both.set("hard", Value::Number(hard));
            both
        };
        self.service.child("ulimits").set(name, limit);
        true
    }

    /// The networks of the service, and those of the top level: each is one that exists.
    fn networks(&mut self) -> Option<Value> {
        if self.networks.is_empty() {
            if !self.aliases.is_empty() {
                self.skip("--network-alias");
            }
            return None;
        }
        let mut top = Value::map();
        let mut joined = Value::map();
        for name in &self.networks {
            top.child(name).set("external", Value::Truth(true));
            joined
                .child(name)
                .set("aliases", Value::List(self.aliases.clone()));
        }
        let listed = || self.networks.iter().map(|name| Value::text(name)).collect();
        let networks = if self.aliases.is_empty() {
            Value::List(listed())
        } else {
            joined
        };
        self.service.set("networks", networks);
        self.note(Note::ExternalNetwork);
        Some(top)
    }

    /// The volumes of the top level. Each keeps the name that it has in Docker, so that
    /// Compose uses the volume that `docker run` used.
    fn volumes(&mut self) -> Option<Value> {
        if self.volumes.is_empty() {
            return None;
        }
        let mut top = Value::map();
        for name in &self.volumes {
            top.child(name).set("name", Value::text(name));
        }
        self.note(Note::VolumeNameKept);
        Some(top)
    }

    /// The Compose file of the service, with its name.
    pub(super) fn file(&mut self, image: &str, command: Vec<Value>) -> (String, Value) {
        self.service.set("image", Value::text(image));
        if !command.is_empty() {
            self.service.set("command", Value::List(command));
        }
        let networks = self.networks();
        let volumes = self.volumes();
        if let Some(entries) = self.service.entries_mut() {
            let rank = |key: &str| FIRST_KEYS.iter().position(|first| *first == key);
            entries.sort_by_key(|(key, _)| rank(key).unwrap_or(FIRST_KEYS.len()));
        }
        let name = service_name(self.name.as_deref().unwrap_or_else(|| image_name(image)));
        let mut file = Value::map();
        file.child("services")
            .set(&name, std::mem::replace(&mut self.service, Value::map()));
        for (key, value) in [("volumes", volumes), ("networks", networks)] {
            if let Some(value) = value {
                file.set(key, value);
            }
        }
        (name, file)
    }
}
