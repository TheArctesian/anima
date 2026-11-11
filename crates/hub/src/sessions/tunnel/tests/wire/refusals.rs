//! Refusals before the upgrade (K4 Section 2): 403 for every ticket problem, an SSH ticket
//! included, 400 without the subprotocol, 503 when the sweep cannot run, the gateway cannot
//! be reached or the device's host is refused; and the token never reaches the process log
//! (LOG-007).

use std::time::Duration;

use chrono::TimeDelta;
use guac_protocol::Display;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::http::StatusCode;
use uuid::Uuid;

use super::{
    bench::{
        LOOPBACK, PASSWORD, Setup, address, bench, clock_at, docker_host, move_host, now,
        on_tunnel, operator,
    },
    ended_once,
    fake_guacd::{CONNECTION_ID, RDP_ARGS, Script, fake_guacd},
    opened,
    records::{Capture, ended},
    socket::{open, open_with},
};
use crate::{
    events::process_log,
    ids::SessionId,
    sessions::{
        Live, PendingTicket, Presented, Protocol, Session,
        tickets::{TOKEN_BYTES, TicketToken},
    },
};

/// The session row as it stands.
async fn row(pool: &PgPool, bench: &super::Bench) -> Session {
    crate::sessions::store::get(pool, bench.session)
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_2_api_009_no_or_unknown_token_is_403_and_spends_nothing(pool: PgPool) {
    let bench = bench(&pool, "127.0.0.1:9", Setup::default()).await;
    let base = bench.url.split('?').next().unwrap().to_owned();
    let unknown = format!("{base}?token={}", "A".repeat(43));
    let tampered = format!("{base}?token={}x", &bench.token[..42]);
    for url in [base.clone(), format!("{base}?token="), unknown, tampered] {
        assert_eq!(
            open_with(&url, true).await.err(),
            Some(StatusCode::FORBIDDEN),
            "{url}"
        );
    }
    assert!(
        row(&pool, &bench).await.is_live(),
        "the session waits for its own ticket"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_2_api_009_a_used_ticket_is_403(pool: PgPool) {
    let guacd = fake_guacd(Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    })
    .await;
    let bench = bench(&pool, &guacd.address, Setup::default()).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert!(
        row(&pool, &bench).await.is_live(),
        "the open session is untouched"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_2_api_009_an_expired_ticket_is_403_and_swept_timeout(pool: PgPool) {
    let setup = Setup::route(|route| route.clock = clock_at(30));
    let bench = bench(&pool, "127.0.0.1:9", setup).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
    let ended = ended(&pool, bench.session).await;
    assert_eq!(ended.reason, "timeout");
    assert!(
        ended.events.is_empty(),
        "no tunnel opened, so no event (D-13)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_2_sec_002_a_ticket_from_another_address_is_403(pool: PgPool) {
    let setup = Setup {
        ticket_client: address("10.10.0.77"),
        ..Setup::default()
    };
    let bench = bench(&pool, "127.0.0.1:9", setup).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_2_no_guacamole_subprotocol_is_400_and_spends_the_ticket(pool: PgPool) {
    let bench = bench(&pool, "127.0.0.1:9", Setup::default()).await;
    assert_eq!(
        open_with(&bench.url, false).await.err(),
        Some(StatusCode::BAD_REQUEST)
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "protocol_error", "warning", LOOPBACK);
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
}

/// An address where nothing listens.
async fn closed_port() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().to_string()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_2_guacd_down_is_503_and_gateway_error(pool: PgPool) {
    let bench = bench(&pool, &closed_port().await, Setup::default()).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(ended.events[0].3, "The hub could not reach the gateway.");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_032_a_host_that_resolves_to_loopback_is_refused_at_connect(pool: PgPool) {
    let mut guacd = fake_guacd(Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    })
    .await;
    // The hub's own rule, not the tests' loopback exception.
    let setup = Setup {
        live: Live::default(),
        ..Setup::default()
    };
    let bench = bench(&pool, &guacd.address, setup).await;
    // `start` accepted the IP literal; the name is only resolved at connect.
    move_host(&pool, "localhost").await;
    let refused = open_with(&bench.url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert!(ended.events[0].3.contains("loopback"), "{:?}", ended.events);
    assert!(guacd.selected.try_recv().is_err(), "guacd is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_2_a_sweep_that_cannot_run_is_503_and_spends_nothing(pool: PgPool) {
    let setup = Setup::route(|route| {
        // A database that refuses every connection.
        route.db = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(300))
            .connect_lazy("postgres://anima@127.0.0.1:9/anima")
            .unwrap();
    });
    let bench = bench(&pool, "127.0.0.1:9", setup).await;
    // An expired ticket for another session, so the sweep has a row to end.
    let expired = PendingTicket {
        expires_at: now() - TimeDelta::seconds(1),
        ..pending(Protocol::Rdp, 1)
    };
    let token = TicketToken::from_random([9; TOKEN_BYTES]);
    bench.live.issue(token.hash(), expired).unwrap();
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    assert!(row(&pool, &bench).await.is_live());
    assert!(bench.live.redeem(on_tunnel(&bench.token), now()).is_ok());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn log_007_gw_014_the_token_password_and_connection_id_never_reach_the_log(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // While exactly one dispatcher is registered, tracing-core builds a callsite's interest
    // from the registering thread's own default, so another test thread with no subscriber
    // would cache the tunnel's callsites as disabled for this one too. A second registered
    // dispatcher makes every registration ask all of them.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    tracing::info!("log_007 capture marker");

    let guacd = fake_guacd(Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    })
    .await;
    let bench = bench(&pool, &guacd.address, Setup::default()).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    assert_eq!(client.close().await, Some(1000));
    ended(&pool, bench.session).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        open_with(&bench.url, false).await.err(),
        Some(StatusCode::FORBIDDEN)
    );

    let log = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        log.contains("log_007 capture marker"),
        "the capture works: {log}"
    );
    for secret in [bench.token.as_str(), PASSWORD, CONNECTION_ID, "token="] {
        assert!(!log.contains(secret), "{secret} in the log");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_1_k4_2_an_ssh_ticket_on_the_tunnel_is_403_and_spent(pool: PgPool) {
    // SSH tickets are for `/terminal` (R-062): this route refuses them like any other path's.
    let guacd = fake_guacd(Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    })
    .await;
    let bench = bench(&pool, &guacd.address, Setup::default()).await;
    let token = TicketToken::from_random([5; TOKEN_BYTES]);
    bench
        .live
        .issue(token.hash(), pending(Protocol::Ssh, 2))
        .unwrap();
    let base = bench.url.split('?').next().unwrap();
    let url = format!("{base}?token={}", token.expose());
    assert_eq!(
        open_with(&url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
    let again = Presented::terminal(token.expose(), address(LOOPBACK));
    assert!(
        bench.live.redeem(again, now()).is_err(),
        "the ticket is spent, even for its own path"
    );
}

/// A ticket for `protocol` on `docker-host`, for another session numbered `n`.
fn pending(protocol: Protocol, n: u128) -> PendingTicket {
    PendingTicket {
        operator: operator(),
        session: SessionId::from(Uuid::from_u128(
            0x0190_a000_0000_7000_8010_0000_0000_0000 + n,
        )),
        client: address(LOOPBACK),
        device: docker_host(),
        protocol,
        display: Display {
            width: 1280,
            height: 800,
            dpi: 96,
        },
        timezone: None,
        expires_at: now() + TimeDelta::seconds(30),
        wake_sent: false,
        recording: false,
    }
}
