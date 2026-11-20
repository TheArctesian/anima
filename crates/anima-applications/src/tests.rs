//! Tests of part 1 of the applications program (task H31; contract K10 Section 6.4): the
//! settings (`settings`), the pair of a hub (`hub_pair`), the reader of an item (`requests`),
//! the containers that a type reaches (`reaching`), each type (`applying`, `going_back`,
//! `adopting`, `removing`, `pruning`), the final form (`final_forms`) and the start
//! (`starting`). Each test names the rule that it shows, for the permitted and the refused
//! case.

mod adopting;
mod applying;
mod final_forms;
mod going_back;
mod hub_pair;
mod pruning;
mod reaching;
mod removing;
mod requests;
mod settings;
mod starting;

use crate::{
    Command, Container, EngineKind, Installed, Inventory, Pair, PairFile, PairRecord, Paths, Plan,
    Pulled, Records, Settings, Step,
    settings::{Machine, Managed},
    work::Facts,
};

/// One application, its project and the identifiers of three containers.
const APPLICATION: &str = "0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e";
const PROJECT: &str = "anima-0192f6b07c1e7a3b9d2f4e5a6b7c8d9e";
const OF_ANIMA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OF_ADOPTED: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OF_THE_OWNER: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DIGEST: &str = "sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf";
const SET_ON_AT: u64 = 1_795_000_000;
const FILES: &str = "/var/lib/anima/applications/0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e/files";

/// The facts of one machine, which a test changes before it asks.
struct World {
    settings: Settings,
    paths: Paths,
    inventory: Inventory,
    records: Records,
    pair: Option<PairRecord>,
    machine: Machine,
    busy: bool,
}

fn pair() -> Pair {
    Pair::of(
        "https://hub.example:443",
        "0192f000-0000-7000-8000-000000000001",
    )
    .expect("a pair")
}

fn container(id: &str, name: &str, project: Option<&str>) -> Container {
    Container {
        id: id.to_owned(),
        name: name.to_owned(),
        project: project.map(str::to_owned),
        folder: (project == Some(PROJECT)).then(|| FILES.to_owned()),
        running: true,
        restarts: false,
    }
}

fn pulled(service: &str, image: &str) -> Pulled {
    Pulled {
        service: service.to_owned(),
        image: image.to_owned(),
        digest: DIGEST.to_owned(),
    }
}

impl World {
    /// A machine with "Manage applications" on and each other setting as after the install:
    /// one application that Anima installed, one adopted project, and one container of the
    /// owner that Anima does not know.
    fn new() -> Self {
        let mut settings = Settings::after_install();
        settings.managed = Some(Managed {
            engine: EngineKind::System,
            socket: "/run/docker.sock".to_owned(),
            docker: "/usr/bin/docker".to_owned(),
            compose: "/usr/libexec/docker/cli-plugins/docker-compose".to_owned(),
            accepted_user: 1000,
            set_on_at: SET_ON_AT,
        });
        settings.adopted = vec![crate::settings::Adopted {
            name: "tracks".to_owned(),
            folder: "/srv/tracks".to_owned(),
            files: vec!["/srv/tracks/compose.yaml".to_owned()],
        }];
        Self {
            settings,
            paths: Paths {
                docker: "/usr/bin/docker".to_owned(),
                compose: "/usr/libexec/docker/cli-plugins/docker-compose".to_owned(),
                socket: "/run/docker.sock".to_owned(),
                configuration: "/var/lib/anima/docker-configuration".to_owned(),
                root: "/var/lib/anima/applications".to_owned(),
            },
            inventory: Inventory {
                containers: vec![
                    container(OF_ANIMA, "anima-app-1", Some(PROJECT)),
                    container(OF_ADOPTED, "tracks-web-1", Some("tracks")),
                    container(OF_THE_OWNER, "backup", None),
                ],
                ..Inventory::default()
            },
            records: Records {
                applications: vec![Installed {
                    application: APPLICATION.to_owned(),
                    has_files: true,
                    current: vec![pulled("app", "example/tracks:1")],
                    kept: None,
                    pulled: vec![format!("example/tracks@{DIGEST}")],
                }],
            },
            pair: Some(PairRecord {
                set_on_at: SET_ON_AT,
                first: pair(),
                different_seen: false,
            }),
            machine: Machine {
                memory_bytes: 8 << 30,
                processors: 4,
                free_disk_bytes: 100 << 30,
            },
            busy: false,
        }
    }

    fn facts(&self) -> Facts<'_> {
        Facts {
            settings: &self.settings,
            paths: &self.paths,
            inventory: &self.inventory,
            records: &self.records,
            pair: self.pair.as_ref().map_or(PairFile::Missing, PairFile::Read),
            machine: self.machine,
            busy: self.busy,
        }
    }
}

/// The commands of a plan, in their order.
fn commands(plan: &Plan) -> Vec<&Command> {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            Step::Run(command) => Some(command),
            _ => None,
        })
        .collect()
}

/// The arguments of the one command of a plan.
fn arguments(plan: &Plan) -> Vec<&str> {
    let commands = commands(plan);
    assert_eq!(commands.len(), 1, "the plan has one command");
    commands[0].arguments.iter().map(String::as_str).collect()
}

/// The first arguments of each command: the socket and the configuration folder.
const FIRST: [&str; 4] = [
    "--host",
    "unix:///run/docker.sock",
    "--config",
    "/var/lib/anima/docker-configuration",
];

/// The text of an item of the type `kind` with `parameters`.
fn item(kind: &str, parameters: &str) -> String {
    let secrets = if kind == "application_apply" {
        "\"compose_file\""
    } else {
        ""
    };
    format!(
        r#"{{"id":"0192f6aa-0000-7000-8000-000000000002","type":"{kind}","kind":"host","attempt":1,"lease":{{"until":"2026-11-20T10:01:30Z"}},"limits":{{"seconds":120,"idleSeconds":0,"memoryBytes":0,"diskBytes":0,"orphanSeconds":0}},"project":null,"secrets":[{secrets}],"parameters":{parameters}}}"#
    )
}
