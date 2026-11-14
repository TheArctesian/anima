//! The transport sends only to the source's own address (SRC-008, PRJ-018): a request path that
//! names another place is refused before it leaves, and a redirect is never followed.

use anima_hub::engine::sources::Outcome;
use source_exchange::{Method, Request};
use sqlx::PgPool;

use super::{kit, last_run, set_projects, stub::Reply};

#[sqlx::test(migrations = "./migrations")]
async fn src_008_a_path_naming_another_address_is_refused_before_it_is_sent(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    for path in [
        "/https://evil.example/x",
        "//evil.example/x",
        "/x@evil.example",
    ] {
        kit.intake.quirks.lock().unwrap().request = Some(Request {
            method: Method::Get,
            path: path.to_owned(),
            query: Vec::new(),
        });
        assert_eq!(kit.run(id).await, Some(Outcome::Failed), "{path}");
        assert_eq!(
            last_run(&pool, id).await.6.as_deref(),
            Some("The source asked for an address the hub does not send to."),
        );
    }
    assert_eq!(kit.stub.gets(), Vec::<String>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn src_008_a_redirect_is_not_followed(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(&[&[("a", "1")]]);
    let to = format!("{}followed", kit.stub.base);
    kit.stub.reply(
        "/pages/1",
        Reply {
            status: 302,
            headers: vec![("location", to)],
            body: "{}".to_owned(),
        },
    );
    assert_eq!(kit.run(id).await, Some(Outcome::Failed));
    assert_eq!(
        kit.stub.gets(),
        ["/pages/1"],
        "the redirect's target was never asked"
    );
}
