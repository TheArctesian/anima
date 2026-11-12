import type { StorageDrive } from '../api';

type Mount = StorageDrive['mounts'][number];

/** Synthetic drives after the operator's pop-os tree (MET-024): no real device's values. */
export function drive(name: string | null, over: Partial<StorageDrive> = {}): StorageDrive {
  return { name, kind: 'SSD', sizeBytes: 1_000_000_000_000, mounted: true, mounts: [], readBps: null, writeBps: null, busyPercent: null, ...over };
}

export function mount(path: string, percent: number | null, usedBytes: number | null = null, sizeBytes: number | null = null): Mount {
  return { path, percent, usedBytes, sizeBytes };
}

const TB = 1_000_000_000_000;

/** Four drives, 31 TB, one not mounted; the system drive is 81 % full. */
export const POP_OS: readonly StorageDrive[] = [
  drive('nvme0n1', {
    kind: 'NVME',
    sizeBytes: TB,
    mounts: [mount('/', 81, 810 * 1_000_000_000, 1000 * 1_000_000_000), mount('/boot/efi', 7)],
    readBps: 1_048_576,
    writeBps: 524_288,
    busyPercent: 13,
  }),
  drive('sda', { kind: 'HDD', sizeBytes: 8 * TB, mounts: [mount('/mnt/media', 41)], readBps: 0, writeBps: 0, busyPercent: 0 }),
  drive('sdb', { kind: 'HDD', sizeBytes: 8 * TB, mounts: [mount('/mnt/backup', 92)] }),
  drive('sdc', { kind: 'HDD', sizeBytes: 14 * TB, mounted: false }),
];
