//! `anima place` against the scripted hub (PLC-001, PLC-004, PLC-005, PLC-006).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

const ID: &str = "0190a000-0000-7000-9200-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-place-tests")
}

fn place() -> Value {
    json!({ "id": ID, "parentId": null, "name": "Home", "title": "Home", "latitude": 52.0,
            "longitude": 4.0, "radiusM": 100, "category": "home", "icon": "house",
            "aliases": ["Flat"], "geocoderName": null, "siteIds": [],
            "createdAt": "2026-11-18T10:00:00+00:00", "updatedAt": "2026-11-18T10:00:00+00:00" })
}

#[tokio::test]
async fn plc_001_place_add_sends_the_fields_and_prints_the_place() {
    let hub = FakeHub::start().await;
    hub.answer("PlaceAdd", data(json!({ "createPlace": place() })));
    let run = signed_in()
        .run(&[
            "place",
            "add",
            "--name",
            "Home",
            "--category",
            "home",
            "--icon",
            "house",
            "--latitude",
            "52.0",
            "--longitude",
            "4.0",
            "--radius-m",
            "100",
            "--alias",
            "Flat",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("Home") && run.stdout.contains(ID),
        "{}",
        run.stdout
    );
    assert_eq!(
        hub.seen()[0].variables["input"],
        json!({ "name": "Home", "category": "home", "icon": "house", "latitude": 52.0,
                "longitude": 4.0, "radiusM": 100, "aliases": ["Flat"] })
    );
}

#[tokio::test]
async fn plc_001_place_add_without_a_category_asks_the_hub_nothing() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&[
            "place",
            "add",
            "--name",
            "Home",
            "--icon",
            "house",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_ne!(run.code, 0);
    assert!(hub.seen().is_empty());
}

#[tokio::test]
async fn plc_001_place_list_shows_each_place_with_its_centre() {
    let hub = FakeHub::start().await;
    hub.answer("PlaceList", data(json!({ "places": [place()] })));
    let run = signed_in()
        .run(&["place", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run
        .stdout
        .lines()
        .find(|l| l.starts_with("Home"))
        .expect("a row");
    assert!(row.contains("within 100 m") && row.contains(ID), "{row}");
}

#[tokio::test]
async fn plc_004_place_merge_and_remove_send_the_ids() {
    let hub = FakeHub::start().await;
    hub.answer("PlaceMerge", data(json!({ "mergePlaces": place() })));
    hub.answer("PlaceRemove", data(json!({ "deletePlace": ID })));
    let other = "0190a000-0000-7000-9200-000000000002";
    let merged = signed_in()
        .run(&["place", "merge", other, ID, "--hub", &hub.origin])
        .await;
    assert_eq!(merged.code, 0, "{merged:?}");
    assert_eq!(
        hub.seen()[0].variables,
        json!({ "from": other, "into": ID })
    );
    let removed = signed_in()
        .run(&[
            "place",
            "remove",
            other,
            "--replacement",
            ID,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(removed.code, 0, "{removed:?}");
    assert_eq!(
        hub.seen()[1].variables,
        json!({ "id": other, "replacement": ID })
    );
}

#[tokio::test]
async fn plc_006_place_site_with_no_place_removes_the_link() {
    let hub = FakeHub::start().await;
    hub.answer("PlaceSite", data(json!({ "setSitePlace": null })));
    let site = "0190a000-0000-7000-9200-000000000003";
    let run = signed_in()
        .run(&["place", "site", site, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        hub.seen()[0].variables,
        json!({ "site": site, "place": null })
    );
}
