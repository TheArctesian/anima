//! The final form of a project (K10 Section 6.4.4, step 5, and "What the program adds").

use compose_rules::{Rule, check};
use serde_json::{Value, json};

use super::{
    DIGEST, PROJECT,
    applying::{COMPOSE, ENVIRONMENT, digests},
    pulled,
};
use crate::{
    Frame, Refusal,
    final_form::formed,
    settings::{Limits, Machine, OwnLimits},
};

const BINDS: &str = "/var/lib/anima/applications/0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e/binds";

fn own() -> OwnLimits {
    Limits::after_install().own(&Machine {
        memory_bytes: 8 << 30,
        processors: 4,
        free_disk_bytes: 0,
    })
}

/// The text of the final form of `file`, with "Open to the network" as `open`.
fn text_of(file: &str, own: &OwnLimits, open: bool) -> Result<String, Refusal> {
    let settings = compose_rules::Settings {
        open_to_network: open,
        label_prefixes: vec!["traefik.".to_owned()],
    };
    let checked = check(file, ENVIRONMENT, &settings).expect("the file passes the check");
    let digests = digests();
    let frame = Frame {
        project: PROJECT,
        binds: BINDS,
        own,
        open_to_network: open,
        digests: &digests,
    };
    formed(&checked, &frame).map(|form| form.text)
}

fn form_of(file: &str) -> Value {
    let text = text_of(file, &own(), false).expect("a final form");
    serde_json::from_str(&text).expect("the final form is JSON")
}

#[test]
fn k10_6_4_4_the_final_form_names_the_project_and_each_image_by_the_digest_of_its_pull() {
    let form = form_of(COMPOSE);
    assert_eq!(form["name"], PROJECT);
    assert_eq!(
        form["services"]["app"]["image"],
        format!("example/tracks@{DIGEST}")
    );
    assert_eq!(
        form["services"]["db"]["image"],
        format!("postgres@{DIGEST}")
    );
    let registry = COMPOSE.replacen(
        "postgres:17",
        "registry.example:5000/library/postgres:17",
        1,
    );
    let checked =
        check(&registry, ENVIRONMENT, &compose_rules::Settings::default()).expect("passes");
    let digests = [
        pulled("app", "example/tracks:1"),
        pulled("db", "registry.example:5000/library/postgres:17"),
    ];
    let own = own();
    let frame = Frame {
        project: PROJECT,
        binds: BINDS,
        own: &own,
        open_to_network: false,
        digests: &digests,
    };
    let text = formed(&checked, &frame).expect("a final form").text;
    assert!(text.contains(&format!(
        "\"registry.example:5000/library/postgres@{DIGEST}\""
    )));
}

#[test]
fn k10_6_4_4_each_value_is_written_with_its_type() {
    let form = form_of(COMPOSE);
    let app = &form["services"]["app"];
    assert_eq!(app["command"], json!(["bin/rails", "server"]));
    assert_eq!(
        app["healthcheck"],
        json!({"test": ["CMD", "true"], "interval": "60s", "retries": 3})
    );
    assert_eq!(
        app["configs"],
        json!([{"source": "settings", "target": "/etc/tracks.yml", "mode": "0444"}])
    );
    assert_eq!(
        app["volumes"][0],
        json!({"type": "volume", "source": "store", "target": "/store"})
    );
    assert_eq!(form["volumes"], json!({"store": {}}));
    assert_eq!(form["networks"], json!({"inner": {"internal": true}}));
    let db = &form["services"]["db"];
    assert_eq!(
        db["depends_on"],
        json!({"app": {"condition": "service_started"}})
    );
    assert_eq!(db["networks"], json!({"inner": {"aliases": ["database"]}}));
}

#[test]
fn k10_6_4_4_the_program_adds_net_raw_to_cap_drop_and_sets_the_options_that_are_its_own() {
    let form = form_of(COMPOSE);
    assert_eq!(
        form["services"]["app"]["cap_drop"],
        json!(["ALL", "NET_RAW"])
    );
    assert_eq!(form["services"]["db"]["cap_drop"], json!(["NET_RAW"]));
    let twice = COMPOSE.replacen("cap_drop: [ALL]", "cap_drop: [NET_RAW]", 1);
    assert_eq!(
        form_of(&twice)["services"]["app"]["cap_drop"],
        json!(["NET_RAW"])
    );
    assert_eq!(
        form["services"]["app"]["tmpfs"],
        json!(["/run:rw,noexec,nosuid,nodev"])
    );
    let logging = json!({"driver": "json-file", "options": {"max-size": "10m", "max-file": "3"}});
    assert_eq!(form["services"]["db"]["logging"], logging);
    let local = COMPOSE.replacen("    tmpfs: /run", "    logging:\n      driver: local", 1);
    assert_eq!(
        form_of(&local)["services"]["app"]["logging"]["driver"],
        "local"
    );
}

#[test]
fn k10_6_4_4_each_limit_is_the_lower_of_the_file_and_the_own_limit() {
    let form = form_of(COMPOSE);
    let app = &form["services"]["app"];
    // The file is lower: 512 MiB below a quarter of 8 GiB, half a processor below four.
    assert_eq!(app["mem_limit"], 512 << 20);
    assert_eq!(app["cpus"], 0.5);
    // The own limit is lower: 90 seconds in the file, 30 seconds at the machine.
    assert_eq!(app["stop_grace_period"], "30s");
    // The file has none: the own limit.
    assert_eq!(app["pids_limit"], 512);
    let db = &form["services"]["db"];
    assert_eq!(db["mem_limit"], 2_u64 << 30);
    assert_eq!(db["cpus"], 4.0);
    assert_eq!(db["stop_grace_period"], "30s");
    let narrow = OwnLimits {
        memory_bytes: 100 << 20,
        processes: 50,
        processor_billionths: 250_000_000,
        stop_seconds: 5,
    };
    let more = COMPOSE.replacen("    tmpfs: /run", "    pids_limit: 4000", 1);
    let text = text_of(&more, &narrow, false).expect("a final form");
    let form: Value = serde_json::from_str(&text).expect("JSON");
    let app = &form["services"]["app"];
    assert_eq!(app["mem_limit"], 100 << 20);
    assert_eq!(app["pids_limit"], 50);
    assert_eq!(app["cpus"], 0.25);
    assert_eq!(app["stop_grace_period"], "5s");
    assert!(text.contains("\"cpus\":0.25,"));
}

#[test]
fn k10_6_4_4_a_port_is_on_the_machine_itself_unless_open_to_the_network_is_on() {
    let closed = json!([{"target": 3000, "published": 8080, "host_ip": "127.0.0.1"}]);
    assert_eq!(form_of(COMPOSE)["services"]["app"]["ports"], closed);
    let open_file = COMPOSE.replacen("        host_ip: 127.0.0.1\n", "        protocol: udp\n", 1);
    let text = text_of(&open_file, &own(), true).expect("a final form");
    let form: Value = serde_json::from_str(&text).expect("JSON");
    let open = json!([{"target": 3000, "published": 8080, "protocol": "udp"}]);
    assert_eq!(form["services"]["app"]["ports"], open);
    let each = COMPOSE.replacen("host_ip: 127.0.0.1", "host_ip: 192.168.1.9", 1);
    let text = text_of(&each, &own(), true).expect("a final form");
    assert!(text.contains("\"host_ip\":\"192.168.1.9\""));
}

#[test]
fn k10_6_4_5_a_bind_mount_is_the_real_path_of_its_folder_below_binds() {
    let form = form_of(COMPOSE);
    let bind = json!({
        "type": "bind",
        "source": format!("{BINDS}/config"),
        "bind": {"create_host_path": false},
        "target": "/config",
        "read_only": true
    });
    assert_eq!(form["services"]["app"]["volumes"][1], bind);
    let settings = compose_rules::Settings::default();
    let checked = check(COMPOSE, ENVIRONMENT, &settings).expect("passes");
    let digests = digests();
    let own = own();
    let frame = Frame {
        project: PROJECT,
        binds: BINDS,
        own: &own,
        open_to_network: false,
        digests: &digests,
    };
    let form = formed(&checked, &frame).expect("a final form");
    assert_eq!(form.binds, ["config"]);
    assert!(form.holds_volumes);
}

#[test]
fn k10_6_4_4_each_dollar_is_written_two_times_and_each_control_character_is_escaped() {
    let text = text_of(COMPOSE, &own(), false).expect("a final form");
    // `$$5 ${TOKEN}` with TOKEN `s3cr$t` is `$5 s3cr$t`, written with each `$` two times.
    assert!(text.contains(r#""environment":{"PRICE":"$$5 s3cr$$t"}"#));
    assert!(text.contains(r#""content":"a\u2028b""#));
    let controls = COMPOSE.replacen(
        "\"a\\u2028b\"",
        "\"\\x7f\\u0085\\u2029\\uFEFF\\t\\\\ \\\" \\U0001F600\"",
        1,
    );
    let text = text_of(&controls, &own(), false).expect("a final form");
    assert!(text.contains(r#""content":"\u007f\u0085\u2029\ufeff\u0009\\ \" 😀""#));
    assert!(text.is_ascii() || text.contains('😀'));
    assert!(text.ends_with("}\n") && !text[..text.len() - 1].contains('\n'));
}

#[test]
fn k10_6_4_4_a_nul_in_a_value_is_refused() {
    let nul = COMPOSE.replacen("\"a\\u2028b\"", "\"a\\0b\"", 1);
    assert_eq!(
        text_of(&nul, &own(), false),
        Err(Refusal::file(Rule::ValueNotPermitted))
    );
    let in_a_service = COMPOSE.replacen("\"$$5 ${TOKEN}\"", "\"a\\0b\"", 1);
    assert_eq!(
        text_of(&in_a_service, &own(), false),
        Err(Refusal::FileNotPermitted {
            rule: Rule::ValueNotPermitted,
            service: Some(0)
        })
    );
}

#[test]
fn k10_6_4_4_a_label_of_a_named_prefix_and_an_environment_as_a_list_keep_their_form() {
    let file = COMPOSE
        .replacen("    tmpfs: /run", "    labels:\n      traefik.enable: \"true\"\n    expose: [\"9000\"]\n    read_only: true", 1)
        .replacen("    depends_on:\n      app:", "    environment:\n      - POSTGRES_PASSWORD=${TOKEN}\n    depends_on: [app]", 1);
    let form = form_of(&file);
    assert_eq!(
        form["services"]["app"]["labels"],
        json!({"traefik.enable": "true"})
    );
    assert_eq!(form["services"]["app"]["expose"], json!([9000]));
    assert_eq!(form["services"]["app"]["read_only"], true);
    assert_eq!(
        form["services"]["db"]["environment"],
        json!(["POSTGRES_PASSWORD=s3cr$$t"])
    );
    assert_eq!(form["services"]["db"]["depends_on"], json!(["app"]));
}
