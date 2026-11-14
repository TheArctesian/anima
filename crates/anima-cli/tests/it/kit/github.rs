//! A loopback GitHub for the real hub: the hub's own test stub, shared by path so both suites
//! script one stub, with GitHub's listing and file trees. Never the real GitHub.

#[path = "../../../../hub/tests/it/sources/stub.rs"]
#[expect(
    dead_code,
    reason = "this suite uses only the stub's replies and request log"
)]
mod stub;

use serde_json::{Value, json};

pub use stub::{Reply, Stub};

fn ok(body: &Value) -> Reply {
    Reply {
        status: 200,
        headers: Vec::new(),
        body: body.to_string(),
    }
}

/// The installation lists the repositories `full_names`, ids from 1, each with one file
/// `Dockerfile` and no deployments.
pub fn repositories(stub: &Stub, full_names: &[&str]) {
    let listed: Vec<Value> = (1_u64..)
        .zip(full_names)
        .map(|(id, full_name)| {
            json!({ "id": id, "full_name": full_name,
                    "html_url": format!("https://github.com/{full_name}"),
                    "default_branch": "main", "private": true, "archived": false, "fork": false,
                    "description": null, "pushed_at": "2030-05-01T10:00:00Z" })
        })
        .collect();
    let listing = json!({ "total_count": listed.len(), "repositories": listed });
    stub.reply("/installation/repositories", ok(&listing));
    for full_name in full_names {
        let tree =
            json!({ "tree": [{ "path": "Dockerfile", "type": "blob" }], "truncated": false });
        stub.reply(&format!("/repos/{full_name}/git/trees/main"), ok(&tree));
        stub.reply(&format!("/repos/{full_name}/deployments"), ok(&json!([])));
    }
}
