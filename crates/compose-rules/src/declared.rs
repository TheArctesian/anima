//! The rules of the values of one service (K10 Section 6.4.4).

use crate::{
    Node, Refusal, Rule, Settings,
    forms::{bytes, is_image, is_line, is_name, is_processors, is_target, seconds, truth, whole},
    mounts,
    project::Names,
};

/// The shortest time between two runs of a health check, in seconds.
const SHORTEST_INTERVAL: u64 = 5;

/// One service and where it is, for the refusals.
pub(crate) struct Declared<'a> {
    pub node: &'a Node,
    pub position: usize,
}

impl Declared<'_> {
    pub(crate) const fn refusal(&self, rule: Rule) -> Refusal {
        Refusal::of(rule, self.position)
    }

    /// The text of a key; `None` when the key is not there or has no value.
    pub(crate) fn text(&self, key: &str) -> Option<&str> {
        self.node.get(key).and_then(Node::text)
    }

    /// The scalars of a key that is one scalar or a list of them.
    pub(crate) fn texts(&self, key: &str) -> Vec<&str> {
        match self.node.get(key) {
            Some(Node::Text(text)) => vec![text],
            Some(Node::List(items)) => items.iter().filter_map(Node::text).collect(),
            _ => Vec::new(),
        }
    }

    /// Checks the text of `key`, when it is there, with `rule`.
    fn each(&self, key: &str, kind: Rule, rule: impl Fn(&str) -> bool) -> Result<(), Refusal> {
        match self.node.get(key) {
            None => Ok(()),
            Some(_) if self.texts(key).iter().all(|text| rule(text)) => Ok(()),
            Some(_) => Err(self.refusal(kind)),
        }
    }
}

fn is_restart(text: &str) -> bool {
    match text.split_once(':') {
        Some(("on-failure", count)) => whole(count).is_some(),
        Some(_) => false,
        None => matches!(text, "no" | "always" | "unless-stopped" | "on-failure"),
    }
}

/// The name and the value of each entry of `environment` or `labels`: a map, or a list of
/// `NAME=value`. An entry with a name and no value is refused: Compose would fill it from its
/// own process.
fn pairs<'a>(service: &Declared<'a>, key: &str) -> Result<Vec<(&'a str, &'a str)>, Refusal> {
    let wrong = service.refusal(Rule::ValueNotPermitted);
    match service.node.get(key) {
        None => Ok(Vec::new()),
        Some(Node::Map(entries)) => entries
            .iter()
            .map(|(name, value)| Some((name.as_str(), value.text()?)))
            .collect::<Option<_>>()
            .ok_or(wrong),
        Some(Node::List(items)) => items
            .iter()
            .map(|item| item.text()?.split_once('='))
            .collect::<Option<_>>()
            .ok_or(wrong),
        Some(_) => Err(wrong),
    }
}

/// A label is permitted only when its key starts with a prefix that the owner named at the
/// machine, and never with the prefix of Compose itself.
fn labels(service: &Declared<'_>, settings: &Settings) -> Result<(), Refusal> {
    let permitted = |name: &str| {
        !name.starts_with("com.docker.")
            && settings
                .label_prefixes
                .iter()
                .any(|prefix| !prefix.is_empty() && name.starts_with(prefix.as_str()))
    };
    if pairs(service, "labels")?
        .iter()
        .all(|(name, _)| permitted(name))
    {
        Ok(())
    } else {
        Err(service.refusal(Rule::LabelNotPermitted))
    }
}

/// The limits: each is above zero. A value of zero means "no limit" to the engine.
fn limits(service: &Declared<'_>) -> Result<(), Refusal> {
    let kind = Rule::LimitNotPermitted;
    service.each("mem_limit", kind, |text| {
        bytes(text).is_some_and(|size| size > 0)
    })?;
    service.each("pids_limit", kind, |text| {
        whole(text).is_some_and(|count| count > 0)
    })?;
    service.each("cpus", kind, is_processors)?;
    service.each("stop_grace_period", kind, |text| {
        seconds(text).is_some_and(|time| time > 0)
    })
}

fn healthcheck(service: &Declared<'_>) -> Result<(), Refusal> {
    let Some(check) = service.node.get("healthcheck") else {
        return Ok(());
    };
    let wrong = service.refusal(Rule::ValueNotPermitted);
    let time = |key: &str, least: u64| match check.get(key).map(Node::text) {
        None => true,
        Some(text) => text.and_then(seconds).is_some_and(|time| time >= least),
    };
    let whole_or_none = |key: &str, rule: fn(&str) -> bool| match check.get(key).map(Node::text) {
        None => true,
        Some(text) => text.is_some_and(rule),
    };
    let sound = time("interval", SHORTEST_INTERVAL)
        && time("timeout", 1)
        && time("start_period", 0)
        && time("start_interval", 1)
        && whole_or_none("retries", |text| whole(text).is_some())
        && whole_or_none("disable", |text| truth(text).is_some());
    if sound { Ok(()) } else { Err(wrong) }
}

/// `depends_on` names services of this file; a condition is one that Compose knows.
fn depends_on(service: &Declared<'_>, all: &Names<'_>) -> Result<(), Refusal> {
    let wrong = service.refusal(Rule::ValueNotPermitted);
    let known = |name: &str| all.services.contains(&name);
    match service.node.get("depends_on") {
        None => Ok(()),
        Some(Node::List(items)) => items
            .iter()
            .all(|item| item.text().is_some_and(known))
            .then_some(())
            .ok_or(wrong),
        Some(node) => {
            let conditions = [
                "service_started",
                "service_healthy",
                "service_completed_successfully",
            ];
            let sound = node.entries().unwrap_or(&[]).iter().all(|(name, rule)| {
                let condition = rule.get("condition").and_then(Node::text);
                let flag = |key: &str| {
                    rule.get(key)
                        .is_none_or(|value| value.text().and_then(truth).is_some())
                };
                known(name)
                    && condition.is_none_or(|condition| conditions.contains(&condition))
                    && flag("restart")
                    && flag("required")
            });
            sound.then_some(()).ok_or(wrong)
        }
    }
}

/// The single values of a service that are one line of text, a truth value or a list.
fn plain_values(service: &Declared<'_>) -> Result<(), Refusal> {
    let kind = Rule::ValueNotPermitted;
    for key in [
        "command",
        "entrypoint",
        "user",
        "working_dir",
        "stop_signal",
    ] {
        service.each(key, kind, is_line)?;
    }
    service.each("hostname", kind, is_name)?;
    service.each("restart", kind, is_restart)?;
    service.each("read_only", kind, |text| truth(text).is_some())?;
    service.each("init", kind, |text| truth(text).is_some())?;
    service.each("expose", kind, |text| {
        whole(text).is_some_and(|port| (1..=65535).contains(&port))
    })?;
    service.each("cap_drop", kind, |text| {
        !text.is_empty()
            && text
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte == b'_')
    })?;
    service.each("security_opt", kind, |text| {
        text == "no-new-privileges:true"
    })?;
    service.each("tmpfs", Rule::MountNotPermitted, |text| {
        is_target(text) && !text.contains([':', ','])
    })?;
    match service
        .node
        .get("logging")
        .map(|logging| logging.get("driver").and_then(Node::text))
    {
        None | Some(Some("json-file" | "local")) => Ok(()),
        Some(_) => Err(service.refusal(kind)),
    }
}

/// Applies each rule to one service.
///
/// # Errors
/// The [`Refusal`] of the first rule that a value breaks.
pub(crate) fn checked(
    node: &Node,
    position: usize,
    all: &Names<'_>,
    settings: &Settings,
) -> Result<(), Refusal> {
    let service = Declared { node, position };
    match service.text("image") {
        Some(image) if is_image(image) => {}
        _ => return Err(service.refusal(Rule::ImageNotPermitted)),
    }
    plain_values(&service)?;
    let environment = pairs(&service, "environment")?;
    if !environment
        .iter()
        .all(|(name, _)| is_line(name) && !name.contains([' ', '=']))
    {
        return Err(service.refusal(Rule::ValueNotPermitted));
    }
    labels(&service, settings)?;
    limits(&service)?;
    healthcheck(&service)?;
    depends_on(&service, all)?;
    mounts::ports(&service, settings)?;
    mounts::checked(&service, all)
}
