//! What the tool check reads from this computer at the OS edge (BLD-028, BLD-083, D-19): where
//! the updater key stands, whether the app's memory is guarded, and the signing identities in
//! the login keychain. The decisions on these readings are in `tools`.

use super::KeyReadiness;
use crate::updater_key::{self, UnlockedKey};

/// Where the updater key stands; a key store that cannot be read counts as no key.
pub fn key_readiness(unlocked: &UnlockedKey) -> KeyReadiness {
    if !updater_key::is_present().unwrap_or(false) {
        KeyReadiness::Missing
    } else if unlocked.is_unlocked() {
        KeyReadiness::Unlocked
    } else {
        KeyReadiness::Locked
    }
}

/// Yama's `ptrace_scope` and whether this process is dumpable, read at the edge (D-19). macOS
/// writes no core dump of the app and lets no other process of the user read its memory.
pub fn memory_guarded() -> bool {
    let scope = std::fs::read_to_string("/proc/sys/kernel/yama/ptrace_scope").ok();
    cfg!(target_os = "macos") || super::memory_guarded(scope.as_deref(), updater_key::dumpable())
}

/// The lines `security find-identity -v -p codesigning` prints for the login keychain; none when
/// it cannot run (BLD-083).
pub fn keychain_identities() -> Vec<String> {
    std::process::Command::new("/usr/bin/security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
