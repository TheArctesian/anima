//! All SQL on schema `metrics`: I/O only; rows are mapped in `rows`. Ids and time come in as
//! parameters.

mod chart;
mod gpus;
mod latest;
mod record;
mod retention;
mod rows;

pub(super) use chart::chart;
pub(super) use gpus::gpus;
pub(super) use latest::latest;
pub(super) use record::record;
pub(super) use retention::{
    create_days_ahead, database_now, delete_idle_series, delete_orphan_samples, drop_days_before,
    trim_aggregates,
};
