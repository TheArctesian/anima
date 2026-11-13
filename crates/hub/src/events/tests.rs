use std::net::{IpAddr, Ipv4Addr};

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema, value};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool, Row};
use tokio::sync::broadcast::error::TryRecvError;
use uuid::Uuid;

use super::{
    AuditAction, AuditTarget, Bus, ClientAddr, EventSummary, NewAudit, NewEvent, Operator,
    Severity, Signal, Source, audit, record,
};
use crate::ids::{DeviceId, EventId, OperatorId};

mod partition_grants;
mod partitions;
mod privileges;
mod redaction;
mod retention;
mod runtime_group;
mod runtime_role;
mod source;
mod upkeep;

const DEVICE: &str = "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d";
const OPERATOR: &str = "0192f5a0-7c3e-7a52-9d1b-000000000001";

/// A one-connection pool on the test database: the test superuser logs in, and its session
/// runs as `role` (like `SET ROLE`).
async fn pool_as(pool: &PgPool, role: &str) -> PgPool {
    connect((*pool.connect_options()).clone().options([("role", role)])).await
}

/// A one-connection pool on the test database that logs in as the LOGIN role `login`. The
/// throwaway server trusts local connections, so no password is needed (R-009).
async fn login_pool(pool: &PgPool, login: &str) -> PgPool {
    connect((*pool.connect_options()).clone().username(login)).await
}

async fn connect(options: sqlx::postgres::PgConnectOptions) -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap()
}

/// A cluster-wide role name no other test uses.
fn test_role(purpose: &str) -> String {
    format!("anima_test_{}_{purpose}", Uuid::now_v7().simple())
}

fn day(year: i32, month: u32, date: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, date, 12, 30, 0).unwrap()
}

fn device_event(message: &str) -> NewEvent {
    NewEvent {
        severity: Severity::Warning,
        source: Source::Check,
        device: Some((DEVICE.parse().unwrap(), "lab-router".to_owned())),
        message: message.to_owned(),
        actor: crate::events::ActorKind::Hub,
    }
}

fn new_audit(change: Value) -> NewAudit {
    NewAudit {
        operator: Operator {
            id: OPERATOR.parse().unwrap(),
            name: "operator".to_owned(),
            actor: crate::events::Actor::for_tests(),
        },
        action: AuditAction::Update,
        target: AuditTarget::Device(DEVICE.parse().unwrap()),
        client: ClientAddr(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 7))),
        change,
    }
}

const SEVERITIES: [(Severity, &str); 3] = [
    (Severity::Info, "info"),
    (Severity::Warning, "warning"),
    (Severity::Error, "error"),
];

const SOURCES: [(Source, &str); 8] = [
    (Source::Check, "check"),
    (Source::Session, "session"),
    (Source::Device, "device"),
    (Source::Access, "access"),
    (Source::Hub, "hub"),
    (Source::Metrics, "metrics"),
    (Source::Notifications, "notification"),
    (Source::Vpn, "vpn"),
];

/// The value whose contract text is `text`; panics on text the contract does not list.
fn from_stored<T: Copy>(table: &[(T, &str)], text: &str) -> T {
    table
        .iter()
        .find(|(_, stored)| *stored == text)
        .map_or_else(
            || panic!("unexpected stored text {text:?}"),
            |(value, _)| *value,
        )
}

async fn stored_event(conn: &mut PgConnection, id: EventId) -> EventSummary {
    let row = sqlx::query(
        "select at, severity, source, device_id, device_name, message \
         from events.event_log where id = $1",
    )
    .bind(id)
    .fetch_one(conn)
    .await
    .unwrap();
    let device_id: Option<DeviceId> = row.get("device_id");
    let device_name: Option<String> = row.get("device_name");
    EventSummary {
        id,
        at: row.get("at"),
        severity: from_stored(&SEVERITIES, row.get("severity")),
        source: from_stored(&SOURCES, row.get("source")),
        device: device_id.zip(device_name),
        message: row.get("message"),
        actor: Some(crate::events::ActorKind::Hub),
    }
}

async fn partition_of(conn: &mut PgConnection, id: EventId) -> String {
    sqlx::query_scalar("select tableoid::regclass::text from events.event_log where id = $1")
        .bind(id)
        .fetch_one(conn)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_004_record_stores_the_event_with_the_copied_device_name(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let bus = Bus::new(4);
    let at = day(2026, 3, 14);

    let id = record(&mut conn, &bus, at, device_event("Ping lost"))
        .await
        .unwrap();

    let stored = stored_event(&mut conn, id).await;
    assert_eq!(stored.at, at);
    assert_eq!(stored.severity, Severity::Warning);
    assert_eq!(
        stored.device,
        Some((DEVICE.parse().unwrap(), "lab-router".to_owned()))
    );
    assert_eq!(stored.message, "Ping lost");
}

#[sqlx::test(migrations = "./migrations")]
async fn log_010_record_publishes_exactly_the_stored_event(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let bus = Bus::new(4);
    let mut receiver = bus.subscribe();
    let at = Utc.timestamp_opt(1_773_489_600, 123_456_789).unwrap();

    let id = record(&mut conn, &bus, at, device_event("Ping lost"))
        .await
        .unwrap();

    let stored = stored_event(&mut conn, id).await;
    assert_eq!(receiver.try_recv().unwrap(), Signal::Event(stored));
    assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
}

#[sqlx::test(migrations = "./migrations")]
async fn log_004_record_without_a_device_or_subscriber_succeeds(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let event = NewEvent {
        severity: Severity::Info,
        source: Source::Hub,
        device: None,
        message: "Hub started".to_owned(),
        actor: crate::events::ActorKind::Hub,
    };

    let id = record(&mut conn, &Bus::new(1), day(2026, 3, 14), event)
        .await
        .unwrap();

    let device: Option<Uuid> =
        sqlx::query_scalar("select device_id from events.event_log where id = $1")
            .bind(id)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert_eq!(device, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_004_every_severity_and_source_round_trips_through_its_stored_text(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let bus = Bus::new(1);
    for (severity, _) in SEVERITIES {
        for (source, _) in SOURCES {
            let event = NewEvent {
                severity,
                source,
                ..device_event("round trip")
            };

            let id = record(&mut conn, &bus, day(2026, 3, 14), event)
                .await
                .unwrap();

            let stored = stored_event(&mut conn, id).await;
            assert_eq!((stored.severity, stored.source), (severity, source));
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_audit_records_operator_client_and_change(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let change = json!({ "name": { "from": "lab-router", "to": "core-router" } });

    audit(&mut conn, day(2031, 6, 30), new_audit(change.clone()))
        .await
        .unwrap();

    let row = sqlx::query(
        "select at, operator_id, operator_name, action, target_type, target_id, client_addr, \
         change, tableoid::regclass::text as partition from events.audit_log",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(row.get::<DateTime<Utc>, _>("at"), day(2031, 6, 30));
    assert_eq!(
        row.get::<OperatorId, _>("operator_id"),
        OPERATOR.parse().unwrap()
    );
    assert_eq!(row.get::<&str, _>("operator_name"), "operator");
    assert_eq!(row.get::<&str, _>("action"), "update");
    assert_eq!(row.get::<&str, _>("target_type"), "device");
    assert_eq!(row.get::<&str, _>("target_id"), DEVICE);
    assert_eq!(
        row.get::<IpAddr, _>("client_addr"),
        IpAddr::V4(Ipv4Addr::new(100, 64, 0, 7))
    );
    assert_eq!(row.get::<Value, _>("change"), change);
    assert_eq!(row.get::<&str, _>("partition"), "events.audit_log_20310630");
}

#[test]
fn audit_action_and_target_have_stable_stored_text() {
    let device: DeviceId = DEVICE.parse().unwrap();
    assert_eq!(AuditAction::Deactivate.as_str(), "deactivate");
    assert_eq!(AuditAction::Login.as_str(), "login");
    assert_eq!(AuditTarget::HostKey(device).kind(), "host_key");
    assert_eq!(AuditTarget::Credential(device).kind(), "credential");
    assert_eq!(
        AuditTarget::Window(Uuid::nil().into()).kind(),
        "maintenance_window"
    );
    assert_eq!(AuditTarget::Credential(device).id(), DEVICE);
}

fn device_down() -> Signal {
    Signal::DeviceStatus {
        device: DEVICE.parse().unwrap(),
        status: crate::status::Status::Down,
        at: "2026-11-04T09:30:00Z".parse().unwrap(),
    }
}

#[test]
fn bus_publish_without_subscribers_is_not_an_error() {
    let bus = Bus::new(1);
    bus.publish(device_down());
    let mut late = bus.subscribe();
    assert_eq!(late.try_recv(), Err(TryRecvError::Empty));
}

#[test]
fn log_010_r_028_a_subscriber_receives_the_status_change_with_its_time() {
    let bus = Bus::new(1);
    let mut live = bus.subscribe();
    bus.publish(device_down());
    assert_eq!(live.try_recv(), Ok(device_down()));
}

struct Query;

#[Object]
impl Query {
    async fn severity(&self) -> Severity {
        Severity::Warning
    }

    async fn source(&self) -> Source {
        Source::Hub
    }
}

#[tokio::test]
async fn log_004_severity_and_source_use_the_contract_graphql_names() {
    let schema = Schema::new(Query, EmptyMutation, EmptySubscription);

    let response = schema.execute("{ severity source }").await;

    assert_eq!(
        response.data,
        value!({ "severity": "WARNING", "source": "HUB" })
    );
    let sdl = schema.sdl();
    assert!(
        sdl.contains("enum Severity {\n\tINFO\n\tWARNING\n\tERROR\n}"),
        "{sdl}"
    );
    assert!(
        sdl.contains(
            "enum EventSource {\n\tCHECK\n\tSESSION\n\tDEVICE\n\tACCESS\n\tHUB\n\tMETRICS\n\tNOTIFICATION\n\tVPN\n}"
        ),
        "{sdl}"
    );
}
