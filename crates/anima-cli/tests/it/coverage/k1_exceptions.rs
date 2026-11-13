//! The K1 root fields the command line does not use yet, or never will, each with its reason
//! (CMD-031). The marker names the task of docs/cli-mcp-plan.md Section 10 that removes the
//! entry. A builder who adds a document deletes the entries it uses; the coverage test fails
//! until they do.

pub const EXCEPTIONS: &[(&str, &str)] = &[
    // Permanent.
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
    // T2: device (the rest), site, tag, web, agent.
    // T4: event, audit, notify, retention, key, and the watch commands.
    ("Subscription.deviceStatusChanged", "device watch"), // T4
    ("Subscription.heartbeatAdded", "check watch"),       // T4
    ("Subscription.sessionChanged", "session watch"),     // T4
    ("Subscription.metricsAdded", "device metrics --watch"), // T4
    ("Subscription.buildJobUpdated", "release log --follow"), // T4
    ("Subscription.vpnChanged", "vpn status --watch"),    // T4
    // T6: session, ssh, files.
    ("Query.sessions", "session list"),              // T6
    ("Mutation.startSession", "ssh <device>"),       // T6
    ("Mutation.endSession", "session end"),          // T6
    ("Mutation.openFiles", "files ls, get and put"), // T6
];
