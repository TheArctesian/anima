import { decodeHubMessage, type HubMessage, type Listing, type Refusal } from './codec';
import type { Progress, UploadPump } from './upload';

/** Why a request gave no value. Each is typed; none holds a path, a file's bytes or device text. */
export type FilesProblem =
  /** The hub refused it (K4 §9). */
  | { readonly kind: 'refused'; readonly reason: Refusal }
  /** Another list or transfer is in flight: one at a time (K4 §9, D-5); nothing was sent. */
  | { readonly kind: 'busy' }
  /** The link is not open; nothing was sent. */
  | { readonly kind: 'offline' }
  /** The path is empty, holds NUL, is not UTF-8 or is over 4096 bytes, or the size is not a byte count; nothing was sent. */
  | { readonly kind: 'bad-request' }
  /** `cancel` ended it. */
  | { readonly kind: 'cancelled' }
  /** The upload's file could not be read; the transfer was cancelled. */
  | { readonly kind: 'unreadable' }
  /** The link ended first. */
  | { readonly kind: 'ended' };

export type FilesResult<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: FilesProblem };

/** Why a transfer the client stopped is over, once the hub confirms. */
type Stopped = 'cancelled' | 'unreadable';

export interface ListInFlight {
  readonly kind: 'list';
  readonly settle: (result: FilesResult<Listing>) => void;
}

export interface DownloadInFlight {
  readonly kind: 'download';
  readonly settle: (result: FilesResult<Blob>) => void;
  readonly progress: Progress;
  /** Announced by the hub's `download {size}`. */
  size: number | undefined;
  received: number;
  readonly chunks: ArrayBuffer[];
  stopped: Stopped | undefined;
}

export interface UploadInFlight {
  readonly kind: 'upload';
  readonly settle: (result: FilesResult<undefined>) => void;
  readonly progress: Progress;
  readonly size: number;
  readonly pump: UploadPump;
  stopped: Stopped | undefined;
}

/** The one request in flight (K4 §9). */
export type InFlight = ListInFlight | DownloadInFlight | UploadInFlight;

/**
 * What a hub frame did to the request in flight: `running` it goes on; `over` it settled; `over-cancel` it settled
 * on a refused upload, so the client sends `cancel` (WP-27 handoff); `breach` the frame does not fit what the client
 * asked, and the link closes (fail closed).
 */
export type Step = 'running' | 'over' | 'over-cancel' | 'breach';

const failed = (error: FilesProblem) => ({ ok: false, error }) as const;

/** A binary frame: download bytes, only after `download {size}`, never past it (D-4). */
function hearBytes(request: InFlight | undefined, bytes: ArrayBuffer): Step {
  if (request?.kind !== 'download' || request.size === undefined) return 'breach';
  request.received += bytes.byteLength;
  if (request.received > request.size) return 'breach';
  if (request.stopped === undefined) {
    request.chunks.push(bytes);
    request.progress(request.received, request.size);
  }
  return 'running';
}

function downloadHears(request: DownloadInFlight, message: HubMessage): Step {
  switch (message.kind) {
    case 'download':
      if (request.size !== undefined) return 'breach';
      request.size = message.size;
      request.progress(0, message.size);
      return 'running';
    case 'done':
      if (request.stopped !== undefined) request.settle(failed({ kind: request.stopped }));
      else if (request.size === request.received) request.settle({ ok: true, value: new Blob(request.chunks) });
      else return 'breach';
      return 'over';
    case 'refused':
      request.settle(failed(request.stopped === undefined ? { kind: 'refused', reason: message.reason } : { kind: request.stopped }));
      return 'over';
    default:
      return 'breach';
  }
}

function uploadHears(request: UploadInFlight, message: HubMessage): Step {
  switch (message.kind) {
    case 'done':
      if (request.stopped !== undefined) request.settle(failed({ kind: request.stopped }));
      else if (request.pump.sent()) {
        request.progress(request.size, request.size);
        request.settle({ ok: true, value: undefined });
      } else return 'breach';
      request.pump.stop();
      return 'over';
    case 'refused':
      request.pump.stop();
      if (request.stopped !== undefined) {
        request.settle(failed({ kind: request.stopped }));
        return 'over';
      }
      request.settle(failed({ kind: 'refused', reason: message.reason }));
      return 'over-cancel';
    default:
      return 'breach';
  }
}

/** A text message other than `ping`, against the request in flight. */
function hearMessage(request: InFlight | undefined, message: HubMessage): Step {
  if (request === undefined) return 'breach';
  if (request.kind === 'download') return downloadHears(request, message);
  if (request.kind === 'upload') return uploadHears(request, message);
  if (message.kind === 'entries') request.settle({ ok: true, value: message.listing });
  else if (message.kind === 'refused') request.settle(failed({ kind: 'refused', reason: message.reason }));
  else return 'breach';
  return 'over';
}

/** One frame from the hub against the request in flight: a `ping` changes nothing; a frame outside K4 §9 is a breach. */
export function hearFrame(request: InFlight | undefined, data: unknown): Step {
  if (data instanceof ArrayBuffer) return hearBytes(request, data);
  const message = typeof data === 'string' ? decodeHubMessage(data) : undefined;
  if (message === undefined) return 'breach';
  return message.kind === 'ping' ? 'running' : hearMessage(request, message);
}

/** The client stops the transfer in flight; `false` when there is none to stop or it is already stopping. */
export function stopTransfer(request: InFlight | undefined, why: Stopped): boolean {
  if (request === undefined || request.kind === 'list' || request.stopped !== undefined) return false;
  request.stopped = why;
  if (request.kind === 'upload') request.pump.stop();
  else request.chunks.length = 0;
  return true;
}

/** The link ended with `request` in flight: it settles as `ended`. */
export function abandon(request: InFlight | undefined): void {
  if (request === undefined) return;
  if (request.kind === 'upload') request.pump.stop();
  request.settle(failed({ kind: 'ended' }));
}
