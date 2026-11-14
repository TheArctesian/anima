//! K-E1 Section 13.1: the live path leaves the sync alone. A live read, a live 401 and a change
//! leave every row of `sources.accounts` and `sources.sync_runs` as it was.

use anima_hub::engine::sources::{Keep, LiveFailure, Outcome, ask, change, standing};
use sqlx::PgPool;

use super::{COMMENT_PATH, PULLS, comment, get, live_kit, ok, reply, striding};
use crate::{hub::start, sources::set_projects};

/// Every row of the two tables the sync owns, as text.
async fn sync_rows(pool: &PgPool) -> (Vec<String>, Vec<String>) {
    let accounts = sqlx::query_scalar("SELECT a::text FROM sources.accounts a ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap();
    let runs = sqlx::query_scalar("SELECT r::text FROM sources.sync_runs r ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap();
    (accounts, runs)
}

#[sqlx::test(migrations = "./migrations")]
async fn src_007_a_live_read_a_live_401_and_a_change_write_nothing_of_the_sync(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(&[&[("a", "1")]]);
    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    let before = sync_rows(&pool).await;
    assert_eq!((before.0.len(), before.1.len()), (1, 1));

    let s = striding(&kit, kit.reach.transport.clone());
    kit.stub.reply(PULLS, ok("[]"));
    ask(&s, id, &get(PULLS), Keep::for_seconds(30))
        .await
        .unwrap();
    kit.stub.reply("/refused", reply(401, &[], "{}"));
    let refused = ask(&s, id, &get("/refused"), Keep::for_seconds(30)).await;
    assert_eq!(refused.unwrap_err(), LiveFailure::KeyRefused);
    assert!(kit.reach.transport.tokens().fresh(id, start()).is_none());
    kit.stub.reply(COMMENT_PATH, reply(401, &[], "{}"));
    change(&s, id, &comment("hello")).await.unwrap();

    assert_eq!(sync_rows(&pool).await, before);
    let mut conn = pool.acquire().await.unwrap();
    let standings = standing(&mut conn, &[id]).await.unwrap();
    assert_eq!(
        standings,
        [(id, anima_hub::engine::sources::Standing::Ready)]
    );
}
