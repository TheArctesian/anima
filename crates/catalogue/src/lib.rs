//! The recipes of the catalogue that ships with Anima, and the static test of each recipe
//! (HOST-047, R-300, task H36).
//!
//! A recipe is one folder: the entry file (`entry.json`), the Compose file (`compose.yaml`) and
//! an example environment file with no secret value (`environment.example`). The static test
//! reads only these three texts. It installs nothing, starts no container and reaches no
//! machine: the test on a machine is task H37. Each failure is a fixed code with the
//! identifier of the recipe, never text of a file.

mod database;
pub mod entry;
mod example;
mod fields;
mod file;
mod folder;
pub mod postgresql;
mod privileges;
mod shipped;

#[cfg(test)]
mod tests;

pub use folder::{ReadError, read};
pub use privileges::Privilege;
pub use shipped::shipped;

use entry::{Entry, MachineTest};

/// The three texts of one recipe folder. A file that is not in the folder is `None`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recipe {
    /// The name of the folder, which is the identifier of the entry.
    pub folder: String,
    pub entry: Option<String>,
    pub compose: Option<String>,
    pub environment: Option<String>,
}

/// The fixed code of one failure of the static test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Code {
    /// The entry file is not there, is not JSON, or has a key or a form that is not permitted.
    EntryNotReadable,
    /// The identifier is not a name, or is not the name of the folder.
    IdNotPermitted,
    /// A text field or the list of tags is empty.
    FieldEmpty,
    /// An address is not `https://`, or a file address is not the fixed name in the folder.
    AddressNotPermitted,
    /// The Compose file or the example environment file is not in the folder.
    FileNotFound,
    /// A date is not `YYYY-MM-DD`.
    DateNotPermitted,
    /// An image is not `repository@sha256:<64 hex digits>`.
    ImageNotPinned,
    /// A service of the Compose file has an image that the list of the entry does not give.
    ImageNotListed,
    /// The list of the entry gives an image that no service of the Compose file has.
    ImageNotUsed,
    /// The rules of a Compose file (task H28) refuse the file with the settings of its marks.
    ComposeRefused(compose_rules::Refusal),
    /// The file passes the default rules, so a mark is not needed.
    MarkNotNeeded,
    /// The ports on each address of the machine are not the ports of the `network-port` marks.
    MarkNotMatched,
    /// A mark for which no rule exists yet (R-247 for the socket of the engine).
    MarkNotPermitted(entry::Mark),
    /// The test of a machine is not complete.
    TestNotComplete,
    /// A top-level volume has no kind of copy, or a described volume is not in the file.
    VolumeNotDescribed,
    /// The example environment file has a value for a secret.
    SecretInExample,
    /// A generated variable is not marked "generated at install" with no value, or the reverse.
    GeneratedNotMarked,
    /// The last test on a machine failed: the entry leaves the list.
    MachineTestFailed,
    /// The fields about a database are not complete: a server of kind `database`, or the
    /// need of an application (its variables, each generated at install).
    DatabaseNotDescribed,
    /// A database server publishes a port on the machine (Section 14.1 of the proposal).
    DatabasePortPublished,
    /// The image, a published port or its address is a variable, which the install could
    /// change after the static test (SF-226).
    VariableNotPermitted,
    /// The Compose file gives a variable whose name marks a secret a value that is not one
    /// variable (SF-225).
    SecretInFile,
    /// A service keeps rights that it does not need, or changes its code at the start
    /// (SF-224).
    PrivilegeNotPermitted(Privilege),
}

impl Code {
    /// The code as the report writes it.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EntryNotReadable => "entry_not_readable",
            Self::IdNotPermitted => "id_not_permitted",
            Self::FieldEmpty => "field_empty",
            Self::AddressNotPermitted => "address_not_permitted",
            Self::FileNotFound => "file_not_found",
            Self::DateNotPermitted => "date_not_permitted",
            Self::ImageNotPinned => "image_not_pinned",
            Self::ImageNotListed => "image_not_listed",
            Self::ImageNotUsed => "image_not_used",
            Self::ComposeRefused(_) => "compose_refused",
            Self::MarkNotNeeded => "mark_not_needed",
            Self::MarkNotMatched => "mark_not_matched",
            Self::MarkNotPermitted(_) => "mark_not_permitted",
            Self::TestNotComplete => "test_not_complete",
            Self::VolumeNotDescribed => "volume_not_described",
            Self::SecretInExample => "secret_in_example",
            Self::GeneratedNotMarked => "generated_not_marked",
            Self::MachineTestFailed => "machine_test_failed",
            Self::DatabaseNotDescribed => "database_not_described",
            Self::DatabasePortPublished => "database_port_published",
            Self::VariableNotPermitted => "variable_not_permitted",
            Self::SecretInFile => "secret_in_file",
            Self::PrivilegeNotPermitted(_) => "privilege_not_permitted",
        }
    }

    /// The fixed second part of a code: the rule of a refused Compose file, the mark, or the
    /// privilege.
    #[must_use]
    pub const fn detail(&self) -> Option<&'static str> {
        match self {
            Self::ComposeRefused(compose_rules::Refusal::File { rule, .. }) => Some(rule.code()),
            Self::ComposeRefused(compose_rules::Refusal::NotAtMachine { .. }) => {
                Some("not_at_machine")
            }
            Self::MarkNotPermitted(mark) => Some(entry::mark_name(mark)),
            Self::PrivilegeNotPermitted(privilege) => Some(privilege.code()),
            _ => None,
        }
    }
}

/// One failure of one recipe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The name of the recipe folder.
    pub recipe: String,
    pub code: Code,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.recipe, self.code.code())?;
        match self.code.detail() {
            Some(detail) => write!(formatter, " ({detail})"),
            None => Ok(()),
        }
    }
}

/// The static test of one recipe: each failure, or none for a recipe that may be listed.
#[must_use]
pub fn check(recipe: &Recipe) -> Vec<Finding> {
    codes(recipe)
        .into_iter()
        .map(|code| Finding {
            recipe: recipe.folder.clone(),
            code,
        })
        .collect()
}

fn codes(recipe: &Recipe) -> Vec<Code> {
    let Some(entry) = recipe
        .entry
        .as_deref()
        .and_then(|text| serde_json::from_str::<Entry>(text).ok())
    else {
        return vec![Code::EntryNotReadable];
    };
    let mut codes = fields::check(&entry, &recipe.folder);
    codes.extend(database::check(&entry.anima));
    if matches!(entry.anima.last_test, MachineTest::Failed { .. }) {
        codes.push(Code::MachineTestFailed);
    }
    let (Some(compose), Some(environment)) = (&recipe.compose, &recipe.environment) else {
        codes.push(Code::FileNotFound);
        return codes;
    };
    codes.extend(example::check(environment, &entry.anima.generated));
    codes.extend(file::check(&entry.anima, compose, environment));
    codes
}
