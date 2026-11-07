import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FakeFilesSocket } from './fake/socket.fixture';
import { connected } from './fake/link.fixture';
import { PING_AFTER_MS } from './link';

beforeEach(() => {
  vi.useFakeTimers();
  FakeFilesSocket.reset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('download bytes (K4 §9, D-4)', () => {
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
});

describe('download liveness and cancel (K4 §9, WP-27 handoff)', () => {
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

  it('MOT-009: after cancel, frames still arriving move no progress', () => {
    const { link, socket } = connected();
    const progress: number[] = [];
    void link.download('/f', (done) => progress.push(done));
    socket.text({ type: 'download', size: 10 });
    socket.bytes(3);
    link.cancel();
    socket.bytes(3);
    expect(progress).toEqual([0, 3]);
  });

  it('S1: once every byte has arrived, cancel sends nothing and done gives the file, as the hub records it', async () => {
    const { link, socket } = connected();
    const result = link.download('/f');
    socket.text({ type: 'download', size: 4 });
    socket.bytes(4, 9);
    expect(link.cancel()).toBe(false);
    socket.text({ type: 'done' });
    const got = await result;
    if (!got.ok) throw new Error('download failed');
    expect(got.value.size).toBe(4);
    expect(socket.types()).toEqual(['download']);
  });

  it('cancel with nothing in flight, or during a list, sends nothing', () => {
    const { link, socket } = connected();
    expect(link.cancel()).toBe(false);
    void link.list('/');
    expect(link.cancel()).toBe(false);
    expect(socket.types()).toEqual(['list']);
  });
});
