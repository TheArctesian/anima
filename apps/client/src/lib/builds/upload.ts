import { BuilderShellError, type BuilderShell } from '$lib/desktop/builder/shell';
import type { HubResult } from '$lib/hub/hub';
import type { BuildsApi, BuildTarget, ClaimedJob, JobFile, UploadUrl } from './api';
import { buildFeed } from './feed';

/** Where the release bucket's files are public (DNS-007); the hub checks each feed URL against its own base (BLD-065). */
export const RELEASES_BASE = 'https://releases.anima.ing';
/** The release folder of each target, as the hub names it (BLD-060). */
const FOLDERS: Readonly<Record<BuildTarget, string>> = { LINUX_X86_64: 'linux-x86_64', DARWIN_UNIVERSAL: 'darwin-universal' };
/** The platform of each target in the proposed feed (BLD-067). */
const PLATFORMS = { LINUX_X86_64: 'linux', DARWIN_UNIVERSAL: 'macos' } as const;

/** The hub refused or could not take a call of the upload; only this fixed text is kept (LOG-007). */
export class UploadRefusedError extends Error {
  constructor() {
    super('The hub did not take the upload.');
    this.name = 'UploadRefusedError';
  }
}

function accepted<T>(result: HubResult<T>): T {
  if (!result.ok) throw new UploadRefusedError();
  return result.value;
}

/** The upload of each file, in the order of `files`; refused unless the hub gave exactly one for each. */
function uploadsFor(files: readonly JobFile[], uploads: readonly UploadUrl[]): UploadUrl[] {
  const byName = files.map((file) => uploads.filter((upload) => upload.name === file.name));
  // An answer with more uploads than files is not the hub's own; it is refused rather than half used.
  if (uploads.length !== files.length || byName.some((found) => found.length !== 1)) throw new UploadRefusedError();
  return byName.flat();
}

/** What the upload needs; the date of the feed comes from `now` (CQ-021). */
export interface UploadPorts {
  readonly api: Pick<BuildsApi, 'uploadUrls' | 'reportUploaded'>;
  readonly shell: Pick<BuilderShell, 'releaseFiles' | 'upload'>;
  readonly now: () => Date;
}

/**
 * Uploads the files of a built and verified job with the `SHA256SUMS` the shell wrote, then reports them and the proposed feed; the hub
 * publishes from there (BLD-060 to BLD-063). No file is sent once `stopping` is true (BLD-026). Rejects with the shell's
 * error, or `UploadRefusedError` for the hub's.
 */
export async function uploadRelease(job: ClaimedJob, ports: UploadPorts, stopping: () => boolean): Promise<void> {
  const release = await ports.shell.releaseFiles(job);
  const files = release.files;
  const uploads = uploadsFor(files, accepted(await ports.api.uploadUrls(job.id, files)));
  for (const upload of uploads) {
    if (stopping()) throw new BuilderShellError('cancelled');
    await ports.shell.upload(job, upload);
  }
  const feed = buildFeed({
    version: job.version,
    baseUrl: `${RELEASES_BASE}/${FOLDERS[job.target]}/${job.version}`,
    date: ports.now(),
    built: { [PLATFORMS[job.target]]: release.updater },
  });
  accepted(await ports.api.reportUploaded(job.id, files, JSON.stringify(feed)));
}
