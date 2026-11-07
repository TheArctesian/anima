import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FRAME_MAX_BYTES } from './codec';
import { FakeFilesSocket } from './fake/socket.fixture';
import { connected, settle, source } from './fake/link.fixture';
import { HOLD_BACK_BYTES, type UploadSource } from './upload';

/** A source of `size` bytes whose reads finish only when the test runs what lands in `reads`. */
function gated(size: number, reads: (() => void)[]): UploadSource {
  return {
    size,
    slice: (start, end) => ({
      arrayBuffer: () =>
        new Promise((resolve) => {
          reads.push(() => {
            resolve(new ArrayBuffer(end - start));
          });
        }),
    }),
  };
}

const sum = (sizes: number[]) => sizes.reduce((total, size) => total + size, 0);

beforeEach(() => {
  vi.useFakeTimers();
  FakeFilesSocket.reset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('upload frames (K4 §9, D-3)', () => {
  it('D-3: upload {path, size}, then frames of at most 64 KiB totalling size, then done; progress rises to size', async () => {
    const { link, socket } = connected();
    const size = 2 * FRAME_MAX_BYTES + 1000;
    const progress: number[] = [];
    const result = link.upload('/up.bin', source(size), (done) => progress.push(done));
    await settle();
    expect(socket.messages()).toEqual([{ type: 'upload', path: '/up.bin', size }]);
    expect(socket.binarySizes()).toEqual([FRAME_MAX_BYTES, FRAME_MAX_BYTES, 1000]);
    const bytes = socket.frames.filter((frame): frame is ArrayBuffer => typeof frame !== 'string').flatMap((frame) => [...new Uint8Array(frame)]);
    expect(bytes).toEqual(Array.from({ length: size }, (_, at) => at % 251));
    socket.text({ type: 'done' });
    expect(await result).toEqual({ ok: true, value: undefined });
    expect(progress.at(-1)).toBe(size);
    expect(progress.every((done, at) => done <= size && done >= (progress[at - 1] ?? 0))).toBe(true);
  });

  it('D-3: frames are held back while the socket buffers more than the bound, and progress is sent minus buffered', async () => {
    const { link, socket, poll } = connected();
    const size = 3 * FRAME_MAX_BYTES;
    const progress: number[] = [];
    void link.upload('/up', source(size), (done) => progress.push(done));
    socket.bufferedAmount = HOLD_BACK_BYTES + 1;
    await settle();
    expect(socket.binarySizes()).toEqual([]);
    await poll();
    await poll();
    expect(socket.binarySizes()).toEqual([]);
    expect(progress).toEqual([]);
    socket.bufferedAmount = 1000;
    await poll();
    expect(sum(socket.binarySizes())).toBe(size);
    expect(progress.at(-1)).toBe(size - 1000);
    socket.bufferedAmount = 0;
    await poll();
    expect(progress.at(-1)).toBe(size);
  });

  it('a File from the picker is read through slice', async () => {
    vi.useRealTimers();
    const { link, socket } = connected();
    const result = link.upload('/hello.txt', new File(['hello'], 'hello.txt'));
    await vi.waitFor(() => {
      expect(socket.binarySizes()).toEqual([5]);
    });
    socket.text({ type: 'done' });
    expect(await result).toEqual({ ok: true, value: undefined });
    link.close();
  });
});

describe('upload refused and cancelled (WP-27 handoff)', () => {
  it('WP-27 handoff: on refused the upload stops sending at once, then sends cancel', async () => {
    const { link, socket, poll } = connected();
    const result = link.upload('/up', source(4 * FRAME_MAX_BYTES));
    socket.bufferedAmount = HOLD_BACK_BYTES + 1;
    await settle();
    socket.text({ type: 'refused', reason: 'EXISTS' });
    expect(await result).toEqual({ ok: false, error: { kind: 'refused', reason: 'EXISTS' } });
    socket.bufferedAmount = 0;
    await poll();
    await poll();
    expect(socket.binarySizes()).toEqual([]);
    expect(socket.types()).toEqual(['upload', 'cancel']);
    expect(link.state().phase).toBe('connected');
  });

  it('WP-27 handoff: a refusal that lands while a slice is being read keeps that slice from being sent', async () => {
    const { link, socket } = connected();
    const reads: (() => void)[] = [];
    const result = link.upload('/up', gated(10, reads));
    await settle();
    socket.text({ type: 'refused', reason: 'UPLOAD_OFF' });
    for (const read of reads) read();
    await settle();
    expect(await result).toEqual({ ok: false, error: { kind: 'refused', reason: 'UPLOAD_OFF' } });
    expect(socket.binarySizes()).toEqual([]);
    expect(socket.types()).toEqual(['upload', 'cancel']);
  });

  it('WP-27 handoff: cancel stops the frames and sends cancel; done ends it as cancelled', async () => {
    const { link, socket, poll } = connected();
    const result = link.upload('/up', source(4 * FRAME_MAX_BYTES));
    socket.bufferedAmount = HOLD_BACK_BYTES + 1;
    await settle();
    expect(link.cancel()).toBe(true);
    socket.bufferedAmount = 0;
    await poll();
    expect(socket.binarySizes()).toEqual([]);
    expect(await link.list('/')).toEqual({ ok: false, error: { kind: 'busy' } });
    socket.text({ type: 'done' });
    expect(await result).toEqual({ ok: false, error: { kind: 'cancelled' } });
    expect(socket.types()).toEqual(['upload', 'cancel']);
  });

  it('a file that cannot be read stops the frames, sends cancel and ends as unreadable once the hub confirms', async () => {
    const { link, socket } = connected();
    const result = link.upload('/up', source(3 * FRAME_MAX_BYTES, FRAME_MAX_BYTES));
    await settle();
    expect(socket.binarySizes()).toEqual([FRAME_MAX_BYTES]);
    expect(socket.types()).toEqual(['upload', 'cancel']);
    socket.text({ type: 'done' });
    expect(await result).toEqual({ ok: false, error: { kind: 'unreadable' } });
  });
});

describe('upload ending (K4 §9)', () => {
  it('fail closed: done before every byte was sent, or bytes from the hub during an upload, close the link', async () => {
    for (const send of [(s: FakeFilesSocket) => s.text({ type: 'done' }), (s: FakeFilesSocket) => s.bytes(1)]) {
      const { link, socket } = connected();
      const result = link.upload('/up', source(4 * FRAME_MAX_BYTES));
      socket.bufferedAmount = HOLD_BACK_BYTES + 1;
      await settle();
      send(socket);
      expect(await result).toEqual({ ok: false, error: { kind: 'ended' } });
      expect(link.state().end).toBe('misread');
    }
  });

  it('an empty file sends upload with size 0 and no frame', async () => {
    const { link, socket } = connected();
    const result = link.upload('/empty', new Blob([]));
    await settle();
    expect(socket.messages()).toEqual([{ type: 'upload', path: '/empty', size: 0 }]);
    socket.text({ type: 'done' });
    expect(await result).toEqual({ ok: true, value: undefined });
    expect(socket.binarySizes()).toEqual([]);
  });

  it('S1: once every frame is in the socket, cancel sends nothing and done settles the upload as stored', async () => {
    const { link, socket } = connected();
    const result = link.upload('/up', source(FRAME_MAX_BYTES + 1));
    await settle();
    expect(socket.binarySizes()).toEqual([FRAME_MAX_BYTES, 1]);
    expect(link.cancel()).toBe(false);
    socket.text({ type: 'done' });
    expect(await result).toEqual({ ok: true, value: undefined });
    expect(socket.types()).toEqual(['upload']);
  });

  it('the link ending mid-upload stops the frames and ends the upload', async () => {
    const { link, socket, poll } = connected();
    const result = link.upload('/up', source(4 * FRAME_MAX_BYTES));
    socket.bufferedAmount = HOLD_BACK_BYTES + 1;
    await settle();
    socket.closeWith(1001);
    socket.bufferedAmount = 0;
    await poll();
    expect(await result).toEqual({ ok: false, error: { kind: 'ended' } });
    expect(socket.binarySizes()).toEqual([]);
  });
});

describe('no timer outlives the link (C1)', () => {
  it('C1: no timer is left once the link has ended', async () => {
    const { link } = connected(true);
    await vi.advanceTimersByTimeAsync(1);
    link.close();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('C1: no timer is left once the link ends mid-upload with bytes still buffered', async () => {
    const { link, socket } = connected(true);
    void link.upload('/up', source(4 * FRAME_MAX_BYTES));
    socket.bufferedAmount = HOLD_BACK_BYTES + 1;
    await vi.advanceTimersByTimeAsync(200);
    socket.closeWith(1006);
    await settle();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('C1: no timer is left once the link ends while the last frames drain', async () => {
    const { link, socket } = connected(true);
    void link.upload('/up', source(10));
    socket.bufferedAmount = 5;
    await vi.advanceTimersByTimeAsync(200);
    link.close();
    await settle();
    expect(vi.getTimerCount()).toBe(0);
  });
});
