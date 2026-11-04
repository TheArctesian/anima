//! HTTP check secrets: sealed apart from `config`, bound to their check, opened only for the
//! probe, and kept on update when left empty (SEC-003, LOG-007, R-028).

use anima_hub::{
    checks::{
        self, CheckChange, Definition, Error,
        config::{CheckConfig, HttpCredentials, HttpInput, HttpMethod, HttpSecrets, StatusRange},
    },
    credentials::Secret,
    ids::CheckId,
};
use sqlx::PgPool;

use super::{PVE_01, create, new_check, vault, vault_with};

macro_rules! devices_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const PASSWORD: &str = "pw-SECRET-4711";
const COOKIE: &str = "session=cookie-SECRET-0815";
const TOKEN: &str = "tok-SECRET-9000";

fn http(url: &str, cookie: &str, credentials: HttpCredentials) -> Definition {
    Definition::Http(HttpInput {
        url: url.to_owned(),
        method: HttpMethod::Get,
        headers: vec![
            ("Cookie".to_owned(), Secret::from(cookie.to_owned())),
            ("X-Plain".to_owned(), Secret::from("visible".to_owned())),
        ],
        body: None,
        max_redirects: 3,
        accepted_status: vec![StatusRange { from: 200, to: 299 }],
        keyword: None,
        json_query: None,
        json_expected: None,
        credentials,
        verify_tls: true,
        timeout_ms: 5000,
    })
}

fn basic(password: &str) -> HttpCredentials {
    HttpCredentials::Basic {
        username: "operator".to_owned(),
        password: Secret::from(password.to_owned()),
    }
}

fn text(secret: &Secret) -> &str {
    std::str::from_utf8(secret.expose()).unwrap()
}

async fn secrets(pool: &PgPool, id: CheckId) -> HttpSecrets {
    checks::load_http_secrets(pool, &vault(), id)
        .await
        .unwrap()
        .unwrap()
}

async fn update(pool: &PgPool, id: CheckId, definition: Definition) -> Result<(), Error> {
    let mut conn = pool.acquire().await.unwrap();
    let change = CheckChange {
        definition: Some(definition),
        ..CheckChange::default()
    };
    checks::update_check(&mut conn, &vault(), id, change)
        .await
        .map(|_| ())
}

async fn web_check(pool: &PgPool, name: &str) -> CheckId {
    let definition = http("https://pve-01.home.lan:8006/", COOKIE, basic(PASSWORD));
    create(pool, new_check(PVE_01, name, definition)).await
}

devices_test! {
    async fn sec_003_http_secrets_are_sealed_and_appear_nowhere_in_the_row(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let row: String = sqlx::query_scalar("SELECT row_to_json(c)::text FROM checks.checks c WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let in_sealed: bool = sqlx::query_scalar(
            "SELECT position(convert_to($2, 'UTF8') IN secrets_sealed) > 0 \
                 OR position(convert_to($3, 'UTF8') IN secrets_sealed) > 0 \
             FROM checks.checks WHERE id = $1",
        )
        .bind(id)
        .bind(PASSWORD)
        .bind(COOKIE)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!in_sealed);
        for secret in [PASSWORD, COOKIE, "SECRET"] {
            assert!(!row.contains(secret), "{secret} in the row");
        }
        assert!(row.contains("visible"), "the plain header stays in config");
        let check = checks::get(&pool, id).await.unwrap();
        assert!(!format!("{check:?}").contains("SECRET"));

        let opened = secrets(&pool, id).await;
        assert_eq!(opened.password.as_ref().map(text), Some(PASSWORD));
        assert_eq!(opened.headers.iter().map(text).collect::<Vec<_>>(), [COOKIE]);
        assert!(opened.bearer_token.is_none());
    }

    async fn sec_003_sealed_secrets_open_only_for_their_own_check_and_key(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let other = web_check(&pool, "Copy target").await;
        sqlx::query(
            "UPDATE checks.checks SET secrets_sealed = (SELECT secrets_sealed FROM checks.checks WHERE id = $1) \
             WHERE id = $2",
        )
        .bind(id)
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
        let copied = checks::load_http_secrets(&pool, &vault(), other).await;
        assert!(matches!(copied, Err(Error::Credentials(_))), "a copied row opened");
        let wrong_key = checks::load_http_secrets(&pool, &vault_with(8), id).await;
        assert!(matches!(wrong_key, Err(Error::Credentials(_))), "another key opened");
        assert!(checks::load_http_secrets(&pool, &vault(), id).await.unwrap().is_some());
    }

    async fn sec_003_a_check_without_secrets_stores_none(pool: PgPool) {
        let definition = http("http://ha.home.lan:8123/", COOKIE, HttpCredentials::None);
        let Definition::Http(mut input) = definition else { unreachable!() };
        input.headers.remove(0);
        let id = create(&pool, new_check(PVE_01, "Open", Definition::Http(input))).await;
        assert!(checks::load_http_secrets(&pool, &vault(), id).await.unwrap().is_none());
        let icmp = create(&pool, new_check(PVE_01, "Ping", super::icmp())).await;
        assert!(checks::load_http_secrets(&pool, &vault(), icmp).await.unwrap().is_none());
    }

    async fn sec_003_an_update_with_empty_secrets_keeps_the_stored_ones(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        update(&pool, id, http("https://pve-01.home.lan:8007/", "", basic(""))).await.unwrap();
        let check = checks::get(&pool, id).await.unwrap();
        let CheckConfig::Http(config) = check.config else { panic!("not http") };
        assert_eq!(config.url, "https://pve-01.home.lan:8007/");
        let kept = secrets(&pool, id).await;
        assert_eq!(kept.password.as_ref().map(text), Some(PASSWORD));
        assert_eq!(kept.headers.iter().map(text).collect::<Vec<_>>(), [COOKIE]);
    }

    async fn sec_003_an_update_with_new_secrets_replaces_them(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let bearer = HttpCredentials::Bearer(Secret::from(TOKEN.to_owned()));
        update(&pool, id, http("https://pve-01.home.lan:8006/", "session=new", bearer)).await.unwrap();
        let replaced = secrets(&pool, id).await;
        assert!(replaced.password.is_none());
        assert_eq!(replaced.bearer_token.as_ref().map(text), Some(TOKEN));
        assert_eq!(replaced.headers.iter().map(text).collect::<Vec<_>>(), ["session=new"]);
    }

    async fn sec_003_an_empty_secret_with_nothing_stored_is_refused(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let bearer = HttpCredentials::Bearer(Secret::from(String::new()));
        let error = update(&pool, id, http("https://pve-01.home.lan:8006/", COOKIE, bearer)).await.unwrap_err();
        assert!(matches!(&error, Error::Validation(e) if e[0].field == "auth"), "{error:?}");
        let Definition::Http(mut input) = http("https://pve-01.home.lan:8006/", COOKIE, basic(PASSWORD)) else { unreachable!() };
        input.headers.push(("Authorization".to_owned(), Secret::from(String::new())));
        let error = update(&pool, id, Definition::Http(input)).await.unwrap_err();
        assert!(matches!(&error, Error::Validation(e) if e[0].field == "headers"), "{error:?}");
        let mut conn = pool.acquire().await.unwrap();
        let fresh = new_check(PVE_01, "Fresh", http("https://x.lan/", "", basic(PASSWORD)));
        let error = checks::create_check(&mut conn, &vault(), fresh).await.unwrap_err();
        assert!(matches!(&error, Error::Validation(e) if e[0].field == "headers"), "{error:?}");
        assert_eq!(secrets(&pool, id).await.password.as_ref().map(text), Some(PASSWORD));
    }

    async fn log_007_errors_never_echo_a_secret(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let Definition::Http(mut input) = http("https://pve-01.home.lan:8006/", COOKIE, basic(PASSWORD)) else { unreachable!() };
        input.headers.push(("Cookie".to_owned(), Secret::from(format!("{PASSWORD}\r\nX: y"))));
        let error = update(&pool, id, Definition::Http(input)).await.unwrap_err();
        let shown = format!("{error} {error:?}");
        assert!(!shown.contains("SECRET"), "{shown}");
    }
}
