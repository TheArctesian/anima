import { describe, expect, it } from 'vitest';
import { added } from '../metrics.fixture';
import { formatBytes } from '../units';
import { mergeStorage, storageRows, summaryText } from './storage';
import { drive, mount, POP_OS } from './storage.fixture';

describe('summaryText', () => {
  it('MET-024: counts the drives, adds their sizes and the drives not mounted', () => {
    expect(summaryText(POP_OS)).toBe(`4 drives · ${formatBytes(31_000_000_000_000)} · 1 not mounted`);
  });

  it('MET-024: one drive is singular; a known size and nothing unmounted say only what is known', () => {
    expect(summaryText([drive('sda', { sizeBytes: 1024 })])).toBe('1 drive · 1.0 KiB');
    expect(summaryText([drive(null, { sizeBytes: null, kind: null })])).toBe('1 drive');
  });
});

describe('storageRows', () => {
  it('MET-024: a drive row has its name, its kind as a word, its size, its mounts and its activity', () => {
    const [system] = storageRows(POP_OS);
    expect(system?.title).toBe('nvme0n1');
    expect(system?.kind).toBe('NVMe');
    expect(system?.size).toBe(formatBytes(1_000_000_000_000));
    expect(system?.mounts.map((one) => one.path)).toEqual(['/', '/boot/efi']);
    expect(system?.mounts[0]?.text).toBe(`${formatBytes(1_000_000_000_000)} · 81 %`);
    expect(system?.mounts[0]?.spoken).toBe(`/, ${formatBytes(810_000_000_000)} used of ${formatBytes(1_000_000_000_000)}, 81 %`);
    expect(system?.activity).toBe('Read 1.0 MiB/s · Write 512.0 KiB/s · Busy 13 %');
    expect(system?.showPaths).toBe(true);
  });

  it('MET-022: every kind reads as a word', () => {
    const kinds = storageRows([drive('a', { kind: 'NVME' }), drive('b', { kind: 'SSD' }), drive('c', { kind: 'HDD' }), drive('d', { kind: 'DISK' })]);
    expect(kinds.map((row) => row.kind)).toEqual(['NVMe', 'SSD', 'HDD', 'Disk']);
  });

  it('MET-004: a mount above 85 % is degraded; a share is held between 0 and 100', () => {
    const rows = storageRows(POP_OS);
    expect(rows[2]?.mounts[0]?.degraded).toBe(true);
    expect(rows[1]?.mounts[0]?.degraded).toBe(false);
    const [odd] = storageRows([drive('x', { mounts: [mount('/a', 140), mount('/b', -3)] })]);
    expect(odd?.mounts.map((one) => one.percent)).toEqual([100, 0]);
  });

  it('MET-024: a drive not mounted has no mounts; a mount without a sample has no bar and says so', () => {
    const rows = storageRows([...POP_OS, drive('sdd', { mounts: [mount('/new', null)] })]);
    expect(rows[3]?.mounted).toBe(false);
    expect(rows[3]?.mounts).toEqual([]);
    expect(rows[4]?.mounts[0]).toMatchObject({ percent: null, text: '—', spoken: '/new, no reading', degraded: false });
  });

  it('MET-026: a lone mount is titled by its path, with no kind, no activity and no second path', () => {
    const [lone] = storageRows([drive(null, { kind: null, sizeBytes: null, mounts: [mount('/System/Volumes/Data', 40)] })]);
    expect(lone).toMatchObject({ title: '/System/Volumes/Data', kind: null, size: null, activity: null, showPaths: false, key: 'mount:/System/Volumes/Data' });
  });
});

describe('mergeStorage', () => {
  it('MET-023: a live batch updates a mount by path and a drive activity by name, and nothing else', () => {
    const batch = added('2026-11-12T09:00:00Z', [
      { kind: 'DISK_PERCENT', label: '/mnt/media', value: 42 },
      { kind: 'DISK_BYTES', label: '/mnt/media', value: 3e12 },
      { kind: 'DISK_READ_BPS', label: 'sdb', value: 7 },
      { kind: 'DISK_WRITE_BPS', label: 'sdb', value: 8 },
      { kind: 'DISK_BUSY_PERCENT', label: 'sdb', value: 9 },
      { kind: 'DISK_PERCENT', label: '/elsewhere', value: 1 },
      { kind: 'CPU_PERCENT', label: 'sdb', value: 99 },
    ]);
    const merged = mergeStorage(POP_OS, batch);
    expect(merged[1]?.mounts[0]).toMatchObject({ percent: 42, usedBytes: 3e12 });
    expect(merged[2]).toMatchObject({ readBps: 7, writeBps: 8, busyPercent: 9 });
    expect(merged[0]).toBe(POP_OS[0]);
    expect(merged[3]).toBe(POP_OS[3]);
    expect(merged).toHaveLength(4);
    expect(POP_OS[1]?.mounts[0]?.percent).toBe(41);
  });
});
