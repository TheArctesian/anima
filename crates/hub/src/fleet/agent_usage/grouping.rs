//! Pure: the sums by model folded into the groups the operator asked for, each with the
//! estimate of its cost (AIS-021); and the groups joined to their devices in the order the
//! answer lists them.

use std::{cmp::Reverse, collections::HashMap};

use daemon_core::agent_usage::UsageState;
use uuid::Uuid;

use super::{
    model::{DeviceState, Dimension, DimensionSet, Grain, Group, GroupKey},
    prices,
};
use crate::{fleet::devices::Device, ids::DeviceId};

/// The groups of `grains` by the dimensions of `by`. The cost is estimated for each model
/// before the models are folded, so a group of several models has the sum of their estimates;
/// the tokens of a model with no price are counted in `unpriced_tokens` instead.
#[must_use]
pub fn fold(grains: Vec<Grain>, by: DimensionSet) -> Vec<Group> {
    let mut groups: HashMap<GroupKey, Group> = HashMap::new();
    for grain in grains {
        let tokens = grain.sums.tokens;
        let estimate = grain
            .key
            .model
            .as_deref()
            .and_then(|model| prices::estimate(model, tokens));
        let mut key = grain.key;
        if !by.has(Dimension::Model) {
            key.model = None;
        }
        let group = groups.entry(key.clone()).or_insert_with(|| Group {
            key,
            ..Group::default()
        });
        group.sums = group.sums.plus(grain.sums);
        if tokens.all() > 0.0 {
            match estimate {
                Some(cost) => {
                    group.estimated_cost_usd = Some(group.estimated_cost_usd.unwrap_or(0.0) + cost);
                }
                None => group.unpriced_tokens += tokens.all(),
            }
        }
    }
    groups.into_values().collect()
}

/// A group with its device, when the query groups by device.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub device: Option<Device>,
    pub group: Group,
}

/// `groups` with their devices, newest day first, then by device name (byte order, then ID),
/// source, model and repository, a missing model or repository last. A group whose device is
/// not in `devices` (deleted meanwhile) is left out.
#[must_use]
pub fn arranged(groups: Vec<Group>, devices: &[Device]) -> Vec<Placed> {
    let by_id: HashMap<DeviceId, &Device> = devices.iter().map(|one| (one.id, one)).collect();
    let mut placed: Vec<Placed> = groups
        .into_iter()
        .filter_map(|group| {
            let device = match group.key.device {
                Some(id) => Some((*by_id.get(&id)?).clone()),
                None => None,
            };
            Some(Placed { device, group })
        })
        .collect();
    placed.sort_by(|one, other| order(one).cmp(&order(other)));
    placed
}

type Order<'a> = (
    Reverse<Option<chrono::NaiveDate>>,
    Option<(&'a str, Uuid)>,
    Option<daemon_core::agent_usage::Source>,
    (bool, Option<&'a str>),
    (bool, Option<&'a str>),
);

fn order(row: &Placed) -> Order<'_> {
    let key = &row.group.key;
    (
        Reverse(key.day),
        row.device
            .as_ref()
            .map(|device| (device.name.as_str(), Uuid::from(device.id))),
        key.source,
        (key.model.is_none(), key.model.as_deref()),
        (key.repository.is_none(), key.repository.as_deref()),
    )
}

/// Each of `states` with its device, by device name (byte order, then ID); a state whose
/// device is not in `devices` is left out.
#[must_use]
pub fn states_with_devices(
    states: &[DeviceState],
    devices: &[Device],
) -> Vec<(Device, UsageState)> {
    let mut joined: Vec<(Device, UsageState)> = states
        .iter()
        .filter_map(|state| {
            let device = devices.iter().find(|device| device.id == state.device)?;
            Some((device.clone(), state.state))
        })
        .collect();
    joined.sort_by(|(one, _), (other, _)| {
        (one.name.as_str(), Uuid::from(one.id)).cmp(&(other.name.as_str(), Uuid::from(other.id)))
    });
    joined
}
