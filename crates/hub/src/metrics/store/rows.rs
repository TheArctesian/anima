//! Stored rows and their mapping to the output types.

use chrono::{DateTime, Utc};

use crate::{
    ids::{DeviceId, MetricSeriesId},
    metrics::{Error, Gpu, GpuVendor, MetricKind, MetricPoint, MetricSeries, latest::Latest},
};

#[derive(sqlx::FromRow)]
pub(super) struct ChartRow {
    series_id: MetricSeriesId,
    kind: String,
    label: String,
    gpu_index: Option<i32>,
    at: DateTime<Utc>,
    value: f64,
    min: f64,
    max: f64,
}

/// The series of rows ordered by series then time: one `MetricSeries` per run of rows of the
/// same series.
pub(super) fn chart_of(rows: Vec<ChartRow>) -> Result<Vec<MetricSeries>, Error> {
    let mut charts: Vec<(MetricSeriesId, MetricSeries)> = Vec::new();
    for row in rows {
        let point = MetricPoint {
            at: row.at,
            value: row.value,
            min: Some(row.min),
            max: Some(row.max),
        };
        if let Some((id, last)) = charts.last_mut()
            && *id == row.series_id
        {
            last.points.push(point);
            continue;
        }
        let kind = MetricKind::parse(&row.kind).ok_or(Error::Unreadable)?;
        charts.push((
            row.series_id,
            MetricSeries {
                kind,
                label: row.label,
                gpu_index: row.gpu_index,
                points: vec![point],
            },
        ));
    }
    Ok(charts.into_iter().map(|(_, series)| series).collect())
}

#[derive(sqlx::FromRow)]
pub(super) struct LatestRow {
    device_id: DeviceId,
    kind: String,
    at: DateTime<Utc>,
    value: f64,
}

impl TryFrom<LatestRow> for Latest {
    type Error = Error;

    fn try_from(row: LatestRow) -> Result<Self, Error> {
        Ok(Self {
            device: row.device_id,
            kind: MetricKind::parse(&row.kind).ok_or(Error::Unreadable)?,
            at: row.at,
            value: row.value,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct GpuRow {
    pub(super) device_id: DeviceId,
    index: i32,
    name: String,
    vendor: String,
    has_vram: bool,
}

impl TryFrom<GpuRow> for Gpu {
    type Error = Error;

    fn try_from(row: GpuRow) -> Result<Self, Error> {
        Ok(Self {
            index: row.index,
            name: row.name,
            vendor: GpuVendor::parse(&row.vendor).ok_or(Error::Unreadable)?,
            has_vram: row.has_vram,
        })
    }
}
