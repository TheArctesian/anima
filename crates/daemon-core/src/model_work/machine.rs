//! The settings at the machine (Section 10.1): the file `model-work.json` in the daemon folder.
//! Only the owner of the machine writes it; the hub cannot. A member that the file leaves out
//! has the default that the operator chose. The actor check (Section 3) reads `agentTypes`.

use serde_json::{Map, Value};

use super::names::{Actor, is_source_name};
use crate::wire::{MODEL_TYPES, WorkType};

/// The organisations of the default list (R-NEW-mlxcommunity). The publishers that a release
/// of the daemon names join this list with that release; this release names none.
pub const DEFAULT_ORGANISATIONS: [&str; 2] = ["unsloth", "mlx-community"];

/// The values of `model-work.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineSettings {
    /// Start each runtime in an isolation (Section 9; default off, R-NEW-sandboxrisk).
    pub sandbox: bool,
    /// Check each LFS hash against Hugging Face too (Section 7; default off, R-NEW-hubhash).
    pub second_hash: bool,
    /// The lowest age of a revision in days; 0 is no check (Section 7).
    pub revision_age_days: u32,
    /// The organisations that a pull may name (Section 7).
    pub organisations: Vec<String>,
    /// The types that the machine takes from an item whose actor is `Agent` (Section 3).
    pub agent_types: Vec<WorkType>,
}

impl Default for MachineSettings {
    fn default() -> Self {
        Self {
            sandbox: false,
            second_hash: false,
            revision_age_days: 0,
            organisations: DEFAULT_ORGANISATIONS.map(str::to_owned).to_vec(),
            agent_types: MODEL_TYPES.to_vec(),
        }
    }
}

/// Why `model-work.json` does not read. The daemon then takes no model item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    #[error("model-work.json is not a JSON object")]
    NotObject,
    #[error("model-work.json has a member that is not a setting: {0}")]
    Unknown(&'static str),
    #[error("model-work.json has a value of the wrong type or range for {0}")]
    Value(&'static str),
}

const MEMBERS: [&str; 5] = [
    "sandbox",
    "secondHash",
    "revisionAgeDays",
    "organisations",
    "agentTypes",
];

fn words<T>(
    value: &Value,
    name: &'static str,
    read: impl Fn(&str) -> Option<T>,
) -> Result<Vec<T>, SettingsError> {
    let list = value.as_array().ok_or(SettingsError::Value(name))?;
    list.iter()
        .map(|word| {
            word.as_str()
                .and_then(&read)
                .ok_or(SettingsError::Value(name))
        })
        .collect()
}

fn flag(object: &Map<String, Value>, name: &'static str) -> Result<Option<bool>, SettingsError> {
    object
        .get(name)
        .map(|value| value.as_bool().ok_or(SettingsError::Value(name)))
        .transpose()
}

impl MachineSettings {
    /// Reads the text of `model-work.json`.
    ///
    /// # Errors
    /// [`SettingsError`] for a text that is not an object of the five settings.
    pub fn parse(text: &str) -> Result<Self, SettingsError> {
        let value: Value = serde_json::from_str(text).map_err(|_| SettingsError::NotObject)?;
        let object = value.as_object().ok_or(SettingsError::NotObject)?;
        if object.keys().any(|key| !MEMBERS.contains(&key.as_str())) {
            return Err(SettingsError::Unknown("a member"));
        }
        let mut settings = Self::default();
        settings.sandbox = flag(object, "sandbox")?.unwrap_or(settings.sandbox);
        settings.second_hash = flag(object, "secondHash")?.unwrap_or(settings.second_hash);
        if let Some(days) = object.get("revisionAgeDays") {
            let days = days.as_u64().and_then(|days| u32::try_from(days).ok());
            settings.revision_age_days = days.ok_or(SettingsError::Value("revisionAgeDays"))?;
        }
        if let Some(list) = object.get("organisations") {
            let read = |name: &str| is_source_name(name).then(|| name.to_owned());
            settings.organisations = words(list, "organisations", read)?;
        }
        if let Some(list) = object.get("agentTypes") {
            let read = |name: &str| WorkType::parse(name).filter(|kind| kind.is_model());
            settings.agent_types = words(list, "agentTypes", read)?;
        }
        Ok(settings)
    }

    /// Whether the machine takes an item of `work_type` that `actor` asked for (Section 3,
    /// R-NEW-agentmodelwork). The hub writes `actor`, so this limits an honest hub only.
    #[must_use]
    pub fn actor_permits(&self, actor: Actor, work_type: WorkType) -> bool {
        actor != Actor::Agent || self.agent_types.contains(&work_type)
    }

    /// Whether a pull may name `organisation` (Section 7).
    #[must_use]
    pub fn organisation_permitted(&self, organisation: &str) -> bool {
        self.organisations.iter().any(|name| name == organisation)
    }

    /// The values as `model_report` gives them (Section 10.1).
    #[must_use]
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "sandbox": self.sandbox,
            "secondHash": self.second_hash,
            "revisionAgeDays": self.revision_age_days,
            "organisations": self.organisations,
            "agentTypes": self.agent_types.iter().map(|kind| kind.as_str()).collect::<Vec<_>>(),
        })
    }
}
