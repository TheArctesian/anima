//! The K1 root fields the command line never uses, each with its reason (CMD-031). A builder who
//! adds a document for a field deletes its entry here; the coverage test fails until they do,
//! and for any K1 root field that is neither used nor listed.

/// The operational keys have no command yet: K-E5 Section 2.2 names `anima key`, the API key noun.
const NO_KEY_NOUN: &str = "no command yet: K-E5 Section 2.2 names the noun `anima key`, which is \
    already the API key noun (INT-003, `key list|create|revoke`); the noun waits for a ruling \
    (E0-6 T3)";

/// The project groups of task G1 have their command and MCP tools in task G2, the next branch.
const GROUPS_IN_G2: &str = "no command yet: `anima project group` comes with task G2 of \
    projects-and-builds-plan.md (PRO-059 to PRO-063)";

pub const EXCEPTIONS: &[(&str, &str)] = &[
    ("Query.projectGroups", GROUPS_IN_G2),
    ("Mutation.createProjectGroup", GROUPS_IN_G2),
    ("Mutation.renameProjectGroup", GROUPS_IN_G2),
    ("Mutation.deleteProjectGroup", GROUPS_IN_G2),
    ("Mutation.setProjectsGroup", GROUPS_IN_G2),
    (
        "Mutation.hostedDeviceInvite",
        "the Anima.ing cloud only: takes the cloud key, a secret (R-129)",
    ),
    (
        "Mutation.hostedBrowserTicket",
        "the Anima.ing cloud only: takes the cloud key, a secret (HMD-061)",
    ),
    (
        "Mutation.hostedBrowserSignIn",
        "the gateway of the pool only: sets the __Host-anima cookie for a browser (HMD-061)",
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
    ("Query.operationalKeys", NO_KEY_NOUN),
    ("Query.operationalKey", NO_KEY_NOUN),
    ("Mutation.createOperationalKey", NO_KEY_NOUN),
    ("Mutation.updateOperationalKey", NO_KEY_NOUN),
    ("Mutation.removeOperationalKey", NO_KEY_NOUN),
    ("Mutation.rotateOperationalKey", NO_KEY_NOUN),
    ("Mutation.revokeOperationalKey", NO_KEY_NOUN),
    ("Mutation.grantOperationalKey", NO_KEY_NOUN),
    ("Mutation.ungrantOperationalKey", NO_KEY_NOUN),
];
