//! What a Compose file from the hub may have (HOST-028; contract K10 Section 6.4.4).
//!
//! One check for the hub and for the applications program of a machine. It reads the file
//! with its own reader, not with the reader of Compose: it permits only the keys of a fixed
//! list, fills the variables itself from the environment file of the item, and applies the
//! rules of the values to the result. It runs nothing, opens no file and reaches no machine.
//! A refusal is a fixed code and the position of a service, never text of the file.

mod declared;
pub mod forms;
mod keys;
mod mounts;
mod project;
mod variables;
mod yaml;

#[cfg(test)]
mod tests;

pub use yaml::Node;

/// The most bytes of a Compose file.
pub const LARGEST_FILE: usize = 256 * 1024;
/// The most bytes of an environment file.
pub const LARGEST_ENVIRONMENT: usize = 64 * 1024;

/// What the rules need to know of the machine. The administrator sets each at the machine;
/// the hub gives its own copy only to show a refusal early.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// "Open to the network": a file may publish a port on each address of the machine.
    pub open_to_network: bool,
    /// The prefixes of the labels that a file may set. Empty after the install.
    pub label_prefixes: Vec<String>,
}

/// The fixed code of the rule that a file breaks (K10 Section 7, `rule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    KeyNotPermitted,
    ValueNotPermitted,
    VariableNotPermitted,
    NameNotPermitted,
    TooLarge,
    YamlNotPermitted,
    ImageNotPermitted,
    PortNotPermitted,
    MountNotPermitted,
    LabelNotPermitted,
    LimitNotPermitted,
}

impl Rule {
    /// The code as the contract writes it.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::KeyNotPermitted => "key_not_permitted",
            Self::ValueNotPermitted => "value_not_permitted",
            Self::VariableNotPermitted => "variable_not_permitted",
            Self::NameNotPermitted => "name_not_permitted",
            Self::TooLarge => "too_large",
            Self::YamlNotPermitted => "yaml_not_permitted",
            Self::ImageNotPermitted => "image_not_permitted",
            Self::PortNotPermitted => "port_not_permitted",
            Self::MountNotPermitted => "mount_not_permitted",
            Self::LabelNotPermitted => "label_not_permitted",
            Self::LimitNotPermitted => "limit_not_permitted",
        }
    }
}

/// Why a file is refused. No variant holds text of the file (K10 Section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// `file_not_permitted`: the rule, and the position of the service in the file, from 0;
    /// `None` for a rule of the top level.
    #[error("the file breaks the rule {} (service {service:?})", rule.code())]
    File { rule: Rule, service: Option<usize> },
    /// `not_at_machine`: the file needs a setting that is off at the machine.
    #[error("the file needs a setting that is off at the machine (service {service:?})")]
    NotAtMachine { service: Option<usize> },
}

impl Refusal {
    pub(crate) const fn top(rule: Rule) -> Self {
        Self::File {
            rule,
            service: None,
        }
    }

    pub(crate) const fn of(rule: Rule, service: usize) -> Self {
        Self::File {
            rule,
            service: Some(service),
        }
    }
}

/// A Compose file that passed each rule, with its variables filled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    /// The file as a tree, after the variables are filled. The applications program writes
    /// the final form from it.
    pub document: Node,
}

/// Checks `file` with `environment` as the one source of its variables.
///
/// # Errors
/// The [`Refusal`] of the first rule that the file breaks.
pub fn check(file: &str, environment: &str, settings: &Settings) -> Result<Checked, Refusal> {
    if file.len() > LARGEST_FILE || environment.len() > LARGEST_ENVIRONMENT {
        return Err(Refusal::top(Rule::TooLarge));
    }
    let document = yaml::read(file)?;
    keys::permitted(&document)?;
    let values = variables::environment(environment)?;
    let document = variables::filled(document, &values)?;
    keys::permitted(&document)?;
    project::checked(&document, settings)?;
    Ok(Checked { document })
}
