//! The SQL for `access.permitted_networks` (NET-001, D-10).

use ipnet::IpNet;
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

/// Makes the networks labelled `label` exactly `networks`: deletes the others with that label
/// and inserts the missing ones. A network stored under another label is left as it is.
pub(in crate::fleet::access) async fn replace_labelled_networks(
    conn: &mut PgConnection,
    label: &str,
    networks: &[IpNet],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "delete from access.permitted_networks where label = $1 and cidr <> all($2::cidr[])",
    )
    .bind(label)
    .bind(networks)
    .execute(&mut *conn)
    .await?;
    let ids: Vec<Uuid> = networks.iter().map(|_| Uuid::now_v7()).collect();
    sqlx::query(
        "insert into access.permitted_networks (id, cidr, label) \
         select id, cidr, $3 from unnest($1::uuid[], $2::cidr[]) as given (id, cidr) \
         on conflict (cidr) do nothing",
    )
    .bind(ids)
    .bind(networks)
    .bind(label)
    .execute(conn)
    .await?;
    Ok(())
}

/// Every permitted network.
pub(in crate::fleet::access) async fn permitted_networks(
    db: impl PgExecutor<'_>,
) -> Result<Vec<IpNet>, sqlx::Error> {
    sqlx::query_scalar("select cidr from access.permitted_networks")
        .fetch_all(db)
        .await
}
