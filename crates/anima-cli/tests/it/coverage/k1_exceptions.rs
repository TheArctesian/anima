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
    (
        "Query.pullRequests",
        "temporary: the command-line task W5 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.pullRequest",
        "temporary: the command-line task W5 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.issues",
        "temporary: the command-line task W5 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.issue",
        "temporary: the command-line task W5 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.issueComments",
        "temporary: the command-line task W5 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.workflows",
        "temporary: the command-line task W5 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.workflowRuns",
        "temporary: the command-line task W5 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.workflowRun",
        "temporary: the command-line task W5 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.commits",
        "temporary: the command-line task W8 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.branches",
        "temporary: the command-line task W8 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.repositoryReleases",
        "temporary: the command-line task W8 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.codeDirectory",
        "temporary: the command-line task W8 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Query.codeFile",
        "temporary: the command-line task W8 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.createIssue",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.addIssueComment",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.setIssueOpen",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.mergePullRequest",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.mergePullRequests",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.rerunWorkflowRun",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.rerunFailedWorkflowJobs",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.cancelWorkflowRun",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.dispatchWorkflow",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
    (
        "Mutation.setCommitStatus",
        "temporary: the command-line task W13 adds its document and deletes this line (CMD-031)",
    ),
];
