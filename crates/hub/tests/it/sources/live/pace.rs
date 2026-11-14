//! Decision 6: at most two live reads of an account in flight; its changes one at a time and a
//! second apart; another account is not held up by either.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anima_hub::engine::sources::{Keep, ask, change};
use source_exchange::{Change, RepositoryName, RunId};
use sqlx::PgPool;
use tokio::sync::Semaphore;

use super::{COMMENT_PATH, comment, get, heard_gets, live_kit, ok, reply, second_account};
use crate::hub::start;

const CANCEL_PATH: &str = "/repos/octo/widgets/test-changes/cancel_workflow_run";

/// Long enough for a request that is free to go to reach the stub.
const SETTLE: Duration = Duration::from_millis(300);

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_third_read_of_one_account_waits_for_one_of_two_to_end(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    let other = second_account(&kit, "78").await;
    let gate = Arc::new(Semaphore::new(0));
    let held = ["/held/1", "/held/2", "/held/3"];
    for path in held {
        kit.stub.gate(path, &gate);
        kit.stub.reply(path, ok("[]"));
    }
    kit.stub.reply("/free", ok("[]"));
    let reads: Vec<_> = held
        .iter()
        .map(|path| {
            let (s, request) = (kit.at(start()), get(path));
            tokio::spawn(async move { ask(&s, id, &request, Keep::never()).await })
        })
        .collect();

    heard_gets(&kit.stub, 2).await;
    tokio::time::sleep(SETTLE).await;
    assert_eq!(kit.stub.gets().len(), 2, "the third read waits");
    let (s, free) = (kit.at(start()), get("/free"));
    let freed = tokio::time::timeout(SETTLE, ask(&s, other, &free, Keep::never())).await;
    assert_eq!(
        freed.unwrap().unwrap().response().status,
        200,
        "another account goes"
    );

    gate.add_permits(1);
    heard_gets(&kit.stub, 4).await;
    gate.add_permits(2);
    for read in reads {
        assert_eq!(read.await.unwrap().unwrap().response().status, 200);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_two_changes_of_one_account_are_a_second_apart_and_another_account_is_not_held(
    pool: PgPool,
) {
    let (kit, id) = live_kit(&pool).await;
    let other = second_account(&kit, "78").await;
    kit.stub.reply(COMMENT_PATH, reply(201, &[], "{}"));
    let cancel = Change::CancelWorkflowRun {
        repository: RepositoryName::parse("octo/widgets").unwrap(),
        run: RunId::parse("99").unwrap(),
    };
    let changes: Vec<_> = [
        (id, comment("hello")),
        (id, comment("again")),
        (other, cancel),
    ]
    .into_iter()
    .map(|(account, made)| {
        let s = kit.at(start());
        tokio::spawn(async move { change(&s, account, &made).await })
    })
    .collect();
    for made in changes {
        made.await.unwrap().unwrap();
    }

    let heard = |path: &str| -> Vec<Instant> {
        kit.stub.with(|s| {
            s.heard
                .iter()
                .filter(|heard| heard.path == path)
                .map(|heard| heard.at)
                .collect()
        })
    };
    let (ours, theirs) = (heard(COMMENT_PATH), heard(CANCEL_PATH));
    assert_eq!((ours.len(), theirs.len()), (2, 1));
    assert!(ours[1] - ours[0] >= Duration::from_secs(1), "{ours:?}");
    assert!(
        theirs[0] < ours[1],
        "the other account did not wait for this one"
    );
}
