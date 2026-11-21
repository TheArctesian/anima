//! The three texts of a generated recipe, from its plan.

use std::fmt::Write as _;

use super::{
    Image, Pin,
    entry::{Anima, Device, Exception, Kept, LastTest, Origin, Pinned, Stream, Test, Written},
    plan::{Plan, Published, Value},
};
use crate::{
    Recipe,
    entry::Protocol,
    example::GENERATED,
    fields::{COMPOSE, ENVIRONMENT},
};

/// The registry address of the images of the publisher.
const REGISTRY: &str = "lscr.io/linuxserver/";
/// The longest description, in characters.
const LONGEST_DESCRIPTION: usize = 300;
/// The time limit of the test of a machine of a generated entry, in seconds.
const TEST_SECONDS: u32 = 300;
/// The capabilities that a service with the exception of the init drops (Section 13.3).
const INIT_DROP: [&str; 9] = [
    "AUDIT_WRITE",
    "FSETID",
    "KILL",
    "MKNOD",
    "NET_BIND_SERVICE",
    "NET_RAW",
    "SETFCAP",
    "SETPCAP",
    "SYS_CHROOT",
];

const fn protocol(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Tcp => "tcp",
        Protocol::Udp => "udp",
    }
}

/// A text as a scalar of YAML in double quotes: the form of JSON is a form of YAML.
fn quoted(text: &str) -> String {
    serde_json::Value::from(text).to_string()
}

/// The port that the test of a machine reaches: the first TCP port.
fn test_port(plan: &Plan) -> u16 {
    plan.ports
        .iter()
        .find(|port| port.protocol == Protocol::Tcp)
        .map_or(0, |port| port.target)
}

fn compose(plan: &Plan, image: &str) -> String {
    let mut text = format!("services:\n  {}:\n    image: {image}\n", plan.service);
    text.push_str("    environment:\n");
    for setting in &plan.settings {
        let value = match &setting.value {
            Value::Fixed(value) => quoted(value),
            Value::Example(_) | Value::Generated => format!("${{{}}}", setting.name),
        };
        writeln!(text, "      {}: {value}", setting.name).expect("a String takes text");
    }
    text.push_str(
        "    restart: unless-stopped\n    security_opt:\n      - no-new-privileges:true\n",
    );
    text.push_str("    cap_drop:\n");
    for capability in INIT_DROP {
        writeln!(text, "      - {capability}").expect("a String takes text");
    }
    text.push_str("    ports:\n");
    for Published {
        target,
        on_machine,
        protocol: kind,
    } in &plan.ports
    {
        writeln!(text, "      - target: {target}\n        published: {on_machine}\n        host_ip: 127.0.0.1\n        protocol: {}",
            protocol(*kind)
        ).expect("a String takes text");
    }
    if !plan.volumes.is_empty() {
        text.push_str("    volumes:\n");
        for (name, target) in &plan.volumes {
            writeln!(
                text,
                "      - type: volume\n        source: {name}\n        target: {target}"
            )
            .expect("a String takes text");
        }
        text.push_str("volumes:\n");
        for (name, _) in &plan.volumes {
            writeln!(text, "  {name}:").expect("a String takes text");
        }
    }
    text
}

fn environment(plan: &Plan, name: &str) -> String {
    let mut text = format!(
        "# {name}, generated from the data of LinuxServer.io. The install makes each value that\n# is marked below; this file holds no secret value.\n"
    );
    for setting in &plan.settings {
        match &setting.value {
            Value::Fixed(_) => {}
            Value::Example(value) => {
                writeln!(text, "{}={value}", setting.name).expect("a String takes text");
            }
            Value::Generated => {
                writeln!(text, "{GENERATED}\n{}=", setting.name).expect("a String takes text");
            }
        }
    }
    text
}

/// The first sentence of the description of the publisher, on one line.
fn description(image: &Image) -> String {
    let flat = image
        .description
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let first = flat
        .split(". ")
        .next()
        .unwrap_or_default()
        .trim_end_matches('.');
    let short: String = first.chars().take(LONGEST_DESCRIPTION).collect();
    if short.is_empty() {
        format!("The image {} of LinuxServer.io.", image.name)
    } else {
        format!("{short}.")
    }
}

fn tags(image: &Image) -> Vec<String> {
    let mut tags = vec!["linuxserver".to_owned()];
    for tag in image.category.split(',') {
        let tag = tag.trim().to_lowercase();
        if !tag.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

/// The streams: each published port but the TCP port of the test, with its protocols.
fn streams(plan: &Plan) -> Vec<Stream> {
    let tested = test_port(plan);
    let mut streams: Vec<Stream> = Vec::new();
    for port in &plan.ports {
        if port.target == tested && port.protocol == Protocol::Tcp {
            continue;
        }
        let kind = protocol(port.protocol);
        match streams.iter_mut().find(|stream| stream.port == port.target) {
            Some(stream) => stream.protocols.push(kind),
            None => streams.push(Stream {
                port: port.target,
                protocols: vec![kind],
            }),
        }
    }
    streams
}

/// The fields of Anima of a generated entry.
fn anima<'a>(plan: &'a Plan, pin: &'a Pin, reference: String) -> Anima<'a> {
    let service = plan.service.as_str();
    Anima {
        class: if plan.shell { "shell" } else { "application" },
        origin: Origin {
            from: "linuxserver",
            read: &pin.read,
        },
        images: [Pinned {
            service,
            image: reference,
            tag: &pin.tag,
            resolved: &pin.resolved,
        }],
        marks: [],
        exceptions: [Exception {
            exception: "init-capabilities",
            services: [service],
        }],
        streams: streams(plan),
        optional: plan
            .devices
            .iter()
            .map(|path| Device {
                part: "device",
                path,
            })
            .collect(),
        volumes: plan
            .volumes
            .iter()
            .map(|(volume, target)| Kept {
                volume,
                copy: if target == "/config" {
                    "sqlite"
                } else {
                    "files"
                },
            })
            .collect(),
        generated: plan
            .settings
            .iter()
            .filter(|setting| setting.value == Value::Generated)
            .map(|setting| setting.name.as_str())
            .collect(),
        test: Test {
            service,
            port: test_port(plan),
            reach: "tcp",
            path: None,
            statuses: [],
            body: None,
            limit_seconds: TEST_SECONDS,
        },
        pinned: &pin.resolved,
        last_test: LastTest {
            result: "not-tested",
        },
    }
}

fn entry(image: &Image, plan: &Plan, pin: &Pin, reference: String) -> String {
    let service = plan.service.as_str();
    let written = Written {
        id: service,
        name: &image.name,
        description: description(image),
        version: &pin.tag,
        author: "LinuxServer.io",
        tags: tags(image),
        compose_url: COMPOSE,
        env_url: ENVIRONMENT,
        documentation_url: &image.github_url,
        icon_url: Some(image.project_logo.as_str()).filter(|logo| logo.starts_with("https://")),
        anima: anima(plan, pin, reference),
    };
    let mut text =
        serde_json::to_string_pretty(&written).expect("an entry of texts and numbers is JSON");
    text.push('\n');
    text
}

/// The recipe folder of `image` from its plan.
pub(super) fn recipe(image: &Image, plan: &Plan, pin: &Pin) -> Recipe {
    let reference = format!("{REGISTRY}{}@{}", image.name, pin.digest);
    Recipe {
        folder: plan.service.clone(),
        entry: Some(entry(image, plan, pin, reference.clone())),
        compose: Some(compose(plan, &reference)),
        environment: Some(environment(plan, &image.name)),
    }
}
