//! The checks of each Mach-O file of the macOS app (UPD-011, S12 2.3, D-3 to D-5): both
//! architectures, a Developer ID signature of the release's Team ID, the hardened runtime, a
//! secure timestamp and no entitlement. A file is Mach-O by its first bytes, never by its name or
//! folder (D-4). The plans and the decisions on their output are pure; the steps run them.

use std::path::Path;

use crate::builder::{
    error::BuilderError,
    plan::{Command, Kept, Program, command},
};

/// The first four bytes of a Mach-O file, thin or fat, 32 or 64 bits, in either byte order.
const MAGICS: [[u8; 4]; 8] = [
    [0xfe, 0xed, 0xfa, 0xce],
    [0xce, 0xfa, 0xed, 0xfe],
    [0xfe, 0xed, 0xfa, 0xcf],
    [0xcf, 0xfa, 0xed, 0xfe],
    [0xca, 0xfe, 0xba, 0xbe],
    [0xbe, 0xba, 0xfe, 0xca],
    [0xca, 0xfe, 0xba, 0xbf],
    [0xbf, 0xba, 0xfe, 0xca],
];
/// The architectures each file must hold; the display checks run for each.
pub const ARCHS: [&str; 2] = ["x86_64", "arm64"];

/// Whether `header`, the first bytes of a file, is a Mach-O magic number. A Java class file has
/// the fat magic too; the checks then refuse it, which is safe.
pub fn is_mach_o(header: &[u8]) -> bool {
    header
        .get(..4)
        .is_some_and(|magic| MAGICS.iter().any(|known| known == magic))
}

/// The Team ID of a Developer ID identity: ten characters of `A-Z0-9`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamId(String);

impl TeamId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The identity that signs a release and the Team ID it names (D-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    pub identity: String,
    pub team: TeamId,
}

impl Signer {
    /// `identity` with the Team ID in its last parentheses, as `Developer ID Application: Name
    /// (7GDLG48PV6)`. The Team ID goes into a code requirement, so anything but ten characters
    /// of `A-Z0-9` at the end is refused.
    pub fn from_identity(identity: &str) -> Result<Self, BuilderError> {
        let team = identity
            .strip_suffix(')')
            .and_then(|rest| rest.rsplit_once('('))
            .map(|(_, team)| team)
            .filter(|team| {
                team.len() == 10
                    && team
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            })
            .ok_or(BuilderError::Bundle)?;
        Ok(Self {
            identity: identity.to_owned(),
            team: TeamId(team.to_owned()),
        })
    }
}

/// The code requirement of a Developer ID Application certificate of `team`: Apple's anchor, the
/// Developer ID intermediate, the Developer ID Application leaf, and the team as its OU.
pub fn requirement(team: &TeamId) -> String {
    format!(
        "anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and \
         certificate leaf[field.1.2.840.113635.100.6.1.13] exists and \
         certificate leaf[subject.OU] = \"{}\"",
        team.as_str()
    )
}

fn on_file(program: Program, file: &Path, args: &[&str]) -> Command {
    let mut step = command(program, file.parent().unwrap_or(file), args);
    step.args.push(file.as_os_str().to_owned());
    step
}

/// `lipo -archs`; its output must name `x86_64` and `arm64`.
pub fn archs(file: &Path) -> Command {
    on_file(Program::Lipo, file, &["-archs"])
}

/// `codesign --verify --strict` against the Developer ID requirement of `team`; exits 0 only
/// when every architecture passes.
pub fn verify(file: &Path, team: &TeamId) -> Command {
    let requirement = format!("-R={}", requirement(team));
    on_file(
        Program::Codesign,
        file,
        &["--verify", "--strict", &requirement],
    )
}

/// `codesign -d --verbose=4` of one architecture; its lines are on standard error.
pub fn display(file: &Path, arch: &str) -> Command {
    let mut step = on_file(
        Program::Codesign,
        file,
        &["-d", "--verbose=4", "--arch", arch],
    );
    step.kept = Kept::Errors;
    step
}

/// `codesign -d --entitlements - --xml` of one architecture: the entitlements on standard output,
/// nothing when there are none.
pub fn entitlements(file: &Path, arch: &str) -> Command {
    on_file(
        Program::Codesign,
        file,
        &["-d", "--entitlements", "-", "--xml", "--arch", arch],
    )
}

/// Whether `codesign -d --verbose=4` printed one code directory with the `runtime` flag
/// (`flags=0x10000(runtime)`) and a secure timestamp (`Timestamp=`). A signature with no
/// timestamp prints `Signed Time=` instead.
pub fn hardened_and_timestamped(lines: &[String]) -> bool {
    let directories: Vec<&String> = lines
        .iter()
        .filter(|line| line.starts_with("CodeDirectory "))
        .collect();
    let runtime = matches!(directories[..], [line] if line
        .split_whitespace()
        .filter_map(|word| word.strip_prefix("flags="))
        .filter_map(|flags| flags.split_once('(')?.1.strip_suffix(')'))
        .any(|names| names.split(',').any(|name| name == "runtime")));
    let timestamp = lines.iter().any(|line| {
        line.strip_prefix("Timestamp=")
            .is_some_and(|time| !time.trim().is_empty() && time.trim() != "none")
    });
    runtime && timestamp
}

/// Whether `codesign -d --entitlements - --xml` printed no entitlement: nothing, or a property
/// list whose one dictionary is empty (D-3).
pub fn no_entitlements(lines: &[String]) -> bool {
    let text: String = lines
        .concat()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    text.is_empty()
        || (text.starts_with("<?xml")
            && (text.ends_with("<plistversion=\"1.0\"><dict></dict></plist>")
                || text.ends_with("<plistversion=\"1.0\"><dict/></plist>"))
            && text.matches("<dict").count() == 1
            && !text.contains("<key>"))
}
