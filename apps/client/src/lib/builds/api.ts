import { graphql } from '$lib/hub/generated';
import type { BuildJobState, BuildTarget, BuilderPlatformInput } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

const BUILDERS = graphql(`
  query Builders {
    builders {
      id
      name
      platform {
        os
        arch
      }
      online
    }
  }
`);

const ENROL_BUILDER = graphql(`
  mutation EnrolBuilder($name: String!, $platform: BuilderPlatformInput!) {
    enrolBuilder(name: $name, platform: $platform) {
      builder {
        id
      }
      token
    }
  }
`);

const REVOKE_BUILDER = graphql(`
  mutation RevokeBuilder($id: ID!) {
    revokeBuilder(id: $id)
  }
`);

const CLAIM_BUILD_JOB = graphql(`
  mutation ClaimBuildJob($platform: BuilderPlatformInput!) {
    claimBuildJob(platform: $platform) {
      id
      target
      version
      commit
    }
  }
`);

const REPORT_BUILD_JOB = graphql(`
  mutation ReportBuildJob($id: ID!, $state: BuildJobState, $logLines: [String!]!, $failureReason: String) {
    reportBuildJob(id: $id, state: $state, logLines: $logLines, failureReason: $failureReason) {
      id
      state
    }
  }
`);

const BUILD_UPLOAD_URLS = graphql(`
  mutation BuildUploadUrls($id: ID!, $files: [BuildFileInput!]!) {
    buildUploadUrls(id: $id, files: $files) {
      name
      url
      headers {
        name
        value
      }
      expiresAt
    }
  }
`);

const REPORT_BUILD_UPLOADED = graphql(`
  mutation ReportBuildUploaded($id: ID!, $files: [BuildFileInput!]!, $feed: String!) {
    reportBuildUploaded(id: $id, files: $files, feed: $feed) {
      id
      state
    }
  }
`);

export type { BuildJobState, BuildTarget, BuilderPlatformInput as BuilderPlatform };

/** A builder as the Settings row lists it (BLD-025). */
export interface Builder {
  readonly id: string;
  readonly name: string;
  readonly platform: BuilderPlatformInput;
  readonly online: boolean;
}

/** What `enrolBuilder` returns: the builder's ID and its token, shown once (BLD-021, BLD-023). */
export interface Enrolment {
  readonly builderId: string;
  readonly token: string;
}

/** A claimed job, as the hub sent it. Its fields are checked again by the shell before any process starts. */
export interface ClaimedJob {
  readonly id: string;
  readonly target: BuildTarget;
  readonly version: string;
  readonly commit: string;
}

/** One report: the state change, if any, the cleaned log lines, and the reason of a failure (BLD-010, BLD-048). */
export interface JobReport {
  readonly state: BuildJobState | null;
  readonly logLines: readonly string[];
  readonly failureReason: string | null;
}

/** One file of a job as the hub records it: its name, lower-case hex SHA-256 and size in bytes (BLD-063). */
export interface JobFile {
  readonly name: string;
  readonly sha256: string;
  readonly size: number;
}

/** A presigned `PUT` for one file (BLD-060). The URL is a credential: it is never logged or reported (BLD-049). */
export interface UploadUrl {
  readonly name: string;
  readonly url: string;
  readonly headers: readonly { readonly name: string; readonly value: string }[];
}

/**
 * The builds calls of the hub. `builders`, `enrol` and `revoke` need the operator's session (BLD-021, BLD-025); `claim`,
 * `report`, `uploadUrls` and `reportUploaded` need the builder token (D-1), so the builder loop sends them through a hub that reads that token.
 */
export interface BuildsApi {
  builders(): Promise<HubResult<readonly Builder[]>>;
  enrol(name: string, platform: BuilderPlatformInput): Promise<HubResult<Enrolment>>;
  revoke(id: string): Promise<HubResult<string>>;
  /** The job this builder now holds, or `null` when none waits (BLD-004). */
  claim(platform: BuilderPlatformInput): Promise<HubResult<ClaimedJob | null>>;
  /** Sends one report and extends the lease (BLD-005). */
  report(id: string, report: JobReport): Promise<HubResult<BuildJobState>>;
  /** One presigned upload URL for each file of an `UPLOADING` job; extends the lease (BLD-060, D-26). */
  uploadUrls(id: string, files: readonly JobFile[]): Promise<HubResult<readonly UploadUrl[]>>;
  /** Reports the uploaded files and the proposed feed; the hub then publishes in the background (BLD-063). */
  reportUploaded(id: string, files: readonly JobFile[], feed: string): Promise<HubResult<BuildJobState>>;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

/** A hub that can send requests; a builder's hub has no socket (D-1: the builder token opens none). */
type Requests = Pick<Hub, 'request'>;

export function buildsApi(hub: Requests): BuildsApi {
  return {
    builders: async () => mapValue(await hub.request(BUILDERS, {}), (value) => value.builders),
    enrol: async (name, platform) =>
      mapValue(await hub.request(ENROL_BUILDER, { name, platform }), (value) => ({
        builderId: value.enrolBuilder.builder.id,
        token: value.enrolBuilder.token,
      })),
    revoke: async (id) => mapValue(await hub.request(REVOKE_BUILDER, { id }), (value) => value.revokeBuilder),
    claim: async (platform) => mapValue(await hub.request(CLAIM_BUILD_JOB, { platform }), (value) => value.claimBuildJob ?? null),
    report: async (id, report) =>
      mapValue(
        await hub.request(REPORT_BUILD_JOB, {
          id,
          state: report.state,
          logLines: [...report.logLines],
          failureReason: report.failureReason,
        }),
        (value) => value.reportBuildJob.state,
      ),
    uploadUrls: async (id, files) =>
      mapValue(await hub.request(BUILD_UPLOAD_URLS, { id, files: [...files] }), (value) => value.buildUploadUrls),
    reportUploaded: async (id, files, feed) =>
      mapValue(
        await hub.request(REPORT_BUILD_UPLOADED, { id, files: [...files], feed }),
        (value) => value.reportBuildUploaded.state,
      ),
  };
}
