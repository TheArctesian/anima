//! PRJ-032, K-E1 Section 13.1: the cache sends nothing while an answer is fresh, reads again on
//! `fresh` and on expiry, serves the held answer stale when a refresh fails, keeps no file
//! content, and forgets exactly one repository's paths.

use anima_hub::engine::sources::{Keep, LiveFailure, ask, forget};
use chrono::TimeDelta;
use source_exchange::Body;
use sqlx::PgPool;

use super::{PULLS, get, live_kit, ok, reply};
use crate::hub::start;

fn secs(n: i64) -> TimeDelta {
    TimeDelta::seconds(n)
}

const KEEP: Keep = Keep::for_seconds(30);

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_second_read_inside_the_window_sends_nothing(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    kit.stub.reply(PULLS, ok("[1]"));

    let first = ask(&kit.at(start()), id, &get(PULLS), KEEP).await.unwrap();
    let second = ask(&kit.at(start() + secs(29)), id, &get(PULLS), KEEP)
        .await
        .unwrap();

    assert_eq!(kit.stub.gets(), [PULLS]);
    assert_eq!(second.response().body, Body::Read(b"[1]"));
    assert_eq!((second.at(), second.stale()), (first.at(), false));
    assert_eq!(first.at(), start());
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_fresh_and_an_expired_answer_read_again(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    kit.stub.reply(PULLS, ok("[1]"));
    ask(&kit.at(start()), id, &get(PULLS), KEEP).await.unwrap();

    let young = ask(
        &kit.at(start() + secs(4)),
        id,
        &get(PULLS),
        KEEP.fresh(true),
    )
    .await
    .unwrap();
    assert_eq!(young.at(), start(), "under five seconds old");
    assert_eq!(kit.stub.gets().len(), 1);

    kit.stub.reply(PULLS, ok("[1,2]"));
    let forced = ask(
        &kit.at(start() + secs(5)),
        id,
        &get(PULLS),
        KEEP.fresh(true),
    )
    .await
    .unwrap();
    assert_eq!(forced.response().body, Body::Read(b"[1,2]"));
    assert_eq!(kit.stub.gets().len(), 2);

    let expired = ask(&kit.at(start() + secs(35)), id, &get(PULLS), KEEP)
        .await
        .unwrap();
    assert_eq!(expired.at(), start() + secs(35));
    assert_eq!(kit.stub.gets().len(), 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_failed_refresh_serves_the_held_answer_stale_for_an_hour(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    kit.stub.reply(PULLS, ok("[1]"));
    ask(&kit.at(start()), id, &get(PULLS), KEEP).await.unwrap();
    kit.stub
        .reply(PULLS, reply(502, &[], "{\"message\":\"down\"}"));

    let stale = ask(&kit.at(start() + secs(60)), id, &get(PULLS), KEEP)
        .await
        .unwrap();
    assert!(stale.stale());
    assert_eq!(stale.problem(), Some(LiveFailure::Unusable));
    assert_eq!((stale.at(), stale.response().status), (start(), 200));
    assert_eq!(stale.response().body, Body::Read(b"[1]"));

    let too_old = ask(&kit.at(start() + secs(3601)), id, &get(PULLS), KEEP).await;
    assert_eq!(too_old.unwrap_err(), LiveFailure::Unusable);
    kit.stub
        .reply("/repos/octo/widgets/issues", reply(502, &[], "{}"));
    let never_held = ask(
        &kit.at(start()),
        id,
        &get("/repos/octo/widgets/issues"),
        KEEP,
    )
    .await;
    assert_eq!(never_held.unwrap_err(), LiveFailure::Unusable);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_reply_that_is_not_2xx_is_returned_and_not_held(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    kit.stub.reply(PULLS, reply(404, &[], "{}"));

    let missing = ask(&kit.at(start()), id, &get(PULLS), KEEP).await.unwrap();
    ask(&kit.at(start()), id, &get(PULLS), KEEP).await.unwrap();

    assert_eq!(missing.response().status, 404);
    assert_eq!(kit.stub.gets().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_file_content_is_never_kept(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    let file = "/repos/octo/widgets/contents/README.md";
    kit.stub.reply(file, ok("{\"content\":\"c2VjcmV0\"}"));

    ask(&kit.at(start()), id, &get(file), Keep::never())
        .await
        .unwrap();
    ask(&kit.at(start()), id, &get(file), Keep::never())
        .await
        .unwrap();
    kit.stub.reply(file, reply(502, &[], "{}"));
    let failed = ask(&kit.at(start()), id, &get(file), Keep::never()).await;

    assert_eq!(kit.stub.gets().len(), 3);
    assert_eq!(failed.unwrap_err(), LiveFailure::Unusable, "nothing stale");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_028_forget_drops_exactly_that_repositorys_answers(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    let paths = [
        PULLS,
        "/repos/octo/widgets/pulls/12",
        "/repos/octo/gadgets/pulls",
    ];
    for path in paths {
        kit.stub.reply(path, ok("[]"));
        ask(&kit.at(start()), id, &get(path), KEEP).await.unwrap();
    }

    forget(&kit.reach.transport, id, "/repos/octo/widgets");
    for path in paths {
        ask(&kit.at(start()), id, &get(path), KEEP).await.unwrap();
    }

    let gets = kit.stub.gets();
    assert_eq!(gets.len(), 5, "{gets:?}");
    assert_eq!(gets[3..], [PULLS, "/repos/octo/widgets/pulls/12"]);
}
