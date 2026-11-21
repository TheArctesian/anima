//! The root fields that no API key may select (D-23).

/// The root fields an API key may never select, in any operation (D-23, WP-43 A-6): managing
/// keys, signing a browser in or out, sessions, files and device credentials. `logout` would
/// delete the key, which is managing it. `enrolDaemon` and `revokeDaemonToken` are refused by
/// name before their resolvers exist (R-084, CR-8). A key may not pin a device identity, or
/// move or delete a device: moving a device and accepting the new host key would send its
/// stored password to another host (R-100). Enrolling or revoking a builder, and publishing or
/// cancelling a release, are supply-chain actions only a login or browser token may take: a
/// builder token outlives the key that enrolled it (D-23, R-107). A VPN key, a registration or a
/// renewal admits a computer to the network: login or browser token only (VP-7, VP-10). Switching an
/// extension on starts requests to outside services: login or browser token only (EXT-011).
/// Connecting a source takes its private key, and removing one can delete what it imported:
/// login or browser token only (PRJ-018, ITG-004). Connecting GitHub from the hub makes and keeps
/// an App key, and its one-time link binds a browser: login or browser token only (PRJ-039,
/// PRJ-041). A file of a repository's code can hold a secret the hub does not look for: login or
/// browser token only (PRJ-033, K-E1 Section 13.5).
/// Running a workflow again, starting one or cancelling one makes code run with a repository's
/// secrets or stops it, and a commit status can satisfy a required check: login or browser
/// token only (PRJ-034, PRJ-035, PRO-010, K-E1 Section 13.5).
/// A pull request merge runs code with the repository's secrets: login or browser token only.
/// A link invite admits a device to the hub, and a revocation is a trust decision: login or
/// browser token only (FRN-009, OR-S6-3). An operational key change takes a secret or is a
/// trust decision (a grant): login or browser token only (R-147, K-E5 Section 2.2).
/// The log of a container can hold a secret, as private code can, and so can the command or a
/// label in its details: login or browser token only (HOST-001, HOST-017, MCP-004, R-149, R-150).
pub(in crate::fleet::access) const API_KEY_REFUSED_ROOTS: [&str; 59] = [
    "createApiKey",
    "revokeApiKey",
    "apiKeys",
    "signInBrowser",
    "logout",
    "startSession",
    "openFiles",
    "setSshCredential",
    "clearSshCredential",
    "setRdpCredential",
    "clearRdpCredential",
    "setVncCredential",
    "clearVncCredential",
    "enrolDaemon",
    "revokeDaemonToken",
    "acceptHostKey",
    "acceptServerCertificate",
    "acceptWebCertificate",
    "updateDevice",
    "deleteDevice",
    "setRetention",
    "enrolBuilder",
    "revokeBuilder",
    "createRelease",
    "cancelBuildJob",
    "createVpnInvite",
    "createRouterKey",
    "registerVpnNode",
    "renewVpnKey",
    "approveSiteRoute",
    "setExtension",
    "connectGithubApp",
    "removeSourceAccount",
    "startGithubConnect",
    "openGithubConnect",
    "cancelGithubConnect",
    "codeFile",
    "rerunWorkflowRun",
    "rerunFailedWorkflowJobs",
    "cancelWorkflowRun",
    "dispatchWorkflow",
    "setCommitStatus",
    "mergePullRequest",
    "mergePullRequests",
    "createLinkInvite",
    "revokeLinkDevice",
    "createOperationalKey",
    "updateOperationalKey",
    "removeOperationalKey",
    "rotateOperationalKey",
    "revokeOperationalKey",
    "grantOperationalKey",
    "ungrantOperationalKey",
    "containerLog",
    "containerDetail",
    "createLocationIngestToken",
    "connectDawarich",
    // The approved model sources: only a person changes them (R-NEW-modellibrary).
    "addApprovedModelSource",
    "setModelPublisherApproval",
];
