//! A site on the tailnet's own range is reached directly: no router key, no route approval.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    graphql::create_site,
    mutations::{APPROVE, ROUTER_KEY, answering, asked, code, hub_over, new_key},
};

#[sqlx::test(migrations = "./migrations")]
async fn vpn_009_a_site_on_the_tailnet_range_has_no_router_key_or_route_approval(pool: PgPool) {
    let fake = answering(|_| new_key());
    let (hub, token, _, reader) = hub_over(&pool, &fake).await;
    let inside = create_site(&hub, &token, "vpn", "100.64.0.0/10").await;

    let key = hub
        .post(Some(&token), ROUTER_KEY, json!({ "s": inside }))
        .await;
    let approval = hub
        .post(Some(&token), APPROVE, json!({ "s": inside }))
        .await;
    reader.abort();

    for response in [&key, &approval] {
        assert_eq!(code(response), "VALIDATION", "{response}");
        assert_eq!(
            response["errors"][0]["extensions"]["fields"][0]["field"], "siteId",
            "{response}"
        );
    }
    assert_eq!(asked(&fake), Vec::<Value>::new());
}
