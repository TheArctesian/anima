//! D-11: once the 30-a-minute budget of refusal events is spent, one event a minute counts the
//! refusals left out.

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{assert_forbidden, edge, events, typename};

fn left_out(count: usize) -> (String, String, String) {
    (
        "warning".to_owned(),
        "access".to_owned(),
        format!("{count} more refusals were not recorded: at most 30 are recorded a minute."),
    )
}

#[sqlx::test(migrations = "./migrations")]
async fn d_11_refusals_past_the_budget_are_counted_in_one_event_a_minute(pool: PgPool) {
    let edge = edge(&pool, &["100.64.0.0/10"]).await;
    for n in 0..40 {
        let peer = format!("203.0.113.{n}:40000");
        assert_forbidden(&edge.send_from(&peer, typename()).await);
    }
    assert_eq!(events(&pool).await.len(), 30, "the budget");
    edge.clock.advance(TimeDelta::seconds(59));
    assert_forbidden(&edge.send_from("198.51.100.1:40000", typename()).await);
    assert_eq!(
        events(&pool).await.len(),
        30,
        "not yet a minute since the first left out"
    );

    edge.clock.advance(TimeDelta::seconds(1));
    assert_forbidden(&edge.send_from("198.51.100.2:40000", typename()).await);
    let written = events(&pool).await;
    assert_eq!(written.len(), 32, "the new refusal and the count");
    assert!(written.contains(&left_out(11)), "{written:?}");

    edge.clock.advance(TimeDelta::minutes(2));
    assert_forbidden(&edge.send_from("198.51.100.3:40000", typename()).await);
    assert_eq!(events(&pool).await.len(), 33, "nothing more was left out");
}
