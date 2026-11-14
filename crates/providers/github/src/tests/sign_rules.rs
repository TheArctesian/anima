use crate::signs::{from_creators, from_paths};

use super::*;

fn kinds(paths: &[&str]) -> Vec<(SignKind, String)> {
    from_paths(paths.iter().copied())
        .into_iter()
        .map(|sign| (sign.kind, sign.path.unwrap()))
        .collect()
}

#[test]
fn pro_003_each_file_name_gives_its_sign() {
    let found = kinds(&[
        "wrangler.toml",
        "svc/wrangler.json",
        "wrangler.jsonc",
        "vercel.json",
        "Dockerfile",
        "compose.yml",
        "compose.yaml",
        "docker-compose.yml",
        "docker-compose.yaml",
        "tauri.conf.json",
        "README.md",
        "Dockerfile.dev",
    ]);

    let of = |kind| found.iter().filter(|(k, _)| *k == kind).count();
    assert_eq!(of(SignKind::WranglerConfig), 3);
    assert_eq!(of(SignKind::VercelJson), 1);
    assert_eq!(of(SignKind::Dockerfile), 1);
    assert_eq!(of(SignKind::ComposeFile), 4);
    assert_eq!(of(SignKind::TauriConf), 1);
    assert_eq!(found.len(), 10);
}

#[test]
fn pro_003_skipped_folders_and_dot_folders_give_no_sign() {
    for path in [
        "node_modules/a/Dockerfile",
        "vendor/x/vercel.json",
        "third_party/x/Dockerfile",
        "examples/web/wrangler.toml",
        "example/wrangler.toml",
        "src/fixtures/tauri.conf.json",
        "testdata/Dockerfile",
        "test/Dockerfile",
        "Tests/Dockerfile",
        "docs/Dockerfile",
        ".github/workflows/Dockerfile",
        "app/.cache/Dockerfile",
    ] {
        assert_eq!(kinds(&[path]).len(), 0, "{path}");
    }
}

#[test]
fn pro_003_at_most_five_paths_per_sign_shortest_first() {
    let paths: Vec<String> = (0..8)
        .map(|n| format!("{}Dockerfile", "a/".repeat(8 - n)))
        .collect();

    let found = kinds(&paths.iter().map(String::as_str).collect::<Vec<_>>());

    assert_eq!(found.len(), 5);
    assert_eq!(found[0].1, "a/Dockerfile");
    assert!(
        found
            .windows(2)
            .all(|pair| pair[0].1.len() <= pair[1].1.len())
    );
}

#[test]
fn pro_003_signs_come_in_a_fixed_order_whatever_the_tree_order() {
    let forward = kinds(&["Dockerfile", "vercel.json", "wrangler.toml"]);
    let backward = kinds(&["wrangler.toml", "vercel.json", "Dockerfile"]);

    assert_eq!(forward, backward);
    assert_eq!(forward[0].0, SignKind::WranglerConfig);
}

#[test]
fn pro_003_only_the_vercel_bot_makes_a_deployment_sign() {
    assert_eq!(from_creators(["a", "vercel[bot]"]).len(), 1);
    assert_eq!(
        from_creators(["vercel", "github-actions[bot]", "vercel[bot]x"]).len(),
        0
    );
    assert_eq!(from_creators([]).len(), 0);
}

#[test]
fn prj_016_only_owner_slash_name_goes_into_a_path() {
    use crate::name::{check, encode_branch};

    for good in ["a/b", "A-b_c.d/e.f-g", "arctesian/anima"] {
        assert!(check(good).is_ok(), "{good}");
    }
    for bad in [
        "", "a", "a/", "/b", "a/b/c", "a/../b", "a/b c", "a/b@c", "a/b%2F", "./b", "a/..",
    ] {
        assert!(check(bad).is_err(), "{bad}");
    }
    assert_eq!(encode_branch("release/1.0"), "release/1.0");
    assert_eq!(encode_branch("a b"), "a%20b");
}
