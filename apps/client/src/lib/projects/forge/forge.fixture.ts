import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { Branch, CodeEntry, Commit, Issue, IssueComment, PullRequest, RepositoryAccess, WorkflowRun } from './types';

// Test support: a hub that answers the draft work, code and history calls (PRJ-027 to PRJ-035) from memory.

export const NOW = new Date('2026-11-14T12:00:00Z');
const at = (minutes: number) => new Date(NOW.getTime() - minutes * 60_000).toISOString();
const SHA = (digit: string) => digit.repeat(40);
const READ = { at: at(0), stale: false, problem: null };

export function pull(number: number, more: Partial<PullRequest> = {}): PullRequest {
  return {
    repository: 'r1',
    number,
    title: `Pull request ${String(number)}`,
    state: 'OPEN',
    draft: false,
    authorLogin: 'octo',
    headBranch: `feature-${String(number)}`,
    headSha: SHA(String(number % 10)),
    baseBranch: 'main',
    fromFork: false,
    headRepository: 'acme/acme-site',
    labels: [],
    updatedAt: at(number),
    url: `https://github.com/acme/acme-site/pull/${String(number)}`,
    mergeability: { state: 'CLEAN', canMerge: true },
    checks: { state: 'PASSED', passed: 3, failed: 0, pending: 0, failing: [] },
    ...more,
  };
}

export function issue(number: number, more: Partial<Issue> = {}): Issue {
  return {
    repository: 'r1',
    number,
    title: `Issue ${String(number)}`,
    open: true,
    closeReason: null,
    authorLogin: 'octo',
    labels: [],
    assignees: [],
    comments: 0,
    updatedAt: at(number),
    url: `https://github.com/acme/acme-site/issues/${String(number)}`,
    body: null,
    bodyCut: false,
    ...more,
  };
}

export function run(id: string, more: Partial<WorkflowRun> = {}): WorkflowRun {
  return {
    repository: 'r1',
    id,
    name: 'CI',
    runNumber: Number(id),
    event: 'push',
    status: 'COMPLETED',
    conclusion: 'SUCCESS',
    branch: 'main',
    headSha: SHA('a'),
    commitTitle: 'Pin traefik to 3.1',
    actorLogin: 'octo',
    fromFork: false,
    createdAt: at(10),
    url: `https://github.com/acme/acme-site/actions/runs/${id}`,
    rerunnable: true,
    failedJobsRerunnable: false,
    cancellable: false,
    ...more,
  };
}

export function commit(sha: string, minutes: number, more: Partial<Commit> = {}): Commit {
  return {
    sha: SHA(sha),
    title: `Commit ${sha}`,
    authorName: 'Octo Cat',
    authorLogin: 'octo',
    committedAt: at(minutes),
    url: `https://github.com/acme/acme-site/commit/${SHA(sha)}`,
    checks: { state: 'PASSED', passed: 2, failed: 0, pending: 0, failing: [] },
    ...more,
  };
}

const entry = (path: string, kind: CodeEntry['kind'], size = 0): CodeEntry => ({ name: path.split('/').at(-1) ?? path, path, kind, size });

export interface FakeRepository {
  access: RepositoryAccess;
  pulls: PullRequest[];
  issues: Issue[];
  comments: IssueComment[];
  runs: WorkflowRun[];
  commits: Commit[];
  branches: Branch[];
}

export function fakeRepository(): FakeRepository {
  return {
    access: 'READY',
    pulls: [pull(12), pull(11, { mergeability: { state: 'UNSTABLE', canMerge: false }, checks: { state: 'FAILED', passed: 2, failed: 1, pending: 0, failing: ['lint'] } }), pull(10, { fromFork: true, headRepository: 'stranger/acme-site' })],
    issues: [issue(5, { body: '<b>Logs</b> do not scroll', comments: 1 }), issue(4), issue(3, { open: false, closeReason: 'COMPLETED' })],
    comments: [{ id: 'c1', authorLogin: 'octo', body: 'Seen on **Safari**', bodyCut: false, createdAt: at(30), url: 'https://github.com/acme/acme-site/issues/5#c1' }],
    runs: [run('900', { status: 'IN_PROGRESS', conclusion: null, rerunnable: false, cancellable: true }), run('899', { conclusion: 'FAILURE', failedJobsRerunnable: true }), run('898')],
    commits: [commit('a', 30), commit('b', 60 * 26, { checks: { state: 'FAILED', passed: 1, failed: 1, pending: 0, failing: ['test'] } })],
    branches: [{ name: 'main', sha: SHA('a'), protected: true, default: true }, { name: 'feature-12', sha: SHA('2'), protected: false, default: false }],
  };
}

const TREE: Record<string, CodeEntry[]> = {
  '': [entry('src', 'DIRECTORY'), entry('README.md', 'FILE', 31), entry('logo.png', 'FILE', 2048)],
  src: [entry('src/main.ts', 'FILE', 22)],
};
const FILES: Record<string, { content: 'TEXT' | 'BINARY'; text: string | null }> = {
  'README.md': { content: 'TEXT', text: '# Acme\n<script>alert(1)</script>' },
  'src/main.ts': { content: 'TEXT', text: 'export const answer = 42;' },
  'logo.png': { content: 'BINARY', text: null },
};

export interface FakeForgeHub extends Pick<Hub, 'request'> {
  readonly data: FakeRepository;
  /** Each call as `Operation` with its variables, in order. */
  readonly calls: { name: string; variables: Record<string, unknown> }[];
  /** Operation names that answer with `failure`. */
  readonly failing: Set<string>;
  failure: HubError;
}

const connection = <T>(rows: readonly T[], more: object = {}) => ({
  edges: rows.map((node) => ({ cursor: 'x', node })),
  pageInfo: { hasNextPage: false, endCursor: null },
  read: READ,
  ...more,
});

export function fakeForgeHub(data: FakeRepository = fakeRepository()): FakeForgeHub {
  const hub: FakeForgeHub = {
    data,
    calls: [],
    failing: new Set(),
    failure: { code: 'UNAVAILABLE', message: 'GitHub could not be reached.' },
    request: <R, V extends Record<string, unknown>>(document: TypedDocumentNode<R, V>, variables: V) => {
      const name = (document.definitions[0] as { name?: { value: string } }).name?.value ?? '';
      hub.calls.push({ name, variables });
      if (hub.failing.has(name)) return Promise.resolve({ ok: false, error: hub.failure } as HubResult<R>);
      return Promise.resolve({ ok: true, value: answer(hub.data, name, variables) } as HubResult<R>);
    },
  };
  return hub;
}

function answer(data: FakeRepository, name: string, variables: Record<string, unknown>): unknown {
  const v = variables as Record<string, never>;
  const state = v['state'] as string | undefined;
  switch (name) {
    case 'ProjectAccess':
      return { project: { repositories: [{ id: 'r1', access: data.access }] } };
    case 'ForgePullRequests':
      return { pullRequests: connection(data.pulls.filter((each) => (state === 'CLOSED' ? each.state !== 'OPEN' : each.state === 'OPEN')), { mergeMethods: ['MERGE', 'SQUASH'] }) };
    case 'ForgeIssues':
      return { issues: connection(data.issues.filter((each) => (state === 'CLOSED' ? !each.open : each.open))) };
    case 'ForgeIssueComments':
      return { issueComments: connection(data.comments) };
    case 'ForgeWorkflows':
      return { workflows: [{ id: 'w1', name: 'CI', path: '.github/workflows/ci.yml', active: true }, { id: 'w2', name: 'Deploy', path: '.github/workflows/deploy.yml', active: true }] };
    case 'ForgeWorkflowRuns':
      return { workflowRuns: connection(data.runs) };
    case 'ForgeCommits':
      return { commits: connection(data.commits) };
    case 'ForgeBranches':
      return { branches: connection(data.branches) };
    case 'ForgeReleases':
      return { repositoryReleases: connection([{ id: 'v1', tag: 'v1.0.0', name: 'First', draft: false, prerelease: false, publishedAt: at(60 * 50), createdAt: at(60 * 50), url: 'https://github.com/acme/acme-site/releases/v1.0.0' }]) };
    default:
      return name.startsWith('ForgeCode') ? code(name, v) : change(data, name, v);
  }
}

function code(name: string, v: Record<string, never>): unknown {
  switch (name) {
    case 'ForgeCodeDirectory': {
      const path = String(v['path'] ?? '');
      return { codeDirectory: { ref: v['ref'] ?? 'main', path, entries: TREE[path] ?? [], partial: false, read: READ } };
    }
    case 'ForgeCodeFile': {
      const path = String(v['path']);
      const file = FILES[path] ?? { content: 'TOO_LARGE', text: null };
      return { codeFile: { path, name: path.split('/').at(-1), size: 31, ...file, url: `https://github.com/acme/acme-site/blob/main/${path}` } };
    }
    default:
      throw new Error(`fake forge hub: no ${name}`);
  }
}

function change(data: FakeRepository, name: string, v: Record<string, never>): unknown {
  switch (name) {
    case 'ForgeCreateIssue': {
      const input = v['input'] as { title: string; body: string | null };
      const made = issue(6, { title: input.title, body: input.body });
      data.issues = [made, ...data.issues];
      return { createIssue: made };
    }
    case 'ForgeAddIssueComment': {
      const made = { id: 'c2', authorLogin: 'me', body: String(v['body']), bodyCut: false, createdAt: at(0), url: 'https://github.com/x#c2' };
      data.comments = [...data.comments, made];
      return { addIssueComment: made };
    }
    case 'ForgeSetIssueOpen': {
      const changed = { ...(data.issues.find((each) => each.number === v['number']) as Issue), open: Boolean(v['open']) };
      data.issues = data.issues.map((each) => (each.number === changed.number ? changed : each));
      return { setIssueOpen: changed };
    }
    case 'ForgeMergePullRequest': {
      const input = v['input'] as { number: number };
      const merged = { ...(data.pulls.find((each) => each.number === input.number) as PullRequest), state: 'MERGED' as const };
      data.pulls = data.pulls.map((each) => (each.number === merged.number ? merged : each));
      return { mergePullRequest: merged };
    }
    case 'ForgeMergePullRequests': {
      const targets = v['pullRequests'] as { number: number }[];
      data.pulls = data.pulls.map((each) => (targets.some((target) => target.number === each.number) ? { ...each, state: 'MERGED' } : each));
      return { mergePullRequests: targets.map((target) => ({ number: target.number, outcome: 'MERGED', message: 'Merged.' })) };
    }
    case 'ForgeRerunWorkflowRun':
    case 'ForgeRerunFailedWorkflowJobs':
    case 'ForgeCancelWorkflowRun': {
      const cancel = name === 'ForgeCancelWorkflowRun';
      const was = data.runs.find((each) => each.id === v['runId']) as WorkflowRun;
      const next: WorkflowRun = cancel
        ? { ...was, status: 'COMPLETED', conclusion: 'CANCELLED', cancellable: false, rerunnable: true }
        : { ...was, status: 'QUEUED', conclusion: null, cancellable: true, rerunnable: false, failedJobsRerunnable: false };
      data.runs = data.runs.map((each) => (each.id === next.id ? next : each));
      const field = cancel ? 'cancelWorkflowRun' : name === 'ForgeRerunWorkflowRun' ? 'rerunWorkflowRun' : 'rerunFailedWorkflowJobs';
      return { [field]: next };
    }
    case 'ForgeDispatchWorkflow': {
      const input = v['input'] as { workflowId: string; branch: string };
      return { dispatchWorkflow: { workflowId: input.workflowId, branch: input.branch, acceptedAt: at(0) } };
    }
    default:
      throw new Error(`fake forge hub: no ${name}`);
  }
}
