//! No session reaches the container of the hub: its own device takes no SSH, RDP or VNC, no
//! secret for them and keeps its kind, and no other device takes the kind `hub` (HMD-045).

use anima_hub::fleet::devices::ensure_hub_device;
use sqlx::PgPool;

use super::harness::{data, now, only_error, schema_with_bus, validation_fields};

#[sqlx::test(migrations = "./migrations")]
async fn hmd_045_the_hub_device_refuses_each_way_in(pool: PgPool) {
    let (schema, bus) = schema_with_bus(&pool);
    let mut conn = pool.acquire().await.unwrap();
    let (hub, _) = ensure_hub_device(&mut conn, &bus, now()).await.unwrap();
    let (id, site) = (hub.id, hub.site);
    let update = |input: &str| {
        format!(r#"mutation {{ updateDevice(id: "{id}", input: {input}) {{ id }} }}"#)
    };
    let ways = r#"{ ssh: { enabled: true, username: "root", auth: PASSWORD },
        rdp: { enabled: true, username: "root" }, vnc: { enabled: true } }"#;
    let error = only_error(&schema, &update(ways)).await;
    assert_eq!(validation_fields(&error), ["ssh", "rdp", "vnc"]);
    let error = only_error(&schema, &update(r#"{ kind: "server" }"#)).await;
    assert_eq!(validation_fields(&error), ["kind"]);
    data(&schema, &update(r#"{ name: "Hub of the office" }"#)).await;

    for mutation in ["setSshCredential", "setRdpCredential", "setVncCredential"] {
        let query = format!(
            r#"mutation {{ {mutation}(deviceId: "{id}", credential: {{ password: "pw-of-the-hub" }}) {{ __typename }} }}"#
        );
        let error = only_error(&schema, &query).await;
        assert_eq!(validation_fields(&error), ["deviceId"], "{mutation}");
    }

    let create = format!(
        r#"mutation {{ createDevice(input: {{ siteId: "{site}", name: "second", host: "second.lan", kind: "Hub" }}) {{ id }} }}"#
    );
    let error = only_error(&schema, &create).await;
    assert_eq!(validation_fields(&error), ["kind"]);
}
