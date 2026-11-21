//! The K1 types of the device policy (waits for R-NEW-devicepolicy): `DevicePolicy`,
//! `QuietHours`, `ModelWorkStop` and the inputs of `setDevicePolicy`.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, InputObject, Object, SimpleObject};

use crate::{
    api_errors::FieldError,
    fleet::{
        compute::policy::{self, DevicePolicy, PolicyError, PolicyStatus, Stop},
        devices::Device,
    },
};

/// `DevicePolicy`: the policy of one device and whether it takes model work now.
pub struct DevicePolicyView {
    pub(super) device: Device,
    pub(super) status: PolicyStatus,
}

#[Object(name = "DevicePolicy")]
impl DevicePolicyView {
    async fn device(&self) -> &Device {
        &self.device
    }

    /// The largest part of the usable memory that models may use, in percent.
    async fn memory_share_percent(&self) -> i32 {
        self.status.policy.memory_share.into()
    }

    /// The largest part of the disk that model files may use, in percent.
    async fn disk_share_percent(&self) -> i32 {
        self.status.policy.disk_share.into()
    }

    /// The hours in which the device takes no new model work; null for none.
    async fn quiet_hours(&self) -> Option<QuietHoursView> {
        self.status.policy.quiet_hours.map(|hours| QuietHoursView {
            start: hours.start.format("%H:%M").to_string(),
            end: hours.end.format("%H:%M").to_string(),
            time_zone: hours.zone.name().to_owned(),
        })
    }

    /// Model work only on mains power (CAP-011).
    async fn mains_only(&self) -> bool {
        self.status.policy.mains_only
    }

    /// No new model work while the operator is active at the device (CAP-012).
    async fn idle_only(&self) -> bool {
        self.status.policy.idle_only
    }

    /// True when the operator set no policy for the device and the defaults hold.
    async fn is_default(&self) -> bool {
        !self.status.own
    }

    /// Whether the device takes model work now (CMP-002).
    async fn takes_model_work(&self) -> bool {
        self.status.verdict.is_ok()
    }

    /// Why the device takes no model work now; null when it takes it.
    async fn stop_reason(&self) -> Option<ModelWorkStop> {
        self.status.verdict.err().map(ModelWorkStop::from)
    }
}

/// `QuietHours`: from `start` to `end` (HH:MM) in `timeZone`; past midnight when `end` is not
/// after `start`.
#[derive(Clone, Debug, PartialEq, Eq, SimpleObject)]
#[graphql(name = "QuietHours")]
pub struct QuietHoursView {
    pub start: String,
    pub end: String,
    pub time_zone: String,
}

/// `ModelWorkStop`: why a device takes no model work now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum ModelWorkStop {
    /// The machine never asked this hub for work.
    NoSwitch,
    /// The switch "takes work" at the machine is off.
    SwitchOff,
    /// The operator set the machine off in the hub, or the hub holds it off for an hour.
    OffInHub,
    QuietHours,
    OnBattery,
    OperatorActive,
}

impl From<Stop> for ModelWorkStop {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::NoSwitch => Self::NoSwitch,
            Stop::SwitchOff => Self::SwitchOff,
            Stop::OffInHub => Self::OffInHub,
            Stop::QuietHours => Self::QuietHours,
            Stop::OnBattery => Self::OnBattery,
            Stop::OperatorActive => Self::OperatorActive,
        }
    }
}

/// `DevicePolicyInput`: the whole new policy of a device.
#[derive(Clone, Debug, InputObject)]
pub struct DevicePolicyInput {
    pub memory_share_percent: i32,
    pub disk_share_percent: i32,
    pub quiet_hours: Option<QuietHoursInput>,
    pub mains_only: bool,
    pub idle_only: bool,
}

/// `QuietHoursInput`: `start` and `end` as HH:MM, `timeZone` an IANA name.
#[derive(Clone, Debug, InputObject)]
pub struct QuietHoursInput {
    pub start: String,
    pub end: String,
    pub time_zone: String,
}

impl DevicePolicyInput {
    /// The policy, or the field that is not valid.
    pub(super) fn policy(&self) -> Result<DevicePolicy, FieldError> {
        let share = |value: i32| u8::try_from(value).unwrap_or(0);
        let quiet_hours = self
            .quiet_hours
            .as_ref()
            .map(|hours| policy::quiet_hours(&hours.start, &hours.end, &hours.time_zone))
            .transpose()
            .map_err(field)?;
        DevicePolicy {
            memory_share: share(self.memory_share_percent),
            disk_share: share(self.disk_share_percent),
            quiet_hours,
            mains_only: self.mains_only,
            idle_only: self.idle_only,
        }
        .checked()
        .map_err(field)
    }
}

fn field(error: PolicyError) -> FieldError {
    let name = match error {
        PolicyError::MemoryShare => "memorySharePercent",
        PolicyError::DiskShare => "diskSharePercent",
        PolicyError::EmptyQuietHours | PolicyError::Time => "quietHours",
        PolicyError::Zone => "quietHours.timeZone",
    };
    FieldError::new(name, format!("{}.", capital(&error.to_string())))
}

fn capital(text: &str) -> String {
    let mut letters = text.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}
