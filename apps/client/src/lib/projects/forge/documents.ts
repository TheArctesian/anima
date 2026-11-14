import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { parse } from 'graphql';

/*
 * The documents of the draft second projects slice. They are parsed here, not written with `graphql()`, because codegen
 * checks every `graphql()` document against contracts/schema.graphql and these fields are not in it yet. When the contract
 * lands they move into `graphql()` and take the generated types (API-008).
 */
export type ForgeDocument = TypedDocumentNode<Record<string, unknown>, Record<string, unknown>>;
export const doc = (source: string): ForgeDocument => parse(source) as ForgeDocument;

const READ = 'read { at stale problem }';
const PAGE = 'pageInfo { hasNextPage endCursor }';
const CHECKS = 'checks { state passed failed pending failing }';
const PULL = `repository number title state draft authorLogin headBranch headSha baseBranch fromFork headRepository labels updatedAt url`;
const ISSUE = 'repository number title open closeReason authorLogin labels assignees comments updatedAt url body bodyCut';
const RUN = `repository id name runNumber event status conclusion branch headSha commitTitle actorLogin fromFork createdAt url
  rerunnable failedJobsRerunnable cancellable`;

export const PROJECT_ACCESS = doc(`query ProjectAccess($slug: String!) { project(slug: $slug) { repositories { id access } } }`);

export const PULL_REQUESTS = doc(`
  query ForgePullRequests($repository: ID!, $state: OpenOrClosed, $after: String, $fresh: Boolean) {
    pullRequests(repository: $repository, state: $state, first: 20, after: $after, fresh: $fresh) {
      edges { node { ${PULL} mergeability { state canMerge } ${CHECKS} } } ${PAGE} ${READ} mergeMethods
    }
  }`);

export const ISSUES = doc(`
  query ForgeIssues($repository: ID!, $state: OpenOrClosed, $after: String, $fresh: Boolean) {
    issues(repository: $repository, state: $state, first: 30, after: $after, fresh: $fresh) {
      edges { node { ${ISSUE} } } ${PAGE} ${READ}
    }
  }`);

export const ISSUE_COMMENTS = doc(`
  query ForgeIssueComments($repository: ID!, $number: Int!, $after: String, $fresh: Boolean) {
    issueComments(repository: $repository, number: $number, first: 30, after: $after, fresh: $fresh) {
      edges { node { id authorLogin body bodyCut createdAt url } } ${PAGE} ${READ}
    }
  }`);

export const WORKFLOWS = doc(`
  query ForgeWorkflows($repository: ID!, $fresh: Boolean) { workflows(repository: $repository, fresh: $fresh) { id name path active } }`);

export const WORKFLOW_RUNS = doc(`
  query ForgeWorkflowRuns($repository: ID!, $after: String, $fresh: Boolean) {
    workflowRuns(repository: $repository, first: 20, after: $after, fresh: $fresh) { edges { node { ${RUN} } } ${PAGE} ${READ} }
  }`);

export const COMMITS = doc(`
  query ForgeCommits($repository: ID!, $ref: String, $after: String, $fresh: Boolean) {
    commits(repository: $repository, ref: $ref, first: 30, after: $after, fresh: $fresh) {
      edges { node { sha title authorName authorLogin committedAt url ${CHECKS} } } ${PAGE} ${READ}
    }
  }`);

export const BRANCHES = doc(`
  query ForgeBranches($repository: ID!, $after: String, $fresh: Boolean) {
    branches(repository: $repository, first: 100, after: $after, fresh: $fresh) {
      edges { node { name sha protected default } } ${PAGE} ${READ}
    }
  }`);

export const RELEASES = doc(`
  query ForgeReleases($repository: ID!, $after: String, $fresh: Boolean) {
    repositoryReleases(repository: $repository, first: 20, after: $after, fresh: $fresh) {
      edges { node { id tag name draft prerelease publishedAt createdAt url } } ${PAGE} ${READ}
    }
  }`);

export const CODE_DIRECTORY = doc(`
  query ForgeCodeDirectory($repository: ID!, $ref: String, $path: String, $fresh: Boolean) {
    codeDirectory(repository: $repository, ref: $ref, path: $path, fresh: $fresh) {
      ref path entries { name path kind size } partial ${READ}
    }
  }`);

export const CODE_FILE = doc(`
  query ForgeCodeFile($repository: ID!, $ref: String, $path: String!) {
    codeFile(repository: $repository, ref: $ref, path: $path) { path name size content text url }
  }`);

export const CREATE_ISSUE = doc(`mutation ForgeCreateIssue($input: NewIssueInput!) { createIssue(input: $input) { ${ISSUE} } }`);

export const ADD_COMMENT = doc(`
  mutation ForgeAddIssueComment($repository: ID!, $number: Int!, $body: String!) {
    addIssueComment(repository: $repository, number: $number, body: $body) { id authorLogin body bodyCut createdAt url }
  }`);

export const SET_ISSUE_OPEN = doc(`
  mutation ForgeSetIssueOpen($repository: ID!, $number: Int!, $open: Boolean!, $reason: IssueCloseReason) {
    setIssueOpen(repository: $repository, number: $number, open: $open, reason: $reason) { ${ISSUE} }
  }`);

export const MERGE_PULL_REQUEST = doc(`
  mutation ForgeMergePullRequest($input: MergePullRequestInput!) { mergePullRequest(input: $input) { ${PULL} } }`);

export const MERGE_PULL_REQUESTS = doc(`
  mutation ForgeMergePullRequests($pullRequests: [PullRequestTargetInput!]!, $method: MergeMethod!) {
    mergePullRequests(pullRequests: $pullRequests, method: $method) { number outcome message }
  }`);

export const RERUN = doc(`
  mutation ForgeRerunWorkflowRun($repository: ID!, $runId: ID!, $expectedHeadSha: String!) {
    rerunWorkflowRun(repository: $repository, runId: $runId, expectedHeadSha: $expectedHeadSha) { ${RUN} }
  }`);

export const RERUN_FAILED = doc(`
  mutation ForgeRerunFailedWorkflowJobs($repository: ID!, $runId: ID!, $expectedHeadSha: String!) {
    rerunFailedWorkflowJobs(repository: $repository, runId: $runId, expectedHeadSha: $expectedHeadSha) { ${RUN} }
  }`);

export const CANCEL = doc(`
  mutation ForgeCancelWorkflowRun($repository: ID!, $runId: ID!, $expectedHeadSha: String!) {
    cancelWorkflowRun(repository: $repository, runId: $runId, expectedHeadSha: $expectedHeadSha) { ${RUN} }
  }`);

export const DISPATCH = doc(`
  mutation ForgeDispatchWorkflow($input: DispatchWorkflowInput!) { dispatchWorkflow(input: $input) { workflowId branch acceptedAt } }`);
