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
    // T4: event, audit, notify, retention, key, and the watch commands.
    ("Query.events", "event list"),                        // T4
    ("Query.audit", "audit list"),                         // T4
    ("Query.apiKeys", "key list"),                         // T4
    ("Query.notificationChannels", "notify channel list"), // T4
    ("Query.notificationSettings", "notify settings get"), // T4
    ("Query.deliveries", "notify deliveries"),             // T4
    ("Query.retention", "retention list"),                 // T4
    ("Mutation.createApiKey", "key create"),               // T4
    ("Mutation.revokeApiKey", "key revoke"),               // T4
    (
        "Mutation.createNotificationChannel",
        "notify channel create",
    ), // T4
    (
        "Mutation.updateNotificationChannel",
        "notify channel update",
    ), // T4
    (
        "Mutation.deleteNotificationChannel",
        "notify channel delete",
    ), // T4
    ("Mutation.sendTestNotification", "notify channel test"), // T4
    ("Mutation.setNotificationSettings", "notify settings set"), // T4
    ("Mutation.setRetention", "retention set"),            // T4
    ("Subscription.deviceStatusChanged", "device watch"),  // T4
    ("Subscription.heartbeatAdded", "check watch"),        // T4
    ("Subscription.eventAdded", "event watch"),            // T4
    ("Subscription.sessionChanged", "session watch"),      // T4
    ("Subscription.metricsAdded", "device metrics --watch"), // T4
    ("Subscription.buildJobUpdated", "release log --follow"), // T4
    ("Subscription.vpnChanged", "vpn status --watch"),     // T4
    // T6: session, ssh, files.
    ("Query.sessions", "session list"),              // T6
    ("Mutation.startSession", "ssh <device>"),       // T6
    ("Mutation.endSession", "session end"),          // T6
    ("Mutation.openFiles", "files ls, get and put"), // T6
];
