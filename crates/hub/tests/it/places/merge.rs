//! Merge and split move the references of a place (PLC-004).

use serde_json::json;
use sqlx::PgPool;

use super::{add, child, id, link, place, run, signed_in, top};

const MERGE: &str =
    "mutation($f: ID!, $i: ID!) { mergePlaces(from: $f, into: $i) { id name aliases siteIds } }";

async fn links_of(pool: &PgPool, place: &str) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "SELECT source, target, kind FROM links.record_link WHERE source = $1 OR target = $1 \
         ORDER BY source, target, kind",
    )
    .bind(format!("place:{place}"))
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_004_a_merge_moves_every_reference_to_the_remaining_place(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let mut input = top("Old flat");
    input["aliases"] = json!(["Flat 1"]);
    let from = id(&add(&hub, &token, input).await);
    let into = id(&add(&hub, &token, top("Home")).await);
    let room = id(&add(&hub, &token, child("Room", &from)).await);
    link(&pool, "book:1", &format!("place:{from}"), "stored_at").await;
    link(&pool, &format!("place:{from}"), "person:x", "lived_by").await;
    let site = run(
        &hub,
        &token,
        "mutation { createSite(name: \"a\", subnet: \"10.40.0.0/24\") { id } }",
        json!({}),
    )
    .await["data"]["createSite"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    run(
        &hub,
        &token,
        "mutation($s: ID!, $p: ID) { setSitePlace(site: $s, place: $p) { id } }",
        json!({ "s": site, "p": from }),
    )
    .await;

    let merged = run(&hub, &token, MERGE, json!({ "f": from, "i": into })).await;
    let kept = &merged["data"]["mergePlaces"];
    assert_eq!(kept["id"], json!(into), "{merged}");
    assert_eq!(kept["aliases"], json!(["Old flat", "Flat 1"]));
    assert_eq!(kept["siteIds"], json!([site]));

    assert_eq!(links_of(&pool, &from).await, []);
    assert_eq!(links_of(&pool, &into).await.len(), 2);
    assert_eq!(place(&hub, &token, &room).await["parentId"], json!(into));
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_004_a_merged_place_is_not_listed_cannot_change_and_its_reference_resolves(
    pool: PgPool,
) {
    let (hub, token) = signed_in(&pool).await;
    let from = id(&add(&hub, &token, top("Old")).await);
    let into = id(&add(&hub, &token, top("New")).await);
    link(&pool, "book:1", &format!("place:{from}"), "stored_at").await;
    run(&hub, &token, MERGE, json!({ "f": from, "i": into })).await;

    let listed = run(&hub, &token, "{ places { id } }", json!({})).await;
    assert_eq!(listed["data"]["places"], json!([{ "id": into }]));
    assert_eq!(place(&hub, &token, &from).await["id"], json!(into));

    let refused = run(
        &hub,
        &token,
        "mutation($id: ID!, $i: PlaceChangeInput!) { updatePlace(id: $id, input: $i) { id } }",
        json!({ "id": from, "i": { "name": "Again" } }),
    )
    .await;
    assert_eq!(crate::hub::error(&refused).1, "CONFLICT");

    // The link kept its target text: the old key still names the place through `place`, and the
    // title of the old key is the title of the remaining place.
    link(&pool, "book:2", &format!("place:{from}"), "stored_at").await;
    let titled = run(
        &hub,
        &token,
        "query($r: String!) { recordLinks(record: $r) { targetTitle } }",
        json!({ "r": format!("place:{from}") }),
    )
    .await;
    assert_eq!(titled["data"]["recordLinks"][0]["targetTitle"], "New");
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_004_a_merge_removes_a_link_to_itself_and_a_repeated_link(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let from = id(&add(&hub, &token, top("Old")).await);
    let into = id(&add(&hub, &token, top("New")).await);
    let (a, b) = (format!("place:{from}"), format!("place:{into}"));
    link(&pool, &a, &b, "near").await; // would become a link to itself
    link(&pool, &b, &a, "near").await; // the same
    link(&pool, "book:1", &a, "stored_at").await; // repeated below
    link(&pool, "book:1", &b, "stored_at").await;
    link(&pool, "book:2", &a, "stored_at").await; // moves
    run(&hub, &token, MERGE, json!({ "f": from, "i": into })).await;
    let after = links_of(&pool, &into).await;
    assert_eq!(
        after,
        [
            ("book:1".to_owned(), b.clone(), "stored_at".to_owned()),
            ("book:2".to_owned(), b, "stored_at".to_owned()),
        ]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_004_a_place_cannot_merge_into_itself_or_into_its_own_child(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    let room = id(&add(&hub, &token, child("Room", &home)).await);
    for (from, into) in [(&home, &home), (&home, &room)] {
        let refused = run(&hub, &token, MERGE, json!({ "f": from, "i": into })).await;
        assert_eq!(crate::hub::error(&refused).1, "VALIDATION");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_004_a_split_moves_only_the_named_references(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    let garage = id(&add(&hub, &token, child("Garage", &home)).await);
    let kitchen = id(&add(&hub, &token, child("Kitchen", &home)).await);
    link(&pool, "book:1", &format!("place:{home}"), "stored_at").await;
    link(&pool, "book:2", &format!("place:{home}"), "stored_at").await;
    let named: String =
        sqlx::query_scalar("SELECT id::text FROM links.record_link WHERE source = 'book:1'")
            .fetch_one(&pool)
            .await
            .unwrap();

    let split = run(
        &hub,
        &token,
        "mutation($id: ID!, $i: PlaceSplitInput!) { splitPlace(id: $id, input: $i) { id name } }",
        json!({ "id": home, "i": { "new": top("Workshop"), "children": [garage], "links": [named] } }),
    )
    .await;
    let new = split["data"]["splitPlace"]["id"].as_str().unwrap();
    assert_eq!(place(&hub, &token, &garage).await["parentId"], json!(new));
    assert_eq!(place(&hub, &token, &kitchen).await["parentId"], json!(home));
    assert_eq!(links_of(&pool, new).await.len(), 1);
    assert_eq!(links_of(&pool, &home).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_004_a_split_that_names_a_foreign_child_changes_nothing(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    let other = id(&add(&hub, &token, top("Other")).await);
    let stranger = id(&add(&hub, &token, child("Shed", &other)).await);
    let refused = run(
        &hub,
        &token,
        "mutation($id: ID!, $i: PlaceSplitInput!) { splitPlace(id: $id, input: $i) { id } }",
        json!({ "id": home, "i": { "new": top("New"), "children": [stranger] } }),
    )
    .await;
    assert_eq!(crate::hub::error(&refused).1, "VALIDATION");
    let listed = run(&hub, &token, "{ places { id } }", json!({})).await;
    assert_eq!(
        listed["data"]["places"].as_array().unwrap().len(),
        3,
        "the new place was rolled back"
    );
    assert_eq!(
        place(&hub, &token, &stranger).await["parentId"],
        json!(other)
    );
}
