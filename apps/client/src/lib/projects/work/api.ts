import { graphql } from '$lib/hub/generated';
import type { IssueCloseReason } from '$lib/hub/generated/graphql';
import { asked, field, pageOf, paged, type Asker, type PageAsk } from '../forge/request';
import type { MergeMethod, OpenOrClosed, PullRequest, PullRequestPage, WorkflowRun } from '../forge/types';
import type { HubResult } from '$lib/hub/hub';

// The fragments are only read by codegen, which inlines them into the documents that spread them (API-008).
void graphql(`
  fragment PullRequestFields on PullRequest {
    repository
    number
    title
    state
    draft
    authorLogin
    headBranch
    headSha
    baseBranch
    fromFork
    headRepository
    labels
    updatedAt
    url
    mergeability {
      state
      canMerge
    }
    checks {
      ...CommitChecksFields
    }
  }
`);

void graphql(`
  fragment IssueFields on Issue {
    repository
    number
    title
    open
    closeReason
    authorLogin
    labels
    assignees
    comments
    updatedAt
    url
    body
    bodyCut
  }
`);

void graphql(`
  fragment IssueCommentFields on IssueComment {
    id
    authorLogin
    body
    bodyCut
    createdAt
    url
  }
`);

void graphql(`
  fragment WorkflowRunFields on WorkflowRun {
    repository
    id
    name
    runNumber
    event
    status
    conclusion
    branch
    headSha
    commitTitle
    actorLogin
    fromFork
    createdAt
    url
    rerunnable
    failedJobsRerunnable
    cancellable
  }
`);

const PULL_REQUESTS = graphql(`
  query ForgePullRequests($repository: ID!, $state: OpenOrClosed, $after: String, $fresh: Boolean) {
    pullRequests(repository: $repository, state: $state, first: 20, after: $after, fresh: $fresh) {
      edges {
        node {
          ...PullRequestFields
        }
      }
      pageInfo {
        ...ForgePageFields
      }
      read {
        ...ForgeReadFields
      }
      mergeMethods
    }
  }
`);

const ISSUES = graphql(`
  query ForgeIssues($repository: ID!, $state: OpenOrClosed, $after: String, $fresh: Boolean) {
    issues(repository: $repository, state: $state, first: 30, after: $after, fresh: $fresh) {
      edges {
        node {
          ...IssueFields
        }
      }
      pageInfo {
        ...ForgePageFields
      }
      read {
        ...ForgeReadFields
      }
    }
  }
`);

const ISSUE_COMMENTS = graphql(`
  query ForgeIssueComments($repository: ID!, $number: Int!, $after: String, $fresh: Boolean) {
    issueComments(repository: $repository, number: $number, first: 30, after: $after, fresh: $fresh) {
      edges {
        node {
          ...IssueCommentFields
        }
      }
      pageInfo {
        ...ForgePageFields
      }
      read {
        ...ForgeReadFields
      }
    }
  }
`);

const WORKFLOWS = graphql(`
  query ForgeWorkflows($repository: ID!, $fresh: Boolean) {
    workflows(repository: $repository, fresh: $fresh) {
      id
      name
      path
      active
    }
  }
`);

const WORKFLOW_RUNS = graphql(`
  query ForgeWorkflowRuns($repository: ID!, $after: String, $fresh: Boolean) {
    workflowRuns(repository: $repository, first: 20, after: $after, fresh: $fresh) {
      edges {
        node {
          ...WorkflowRunFields
        }
      }
      pageInfo {
        ...ForgePageFields
      }
      read {
        ...ForgeReadFields
      }
    }
  }
`);

const CREATE_ISSUE = graphql(`
  mutation ForgeCreateIssue($input: NewIssueInput!) {
    createIssue(input: $input) {
      ...IssueFields
    }
  }
`);

const ADD_COMMENT = graphql(`
  mutation ForgeAddIssueComment($repository: ID!, $number: Int!, $body: String!) {
    addIssueComment(repository: $repository, number: $number, body: $body) {
      ...IssueCommentFields
    }
  }
`);

const SET_ISSUE_OPEN = graphql(`
  mutation ForgeSetIssueOpen($repository: ID!, $number: Int!, $open: Boolean!, $reason: IssueCloseReason) {
    setIssueOpen(repository: $repository, number: $number, open: $open, reason: $reason) {
      ...IssueFields
    }
  }
`);

const MERGE_PULL_REQUEST = graphql(`
  mutation ForgeMergePullRequest($input: MergePullRequestInput!) {
    mergePullRequest(input: $input) {
      ...PullRequestFields
    }
  }
`);

const MERGE_PULL_REQUESTS = graphql(`
  mutation ForgeMergePullRequests($pullRequests: [PullRequestTargetInput!]!, $method: MergeMethod!) {
    mergePullRequests(pullRequests: $pullRequests, method: $method) {
      number
      outcome
      message
    }
  }
`);

const RERUN = graphql(`
  mutation ForgeRerunWorkflowRun($repository: ID!, $runId: ID!, $expectedHeadSha: String!) {
    rerunWorkflowRun(repository: $repository, runId: $runId, expectedHeadSha: $expectedHeadSha) {
      ...WorkflowRunFields
    }
  }
`);

const RERUN_FAILED = graphql(`
  mutation ForgeRerunFailedWorkflowJobs($repository: ID!, $runId: ID!, $expectedHeadSha: String!) {
    rerunFailedWorkflowJobs(repository: $repository, runId: $runId, expectedHeadSha: $expectedHeadSha) {
      ...WorkflowRunFields
    }
  }
`);

const CANCEL = graphql(`
  mutation ForgeCancelWorkflowRun($repository: ID!, $runId: ID!, $expectedHeadSha: String!) {
    cancelWorkflowRun(repository: $repository, runId: $runId, expectedHeadSha: $expectedHeadSha) {
      ...WorkflowRunFields
    }
  }
`);

const DISPATCH = graphql(`
  mutation ForgeDispatchWorkflow($input: DispatchWorkflowInput!) {
    dispatchWorkflow(input: $input) {
      workflowId
      branch
      acceptedAt
    }
  }
`);

export type RunChange = 'rerun' | 'rerunFailed' | 'cancel';

/** The reads of the Work tab: pull requests, issues and runs, all open to a read key (PRJ-031, PRJ-032). */
function workReads(hub: Asker) {
  return {
    async pullRequests(repository: string, state: OpenOrClosed, ask: PageAsk = {}): Promise<HubResult<PullRequestPage>> {
      const result = await field(hub, PULL_REQUESTS, 'pullRequests', { ...asked(repository, ask), state });
      return result.ok ? { ok: true, value: { ...pageOf(result.value), mergeMethods: result.value.mergeMethods } } : result;
    },
    issues: async (repository: string, state: OpenOrClosed, ask: PageAsk = {}) =>
      paged(await field(hub, ISSUES, 'issues', { ...asked(repository, ask), state })),
    issueComments: async (repository: string, number: number, ask: PageAsk = {}) =>
      paged(await field(hub, ISSUE_COMMENTS, 'issueComments', { ...asked(repository, ask), number })),
    workflows: (repository: string, fresh = false) => field(hub, WORKFLOWS, 'workflows', { repository, fresh }),
    workflowRuns: async (repository: string, ask: PageAsk = {}) => paged(await field(hub, WORKFLOW_RUNS, 'workflowRuns', asked(repository, ask))),
  };
}

/** The changes; each names what the operator saw, so a moved head is refused (PRJ-028, PRJ-033, PRJ-034). */
function workChanges(hub: Asker) {
  return {
    createIssue: (repository: string, title: string, body: string) =>
      field(hub, CREATE_ISSUE, 'createIssue', { input: { repository, title, body: body === '' ? null : body } }),
    addComment: (repository: string, number: number, body: string) => field(hub, ADD_COMMENT, 'addIssueComment', { repository, number, body }),
    setIssueOpen: (repository: string, number: number, open: boolean, reason: IssueCloseReason | null = null) =>
      field(hub, SET_ISSUE_OPEN, 'setIssueOpen', { repository, number, open, reason: open ? null : reason }),
    merge: (pull: PullRequest, method: MergeMethod, options: { allowFailingChecks?: boolean; forkConfirmed?: string | null } = {}) =>
      field(hub, MERGE_PULL_REQUEST, 'mergePullRequest', {
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
      field(hub, MERGE_PULL_REQUESTS, 'mergePullRequests', {
        pullRequests: pulls.map((pull) => ({ repository: pull.repository, number: pull.number, expectedHeadSha: pull.headSha })),
        method,
      }),
    runChange(change: RunChange, target: WorkflowRun): Promise<HubResult<WorkflowRun>> {
      const variables = { repository: target.repository, runId: target.id, expectedHeadSha: target.headSha };
      if (change === 'rerun') return field(hub, RERUN, 'rerunWorkflowRun', variables);
      if (change === 'rerunFailed') return field(hub, RERUN_FAILED, 'rerunFailedWorkflowJobs', variables);
      return field(hub, CANCEL, 'cancelWorkflowRun', variables);
    },
    dispatch: (repository: string, workflowId: string, branch: string) =>
      field(hub, DISPATCH, 'dispatchWorkflow', { input: { repository, workflowId, branch, inputs: [] } }),
  };
}

/** The hub's calls for the Work tab (PRJ-027, PRJ-028, PRJ-033, PRJ-034). */
export const workApi = (hub: Asker) => ({ ...workReads(hub), ...workChanges(hub) });
