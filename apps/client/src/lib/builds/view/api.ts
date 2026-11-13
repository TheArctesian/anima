import { graphql } from '$lib/hub/generated';
import type { BuildJobUpdatedSubscription, BuildTarget, ReleasesQuery } from '$lib/hub/generated/graphql';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';

const RELEASES = graphql(`
  query Releases($first: Int) {
    releases(first: $first) {
      edges {
        node {
          id
          version
          commit
          createdAt
          createdBy
          jobs {
            id
            releaseId
            target
            state
            failureReason
            attempts
            builderName
            stateChangedAt
          }
        }
      }
    }
  }
`);

const NEXT_RELEASE_VERSION = graphql(`
  query NextReleaseVersion {
    nextReleaseVersion
  }
`);

const CREATE_RELEASE = graphql(`
  mutation CreateRelease($version: String!, $commit: String!, $targets: [BuildTarget!]!) {
    createRelease(version: $version, commit: $commit, targets: $targets) {
      id
    }
  }
`);

const CANCEL_BUILD_JOB = graphql(`
  mutation CancelBuildJob($id: ID!) {
    cancelBuildJob(id: $id) {
      id
      state
    }
  }
`);

const BUILD_JOB_LOG = graphql(`
  query BuildJobLog($id: ID!, $after: Int, $first: Int) {
    buildJobLog(id: $id, after: $after, first: $first) {
      sequence
      text
    }
  }
`);

const BUILD_JOB_UPDATED = graphql(`
  subscription BuildJobUpdated($jobId: ID) {
    buildJobUpdated(jobId: $jobId) {
      job {
        id
        releaseId
        target
        state
        failureReason
        attempts
        builderName
        stateChangedAt
      }
      logLines {
        sequence
        text
      }
    }
  }
`);

export type { BuildTarget };

/** One release with its jobs, as the Builds view lists it (BLD-001, BLD-080). */
export type ReleaseView = ReleasesQuery['releases']['edges'][number]['node'];

/** One job of a release (BLD-080). */
export type JobView = ReleaseView['jobs'][number];

/** What the subscription sends: the job as it is now and the log lines the change added (BLD-081). */
export type JobChange = BuildJobUpdatedSubscription['buildJobUpdated'];

/** One stored log line (BLD-081). */
export type LogLineView = JobChange['logLines'][number];

/** How many releases the view asks for; the list is a history, not a feed. */
const RELEASES_SHOWN = 50;

/** The log lines asked for at one time; the view pages until a page comes back short (BLD-081). */
export const LOG_PAGE = 500;

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

/** The operator's builds calls: releases, a new release, cancel, job logs and live job changes (BLD-001, BLD-008, BLD-080, BLD-081). */
export function releasesApi(hub: Pick<Hub, 'request' | 'subscribe'>) {
  return {
    list: async (): Promise<HubResult<readonly ReleaseView[]>> =>
      mapValue(await hub.request(RELEASES, { first: RELEASES_SHOWN }), (value) => value.releases.edges.map((edge) => edge.node)),
    nextVersion: async (): Promise<HubResult<string | null>> =>
      mapValue(await hub.request(NEXT_RELEASE_VERSION, {}), (value) => value.nextReleaseVersion ?? null),
    create: async (version: string, commit: string, targets: readonly BuildTarget[]): Promise<HubResult<string>> =>
      mapValue(await hub.request(CREATE_RELEASE, { version, commit, targets: [...targets] }), (value) => value.createRelease.id),
    cancel: async (id: string): Promise<HubResult<string>> =>
      mapValue(await hub.request(CANCEL_BUILD_JOB, { id }), (value) => value.cancelBuildJob.id),
    log: async (id: string, after: number | null): Promise<HubResult<readonly LogLineView[]>> =>
      mapValue(await hub.request(BUILD_JOB_LOG, { id, after, first: LOG_PAGE }), (value) => value.buildJobLog),
    /** Every job's changes when `jobId` is null; returns the function that stops them. */
    watch: (jobId: string | null, sink: Sink<JobChange>): (() => void) =>
      hub.subscribe(BUILD_JOB_UPDATED, { jobId }, {
        ...sink,
        next: (value) => {
          sink.next(value.buildJobUpdated);
        },
      }),
  };
}
