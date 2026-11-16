import type {
  CommitChecksFieldsFragment,
  CommitFieldsFragment,
  ForgeBranchesQuery,
  ForgeCodeDirectoryQuery,
  ForgeCodeFileQuery,
  ForgeMergePullRequestsMutation,
  ForgeReadFieldsFragment,
  ForgeReleasesQuery,
  ForgeWorkflowsQuery,
  IssueCommentFieldsFragment,
  IssueFieldsFragment,
  MergeMethod,
  PullRequestFieldsFragment,
  WorkflowRunFieldsFragment,
} from '$lib/hub/generated/graphql';

/*
 * The hub's shapes for a project's work, code and history (PRJ-027 to PRJ-035, OD-E14), as codegen writes them from the
 * documents in work/, code/ and history/ (API-008).
 *
 * Every string from GitHub is written by other people: it is shown as plain text and never as HTML or Markdown.
 */

export type {
  ChecksState,
  CodeEntryKind,
  MergeMethod,
  MergeOutcome,
  MergeState,
  OpenOrClosed,
  RepositoryAccess,
  WorkflowConclusion,
  WorkflowRunStatus,
} from '$lib/hub/generated/graphql';

/** When the hub read an answer from GitHub; `stale` is the last good answer after a failed read (PRJ-032). */
export type ForgeRead = ForgeReadFieldsFragment;
export type CommitChecks = CommitChecksFieldsFragment;
/** A fork's pull request is never merged in bulk; it merges alone after the operator types its repository (PRO-013). */
export type PullRequest = PullRequestFieldsFragment;
/** `body` is Markdown source from GitHub, shown as plain text. */
export type Issue = IssueFieldsFragment;
export type IssueComment = IssueCommentFieldsFragment;
export type Workflow = ForgeWorkflowsQuery['workflows'][number];
/** Each confirmation shows `headSha`, and a change names it so a moved head is refused (PRJ-034). */
export type WorkflowRun = WorkflowRunFieldsFragment;
export type Commit = CommitFieldsFragment;
export type Branch = ForgeBranchesQuery['branches']['edges'][number]['node'];
export type RepositoryRelease = ForgeReleasesQuery['repositoryReleases']['edges'][number]['node'];
export type CodeDirectory = ForgeCodeDirectoryQuery['codeDirectory'];
export type CodeEntry = CodeDirectory['entries'][number];
/** TEXT holds `text`, bytes from the repository that are never rendered and never run; BINARY and TOO_LARGE hold nothing. */
export type CodeFile = ForgeCodeFileQuery['codeFile'];
/** `message` is fixed words from the hub, never a provider reply (LOG-007). */
export type PullRequestMerge = ForgeMergePullRequestsMutation['mergePullRequests'][number];

/** One page of a list the hub read from GitHub. */
export interface ForgePage<T> {
  readonly rows: readonly T[];
  readonly next: string | null;
  readonly read: ForgeRead;
}

export interface PullRequestPage extends ForgePage<PullRequest> {
  readonly mergeMethods: readonly MergeMethod[];
}
