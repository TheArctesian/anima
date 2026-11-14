import type { Hub, HubResult } from '$lib/hub/hub';
import * as D from './documents';
import type {
  Branch,
  CodeDirectory,
  CodeFile,
  Commit,
  ForgePage,
  ForgeRead,
  Issue,
  IssueComment,
  MergeMethod,
  OpenOrClosed,
  PullRequest,
  PullRequestMerge,
  PullRequestPage,
  RepositoryAccess,
  RepositoryRelease,
  Workflow,
  WorkflowRun,
} from './types';

type Vars = Record<string, unknown>;
interface Connection<T> {
  edges: { node: T }[];
  pageInfo: { hasNextPage: boolean; endCursor: string | null };
  read: ForgeRead;
}

/** Asks the hub for `field` of `document` and gives back that field's value. */
async function field<T>(hub: Pick<Hub, 'request'>, document: D.ForgeDocument, name: string, variables: Vars): Promise<HubResult<T>> {
  const result = await hub.request(document, variables);
  return result.ok ? { ok: true, value: result.value[name] as T } : result;
}

/** One page of a list: its rows, the cursor of the next page, and when the hub read it. */
async function page<T>(hub: Pick<Hub, 'request'>, document: D.ForgeDocument, name: string, variables: Vars): Promise<HubResult<ForgePage<T>>> {
  const result = await field<Connection<T>>(hub, document, name, variables);
  if (!result.ok) return result;
  const { edges, pageInfo, read } = result.value;
  return { ok: true, value: { rows: edges.map((edge) => edge.node), next: pageInfo.hasNextPage ? pageInfo.endCursor : null, read } };
}

/** What a list call is asked: the next page's cursor, and `fresh` to skip the hub's short cache (PRJ-032). */
export interface PageAsk {
  readonly after?: string | null;
  readonly fresh?: boolean;
}

const asked = (repository: string, ask: PageAsk) => ({ repository, after: ask.after ?? null, fresh: ask.fresh ?? false });

/** The reads, for one repository at a time; all of them are open to a read key except `file` (PRJ-031, PRJ-032). */
function forgeReads(hub: Pick<Hub, 'request'>) {
  return {
    async access(slug: string): Promise<HubResult<ReadonlyMap<string, RepositoryAccess>>> {
      const result = await field<{ repositories: { id: string; access: RepositoryAccess }[] } | null>(hub, D.PROJECT_ACCESS, 'project', { slug });
      if (!result.ok) return result;
      return { ok: true, value: new Map((result.value?.repositories ?? []).map((each) => [each.id, each.access])) };
    },
    async pullRequests(repository: string, state: OpenOrClosed, ask: PageAsk = {}): Promise<HubResult<PullRequestPage>> {
      const result = await field<Connection<PullRequest> & { mergeMethods: MergeMethod[] }>(hub, D.PULL_REQUESTS, 'pullRequests', { ...asked(repository, ask), state });
      if (!result.ok) return result;
      const { edges, pageInfo, read, mergeMethods } = result.value;
      return { ok: true, value: { rows: edges.map((edge) => edge.node), next: pageInfo.hasNextPage ? pageInfo.endCursor : null, read, mergeMethods } };
    },
    issues: (repository: string, state: OpenOrClosed, ask: PageAsk = {}) =>
      page<Issue>(hub, D.ISSUES, 'issues', { ...asked(repository, ask), state }),
    issueComments: (repository: string, number: number, ask: PageAsk = {}) =>
      page<IssueComment>(hub, D.ISSUE_COMMENTS, 'issueComments', { ...asked(repository, ask), number }),
    workflows: (repository: string, fresh = false) => field<Workflow[]>(hub, D.WORKFLOWS, 'workflows', { repository, fresh }),
    workflowRuns: (repository: string, ask: PageAsk = {}) => page<WorkflowRun>(hub, D.WORKFLOW_RUNS, 'workflowRuns', asked(repository, ask)),
    commits: (repository: string, ref: string | null, ask: PageAsk = {}) =>
      page<Commit>(hub, D.COMMITS, 'commits', { ...asked(repository, ask), ref }),
    branches: (repository: string, ask: PageAsk = {}) => page<Branch>(hub, D.BRANCHES, 'branches', asked(repository, ask)),
    releases: (repository: string, ask: PageAsk = {}) =>
      page<RepositoryRelease>(hub, D.RELEASES, 'repositoryReleases', asked(repository, ask)),
    directory: (repository: string, ref: string | null, path: string, fresh = false) =>
      field<CodeDirectory>(hub, D.CODE_DIRECTORY, 'codeDirectory', { repository, ref, path, fresh }),
    file: (repository: string, ref: string | null, path: string) => field<CodeFile>(hub, D.CODE_FILE, 'codeFile', { repository, ref, path }),
  };
}

/** The changes on GitHub; each names what the operator saw, so a moved head is refused (PRJ-028, PRJ-033, PRJ-034). */
function forgeChanges(hub: Pick<Hub, 'request'>) {
  return {
    createIssue: (repository: string, title: string, body: string) =>
      field<Issue>(hub, D.CREATE_ISSUE, 'createIssue', { input: { repository, title, body: body === '' ? null : body } }),
    addComment: (repository: string, number: number, body: string) =>
      field<IssueComment>(hub, D.ADD_COMMENT, 'addIssueComment', { repository, number, body }),
    setIssueOpen: (repository: string, number: number, open: boolean, reason: 'COMPLETED' | 'NOT_PLANNED' | null = null) =>
      field<Issue>(hub, D.SET_ISSUE_OPEN, 'setIssueOpen', { repository, number, open, reason: open ? null : reason }),
    merge: (pull: PullRequest, method: MergeMethod, options: { allowFailingChecks?: boolean; forkConfirmed?: string | null } = {}) =>
      field<PullRequest>(hub, D.MERGE_PULL_REQUEST, 'mergePullRequest', {
        input: {
          repository: pull.repository,
          number: pull.number,
          expectedHeadSha: pull.headSha,
          method,
          allowFailingChecks: options.allowFailingChecks ?? false,
          forkConfirmed: options.forkConfirmed ?? null,
        },
      }),
    mergeMany: (pulls: readonly PullRequest[], method: MergeMethod) =>
      field<PullRequestMerge[]>(hub, D.MERGE_PULL_REQUESTS, 'mergePullRequests', {
        pullRequests: pulls.map((pull) => ({ repository: pull.repository, number: pull.number, expectedHeadSha: pull.headSha })),
        method,
      }),
    runChange: (change: RunChange, run: WorkflowRun) =>
      field<WorkflowRun>(hub, RUN_CHANGES[change].document, RUN_CHANGES[change].field, {
        repository: run.repository,
        runId: run.id,
        expectedHeadSha: run.headSha,
      }),
    dispatch: (repository: string, workflowId: string, branch: string) =>
      field<{ workflowId: string; branch: string; acceptedAt: string }>(hub, D.DISPATCH, 'dispatchWorkflow', {
        input: { repository, workflowId, branch, inputs: [] },
      }),
  };
}

/** The hub's calls for a project's work, code and history (PRJ-027 to PRJ-035). */
export function forgeApi(hub: Pick<Hub, 'request'>) {
  return { ...forgeReads(hub), ...forgeChanges(hub) };
}

export type ForgeApi = ReturnType<typeof forgeApi>;
export type RunChange = 'rerun' | 'rerunFailed' | 'cancel';

const RUN_CHANGES: Record<RunChange, { document: D.ForgeDocument; field: string }> = {
  rerun: { document: D.RERUN, field: 'rerunWorkflowRun' },
  rerunFailed: { document: D.RERUN_FAILED, field: 'rerunFailedWorkflowJobs' },
  cancel: { document: D.CANCEL, field: 'cancelWorkflowRun' },
};
