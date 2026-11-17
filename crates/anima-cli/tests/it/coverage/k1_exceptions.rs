//! The K1 root fields the command line never uses, each with its reason (CMD-031). A builder who
//! adds a document for a field deletes its entry here; the coverage test fails until they do,
//! and for any K1 root field that is neither used nor listed.

pub const EXCEPTIONS: &[(&str, &str)] = &[
    (
        "Mutation.hostedDeviceInvite",
        "the Anima.ing cloud only: takes the cloud key, a secret (R-129)",
    ),
    (
        "Mutation.claimDeviceToken",
        "the desktop app's silent join through the link only: returns a token (R-129, HMD-037)",
    ),
    (
        "Mutation.signInBrowser",
        "browser only: sets the __Host-anima cookie (K7)",
    ),
    (
        "Mutation.claimBuildJob",
        "builder token only: the desktop builder (BLD-022)",
    ),
    (
        "Mutation.reportBuildJob",
        "builder token only: the desktop builder (BLD-022)",
    ),
    (
        "Mutation.buildUploadUrls",
        "builder token only: the desktop builder (BLD-022)",
    ),
    (
        "Mutation.reportBuildUploaded",
        "builder token only: the desktop builder (BLD-022)",
    ),
];
