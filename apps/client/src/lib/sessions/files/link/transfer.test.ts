import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FRAME_MAX_BYTES } from './codec';
import { FAKE_FILES_SOCKET, FakeFilesSocket } from './fake/socket.fixture';
import { PING_AFTER_MS, openFilesLink } from './link';
import { HOLD_BACK_BYTES, type UploadSource } from './upload';

/** Resolves pending promise chains (slices, sends) without moving the clock. */
async function settle(): Promise<void> {
  for (let turn = 0; turn < 50; turn += 1) await Promise.resolve();
}

/** A source of `size` bytes, each its offset modulo 251; `broken` makes reads from that offset fail. */
function source(size: number, broken?: number): UploadSource {
  return {
    size,
    slice: (start, end) => ({
      arrayBuffer: () => {
        if (broken !== undefined && start >= broken) return Promise.reject(new Error('NotReadableError'));
        const bytes = new Uint8Array(end - start).map((_, at) => (start + at) % 251);
        return Promise.resolve(bytes.buffer);
      },
    }),
  };
}

function connected() {
  const waits: (() => void)[] = [];
  const opening = openFilesLink({
    origin: 'https://hub.anima.ing',
    ticket: { tunnelPath: '/files', token: 't' },
    WebSocket: FAKE_FILES_SOCKET,
    wait: () =>
      new Promise((resolve) => {
        waits.push(resolve);
      }),
  });
  if (!opening.ok) throw new Error('not opened');
  const socket = FakeFilesSocket.latest();
  socket.open();
  /** The held-back upload looks at the buffer again. */
  const poll = async () => {
    for (const resolve of waits.splice(0)) resolve();
    await settle();
  };
  return { link: opening.value, socket, poll };
}

const sum = (sizes: number[]) => sizes.reduce((total, size) => total + size, 0);

beforeEach(() => {
  vi.useFakeTimers();
  FakeFilesSocket.reset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('download (K4 §9, D-4)', () => {
  it('D-4: download {size}, then frames summed to size, then done gives the bytes as one Blob; progress is received / size', async () => {
    const { link, socket } = connected();
    const progress: [number, number][] = [];
    const result = link.download('/f.bin', (done, size) => progress.push([done, size]));
    expect(socket.messages()).toEqual([{ type: 'download', path: '/f.bin' }]);
    socket.text({ type: 'download', size: 10 });
    socket.bytes(4, 1);
    socket.bytes(6, 2);
    socket.text({ type: 'done' });
    const got = await result;
    if (!got.ok) throw new Error('download failed');
    expect([...new Uint8Array(await got.value.arrayBuffer())]).toEqual([1, 1, 1, 1, 2, 2, 2, 2, 2, 2]);
    expect(progress).toEqual([
      [0, 10],
      [4, 10],
      [10, 10],
    ]);
  });

  it('D-4: a byte past the announced size closes the link and the download ends without its bytes', async () => {
    const { link, socket } = connected();
    const result = link.download('/f');
    socket.text({ type: 'download', size: 3 });
    socket.bytes(4);
    expect(await result).toEqual({ ok: false, error: { kind: 'ended' } });
    expect(link.state().end).toBe('misread');
    expect(socket.closedByClient).toBe(1000);
  });

  it('fail closed: bytes before download {size}, a second download {size}, or done short of the size close the link', () => {
    const cases = [
      (s: FakeFilesSocket) => s.bytes(1),
      (s: FakeFilesSocket) => (s.text({ type: 'download', size: 2 }), s.text({ type: 'download', size: 2 })),
      (s: FakeFilesSocket) => (s.text({ type: 'download', size: 2 }), s.bytes(1), s.text({ type: 'done' })),
    ];
    for (const send of cases) {
      const { link, socket } = connected();
      void link.download('/f');
      send(socket);
      expect(link.state().end).toBe('misread');
    }
  });

  it('K4 §9: a refused download settles with the reason', async () => {
    const { link, socket } = connected();
    const result = link.download('/f');
    socket.text({ type: 'refused', reason: 'DOWNLOAD_OFF' });
    expect(await result).toEqual({ ok: false, error: { kind: 'refused', reason: 'DOWNLOAD_OFF' } });
  });

  it('K4 §9: pings go out every 5 s during a download, and only cancel and ping are sent while it runs', async () => {
    const { link, socket } = connected();
    void link.download('/f');
    socket.text({ type: 'download', size: 100 });
    for (let beat = 0; beat < 3; beat += 1) {
      socket.bytes(1);
      await vi.advanceTimersByTimeAsync(PING_AFTER_MS);
    }
    expect(await link.list('/')).toEqual({ ok: false, error: { kind: 'busy' } });
    expect(socket.types()).toEqual(['download', 'ping', 'ping', 'ping']);
  });

  it('WP-27 handoff: cancel sends cancel, frames still in flight are taken, and done ends it as cancelled', async () => {
    const { link, socket } = connected();
    const result = link.download('/f');
    socket.text({ type: 'download', size: 10 });
    socket.bytes(3);
    expect(link.cancel()).toBe(true);
    expect(link.cancel()).toBe(false);
    socket.bytes(3);
    socket.text({ type: 'done' });
    expect(await result).toEqual({ ok: false, error: { kind: 'cancelled' } });
    expect(socket.types()).toEqual(['download', 'cancel']);
    expect(link.state().phase).toBe('connected');
  });

  it('cancel with nothing in flight, or during a list, sends nothing', () => {
    const { link, socket } = connected();
    expect(link.cancel()).toBe(false);
    void link.list('/');
    expect(link.cancel()).toBe(false);
    expect(socket.types()).toEqual(['list']);
  });
});

describe('upload (K4 §9, D-3)', () => {
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
    const gated: UploadSource = {
      size: 10,
      slice: (start, end) => ({
        arrayBuffer: () =>
          new Promise((resolve) => {
            reads.push(() => {
              resolve(new ArrayBuffer(end - start));
            });
          }),
      }),
    };
    const result = link.upload('/up', gated);
    await settle();
    socket.text({ type: 'refused', reason: 'UPLOAD_OFF' });
    for (const read of reads) read();
    await settle();
    expect(await result).toEqual({ ok: false, error: { kind: 'refused', reason: 'UPLOAD_OFF' } });
    expect(socket.binarySizes()).toEqual([]);
    expect(socket.types()).toEqual(['upload', 'cancel']);
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
