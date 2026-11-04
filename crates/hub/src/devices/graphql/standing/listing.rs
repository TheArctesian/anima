//! `Query.devices`: the SQL path for the name order, and the in-memory path that joins the
//! checks domain for a status filter or a STATUS or UPTIME order (API-006).
//!
//! The in-memory path loads every device that passes the SQL filters, with no cap, reads their
//! statuses and uptimes in one call each, then filters, orders and pages. Loading every
//! matching device is fine at Phase 1 scale; the SQL join across domains is deferred (issue #1).
//! Its cursors are positional (key and name), so a status change between two pages can make a
//! device be skipped or shown twice.

use std::{collections::HashMap, sync::Arc};

use async_graphql::{Context, Result};

use super::ranking::{self, Readings};
use crate::{
    checks::{self, UptimeWindow},
    clock::Clock,
    devices::{
        graphql::{
            connection::DeviceConnection,
            error::{internal, schema_dependency, to_graphql},
            filter::{DeviceSort, ListArgs},
            pool,
        },
        model::Device,
        store,
    },
    ids::DeviceId,
};

/// The window UPTIME orders by.
const SORT_WINDOW: UptimeWindow = UptimeWindow::Day;

pub(in crate::devices::graphql) async fn list(
    ctx: &Context<'_>,
    sort: DeviceSort,
    args: ListArgs,
) -> Result<DeviceConnection> {
    if sort == DeviceSort::Name && args.statuses.is_empty() {
        return by_name(ctx, args).await;
    }
    let devices = store::filtered(pool(ctx)?, &args.filter)
        .await
        .map_err(to_graphql)?;
    let readings = read(ctx, sort, &args, &devices).await?;
    let arranged = ranking::arrange(devices, sort, &readings, &args.statuses);
    let total = arranged.len();
    let (shown, has_next) = ranking::page(arranged, sort, args.after.as_ref(), args.size);
    Ok(DeviceConnection::with_cursors(shown, has_next, total))
}

async fn by_name(ctx: &Context<'_>, args: ListArgs) -> Result<DeviceConnection> {
    let after = args.after.as_ref().map(|at| at.name.as_str());
    let page = store::list(pool(ctx)?, &args.filter, args.size, after)
        .await
        .map_err(to_graphql)?;
    let total = usize::try_from(page.total).unwrap_or(0);
    Ok(DeviceConnection::new(
        page.devices,
        page.has_next_page,
        total,
    ))
}

/// The statuses and uptimes the filter and the order need, each in one call.
async fn read(
    ctx: &Context<'_>,
    sort: DeviceSort,
    args: &ListArgs,
    devices: &[Device],
) -> Result<Readings> {
    let db = pool(ctx)?;
    let ids: Vec<DeviceId> = devices.iter().map(|device| device.id).collect();
    let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
    let statuses = if sort == DeviceSort::Status || !args.statuses.is_empty() {
        checks::statuses(db, &ids, now).await.map_err(internal)?
    } else {
        HashMap::default()
    };
    let uptimes = if sort == DeviceSort::Uptime {
        checks::uptimes(db, &ids, SORT_WINDOW, now)
            .await
            .map_err(internal)?
    } else {
        HashMap::default()
    };
    Ok(Readings { statuses, uptimes })
}
