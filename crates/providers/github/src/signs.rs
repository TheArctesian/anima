//! Which files say where a repository runs (PRO-003). Pure rules over paths and creators.

use std::collections::BTreeMap;

use crate::record::{Sign, SignKind};

const PATHS_PER_KIND: usize = 5;
const VERCEL_BOT: &str = "vercel[bot]";

/// A path with one of these as a segment is not the project's own file.
const SKIPPED_SEGMENTS: [&str; 10] = [
    "node_modules",
    "vendor",
    "third_party",
    "example",
    "examples",
    "fixtures",
    "testdata",
    "test",
    "tests",
    "docs",
];

fn kind_of(base_name: &str) -> Option<SignKind> {
    match base_name {
        "wrangler.toml" | "wrangler.json" | "wrangler.jsonc" => Some(SignKind::WranglerConfig),
        "vercel.json" => Some(SignKind::VercelJson),
        "Dockerfile" => Some(SignKind::Dockerfile),
        "compose.yml" | "compose.yaml" | "docker-compose.yml" | "docker-compose.yaml" => {
            Some(SignKind::ComposeFile)
        }
        "tauri.conf.json" => Some(SignKind::TauriConf),
        _ => None,
    }
}

fn is_skipped(segment: &str) -> bool {
    segment.starts_with('.')
        || SKIPPED_SEGMENTS
            .iter()
            .any(|skipped| skipped.eq_ignore_ascii_case(segment))
}

/// The signs among `paths` (files only): at most five paths per kind, shortest first, in
/// the order of [`SignKind`].
pub(crate) fn from_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<Sign> {
    let mut found: BTreeMap<SignKind, Vec<&str>> = BTreeMap::new();
    for path in paths {
        if path.split('/').any(is_skipped) {
            continue;
        }
        let base_name = path.rsplit('/').next().unwrap_or(path);
        if let Some(kind) = kind_of(base_name) {
            found.entry(kind).or_default().push(path);
        }
    }
    let mut signs = Vec::new();
    for (kind, mut kind_paths) in found {
        kind_paths.sort_by_key(|path| (path.len(), *path));
        kind_paths.dedup();
        signs.extend(
            kind_paths
                .into_iter()
                .take(PATHS_PER_KIND)
                .map(|path| Sign {
                    kind,
                    path: Some(path.to_owned()),
                }),
        );
    }
    signs
}

/// The deployment sign, when one of the creators is Vercel's bot.
pub(crate) fn from_creators<'a>(logins: impl IntoIterator<Item = &'a str>) -> Vec<Sign> {
    if logins.into_iter().any(|login| login == VERCEL_BOT) {
        vec![Sign {
            kind: SignKind::VercelDeployment,
            path: None,
        }]
    } else {
        Vec::new()
    }
}
