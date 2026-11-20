//! The name of a phone on the link of its hosted hub (mobile plan M3-4, HMD-043). A phone gives an
//! app no name of its own that is safe to use: iOS answers "iPhone" for each one. So the app makes a
//! name one time and keeps it in its data folder; the name is not a secret.

use std::{fs, path::Path};

use super::name;

/// The file of the kept name, in the data folder of the app.
const FILE: &str = "cloud-device";

/// The name of this phone: the kept one, or a new one of `kind` (`iphone`, `android`) with the six
/// hexadecimal digits of `random`, which is then kept. With no folder, or a folder that refuses,
/// the name is new at each start; the cloud then gives the device a new invite. With no random
/// bytes the name is `kind` alone.
pub(super) fn device_name(
    folder: Option<&Path>,
    kind: &str,
    random: impl FnOnce() -> Option<[u8; 3]>,
) -> String {
    let file = folder.map(|folder| folder.join(FILE));
    if let Some(kept) = file.as_deref().and_then(kept) {
        return kept;
    }
    let made = match random() {
        Some([a, b, c]) => name::device_name(&format!("{kind}-{a:02x}{b:02x}{c:02x}")),
        None => name::device_name(kind),
    };
    if let Some(file) = file {
        let _ = fs::write(file, &made);
    }
    made
}

/// The name in `file`, when it is a name that the cloud takes.
fn kept(file: &Path) -> Option<String> {
    let text = fs::read_to_string(file).ok()?;
    let text = text.trim();
    (!text.is_empty() && name::device_name(text) == text).then(|| text.to_owned())
}
