//! All SQL on schema `metrics`: I/O only; rows are mapped in `rows`. Ids and time come in as
//! parameters.

mod chart;
mod gpus;
mod latest;
mod points;
mod record;
mod retention;
mod rows;

pub(super) use chart::chart;
pub(super) use gpus::{Discovery, gpus, replace_gpus, write_gpus};
pub(super) use latest::latest;
pub(super) use points::{PointValue, add_points, live_series};
pub(super) use record::{add_series, record};
pub(super) use retention::{
    create_days_ahead, database_now, delete_idle_series, delete_orphan_samples, drop_days_before,
    trim_aggregates,
};
