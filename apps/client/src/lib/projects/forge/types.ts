/*
 * The hub's shapes for a project's work, code and history (PRJ-027 to PRJ-035, OD-E14), written by hand from the draft
 * schema of the second projects slice. They are not in contracts/schema.graphql yet, so codegen cannot type them; when the
 * contract lands, these give way to the generated types and the documents move into `graphql()`.
 *
 * Every string marked "from GitHub" is written by other people: it is shown as plain text and never as HTML or Markdown.
 */

export type RepositoryAccess = 'READY' | 'NOT_CONNECTED' | 'NEEDS_RECONNECT' | 'PAUSED' | 'LEFT_OUT' | 'GONE';

/** When the hub read an answer from GitHub; `stale` is the last good answer after a failed read (PRJ-032). */
export interface ForgeRead {
  readonly at: string;
  readonly stale: boolean;
  readonly problem?: string | null;
}

export type OpenOrClosed = 'OPEN' | 'CLOSED' | 'ALL';
export type ChecksState = 'NONE' | 'PENDING' | 'PASSED' | 'FAILED';

export interface CommitChecks {
  readonly state: ChecksState;
  readonly passed: number;
  readonly failed: number;
  readonly pending: number;
  /** From GitHub. */
  readonly failing: readonly string[];
}

export type MergeMethod = 'MERGE' | 'SQUASH' | 'REBASE';
export type MergeState = 'CLEAN' | 'UNSTABLE' | 'CONFLICT' | 'BLOCKED' | 'BEHIND' | 'DRAFT' | 'UNKNOWN';

export interface PullRequest {
  readonly repository: string;
  readonly number: number;
  /** From GitHub. */
  readonly title: string;
  readonly state: 'OPEN' | 'CLOSED' | 'MERGED';
  readonly draft: boolean;
  readonly authorLogin: string;
  readonly headBranch: string;
  readonly headSha: string;
  readonly baseBranch: string;
  /** Never merged in bulk; merged alone after the operator types its repository (PRO-013). */
  readonly fromFork: boolean;
  readonly headRepository?: string | null;
  readonly labels: readonly string[];
  readonly updatedAt: string;
  readonly url: string;
  readonly mergeability?: { readonly state: MergeState; readonly canMerge: boolean } | null;
  readonly checks?: CommitChecks | null;
}

export interface Issue {
  readonly repository: string;
  readonly number: number;
  readonly title: string;
  readonly open: boolean;
  readonly closeReason?: 'COMPLETED' | 'NOT_PLANNED' | null;
  readonly authorLogin: string;
  readonly labels: readonly string[];
  readonly assignees: readonly string[];
  readonly comments: number;
  readonly updatedAt: string;
  readonly url: string;
  /** Markdown source from GitHub, shown as plain text. */
  readonly body?: string | null;
  readonly bodyCut: boolean;
}

export interface IssueComment {
  readonly id: string;
  readonly authorLogin: string;
  readonly body: string;
  readonly bodyCut: boolean;
  readonly createdAt: string;
  readonly url: string;
}

export interface Workflow {
  readonly id: string;
  readonly name: string;
  readonly path: string;
  readonly active: boolean;
}

export type WorkflowRunStatus = 'QUEUED' | 'IN_PROGRESS' | 'COMPLETED';
export type WorkflowConclusion =
  | 'SUCCESS'
  | 'FAILURE'
  | 'CANCELLED'
  | 'SKIPPED'
  | 'TIMED_OUT'
  | 'ACTION_REQUIRED'
  | 'NEUTRAL'
  | 'STALE'
  | 'OTHER';

export interface WorkflowRun {
  readonly repository: string;
  readonly id: string;
  readonly name: string;
  readonly runNumber: number;
  readonly event: string;
  readonly status: WorkflowRunStatus;
  readonly conclusion?: WorkflowConclusion | null;
  readonly branch?: string | null;
  /** Each confirmation shows it, and a change names it so a moved head is refused (PRJ-034). */
  readonly headSha: string;
  readonly commitTitle?: string | null;
  readonly actorLogin?: string | null;
  readonly fromFork: boolean;
  readonly createdAt: string;
  readonly url: string;
  readonly rerunnable: boolean;
  readonly failedJobsRerunnable: boolean;
  readonly cancellable: boolean;
}

export interface Commit {
  readonly sha: string;
  readonly title: string;
  readonly authorName: string;
  readonly authorLogin?: string | null;
  readonly committedAt: string;
  readonly url: string;
  readonly checks?: CommitChecks | null;
}

export interface Branch {
  readonly name: string;
  readonly sha: string;
  readonly protected: boolean;
  readonly default: boolean;
}

export interface RepositoryRelease {
  readonly id: string;
  readonly tag: string;
  readonly name?: string | null;
  readonly draft: boolean;
  readonly prerelease: boolean;
  readonly publishedAt?: string | null;
  readonly createdAt: string;
  readonly url: string;
}

export type CodeEntryKind = 'FILE' | 'DIRECTORY' | 'SYMLINK' | 'SUBMODULE';

export interface CodeEntry {
  readonly name: string;
  readonly path: string;
  readonly kind: CodeEntryKind;
  readonly size: number;
}

export interface CodeDirectory {
  readonly ref: string;
  readonly path: string;
  readonly entries: readonly CodeEntry[];
  readonly partial: boolean;
  readonly read: ForgeRead;
}

export interface CodeFile {
  readonly path: string;
  readonly name: string;
  readonly size: number;
  /** TEXT holds `text`; BINARY and TOO_LARGE hold nothing to show. */
  readonly content: 'TEXT' | 'BINARY' | 'TOO_LARGE';
  /** Bytes from the repository: plain text, never rendered and never run. */
  readonly text?: string | null;
  readonly url: string;
}

export type MergeOutcome =
  | 'MERGED'
  | 'ALREADY_MERGED'
  | 'HEAD_MOVED'
  | 'NOT_MERGEABLE'
  | 'NOT_READY'
  | 'FORK_REFUSED'
  | 'METHOD_REFUSED'
  | 'NOT_FOUND'
  | 'FAILED'
  | 'UNKNOWN'
  | 'NOT_ATTEMPTED';

export interface PullRequestMerge {
  readonly number: number;
  readonly outcome: MergeOutcome;
  /** Fixed words from the hub, never a provider reply (LOG-007). */
  readonly message: string;
}

/** One page of a list the hub read from GitHub. */
export interface ForgePage<T> {
  readonly rows: readonly T[];
  readonly next: string | null;
  readonly read: ForgeRead;
}

export interface PullRequestPage extends ForgePage<PullRequest> {
  readonly mergeMethods: readonly MergeMethod[];
}
