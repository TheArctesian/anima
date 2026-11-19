//! Places through the hub API (K-E4 Section 8; PLC-001 to PLC-006).

mod delete;
mod merge;

use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, error, set_operator, start};

const FIELDS: &str = "id parentId name title latitude longitude radiusM category icon aliases \
                      geocoderName siteIds";

pub async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

/// A top-level place with a centre and a radius.
pub fn top(name: &str) -> Value {
    json!({ "name": name, "latitude": 52.0, "longitude": 4.0, "radiusM": 100,
            "category": "home", "icon": "house" })
}

/// A child place: no centre and no radius.
pub fn child(name: &str, parent: &str) -> Value {
    json!({ "name": name, "parentId": parent, "category": "home", "icon": "box" })
}

pub async fn run(hub: &Hub, token: &str, query: &str, variables: Value) -> Value {
    hub.post(Some(token), query, variables).await
}

/// Makes the place and returns its response data; panics when the hub refuses.
pub async fn add(hub: &Hub, token: &str, input: Value) -> Value {
    let query = format!("mutation($i: PlaceInput!) {{ createPlace(input: $i) {{ {FIELDS} }} }}");
    let made = run(hub, token, &query, json!({ "i": input })).await;
    assert!(made["errors"].is_null(), "{made}");
    made["data"]["createPlace"].clone()
}

pub fn id(place: &Value) -> String {
    place["id"].as_str().unwrap().to_owned()
}

pub async fn place(hub: &Hub, token: &str, id: &str) -> Value {
    let query = format!("query($id: ID!) {{ place(id: $id) {{ {FIELDS} }} }}");
    run(hub, token, &query, json!({ "id": id })).await["data"]["place"].clone()
}

pub async fn link(pool: &PgPool, source: &str, target: &str, kind: &str) {
    sqlx::query(
        "INSERT INTO links.record_link (id, source, target, kind, created_at, created_by) \
         VALUES (gen_random_uuid(), $1, $2, $3, now(), 'test')",
    )
    .bind(source)
    .bind(target)
    .bind(kind)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_001_a_place_has_a_name_a_centre_a_radius_a_category_an_icon_and_aliases(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let mut input = top("Home");
    input["aliases"] = json!(["Flat", "Casa"]);
    let made = add(&hub, &token, input).await;
    assert_eq!(made["name"], "Home");
    assert_eq!(made["radiusM"], 100);
    assert_eq!(made["category"], "home");
    assert_eq!(made["icon"], "house");
    assert_eq!(made["aliases"], json!(["Flat", "Casa"]));

    let mut bad = top("");
    bad["category"] = json!("castle");
    let refused = run(
        &hub,
        &token,
        "mutation($i: PlaceInput!) { createPlace(input: $i) { id } }",
        json!({ "i": bad }),
    )
    .await;
    assert_eq!(error(&refused).1, "VALIDATION");
    let fields: Vec<_> = refused["errors"][0]["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["name", "category"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_002_a_child_place_has_no_radius_and_the_tree_is_at_most_five_deep(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    let mut with_radius = child("Bedroom", &home);
    with_radius["radiusM"] = json!(5);
    with_radius["latitude"] = json!(1.0);
    with_radius["longitude"] = json!(1.0);
    let refused = run(
        &hub,
        &token,
        "mutation($i: PlaceInput!) { createPlace(input: $i) { id } }",
        json!({ "i": with_radius }),
    )
    .await;
    assert_eq!(error(&refused).1, "VALIDATION");

    let mut parent = home;
    for name in ["Bedroom", "Wardrobe", "Shelf", "Box"] {
        let made = add(&hub, &token, child(name, &parent)).await;
        assert_eq!(made["radiusM"], Value::Null, "{name}");
        parent = id(&made);
    }
    let too_deep = run(
        &hub,
        &token,
        "mutation($i: PlaceInput!) { createPlace(input: $i) { id } }",
        json!({ "i": child("Sixth", &parent) }),
    )
    .await;
    assert_eq!(error(&too_deep).1, "VALIDATION");
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_002_a_place_cannot_be_moved_inside_itself(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    let room = id(&add(&hub, &token, child("Room", &home)).await);
    let refused = run(
        &hub,
        &token,
        "mutation($id: ID!, $i: PlaceChangeInput!) { updatePlace(id: $id, input: $i) { id } }",
        json!({ "id": home, "i": { "parentId": room } }),
    )
    .await;
    assert_eq!(error(&refused).1, "VALIDATION");
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_003_the_operator_name_wins_over_the_geocoder_name(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let mut input = top("Our home");
    input["geocoderName"] = json!("Keizersgracht 1");
    let made = add(&hub, &token, input).await;
    assert_eq!(made["title"], "Our home");
    assert_eq!(made["geocoderName"], "Keizersgracht 1");

    let changed = run(
        &hub,
        &token,
        &format!(
            "mutation($id: ID!, $i: PlaceChangeInput!) {{ updatePlace(id: $id, input: $i) \
             {{ {FIELDS} }} }}"
        ),
        json!({ "id": made["id"], "i": { "geocoderName": "Herengracht 2" } }),
    )
    .await;
    let after = &changed["data"]["updatePlace"];
    assert_eq!(after["name"], "Our home", "{changed}");
    assert_eq!(after["title"], "Our home");
    assert_eq!(after["geocoderName"], "Herengracht 2");
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_003_a_link_to_a_place_shows_the_operator_name_as_its_title(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    link(&pool, "book:1", &format!("place:{home}"), "stored_at").await;
    let listed = run(
        &hub,
        &token,
        "query($r: String!) { recordLinks(record: $r) { targetTitle } }",
        json!({ "r": format!("place:{home}") }),
    )
    .await;
    assert_eq!(listed["data"]["recordLinks"][0]["targetTitle"], "Home");
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_006_a_site_links_to_one_place_and_a_place_has_many_sites(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let office = id(&add(&hub, &token, top("Office")).await);
    let home = id(&add(&hub, &token, top("Home")).await);
    let mut sites = Vec::new();
    for (name, subnet) in [("a", "10.40.0.0/24"), ("b", "10.41.0.0/24")] {
        let made = run(
            &hub,
            &token,
            "mutation($n: String!, $s: Cidr!) { createSite(name: $n, subnet: $s) { id } }",
            json!({ "n": name, "s": subnet }),
        )
        .await;
        sites.push(
            made["data"]["createSite"]["id"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    let set = "mutation($s: ID!, $p: ID) { setSitePlace(site: $s, place: $p) { id } }";
    for site in &sites {
        run(&hub, &token, set, json!({ "s": site, "p": office })).await;
    }
    assert_eq!(
        place(&hub, &token, &office).await["siteIds"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // The same site moves: it has at most one place.
    run(&hub, &token, set, json!({ "s": sites[0], "p": home })).await;
    assert_eq!(
        place(&hub, &token, &office).await["siteIds"],
        json!([sites[1]])
    );
    assert_eq!(
        place(&hub, &token, &home).await["siteIds"],
        json!([sites[0]])
    );

    run(&hub, &token, set, json!({ "s": sites[0], "p": null })).await;
    assert_eq!(place(&hub, &token, &home).await["siteIds"], json!([]));

    let unknown = run(
        &hub,
        &token,
        set,
        json!({ "s": "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d", "p": home }),
    )
    .await;
    assert_eq!(error(&unknown).1, "NOT_FOUND");
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_001_each_change_has_one_audit_entry_and_no_position(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    run(
        &hub,
        &token,
        "mutation($id: ID!, $i: PlaceChangeInput!) { updatePlace(id: $id, input: $i) { id } }",
        json!({ "id": home, "i": { "icon": "cabin" } }),
    )
    .await;
    // The same value again changes nothing and writes nothing.
    run(
        &hub,
        &token,
        "mutation($id: ID!, $i: PlaceChangeInput!) { updatePlace(id: $id, input: $i) { id } }",
        json!({ "id": home, "i": { "icon": "cabin" } }),
    )
    .await;
    let entries: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM events.audit_log WHERE target_type = 'place' ORDER BY at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(entries, ["create", "update"]);
    let messages: Vec<String> =
        sqlx::query_scalar("SELECT message FROM events.event_log WHERE message LIKE 'Place %'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(messages.len(), 2);
    assert!(messages.iter().all(|m| !m.contains("52")));
}
