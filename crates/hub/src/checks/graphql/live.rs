//! The live views: `DeviceStatusChange` and `HeartbeatAdded` from the bus (LOG-010, E-4).

use async_graphql::{
    ID, SimpleObject,
    futures_util::stream::{self, Stream},
};
use chrono::{DateTime, Utc};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use crate::{
    checks::Heartbeat,
    events::{HeartbeatSummary, Signal},
    ids::{CheckId, DeviceId},
    status::Status,
};

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "DeviceStatusChange")]
pub(super) struct DeviceStatusChange {
    device_id: ID,
    status: Status,
    at: DateTime<Utc>,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "HeartbeatAdded")]
pub(super) struct HeartbeatAdded {
    check_id: ID,
    device_id: ID,
    heartbeat: Heartbeat,
}

/// The device status change a signal carries, when it is about the wanted device (any device
/// when `device` is `None`).
fn device_status(signal: &Signal, device: Option<DeviceId>) -> Option<DeviceStatusChange> {
    match *signal {
        Signal::DeviceStatus {
            device: changed,
            status,
            at,
        } if device.is_none_or(|wanted| wanted == changed) => Some(DeviceStatusChange {
            device_id: ID(changed.to_string()),
            status,
            at,
        }),
        _ => None,
    }
}

/// The heartbeat a signal carries, when it is about the wanted check and device.
fn heartbeat_added(
    signal: &Signal,
    (check, device): (Option<CheckId>, Option<DeviceId>),
) -> Option<HeartbeatAdded> {
    match signal {
        Signal::Heartbeat {
            check: beaten,
            device: owner,
            beat,
        } if check.is_none_or(|wanted| wanted == *beaten)
            && device.is_none_or(|wanted| wanted == *owner) =>
        {
            let HeartbeatSummary {
                at,
                status,
                response_time_ms,
                message,
            } = beat.clone();
            Some(HeartbeatAdded {
                check_id: ID(beaten.to_string()),
                device_id: ID(owner.to_string()),
                heartbeat: Heartbeat {
                    at,
                    status,
                    response_time_ms,
                    message,
                },
            })
        }
        _ => None,
    }
}

/// The items `pick` makes of the signals on `bus`. A subscriber that fell behind skips the
/// signals it missed; the stream ends when the bus is closed.
fn items<T>(
    signals: Receiver<Signal>,
    pick: impl Fn(&Signal) -> Option<T>,
) -> impl Stream<Item = T> {
    stream::unfold((signals, pick), |(mut signals, pick)| async move {
        loop {
            match signals.recv().await {
                Ok(signal) => {
                    if let Some(item) = pick(&signal) {
                        return Some((item, (signals, pick)));
                    }
                }
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return None,
            }
        }
    })
}

pub(super) fn device_status_changes(
    signals: Receiver<Signal>,
    device: Option<DeviceId>,
) -> impl Stream<Item = DeviceStatusChange> {
    items(signals, move |signal| device_status(signal, device))
}

pub(super) fn heartbeats_added(
    signals: Receiver<Signal>,
    wanted: (Option<CheckId>, Option<DeviceId>),
) -> impl Stream<Item = HeartbeatAdded> {
    items(signals, move |signal| heartbeat_added(signal, wanted))
}
