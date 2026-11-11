//! SQL on `devices.sites`: a site is made, listed, and read by id.

use sqlx::{PgConnection, PgPool};

use crate::{
    devices::{
        Error,
        model::{NewSite, Site},
        records::{to_site, write_error},
    },
    ids::SiteId,
};

pub(in crate::devices) async fn create_site(
    conn: &mut PgConnection,
    id: SiteId,
    site: &NewSite,
) -> Result<Site, Error> {
    sqlx::query_as("INSERT INTO devices.sites (id, name, subnet) VALUES ($1, $2, $3) RETURNING id, name, subnet, wake_relay_device_id")
        .bind(id)
        .bind(&site.name)
        .bind(site.subnet.net())
        .fetch_one(conn)
        .await
        .map(to_site)
        .map_err(write_error)
}

pub(in crate::devices) async fn list_sites(db: &PgPool) -> Result<Vec<Site>, Error> {
    let rows =
        sqlx::query_as("SELECT id, name, subnet, wake_relay_device_id FROM devices.sites ORDER BY name COLLATE \"C\"")
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(to_site).collect())
}

pub(in crate::devices) async fn sites_by_ids(
    db: &PgPool,
    ids: &[SiteId],
) -> Result<Vec<Site>, Error> {
    let rows = sqlx::query_as(
        "SELECT id, name, subnet, wake_relay_device_id FROM devices.sites WHERE id = ANY($1) ORDER BY name COLLATE \"C\"",
    )
    .bind(ids)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(to_site).collect())
}
