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
    ("Query.tags", "tag list"),                                     // T2
    ("Mutation.createSite", "site create"),                         // T2
    ("Mutation.setSiteWakeRelay", "site set-wake-relay"),           // T2
    ("Mutation.createDevice", "device create"),                     // T2
    ("Mutation.updateDevice", "device update"),                     // T2
    ("Mutation.deleteDevice", "device delete"),                     // T2
    ("Mutation.testConnection", "device test"),                     // T2
    ("Mutation.wakeDevice", "device wake"),                         // T2
    ("Mutation.setSshCredential", "device credential set ssh"),     // T2
    ("Mutation.clearSshCredential", "device credential clear ssh"), // T2
    ("Mutation.setRdpCredential", "device credential set rdp"),     // T2
    ("Mutation.clearRdpCredential", "device credential clear rdp"), // T2
    ("Mutation.setVncCredential", "device credential set vnc"),     // T2
    ("Mutation.clearVncCredential", "device credential clear vnc"), // T2
    ("Mutation.acceptHostKey", "device trust accept-host-key"),     // T2
    (
        "Mutation.acceptServerCertificate",
        "device trust accept-certificate",
    ), // T2
    ("Mutation.createWebInterface", "web create"),                  // T2
    ("Mutation.updateWebInterface", "web update"),                  // T2
    ("Mutation.deleteWebInterface", "web delete"),                  // T2
    (
        "Mutation.acceptWebInterfaceSuggestion",
        "web suggestion accept",
    ), // T2
    (
        "Mutation.dismissWebInterfaceSuggestion",
        "web suggestion dismiss",
    ), // T2
    ("Mutation.acceptWebCertificate", "web accept-certificate"),    // T2
    ("Mutation.enrolAgent", "agent enrol"),                         // T2
    ("Mutation.revokeAgentToken", "agent revoke"),                  // T2
    // T3: check, maintenance.
    ("Query.check", "check get"),                                  // T3
    ("Query.checkDefaults", "check defaults get"),                 // T3
    ("Query.maintenanceWindows", "maintenance list"),              // T3
    ("Mutation.createCheck", "check create"),                      // T3
    ("Mutation.updateCheck", "check update"),                      // T3
    ("Mutation.deleteCheck", "check delete"),                      // T3
    ("Mutation.setCheckPaused", "check pause and resume"),         // T3
    ("Mutation.setCheckDefaults", "check defaults set"),           // T3
    ("Mutation.setCheckNotifications", "check notifications set"), // T3
    ("Mutation.createMaintenanceWindow", "maintenance create"),    // T3
    ("Mutation.updateMaintenanceWindow", "maintenance update"),    // T3
    ("Mutation.deleteMaintenanceWindow", "maintenance delete"),    // T3
    (
        "Mutation.setManualWindowActive",
        "maintenance start and stop",
    ), // T3
    // T4 follow-up: the other `watch` commands, on the subscription socket of
    // `hub/subscription.rs` (`event watch` is the example).
    ("Subscription.deviceStatusChanged", "device watch"), // T4
    ("Subscription.heartbeatAdded", "check watch"),       // T4
    ("Subscription.sessionChanged", "session watch"),     // T4
    ("Subscription.metricsAdded", "device metrics --watch"), // T4
    ("Subscription.buildJobUpdated", "release log --follow"), // T4
    ("Subscription.vpnChanged", "vpn status --watch"),    // T4
    // T5: release, builder, vpn.
    ("Query.releases", "release list"),                   // T5
    ("Query.nextReleaseVersion", "release next-version"), // T5
    ("Query.builders", "builder list"),                   // T5
    ("Query.buildJobLog", "release log"),                 // T5
    ("Query.vpn", "vpn status"),                          // T5
    ("Mutation.createRelease", "release create"),         // T5
    ("Mutation.cancelBuildJob", "release cancel"),        // T5
    ("Mutation.enrolBuilder", "builder enrol"),           // T5
    ("Mutation.revokeBuilder", "builder revoke"),         // T5
    ("Mutation.createVpnInvite", "vpn invite"),           // T5
    ("Mutation.createRouterKey", "vpn router-key"),       // T5
    ("Mutation.registerVpnNode", "vpn register"),         // T5
    ("Mutation.renewVpnKey", "vpn renew"),                // T5
    ("Mutation.approveSiteRoute", "vpn approve-route"),   // T5
    // T6: session, ssh, files.
    ("Query.sessions", "session list"),              // T6
    ("Mutation.startSession", "ssh <device>"),       // T6
    ("Mutation.endSession", "session end"),          // T6
    ("Mutation.openFiles", "files ls, get and put"), // T6
];
