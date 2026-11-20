//! Commands as people write them: each becomes a Compose file that passes the syntax check
//! and reads back to the values of the command.

use super::super::Note;
use super::{converted, list, text, tree};

#[test]
fn host_022_nginx_with_a_name_and_a_port() {
    let done = converted("docker run -d --name web -p 8080:80 nginx:latest");
    assert_eq!(
        done.compose_file,
        "services:\n  web:\n    image: \"nginx:latest\"\n    container_name: \"web\"\n    \
         ports:\n      - \"8080:80\"\n"
    );
    assert_eq!(done.service, "web");
    assert_eq!(done.notes, [Note::DetachNotNeeded]);
    assert_eq!(done.not_converted, Vec::<String>::new());
}

#[test]
fn host_022_postgres_with_a_password_a_volume_and_a_restart_policy() {
    let done = converted(
        "docker run -d --name db -e POSTGRES_PASSWORD=hunter2 -e POSTGRES_DB=app \
         -v pgdata:/var/lib/postgresql/data --restart unless-stopped -p 127.0.0.1:5432:5432 \
         postgres:17",
    );
    let file = tree(&done.compose_file);
    let db = ["services", "db"];
    assert_eq!(text(&file, &[db[0], db[1], "image"]), "postgres:17");
    assert_eq!(text(&file, &[db[0], db[1], "restart"]), "unless-stopped");
    assert_eq!(
        list(&file, &[db[0], db[1], "environment"]),
        ["POSTGRES_PASSWORD=hunter2", "POSTGRES_DB=app"]
    );
    assert_eq!(
        list(&file, &[db[0], db[1], "volumes"]),
        ["pgdata:/var/lib/postgresql/data"]
    );
    assert_eq!(
        list(&file, &[db[0], db[1], "ports"]),
        ["127.0.0.1:5432:5432"]
    );
    // The volume keeps the name that `docker run` gave it.
    assert_eq!(text(&file, &["volumes", "pgdata", "name"]), "pgdata");
    assert_eq!(done.notes, [Note::DetachNotNeeded, Note::VolumeNameKept]);
}

#[test]
fn host_022_a_command_with_quotes_continuations_and_a_variable() {
    let done = converted(concat!(
        "sudo docker run -it --rm \\\n",
        "  -v \"$PWD/site\":/usr/share/nginx/html:ro \\\n",
        "  -e 'GREETING=it costs $5' \\\n",
        "  -e \"MOTTO=say \\\"hi\\\"\" \\\n",
        "  --network backend --network-alias web \\\n",
        "  --health-cmd 'curl -f http://localhost/ || exit 1' --health-retries 3 \\\n",
        "  ghcr.io/Example/My.App:1.2 serve --port 80\n",
    ));
    assert_eq!(done.service, "my.app");
    let file = tree(&done.compose_file);
    let app = |key: &'static str| ["services", "my.app", key];
    assert_eq!(text(&file, &app("image")), "ghcr.io/Example/My.App:1.2");
    assert_eq!(list(&file, &app("command")), ["serve", "--port", "80"]);
    assert_eq!(
        list(&file, &app("volumes")),
        ["$PWD/site:/usr/share/nginx/html:ro"]
    );
    assert_eq!(
        list(&file, &app("environment")),
        ["GREETING=it costs $$5", "MOTTO=say \"hi\""]
    );
    assert_eq!(text(&file, &app("stdin_open")), "true");
    assert_eq!(text(&file, &app("tty")), "true");
    assert_eq!(
        list(
            &file,
            &["services", "my.app", "networks", "backend", "aliases"]
        ),
        ["web"]
    );
    assert_eq!(
        list(&file, &["services", "my.app", "healthcheck", "test"]),
        ["CMD-SHELL", "curl -f http://localhost/ || exit 1"]
    );
    assert_eq!(
        text(&file, &["services", "my.app", "healthcheck", "retries"]),
        "3"
    );
    assert_eq!(text(&file, &["networks", "backend", "external"]), "true");
    assert_eq!(
        done.notes,
        [
            Note::RemoveNotKept,
            Note::ExternalNetwork,
            Note::VariablesKept,
            Note::DollarWritten
        ]
    );
}

#[test]
fn host_022_the_name_of_the_service_is_made_safe() {
    for (command, service) in [
        ("docker run --name 'My App!' nginx", "my-app"),
        ("docker run --name _x_ nginx", "x"),
        ("docker run localhost:5000/a/b_c.d:tag", "b_c.d"),
        ("docker run ${IMAGE}", "image"),
        ("docker run '!!!'", "app"),
        ("docker run true", "true"),
    ] {
        assert_eq!(converted(command).service, service, "{command}");
    }
    let long = format!("docker run --name {} nginx", "a".repeat(200));
    assert_eq!(converted(&long).service.len(), 63);
}
