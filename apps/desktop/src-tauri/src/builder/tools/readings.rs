//! What the tool check reads from this computer at the OS edge (BLD-028, BLD-083): the signing
//! identities in the login keychain, and the identity this app was built to sign with. The
//! decision on these readings is in `tools`.

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

/// The identity `bundle.macOS.signingIdentity` names in the `tauri.conf.json` this app was built
/// from (BLD-083). Tauri keeps no bundle settings in the running app's config, so the file is
/// read at compile time.
pub fn signing_identity() -> Option<String> {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../../tauri.conf.json")).ok()?;
    config["bundle"]["macOS"]["signingIdentity"]
        .as_str()
        .map(str::to_owned)
}
