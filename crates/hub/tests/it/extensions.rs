mod audit;
mod guard;
mod list;
mod switching;

use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start};

/// A hub at `start()` with the fixture operator and a login token.
async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

const LIST: &str = "{ extensions { key name built fixed on needs worksWith neededBy changedAt \
    dataClass navigation { group label route } } }";

const SET: &str = "mutation($key: String!, $on: Boolean!) { \
    setExtension(key: $key, on: $on) { key on neededBy changedAt } }";

async fn set(hub: &Hub, token: &str, key: &str, on: bool) -> Value {
    hub.post(Some(token), SET, json!({ "key": key, "on": on }))
        .await
}
