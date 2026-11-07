import type { Refusal } from './link/codec';
import type { FilesProblem } from './link/inflight';

const REFUSALS: Readonly<Record<Refusal, string>> = {
  UPLOAD_OFF: 'Uploads are off for this device.',
  DOWNLOAD_OFF: 'Downloads are off for this device.',
  EXISTS: 'A file with this name is already there; Anima never replaces a file.',
  NOT_FOUND: 'The device has no file or folder at that place.',
  DENIED: 'The device did not allow that.',
};

/** Shown when the single-use ticket could not be had (the hub's own words are never shown, LOG-007). */
export const TICKET_FAILED = 'Could not open the file connection. Check that the session is still running.';
/** Shown when the name cannot be made into a path on the device. */
export const NAME_REFUSED = 'That name cannot be used as a file name on the device.';

const OTHERS: Readonly<Record<Exclude<FilesProblem['kind'], 'refused'>, string>> = {
  busy: 'Wait for the transfer in progress to finish.',
  offline: 'The file connection is not open.',
  'bad-request': NAME_REFUSED,
  cancelled: 'The transfer was cancelled.',
  unreadable: 'That file could not be read.',
  ended: 'The file connection ended.',
};

/** The fixed operator text of a failed request; none holds a path, a name or text from the device. */
export function problemText(problem: FilesProblem): string {
  return problem.kind === 'refused' ? REFUSALS[problem.reason] : OTHERS[problem.kind];
}
