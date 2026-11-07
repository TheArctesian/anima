//! The points of a device's series over a window, from one aggregate table (MET-003, D-7).

use sqlx::PgPool;

use super::rows::{ChartRow, chart_of};
use crate::{
    ids::DeviceId,
    metrics::{
        Error, MetricSeries,
        range::{Resolution, Window},
    },
};

/// The bins of every series of a device: the mean weighted by sample count, the lowest minimum
/// and the highest maximum of the rows in each bin. Bins are aligned to 2000-01-01 UTC, so
/// 10-minute and hourly bins start on the clock.
macro_rules! chart {
    ($table:literal) => {
        concat!(
            "SELECT s.id AS series_id, s.kind, s.label, s.gpu_index, \
                    date_bin(make_interval(secs => $4), a.bucket, \
                             TIMESTAMPTZ '2000-01-01 00:00:00+00') AS at, \
                    SUM(a.avg * a.count) / SUM(a.count) AS value, \
                    MIN(a.min) AS min, MAX(a.max) AS max \
             FROM metrics.series s JOIN metrics.",
            $table,
            " a ON a.series_id = s.id \
             WHERE s.device_id = $1 AND a.bucket >= $2 AND a.bucket <= $3 \
             GROUP BY s.id, at ORDER BY s.kind, s.label, s.id, at"
        )
    };
}

const fn chart_sql(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::Minute => chart!("minute"),
        Resolution::Hour => chart!("hour"),
    }
}

/// One `MetricSeries` per series of `device` that has a bucket in the window, in one query.
pub(in crate::metrics) async fn chart(
    db: &PgPool,
    device: DeviceId,
    window: Window,
) -> Result<Vec<MetricSeries>, Error> {
    let rows: Vec<ChartRow> = sqlx::query_as(chart_sql(window.resolution))
        .bind(device)
        .bind(window.start)
        .bind(window.end)
        .bind(window.bin.as_seconds_f64())
        .fetch_all(db)
        .await?;
    chart_of(rows)
}
