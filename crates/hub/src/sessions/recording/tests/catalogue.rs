//! The catalogue (K1 `recordings`, RA-041, API-006, D-15): newest first, a page at a time, of
//! every device or of one, without the key; and the bench the `openRecording` and
//! `deleteRecording` tests share: the recordings GraphQL part wired as `schema.rs` wires it.

use async_graphql::{EmptySubscription, Request, Response, Schema, Value, Variables};
use chrono::{DateTime, Days, TimeDelta, Utc};
use serde_json::{Value as Json, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::{World, new_session};
use crate::{
    access::Identity,
    credentials::Sealed,
    ids::{DeviceId, RecordingId},
    sessions::{
        Protocol, RecordingsMutation, RecordingsQuery,
        recording::{
            format::Format,
            graphql::connection::{decode_cursor, encode_cursor},
            model::Position,
            store::{self, NewRecording},
        },
        tests::fixture::{address, docker_host, now, operator},
    },
};

/// The address the operator's client calls from.
pub(super) const CLIENT: &str = "100.64.0.7";

/// Another device than `docker-host`, as a deleted one: the row has no foreign key (D-15).
const ELSEWHERE: &str = "0190a000-0000-7000-8002-0000000000ff";

pub(super) type Catalogue = Schema<RecordingsQuery, RecordingsMutation, EmptySubscription>;

/// The recordings part with the world's pool, tickets, bus, clock and recording root.
pub(super) fn catalogue(world: &World) -> Catalogue {
    let serving = &world.serving;
    Schema::build(RecordingsQuery, RecordingsMutation, EmptySubscription)
        .data(serving.db.clone())
        .data(serving.live.clone())
        .data(serving.bus.clone())
        .data(serving.clock.clone())
        .data(serving.recording.clone())
        .finish()
}

/// `document` with `variables`, signed in as the operator from [`CLIENT`].
pub(super) async fn asked(world: &World, document: &str, variables: Json) -> Response {
    let identity = Identity::for_tests(operator(), now() + TimeDelta::days(1));
    let request = Request::new(document).variables(Variables::from_json(variables));
    let request = identity.attach(request).data(address(CLIENT));
    catalogue(world).execute(request).await
}

/// The data of a response with no error.
pub(super) fn data(response: Response) -> Json {
    assert_eq!(response.errors, [], "no error");
    response.data.into_json().unwrap()
}

/// The one error's code and message; no data.
pub(super) fn error_of(response: &Response) -> (String, String) {
    assert_eq!(response.data, Value::Null);
    let [error] = response.errors.as_slice() else {
        panic!("one error, got {:?}", response.errors);
    };
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    let Some(Value::String(code)) = code else {
        panic!("no code: {error:?}");
    };
    (code.clone(), error.message.clone())
}

/// A sealed key no test opens.
fn sealed() -> Sealed {
    Sealed::from_parts(&[1; 24], vec![2; 48], "k1".to_owned()).unwrap()
}

/// The plaintext-free bytes a shelved recording's file holds.
pub(super) const FILE_BYTES: &[u8] = b"ANREC1\0 sealed parts";

/// A recording of a new session of `device`, started `minutes` after [`now`], with its row
/// and a file in the world's root.
pub(super) async fn shelve(world: &World, device: DeviceId, minutes: i64) -> RecordingId {
    let id = RecordingId::from(Uuid::now_v7());
    let session = new_session(&world.serving.db).await;
    let started_at = now() + TimeDelta::minutes(minutes);
    let key_sealed = sealed();
    let new = NewRecording {
        id,
        session,
        device,
        device_name: "docker-host",
        protocol: Protocol::Vnc,
        format: Format::Guacamole,
        key_sealed: &key_sealed,
        started_at,
        expires_at: started_at + Days::new(30),
    };
    store::insert(&world.serving.db, &new).await.unwrap();
    std::fs::write(world.path(id), FILE_BYTES).unwrap();
    id
}

const PAGE: &str = "query($device: ID, $first: Int, $after: String) {
    recordings(deviceId: $device, first: $first, after: $after) {
        edges { cursor node { id deviceId startedAt } }
        pageInfo { hasNextPage endCursor }
    }
}";

/// The ids of a page, its end cursor and whether more follow.
async fn page(world: &World, variables: Json) -> (Vec<String>, Option<String>, bool) {
    let found = data(asked(world, PAGE, variables).await);
    let connection = &found["recordings"];
    let ids = connection["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"]["id"].as_str().unwrap().to_owned())
        .collect();
    let end = connection["pageInfo"]["endCursor"]
        .as_str()
        .map(str::to_owned);
    let more = connection["pageInfo"]["hasNextPage"].as_bool().unwrap();
    (ids, end, more)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_041_recordings_page_newest_first_and_filter_by_device(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let elsewhere: DeviceId = ELSEWHERE.parse().unwrap();
    let mut shelved = Vec::new();
    for minutes in 0..5 {
        shelved.push(shelve(&world, docker_host(), minutes).await);
    }
    let other = shelve(&world, elsewhere, 2).await;
    // Two recordings that start at once are ordered by id, newest first.
    let twin = shelve(&world, docker_host(), 4).await;
    let text = |id: &RecordingId| id.to_string();
    let newest_first = [
        twin, shelved[4], shelved[3], other, shelved[2], shelved[1], shelved[0],
    ];

    let (first, end, more) = page(&world, json!({ "first": 3 })).await;
    assert_eq!(
        first,
        newest_first[..3].iter().map(text).collect::<Vec<_>>()
    );
    assert!(more);
    let (second, end, more) = page(&world, json!({ "first": 3, "after": end })).await;
    assert_eq!(
        second,
        newest_first[3..6].iter().map(text).collect::<Vec<_>>()
    );
    assert!(more);
    let (last, end, more) = page(&world, json!({ "first": 3, "after": end })).await;
    assert_eq!(last, [text(&shelved[0])]);
    assert!(!more, "the last page says so");
    let (empty, none, more) = page(&world, json!({ "first": 3, "after": end })).await;
    assert_eq!((empty.len(), none, more), (0, None, false));

    let docker = json!({ "device": docker_host().to_string(), "first": 50 });
    let (only, _, _) = page(&world, docker).await;
    assert_eq!(only.len(), 6);
    assert!(!only.contains(&text(&other)), "{only:?}");
    let (theirs, _, _) = page(&world, json!({ "device": ELSEWHERE })).await;
    assert_eq!(
        theirs,
        [text(&other)],
        "a deleted device's recordings stay listed (D-15)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_041_a_recording_shows_every_k1_field(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    let document = "{ recordings { edges { node {
        id sessionId deviceId deviceName protocol format startedAt endedAt sizeBytes complete
        expiresAt
    } } } }";
    let found = data(asked(&world, document, json!({})).await);
    let session = world.stored(id).await.session;
    assert_eq!(
        found["recordings"]["edges"][0]["node"],
        json!({
            "id": id.to_string(),
            "sessionId": session.to_string(),
            "deviceId": docker_host().to_string(),
            "deviceName": "docker-host",
            "protocol": "VNC",
            "format": "GUACAMOLE",
            "startedAt": now().to_rfc3339(),
            "endedAt": null,
            "sizeBytes": 0.0,
            "complete": false,
            "expiresAt": (now() + Days::new(30)).to_rfc3339(),
        })
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_006_bad_recordings_arguments_are_one_validation_error_listing_each(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let variables = json!({ "device": "not-a-uuid", "first": 0, "after": "r1.0.zz" });
    let response = asked(&world, PAGE, variables).await;
    assert_eq!(error_of(&response).0, "VALIDATION");
    let fields = &response.errors[0]
        .extensions
        .as_ref()
        .unwrap()
        .get("fields");
    let listed = format!("{fields:?}");
    for field in ["deviceId", "first", "after"] {
        assert!(listed.contains(field), "{field}: {listed}");
    }
    for first in [-1, 201] {
        let response = asked(&world, PAGE, json!({ "first": first })).await;
        assert_eq!(error_of(&response).0, "VALIDATION", "{first}");
    }
    let (most, _, _) = page(&world, json!({ "first": 200 })).await;
    assert_eq!(most, Vec::<String>::new(), "200 per page is accepted");
}

#[test]
fn api_006_a_cursor_reads_back_only_as_written() {
    let id = RecordingId::from(Uuid::from_u128(0x0190_a000_0000_7000_8000_0000_0000_00ab));
    let at = |micros| DateTime::<Utc>::from_timestamp_micros(micros).unwrap();
    for micros in [
        0,
        1,
        -1,
        now().timestamp_micros(),
        -62_135_596_800_000_000,
        253_402_300_799_999_999,
    ] {
        let position = Position {
            started_at: at(micros),
            id,
        };
        assert_eq!(decode_cursor(&encode_cursor(position)), Some(position));
    }
    let written = encode_cursor(Position {
        started_at: at(5),
        id,
    });
    assert_eq!(written, "r1.5.0190a0000000700080000000000000ab");
    let hostile = [
        String::new(),
        "r1.".to_owned(),
        "r1.5".to_owned(),
        "s1.5.0190a0000000700080000000000000ab".to_owned(),
        "r1.05.0190a0000000700080000000000000ab".to_owned(),
        "r1.+5.0190a0000000700080000000000000ab".to_owned(),
        "r1.-0.0190a0000000700080000000000000ab".to_owned(),
        "r1.5.0190A0000000700080000000000000AB".to_owned(),
        "r1.5.0190a0000000700080000000000000a".to_owned(),
        "r1.5.0190a0000000700080000000000000abc".to_owned(),
        "r1.5.0190a000-0000-7000-8000-0000000000ab".to_owned(),
        "r1.5.0190a0000000700080000000000000ab.".to_owned(),
        // Past the year 9999, and before the year 1: PostgreSQL would refuse to bind them.
        "r1.253402300800000000.0190a0000000700080000000000000ab".to_owned(),
        "r1.-62135596800000001.0190a0000000700080000000000000ab".to_owned(),
        "r1.99999999999999999999.0190a0000000700080000000000000ab".to_owned(),
        format!("r1.{}.0190a0000000700080000000000000ab", "9".repeat(4096)),
    ];
    for text in hostile {
        assert_eq!(decode_cursor(&text), None, "{text}");
    }
}

#[test]
fn api_006_every_text_near_a_cursor_decodes_only_to_its_own_encoding() {
    // A small property check: mutate each byte of a real cursor to each printable byte; any
    // text that decodes must be the encoding of what it decodes to, so no two texts name one
    // position and none panics.
    let id = RecordingId::from(Uuid::from_u128(0x0190_a000_0000_7000_8000_0000_0000_00ab));
    let written = encode_cursor(Position {
        started_at: now(),
        id,
    });
    for index in 0..written.len() {
        for byte in 0x20_u8..0x7f {
            let mut bytes = written.clone().into_bytes();
            bytes[index] = byte;
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            if let Some(position) = decode_cursor(&text) {
                assert_eq!(encode_cursor(position), text);
            }
        }
    }
}
