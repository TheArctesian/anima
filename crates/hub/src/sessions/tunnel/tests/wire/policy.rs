//! The device's policy on the wire (K4 Sections 4 and 7, RA-021, RA-033): read when the
//! tunnel opens, it keeps a refused paste from guacd and guacd's clipboard from the client
//! with the session still up, closes on audio input or a drive upload the device refuses,
//! and answers a refused download so guacd frees the stream.

use guac_protocol::{Instruction, encode};
use sqlx::PgPool;

use super::{
    bench::{Bench, Setup, bench, docker_host},
    fake_guacd::{Act, FakeGuacd, RDP_ARGS, Script, connect_values, fake_guacd},
    heard, opened,
    records::ended,
    socket::{Socket, open},
};

/// The shared-drive switch, which is off by default.
const DRIVE_ON: &str = "update devices.rdp_settings set shared_drive = true where device_id = $1";
/// guacd's mimetype for a shared-drive folder listing.
const LISTING: &str = "application/vnd.glyptodon.guacamole.stream-index+json";

/// An RDP session opened after `changes` to the started session's device, past the open
/// instruction.
async fn session_after(pool: &PgPool, changes: &[&str]) -> (FakeGuacd, Bench, Socket) {
    let script = Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    };
    let guacd = fake_guacd(script).await;
    let bench = bench(pool, &guacd.address, Setup::default()).await;
    for change in changes {
        sqlx::query(change)
            .bind(docker_host())
            .execute(pool)
            .await
            .unwrap();
    }
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    (guacd, bench, client)
}

/// The client sends `wire`; the hub closes 1002 and guacd hears only the hub leaving.
async fn closes_unheard(guacd: &mut FakeGuacd, client: &mut Socket, wire: &str) {
    client.send(wire).await;
    assert_eq!(client.close_code().await, Some(1002), "{wire}");
    assert_eq!(heard(guacd).await.opcode, "disconnect", "{wire}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_033_paste_off_guacd_sees_no_clipboard_and_the_session_stays_up(pool: PgPool) {
    let off = "update devices.devices set transfer_paste = false where id = $1";
    let (mut guacd, bench, mut client) = session_after(&pool, &[off]).await;
    let handshake = (&mut guacd.handshake).await.unwrap();
    let values = connect_values(RDP_ARGS, handshake.last().unwrap());
    assert_eq!(values["disable-paste"], "true", "guacd is the second guard");

    client
        .send("9.clipboard,1.0,10.text/plain;4.blob,1.0,12.cGFzdGVkIQ==;3.end,1.0;")
        .await;
    client.send("3.key,5.65307,1.1;").await;
    assert_eq!(
        heard(&mut guacd).await,
        Instruction::new("key", ["65307", "1"]),
        "the clipboard, its blob and its end never reach guacd"
    );
    guacd.act.send(Act::Send("4.sync,1.7;".to_owned())).ok();
    assert_eq!(client.text().await, "4.sync,1.7;");
    assert_eq!(client.close().await, Some(1000));
    assert_eq!(ended(&pool, bench.session).await.reason, "operator");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_033_copy_off_the_client_sees_no_clipboard_from_guacd(pool: PgPool) {
    let off = "update devices.devices set transfer_copy = false where id = $1";
    let (mut guacd, _bench, mut client) = session_after(&pool, &[off]).await;
    let copied = "9.clipboard,1.3,10.text/plain;4.blob,1.3,8.Y29waWVk;3.end,1.3;";
    guacd
        .act
        .send(Act::Send(format!("{copied}4.sync,1.8;")))
        .ok();
    assert_eq!(
        client.text().await,
        "4.sync,1.8;",
        "the clipboard stream is dropped"
    );
    client.send("3.key,5.65307,1.1;").await;
    assert_eq!(
        heard(&mut guacd).await.opcode,
        "key",
        "no ack is owed for it"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_021_microphone_off_no_audio_reaches_guacd(pool: PgPool) {
    let off = "update devices.rdp_settings set microphone = false where device_id = $1";
    let (mut guacd, _bench, mut client) = session_after(&pool, &[off]).await;
    closes_unheard(&mut guacd, &mut client, "5.audio,1.1,9.audio/L16;").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_021_drive_off_put_never_reaches_guacd(pool: PgPool) {
    let off = "update devices.rdp_settings set shared_drive = false where device_id = $1";
    let (mut guacd, _bench, mut client) = session_after(&pool, &[off]).await;
    closes_unheard(
        &mut guacd,
        &mut client,
        "3.put,1.0,1.1,10.text/plain,4./a.sh;",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_021_upload_off_with_the_drive_on_file_never_reaches_guacd(pool: PgPool) {
    let changes = [
        DRIVE_ON,
        "update devices.devices set transfer_upload = false where id = $1",
    ];
    let (mut guacd, _bench, mut client) = session_after(&pool, &changes).await;
    closes_unheard(&mut guacd, &mut client, "4.file,1.0,10.text/plain,4.a.sh;").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_021_download_off_get_browses_and_guacd_hears_the_refused_file(pool: PgPool) {
    let changes = [
        DRIVE_ON,
        "update devices.devices set transfer_download = false, transfer_upload = false \
         where id = $1",
    ];
    let (mut guacd, _bench, mut client) = session_after(&pool, &changes).await;
    client.send("3.get,1.0,5./a.sh;").await;
    assert_eq!(
        heard(&mut guacd).await,
        Instruction::new("get", ["0", "/a.sh"])
    );

    let body = encode(&Instruction::new("body", ["0", "4", "text/plain", "/a.sh"]));
    let listing = encode(&Instruction::new("body", ["0", "5", LISTING, "/"]));
    let content = "4.blob,1.4,8.c2VjcmV0;3.end,1.4;";
    guacd
        .act
        .send(Act::Send(format!("{body}{content}{listing}")))
        .ok();
    assert_eq!(
        client.text().await,
        listing,
        "only the folder listing arrives"
    );
    let ack = Instruction::new(
        "ack",
        ["4", "Download is turned off for this device.", "771"],
    );
    assert_eq!(
        heard(&mut guacd).await,
        ack,
        "guacd frees the stream, not waits"
    );
    client.send("3.key,5.65307,1.1;").await;
    assert_eq!(heard(&mut guacd).await.opcode, "key");
}
