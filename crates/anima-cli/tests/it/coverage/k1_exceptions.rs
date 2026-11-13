//! The K1 root fields the command line never uses, each with its reason (CMD-031). A builder who
//! adds a document for a field deletes its entry here; the coverage test fails until they do,
//! and for any K1 root field that is neither used nor listed.

pub const EXCEPTIONS: &[(&str, &str)] = &[
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
    // Temporary: task T7 (CLI nouns extension, source, project) deletes each line below.
    ("Query.extensions", "temporary: task T7 adds `extension list`"),
    ("Query.sources", "temporary: task T7 adds `source kinds`"),
    ("Query.sourceAccounts", "temporary: task T7 adds `source list`"),
    ("Query.projects", "temporary: task T7 adds `project list`"),
    ("Query.project", "temporary: task T7 adds `project get`"),
    ("Mutation.setExtension", "temporary: task T7 adds `extension on|off`"),
    ("Mutation.connectGithubApp", "temporary: task T7 adds `source connect github`"),
    ("Mutation.syncSourceAccount", "temporary: task T7 adds `source sync`"),
    ("Mutation.setSourceAccountPaused", "temporary: task T7 adds `source pause|resume`"),
    ("Mutation.removeSourceAccount", "temporary: task T7 adds `source remove`"),
    ("Mutation.leaveOutRepository", "temporary: task T7 adds `project leave-out`"),
    ("Mutation.restoreRepository", "temporary: task T7 adds `project restore`"),
    ("Mutation.confirmProjectResource", "temporary: task T7 adds `project resource confirm`"),
    ("Subscription.sourceAccountChanged", "temporary: task T7 adds `source watch`"),
];
