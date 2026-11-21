//! Tests of the static test of a recipe (HOST-047): the recipes that ship (`shipped`), and a
//! sound recipe with one fault at a time (`refused`).

mod refused;
mod shipped;

use crate::{Code, Recipe, check};

const IMAGE: &str =
    "example/notes@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// An entry file that passes each check, for the Compose file [`COMPOSE`].
const ENTRY: &str = r#"{
  "id": "notes",
  "name": "Notes",
  "description": "Notes in a browser.",
  "version": "1.2.3",
  "author": "Example",
  "tags": ["notes"],
  "compose_url": "compose.yaml",
  "env_url": "environment.example",
  "documentation_url": "https://example.org/notes",
  "icon_url": null,
  "anima": {
    "images": [
      {
        "service": "notes",
        "image": "example/notes@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "tag": "1.2.3",
        "resolved": "2026-11-20"
      }
    ],
    "marks": [],
    "volumes": [{ "volume": "notes", "copy": "sqlite" }],
    "generated": ["NOTES_SECRET"],
    "test": {
      "service": "notes",
      "port": 8080,
      "reach": "http",
      "path": "/health",
      "statuses": [200],
      "body": null,
      "limit_seconds": 60
    },
    "pinned": "2026-11-20",
    "last_test": { "result": "not-tested" }
  }
}"#;

const COMPOSE: &str = r"services:
  notes:
    image: example/notes@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
    environment:
      SECRET: ${NOTES_SECRET}
    ports:
      - target: 8080
        published: ${NOTES_PORT:-8080}
        host_ip: 127.0.0.1
        protocol: tcp
    volumes:
      - type: volume
        source: notes
        target: /var/notes
volumes:
  notes:
";

const ENVIRONMENT: &str = "# generated at install\nNOTES_SECRET=\nNOTES_PORT=8080\n";

fn sound() -> Recipe {
    Recipe {
        folder: "notes".to_owned(),
        entry: Some(ENTRY.to_owned()),
        compose: Some(COMPOSE.to_owned()),
        environment: Some(ENVIRONMENT.to_owned()),
    }
}

/// The codes of `recipe`.
fn codes(recipe: &Recipe) -> Vec<Code> {
    check(recipe)
        .into_iter()
        .map(|finding| finding.code)
        .collect()
}

/// `text` with `old` replaced by `new`, once; `old` must be there.
fn replaced(text: &str, old: &str, new: &str) -> String {
    assert!(text.contains(old), "{old}");
    text.replacen(old, new, 1)
}

/// [`sound`] with one change of its entry file.
fn with_entry(old: &str, new: &str) -> Recipe {
    Recipe {
        entry: Some(replaced(ENTRY, old, new)),
        ..sound()
    }
}

/// [`sound`] with one change of its Compose file.
fn with_compose(old: &str, new: &str) -> Recipe {
    Recipe {
        compose: Some(replaced(COMPOSE, old, new)),
        ..sound()
    }
}
