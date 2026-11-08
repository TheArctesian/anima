//! The Docker reader over `docker ps -a --no-trunc --format '{{json .}}'` output (D-3).

use serde_json::json;

use super::{assert_unreadable_outputs, finished, verdict};
use crate::{
    checks::probe::{ProbeResult, docker},
    ssh::FixedCommand,
};

/// One recorded line of `docker ps`, with the fields Docker prints besides the three read.
fn line(names: &str, state: &str, status: &str) -> String {
    json!({
        "Command": "\"/docker-entrypoint.sh nginx -g 'daemon off;'\"",
        "CreatedAt": "2026-11-01 08:12:44 +0000 UTC",
        "ID": "3f1a9c0d5b7e4a62c1d8e0f3a4b5c6d7e8f90123456789abcdef0123456789ab",
        "Image": "nginx:1.27",
        "Labels": "maintainer=NGINX Docker Maintainers <docker-maint@nginx.com>",
        "LocalVolumes": "0",
        "Mounts": "",
        "Names": names,
        "Networks": "bridge",
        "Ports": "0.0.0.0:8080->80/tcp, [::]:8080->80/tcp",
        "RunningFor": "5 days ago",
        "Size": "0B",
        "State": state,
        "Status": status,
    })
    .to_string()
}

fn lines(rows: &[(&str, &str, &str)]) -> String {
    let all: Vec<String> = rows.iter().map(|(n, s, t)| line(n, s, t)).collect();
    all.join("\n") + "\n"
}

fn read(rows: &[(&str, &str, &str)], name: &str) -> ProbeResult {
    docker::read(&finished(&lines(rows)), name)
}

fn one(state: &str, status: &str) -> (&'static str, Option<String>) {
    let result = read(&[("web", state, status)], "web");
    let (word, message) = verdict(&result);
    (word, message.map(str::to_owned))
}

fn down(message: &str) -> (&'static str, Option<String>) {
    ("down", Some(message.to_owned()))
}

#[test]
fn chk_012_the_docker_command_is_static_text_and_never_names_the_container() {
    let expected = FixedCommand::new("docker")
        .arg("ps")
        .arg("-a")
        .arg("--no-trunc")
        .arg("--format")
        .arg("'{{json .}}'");
    assert_eq!(docker::command(), expected);
}

#[test]
fn d_3_a_running_container_without_a_health_check_is_up_with_no_response_time() {
    let result = read(&[("web", "running", "Up 5 days")], "web");
    assert_eq!(verdict(&result), ("up", None));
    assert_eq!(result.response_time, None);
}

#[test]
fn d_3_a_healthy_container_is_up() {
    assert_eq!(one("running", "Up 5 days (healthy)"), ("up", None));
}

#[test]
fn d_3_an_unhealthy_container_is_down_with_a_fixed_message() {
    assert_eq!(
        one("running", "Up 3 minutes (unhealthy)"),
        down("the container is running but unhealthy")
    );
}

#[test]
fn d_3_a_container_whose_health_check_is_starting_is_ok_but_starting() {
    let result = read(
        &[("web", "running", "Up 5 seconds (health: starting)")],
        "web",
    );
    assert_eq!(
        verdict(&result),
        (
            "starting",
            Some("the container health check is still starting")
        )
    );
    assert_eq!(result.response_time, None);
}

#[test]
fn d_3_a_container_that_is_not_running_is_down_with_its_state_class() {
    let states = [
        ("exited", "Exited (0) 2 hours ago", "exited"),
        ("restarting", "Restarting (1) 5 seconds ago", "restarting"),
        ("paused", "Up 1 hour (Paused)", "paused"),
        ("created", "Created", "created but not started"),
        ("removing", "Removal In Progress", "being removed"),
        ("dead", "Dead", "dead"),
        ("unheard-of", "whatever the daemon says", "not running"),
        ("", "", "not running"),
    ];
    for (state, status, class) in states {
        assert_eq!(
            one(state, status),
            down(&format!("the container is {class}")),
            "{state}"
        );
    }
}

#[test]
fn d_3_a_state_other_than_running_is_down_whatever_the_status_says() {
    assert_eq!(
        one("exited", "Up 5 days (healthy)"),
        down("the container is exited")
    );
}

#[test]
fn d_3_a_missing_state_field_is_not_running() {
    let output = finished("{\"Names\":\"web\"}\n");
    assert_eq!(
        verdict(&docker::read(&output, "web")),
        ("down", Some("the container is not running"))
    );
}

#[test]
fn d_3_an_absent_container_is_not_found() {
    let rows = [
        ("db", "running", "Up 1 day"),
        ("cache", "running", "Up 1 day"),
    ];
    assert_eq!(
        verdict(&read(&rows, "web")),
        ("down", Some("the container was not found"))
    );
    assert_eq!(
        verdict(&docker::read(&finished(""), "web")),
        ("down", Some("the container was not found"))
    );
}

#[test]
fn d_3_a_name_matches_one_of_the_comma_separated_names_exactly() {
    let rows = [("api,worker,proxy/api-link", "running", "Up 1 day")];
    for name in ["api", "worker", "proxy/api-link"] {
        assert_eq!(verdict(&read(&rows, name)), ("up", None), "{name}");
    }
    for name in ["wor", "worker2", "proxy", "api,worker", "link", ""] {
        assert_eq!(
            verdict(&read(&rows, name)),
            ("down", Some("the container was not found")),
            "{name:?}"
        );
    }
}

#[test]
fn d_3_a_container_is_judged_by_its_own_line_and_not_a_neighbour() {
    let rows = [
        ("web-1", "exited", "Exited (1) 1 hour ago"),
        ("web", "running", "Up 2 days (healthy)"),
        ("web-2", "running", "Up 3 minutes (unhealthy)"),
    ];
    assert_eq!(verdict(&read(&rows, "web")), ("up", None));
    assert_eq!(
        verdict(&read(&rows, "web-1")),
        ("down", Some("the container is exited"))
    );
}

#[test]
fn d_3_blank_lines_are_ignored() {
    let text = format!("\n  \n{}\n\n", line("web", "running", "Up 1 day"));
    assert_eq!(
        verdict(&docker::read(&finished(&text), "web")),
        ("up", None)
    );
    assert_eq!(
        verdict(&docker::read(&finished("\n\n"), "web")),
        ("down", Some("the container was not found"))
    );
}

#[test]
fn d_3_a_line_that_is_not_json_is_skipped_when_another_line_matches() {
    let text = format!(
        "WARNING: an engine notice\n{}\n",
        line("web", "running", "Up 1 day")
    );
    assert_eq!(
        verdict(&docker::read(&finished(&text), "web")),
        ("up", None)
    );
}

#[test]
fn d_3_a_line_that_is_not_json_makes_an_unmatched_output_unreadable() {
    let text = format!(
        "WARNING: an engine notice\n{}\n",
        line("db", "running", "Up 1 day")
    );
    assert_eq!(
        verdict(&docker::read(&finished(&text), "web")),
        ("down", Some("the command output cannot be read"))
    );
}

#[test]
fn d_3_a_recorded_line_of_a_real_docker_is_read_as_printed() {
    let recorded = r#"{"Command":"\"/docker-entrypoint.…\"","CreatedAt":"2026-11-01 08:12:44 +0000 UTC","ID":"3f1a9c0d5b7e","Image":"nginx:1.27","Labels":"maintainer=NGINX","LocalVolumes":"0","Mounts":"","Names":"web","Networks":"bridge","Ports":"0.0.0.0:8080->80/tcp","RunningFor":"5 days ago","Size":"0B","State":"running","Status":"Up 5 days (healthy)"}"#;
    assert_eq!(
        verdict(&docker::read(&finished(recorded), "web")),
        ("up", None)
    );
}

#[test]
fn chk_012_no_message_holds_text_of_the_output() {
    let secret = "SECRET-IMAGE-TOKEN";
    let output = finished(&format!(
        "{}\n",
        json!({"Names": "web", "State": secret, "Status": secret, "Image": secret})
    ));
    let result = docker::read(&output, "web");
    assert!(!result.message.unwrap().contains(secret));
}

#[test]
fn d_3_output_that_cannot_be_read_is_down_with_a_fixed_message() {
    let valid = lines(&[("web", "running", "Up 1 day")]);
    assert_unreadable_outputs(|output| docker::read(output, "web"), &valid);
}
