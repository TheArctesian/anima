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

/// The bins of every series of a device: the mean weighted by sample count (each row's share of
/// the bin's samples, so a huge average cannot overflow a sum of products), the lowest minimum
/// and the highest maximum of the rows in each bin. Bins are aligned to 2000-01-01 UTC, so
/// 10-minute and hourly bins start on the clock.
macro_rules! chart {
    ($table:literal) => {
        concat!(
            "SELECT series_id, kind, label, gpu_index, at, \
                    SUM(avg * weight) AS value, MIN(min) AS min, MAX(max) AS max \
             FROM (SELECT s.id AS series_id, s.kind, s.label, s.gpu_index, \
                    date_bin(make_interval(secs => $4), a.bucket, \
                             TIMESTAMPTZ '2000-01-01 00:00:00+00') AS at, \
                    a.avg, a.min, a.max, \
                    a.count::float8 / SUM(a.count) OVER (PARTITION BY s.id, \
                        date_bin(make_interval(secs => $4), a.bucket, \
                                 TIMESTAMPTZ '2000-01-01 00:00:00+00')) AS weight \
                 FROM metrics.series s JOIN metrics.",
            $table,
            " a ON a.series_id = s.id \
                 WHERE s.device_id = $1 AND a.bucket >= $2 AND a.bucket <= $3) bins \
             GROUP BY series_id, kind, label, gpu_index, at \
             ORDER BY kind, label, series_id, at"
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
