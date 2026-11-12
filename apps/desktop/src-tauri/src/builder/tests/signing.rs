//! The Developer ID identity a Mac builder needs before it claims a job (BLD-083).

use crate::builder::tools::{self, Tool};

const IDENTITY: &str = "Developer ID Application: Stephen Okita (7GDLG48PV6)";

fn keychain(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

#[test]
fn bld_083_a_mac_without_the_named_identity_in_its_keychain_lacks_the_signing_identity() {
    let with = keychain(
        "  1) 3E161B31C91D399231D860B4B87FD275B0F61970 \"Developer ID Application: Stephen Okita (7GDLG48PV6)\"\n     1 valid identities found",
    );
    assert_eq!(tools::signing_identity(Some(IDENTITY), &with), None);
    let other_team = keychain(
        "  1) A732FD386F874517D4020012C02BC39E26FC7ED4 \"Developer ID Application: Other Ltd (L2GL58P3B6)\"\n     1 valid identities found",
    );
    assert_eq!(
        tools::signing_identity(Some(IDENTITY), &other_team),
        Some(Tool::SigningIdentity)
    );
    assert_eq!(
        tools::signing_identity(Some(IDENTITY), &keychain("     0 valid identities found")),
        Some(Tool::SigningIdentity)
    );
    // An app built with no identity named, or an empty one, cannot sign either.
    assert_eq!(
        tools::signing_identity(None, &with),
        Some(Tool::SigningIdentity)
    );
    assert_eq!(
        tools::signing_identity(Some(""), &with),
        Some(Tool::SigningIdentity)
    );
    let json = serde_json::to_string(&[Tool::SigningIdentity]).unwrap();
    assert_eq!(json, r#"["signing-identity"]"#);
}
