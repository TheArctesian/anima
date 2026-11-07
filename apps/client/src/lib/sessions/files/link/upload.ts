import { FRAME_MAX_BYTES } from './codec';

/** The upload waits while the socket holds more than this many unsent bytes (K4 §8.5 backpressure, D-3). */
export const HOLD_BACK_BYTES = 1_048_576;
/** How often a held-back upload looks at the socket's buffer again. */
export const DRAIN_POLL_MS = 50;

/** What an upload reads its bytes from: a `File` or `Blob` (`slice` then `arrayBuffer`). */
export interface UploadSource {
  readonly size: number;
  slice(start: number, end: number): { arrayBuffer(): Promise<ArrayBuffer> };
}

/** Bytes the socket has taken so far and the upload's size (MOT-009: real progress, never above `size`). */
export type Progress = (done: number, size: number) => void;

/** The socket as the upload sees it. */
export interface UploadPort {
  /** Send one binary frame; false when the link can no longer send. */
  readonly send: (frame: ArrayBuffer) => boolean;
  /** The socket's unsent bytes (`WebSocket.bufferedAmount`). */
  readonly buffered: () => number;
  /** Resolves after `ms`; tests pass a fake clock (CQ-021). */
  readonly wait: (ms: number) => Promise<void>;
}

/** One upload's frames on their way out. */
export interface UploadPump {
  /** Send no further frame, at once; safe to call more than once. */
  stop(): void;
  /** Whether every byte was handed to the socket. */
  sent(): boolean;
}

/**
 * Send `source` as frames of at most `FRAME_MAX_BYTES`, `source.size` bytes in all, holding back while the socket's
 * buffer is above `HOLD_BACK_BYTES`. `progress` hears the bytes the socket has taken (sent minus buffered), rising
 * only. `unreadable` is called once when a slice cannot be read or reads short; no frame follows it.
 */
export function pumpUpload(source: UploadSource, port: UploadPort, progress: Progress, unreadable: () => void): UploadPump {
  const size = source.size;
  let stopped = false;
  let offset = 0;
  let taken = 0;

  const report = () => {
    const now = Math.min(size, Math.max(taken, offset - port.buffered()));
    if (now === taken) return;
    taken = now;
    progress(taken, size);
  };

  const run = async () => {
    while (!stopped && offset < size) {
      if (port.buffered() > HOLD_BACK_BYTES) {
        await port.wait(DRAIN_POLL_MS);
        report();
        continue;
      }
      const end = Math.min(size, offset + FRAME_MAX_BYTES);
      let frame: ArrayBuffer;
      try {
        frame = await source.slice(offset, end).arrayBuffer();
      } catch {
        frame = new ArrayBuffer(0);
      }
      if (stopped) return;
      if (frame.byteLength !== end - offset) {
        stopped = true;
        unreadable();
        return;
      }
      if (!port.send(frame)) return;
      offset = end;
      report();
    }
    while (!stopped && taken < size) {
      await port.wait(DRAIN_POLL_MS);
      report();
    }
  };
  // Started after the caller holds the pump, so even a slice that fails at once finds the transfer to stop.
  void Promise.resolve().then(run);

  return {
    stop: () => {
      stopped = true;
    },
    sent: () => offset === size,
  };
}
