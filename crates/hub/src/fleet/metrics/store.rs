//! All SQL on schema `metrics`: I/O only; rows are mapped in `rows`. Ids and time come in as
//! parameters.

mod batteries;
mod chart;
mod drives;
mod gpus;
mod hardware;
mod inventory;
mod latest;
mod points;
mod record;
mod retention;
mod rows;

pub(super) use batteries::{batteries, write_batteries};
pub(super) use chart::chart;
pub(super) use drives::{drives, replace_drives, write_drives};
pub(super) use gpus::{Discovery, gpus, replace_gpus, write_gpus};
pub(super) use hardware::{hardware, write_hardware};
pub(super) use inventory::{inventories, write_inventory};
pub(super) use latest::latest;
pub(super) use points::{PointValue, add_points, created_series};
pub(super) use record::{add_series, record};
pub(super) use retention::{
    create_days_ahead, database_now, delete_idle_series, delete_orphan_samples, drop_days_before,
    trim_aggregates,
};
