//! Tests of the rules of a Compose file from the hub (HOST-028; K10 Section 6.4.4): the reader
//! of YAML (`reading`), the variables (`filling`), the permitted keys (`listed`), and the
//! rules of the values (`values`, `touching`).

mod filling;
mod listed;
mod reading;
mod touching;
mod values;

use crate::{Checked, Refusal, Rule, Settings, check};

/// A file that passes each rule: two services, one volume, one bind mount, one network, one
/// config, and a variable.
const SOUND: &str = r#"
services:
  app:
    image: example/tracks:${TAG:-0.36}
    command: ["bin/rails", "server"]
    environment:
      DATABASE_URL: postgres://tracks:${DB_PASSWORD}@db/tracks
      RAILS_ENV: production
    restart: unless-stopped
    depends_on:
      db:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "curl -f localhost:3000/health"]
      interval: 30s
      retries: 3
    mem_limit: 512m
    pids_limit: 200
    ports:
      - target: 3000
        published: 8080
        host_ip: 127.0.0.1
        protocol: tcp
    volumes:
      - type: volume
        source: data
        target: /data
      - type: bind
        source: binds/config
        target: /config
        read_only: true
    networks: [inner]
    configs:
      - source: settings
        target: /etc/tracks.yml
        mode: "0444"
  db:
    image: postgres@sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf
    environment:
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    networks:
      inner:
        aliases: [database]
volumes:
  data:
networks:
  inner:
    internal: true
configs:
  settings:
    content: |
      price: $$5
"#;

const ENVIRONMENT: &str = "DB_PASSWORD=hunter2\n";

fn checked(file: &str) -> Result<Checked, Refusal> {
    check(file, ENVIRONMENT, &Settings::default())
}

/// The refusal of `file`, which must be refused.
fn refusal(file: &str) -> Refusal {
    checked(file).expect_err("the file must be refused")
}

/// [`SOUND`] with one line of the service `app` (position 0) replaced.
fn with(old: &str, new: &str) -> String {
    assert!(SOUND.contains(old), "{old}");
    SOUND.replacen(old, new, 1)
}

/// The refusal of the service at position 0 for `rule`.
const fn app(rule: Rule) -> Refusal {
    Refusal::File {
        rule,
        service: Some(0),
    }
}

const fn top(rule: Rule) -> Refusal {
    Refusal::File {
        rule,
        service: None,
    }
}
