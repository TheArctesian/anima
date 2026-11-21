//! The Compose file of a recipe against its entry: the rules of task H28 with the settings
//! that exactly the marks permit, the images by digest, the volumes, the rights of the
//! services and the test of a machine.

use compose_rules::{Node, Settings, forms::whole};

use crate::{
    ANIMA_PORTS, Code,
    entry::{Anima, HealthTest, Kind, Mark, Protocol, Reach},
    example, privileges,
};

/// The longest time limit of a test of a machine, in seconds.
const LONGEST_TEST: u32 = 900;
/// The address of the machine itself.
const LOCAL: &str = "127.0.0.1";

/// `repository@sha256:<64 hex digits>`, with no tag.
fn is_pinned(image: &str) -> bool {
    let Some((repository, digest)) = image.split_once('@') else {
        return false;
    };
    let hex = digest.strip_prefix("sha256:").unwrap_or_default();
    let tagless = !repository
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .contains(':');
    compose_rules::forms::is_image(image)
        && tagless
        && hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn services(document: &Node) -> &[(String, Node)] {
    document
        .get("services")
        .and_then(Node::entries)
        .unwrap_or(&[])
}

fn ports(service: &Node) -> &[Node] {
    match service.get("ports") {
        Some(Node::List(ports)) => ports,
        _ => &[],
    }
}

/// The published ports of the file on an address other than the machine itself.
fn open_ports(document: &Node) -> Vec<(u64, Protocol)> {
    let mut open: Vec<(u64, Protocol)> = services(document)
        .iter()
        .flat_map(|(_, service)| ports(service))
        .filter(|port| port.get("host_ip").and_then(Node::text) != Some(LOCAL))
        .filter_map(|port| {
            let published = port.get("published").and_then(Node::text).and_then(whole)?;
            let protocol = match port.get("protocol").and_then(Node::text) {
                Some("udp") => Protocol::Udp,
                _ => Protocol::Tcp,
            };
            Some((published, protocol))
        })
        .collect();
    open.sort_unstable();
    open.dedup();
    open
}

/// True when a service publishes a port of Anima on the machine, on any address.
fn publishes_anima_port(document: &Node) -> bool {
    services(document)
        .iter()
        .flat_map(|(_, service)| ports(service))
        .filter_map(|port| port.get("published").and_then(Node::text).and_then(whole))
        .any(|published| {
            ANIMA_PORTS
                .iter()
                .any(|anima| u64::from(*anima) == published)
        })
}

/// The ports that the `network-port` marks name.
fn marked_ports(marks: &[Mark]) -> Vec<(u64, Protocol)> {
    let mut marked: Vec<(u64, Protocol)> = marks
        .iter()
        .flat_map(|mark| match mark {
            Mark::NetworkPort { port, protocols } => protocols
                .iter()
                .map(|protocol| (u64::from(*port), *protocol))
                .collect(),
            _ => Vec::new(),
        })
        .collect();
    marked.sort_unstable();
    marked.dedup();
    marked
}

fn images(anima: &Anima, document: &Node) -> Vec<Code> {
    let used: Vec<(&str, Option<&str>)> = services(document)
        .iter()
        .map(|(name, service)| (name.as_str(), service.get("image").and_then(Node::text)))
        .collect();
    let mut codes = Vec::new();
    let pinned = used
        .iter()
        .map(|(_, image)| image.unwrap_or_default())
        .chain(anima.images.iter().map(|listed| listed.image.as_str()))
        .all(is_pinned);
    if !pinned {
        codes.push(Code::ImageNotPinned);
    }
    let listed = used.iter().all(|(service, image)| {
        anima
            .images
            .iter()
            .any(|listed| listed.service == *service && Some(listed.image.as_str()) == *image)
    });
    if !listed {
        codes.push(Code::ImageNotListed);
    }
    let each_used = anima.images.iter().all(|listed| {
        used.iter().any(|(service, image)| {
            listed.service == *service && Some(listed.image.as_str()) == *image
        })
    });
    if !each_used {
        codes.push(Code::ImageNotUsed);
    }
    codes
}

fn volumes_described(anima: &Anima, document: &Node) -> bool {
    let mut declared: Vec<&str> = document
        .get("volumes")
        .and_then(Node::entries)
        .unwrap_or(&[])
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let mut described: Vec<&str> = anima
        .volumes
        .iter()
        .map(|kept| kept.volume.as_str())
        .collect();
    declared.sort_unstable();
    described.sort_unstable();
    declared == described
}

/// True when a service of the file has a `ports` entry.
fn publishes(document: &Node) -> bool {
    services(document)
        .iter()
        .any(|(_, service)| !ports(service).is_empty())
}

/// The test reaches a port that its service publishes, and says what a healthy answer is. A
/// database server publishes no port: its test reaches a port that the service exposes on the
/// network of its project.
fn test_complete(test: &HealthTest, kind: Kind, document: &Node) -> bool {
    let port = test.port.to_string();
    let published = services(document)
        .iter()
        .find(|(name, _)| *name == test.service)
        .is_some_and(|(_, service)| match kind {
            Kind::Application => ports(service)
                .iter()
                .any(|entry| entry.get("target").and_then(Node::text) == Some(port.as_str())),
            Kind::Database => match service.get("expose") {
                Some(Node::List(exposed)) => exposed
                    .iter()
                    .any(|entry| entry.text() == Some(port.as_str())),
                _ => false,
            },
        });
    let answer = match test.reach {
        Reach::Http => {
            test.path
                .as_deref()
                .is_some_and(|path| path.starts_with('/'))
                && !test.statuses.is_empty()
                && test
                    .statuses
                    .iter()
                    .all(|status| (100..=599).contains(status))
                && test.body.as_deref().is_none_or(|body| !body.is_empty())
        }
        Reach::Tcp => test.path.is_none() && test.statuses.is_empty() && test.body.is_none(),
    };
    published && answer && (1..=LONGEST_TEST).contains(&test.limit_seconds)
}

/// True when the image, a published port, its address, the command, the entrypoint or the
/// host name is written with a variable: the install fills it after the static test, so that
/// the test would not see the value that runs (SF-NEW-revcatvars, SF-NEW-revcatentry).
fn variable_in_fixed_value(unfilled: &Node) -> bool {
    let has_variable = |node: Option<&Node>| match node {
        Some(Node::Text(text)) => text.contains('$'),
        Some(Node::List(items)) => items
            .iter()
            .any(|item| item.text().is_some_and(|text| text.contains('$'))),
        _ => false,
    };
    services(unfilled).iter().any(|(_, service)| {
        ["image", "command", "entrypoint", "hostname"]
            .into_iter()
            .any(|key| has_variable(service.get(key)))
            || ports(service).iter().any(|port| {
                has_variable(port.get("published")) || has_variable(port.get("host_ip"))
            })
    })
}

/// True when each stream of the entry is a port of the file with each of its protocols.
fn streams_published(anima: &Anima, document: &Node) -> bool {
    let published: Vec<(String, Protocol)> = services(document)
        .iter()
        .flat_map(|(_, service)| ports(service))
        .filter_map(|port| {
            let target = port.get("target").and_then(Node::text)?.to_owned();
            let protocol = match port.get("protocol").and_then(Node::text) {
                Some("udp") => Protocol::Udp,
                _ => Protocol::Tcp,
            };
            Some((target, protocol))
        })
        .collect();
    anima.streams.iter().all(|stream| {
        !stream.protocols.is_empty()
            && stream
                .protocols
                .iter()
                .all(|protocol| published.contains(&(stream.port.to_string(), *protocol)))
    })
}

/// The codes of the file as its author wrote it: the values that no variable may give, the
/// secrets and the rights of the services.
fn written(anima: &Anima, unfilled: &Node) -> Vec<Code> {
    let mut codes = Vec::new();
    if variable_in_fixed_value(unfilled) {
        codes.push(Code::VariableNotPermitted);
    }
    if example::secret_in_file(services(unfilled)) {
        codes.push(Code::SecretInFile);
    }
    codes.extend(
        privileges::check(
            services(unfilled),
            &anima.exceptions,
            anima.origin.is_some(),
        )
        .into_iter()
        .map(Code::PrivilegeNotPermitted),
    );
    codes
}

/// The codes of the Compose file `compose`, filled from the example `environment`.
pub(crate) fn check(anima: &Anima, compose: &str, environment: &str) -> Vec<Code> {
    let mut codes: Vec<Code> = anima
        .marks
        .iter()
        .filter(|mark| !matches!(mark, Mark::NetworkPort { .. }))
        .map(|mark| Code::MarkNotPermitted(mark.clone()))
        .collect();
    let marked = marked_ports(&anima.marks);
    let settings = Settings {
        open_to_network: !marked.is_empty(),
        label_prefixes: Vec::new(),
    };
    let (document, unfilled) = match compose_rules::check(compose, environment, &settings) {
        Ok(checked) => (checked.document, checked.unfilled),
        Err(refusal) => {
            codes.push(Code::ComposeRefused(refusal));
            return codes;
        }
    };
    if settings.open_to_network {
        if compose_rules::check(compose, environment, &Settings::default()).is_ok() {
            codes.push(Code::MarkNotNeeded);
        }
        if open_ports(&document) != marked {
            codes.push(Code::MarkNotMatched);
        }
    }
    codes.extend(written(anima, &unfilled));
    codes.extend(images(anima, &document));
    if !volumes_described(anima, &document) {
        codes.push(Code::VolumeNotDescribed);
    }
    if !streams_published(anima, &document) {
        codes.push(Code::StreamNotPublished);
    }
    if publishes_anima_port(&document) {
        codes.push(Code::AnimaPortPublished);
    }
    if anima.kind == Kind::Database && publishes(&document) {
        codes.push(Code::DatabasePortPublished);
    }
    if !test_complete(&anima.test, anima.kind, &document) {
        codes.push(Code::TestNotComplete);
    }
    codes
}
