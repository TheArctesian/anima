//! A stub intake in the place of Projects: it reads the stub's numbered pages and keeps each
//! item in a test table, comparing before it writes (SRC-012). It can fail after it stored a
//! page, or ask for a request of the test's choice.

use std::sync::{Arc, Mutex};

use anima_hub::{
    engine::{
        extensions::{Key, PROJECTS},
        sources::{Counts, Intake, IntakeError, Page, Taken},
    },
    ids::SourceAccountId,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use source_exchange::{Body, Method, Problem, Request};
use sqlx::{PgConnection, PgPool};

/// The table the stub intake writes; `writes` counts every insert or update of a row.
pub async fn make_table(pool: &PgPool) {
    sqlx::query(
        "CREATE TABLE public.stub_records (account_id uuid NOT NULL, id text NOT NULL, \
         value text NOT NULL, writes int NOT NULL DEFAULT 1, PRIMARY KEY (account_id, id))",
    )
    .execute(pool)
    .await
    .unwrap();
}

/// What the stub intake does beyond reading pages.
#[derive(Default)]
pub struct Quirks {
    /// Fail once on this page, after its records are written.
    pub fail_after_storing: Option<u64>,
    /// Ask for this instead of the next page.
    pub request: Option<Request>,
}

#[derive(Clone, Default)]
pub struct StubIntake {
    pub quirks: Arc<Mutex<Quirks>>,
}

fn page_of(slice: &Value) -> u64 {
    slice["page"].as_u64().unwrap_or(1)
}

/// Writes one item; returns which count it adds to.
async fn keep(
    conn: &mut PgConnection,
    account: SourceAccountId,
    (id, value): (&str, &str),
) -> Result<&'static str, IntakeError> {
    let stored: Option<String> = sqlx::query_scalar(
        "SELECT value FROM public.stub_records WHERE account_id = $1 AND id = $2",
    )
    .bind(account)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    let (sql, count) = match stored.as_deref() {
        Some(old) if old == value => return Ok("unchanged"),
        Some(_) => (
            "UPDATE public.stub_records SET value = $3, writes = writes + 1 \
             WHERE account_id = $1 AND id = $2",
            "changed",
        ),
        None => (
            "INSERT INTO public.stub_records (account_id, id, value) VALUES ($1, $2, $3)",
            "inserted",
        ),
    };
    sqlx::query(sql)
        .bind(account)
        .bind(id)
        .bind(value)
        .execute(conn)
        .await?;
    Ok(count)
}

#[async_trait]
impl Intake for StubIntake {
    fn source(&self) -> &'static str {
        "github"
    }

    fn extension(&self) -> Key {
        PROJECTS
    }

    fn slice_key(&self) -> &'static str {
        "test.items"
    }

    fn begin(&self, slice: &Value) -> Value {
        if slice["done"] == true || slice["v"] != 1 {
            json!({ "v": 1, "page": 1, "done": false })
        } else {
            slice.clone()
        }
    }

    fn next_request(&self, slice: &Value) -> Option<Request> {
        if slice["done"] == true {
            return None;
        }
        if let Some(request) = self.quirks.lock().unwrap().request.clone() {
            return Some(request);
        }
        Some(Request {
            method: Method::Get,
            path: format!("/pages/{}", page_of(slice)),
            query: Vec::new(),
        })
    }

    async fn take(&self, conn: &mut PgConnection, page: Page<'_>) -> Result<Taken, IntakeError> {
        let (200, Body::Read(raw)) = (page.response.status, page.response.body) else {
            let problem = Problem {
                code: "status",
                subject: "page".to_owned(),
                message: "the page was not served".to_owned(),
            };
            return Ok(Taken {
                slice: page.slice.clone(),
                counts: Counts::default(),
                problems: vec![problem],
            });
        };
        let body: Value = serde_json::from_slice(raw).unwrap();
        let mut counts = Counts::default();
        for item in body["items"].as_array().unwrap() {
            let pair = (
                item["id"].as_str().unwrap(),
                item["value"].as_str().unwrap(),
            );
            match keep(conn, page.account, pair).await? {
                "inserted" => counts.inserted += 1,
                "changed" => counts.changed += 1,
                _ => counts.unchanged += 1,
            }
        }
        let number = page_of(page.slice);
        let fail = {
            let mut quirks = self.quirks.lock().unwrap();
            let fail = quirks.fail_after_storing == Some(number);
            if fail {
                quirks.fail_after_storing = None;
            }
            fail
        };
        if fail {
            return Err(IntakeError::Refused("the test fails after storing"));
        }
        let slice = match body["next"].as_u64() {
            Some(next) => json!({ "v": 1, "page": next, "done": false }),
            None => json!({ "v": 1, "page": number, "done": true }),
        };
        Ok(Taken {
            slice,
            counts,
            problems: Vec::new(),
        })
    }

    async fn forget(
        &self,
        conn: &mut PgConnection,
        account: SourceAccountId,
    ) -> Result<u64, IntakeError> {
        let done = sqlx::query("DELETE FROM public.stub_records WHERE account_id = $1")
            .bind(account)
            .execute(conn)
            .await?;
        Ok(done.rows_affected())
    }
}
