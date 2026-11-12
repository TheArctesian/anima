import type { DriveKind } from '$lib/hub/generated/graphql';
import type { NewMetrics, StorageDrive } from '../api';
import { DEGRADED_ABOVE, formatBytes, formatBytesPerSecond, formatPercent } from '../units';

/** One mount of a Storage row: its path, its share used for the bar and the value as text (MET-024). */
export interface MountView {
  readonly path: string;
  /** 0 to 100; `null` without a recent sample, which draws no bar. */
  readonly percent: number | null;
  /** "7.3 TiB · 41 %": the size and the share used, as far as they are known. */
  readonly text: string;
  /** What a screen reader hears for the bar: used, of what, and the share. */
  readonly spoken: string;
  readonly degraded: boolean;
}

/** One row of the Storage section: a drive, or a mount on no drive the hub knows (MET-024, MET-026). */
export interface StorageRow {
  readonly key: string;
  /** The drive's name, or the mount point of a lone mount: both machine text, set in the monospace font. */
  readonly title: string;
  /** NVMe, SSD, HDD or Disk as a word, never a colour alone; `null` when the hub cannot tell (MET-022). */
  readonly kind: string | null;
  /** The drive's size; `null` when it is not known. */
  readonly size: string | null;
  readonly mounted: boolean;
  /** The mounts under the title; a lone mount's path is its title, so its bar stands alone. */
  readonly mounts: readonly MountView[];
  readonly showPaths: boolean;
  /** "Read 1.0 MiB/s · Write 512.0 KiB/s · Busy 13 %"; `null` with no activity value (MET-023). */
  readonly activity: string | null;
}

const KIND_TEXT: Readonly<Record<DriveKind, string>> = { NVME: 'NVMe', SSD: 'SSD', HDD: 'HDD', DISK: 'Disk' };

function plural(count: number, one: string, many: string): string {
  return `${String(count)} ${count === 1 ? one : many}`;
}

/** The one summary line: "4 drives · 31.0 TiB · 1 not mounted"; the size and the count not mounted only when known and non-zero. */
export function summaryText(drives: readonly StorageDrive[]): string {
  const sizes = drives.flatMap((drive) => (drive.sizeBytes === null ? [] : [drive.sizeBytes]));
  const unmounted = drives.filter((drive) => !drive.mounted).length;
  const parts = [plural(drives.length, 'drive', 'drives')];
  if (sizes.length > 0) parts.push(formatBytes(sizes.reduce((sum, size) => sum + size, 0)));
  if (unmounted > 0) parts.push(`${String(unmounted)} not mounted`);
  return parts.join(' · ');
}

function mountView(mount: StorageDrive['mounts'][number]): MountView {
  const { percent, sizeBytes, usedBytes } = mount;
  const shown = percent === null ? null : Math.min(100, Math.max(0, percent));
  const text = [sizeBytes === null ? null : formatBytes(sizeBytes), shown === null ? null : formatPercent(shown)].filter((part) => part !== null).join(' · ');
  const used = usedBytes === null ? null : `${formatBytes(usedBytes)} used${sizeBytes === null ? '' : ` of ${formatBytes(sizeBytes)}`}`;
  const spoken = [mount.path, used, shown === null ? 'no reading' : formatPercent(shown)].filter((part) => part !== null).join(', ');
  return { path: mount.path, percent: shown, text: text === '' ? '—' : text, spoken, degraded: shown !== null && shown > DEGRADED_ABOVE };
}

function activityText(drive: StorageDrive): string | null {
  const parts = [
    drive.readBps === null ? null : `Read ${formatBytesPerSecond(drive.readBps)}`,
    drive.writeBps === null ? null : `Write ${formatBytesPerSecond(drive.writeBps)}`,
    drive.busyPercent === null ? null : `Busy ${formatPercent(drive.busyPercent)}`,
  ].filter((part) => part !== null);
  return parts.length === 0 ? null : parts.join(' · ');
}

/** The rows in the hub's order: mounted drives, lone mounts, then drives that are not mounted (MET-024). */
export function storageRows(drives: readonly StorageDrive[]): readonly StorageRow[] {
  return drives.map((drive) => {
    const lone = drive.name === null;
    const title = drive.name ?? drive.mounts[0]?.path ?? '—';
    return {
      key: `${lone ? 'mount' : 'drive'}:${title}`,
      title,
      kind: drive.kind === null ? null : KIND_TEXT[drive.kind],
      size: drive.sizeBytes === null ? null : formatBytes(drive.sizeBytes),
      mounted: drive.mounted,
      mounts: drive.mounts.map(mountView),
      showPaths: !lone,
      activity: activityText(drive),
    };
  });
}

/** What one live sample changes in a drive: a mount's share or bytes used by mount point, or the drive's activity by name. */
function withSample(drive: StorageDrive, sample: NewMetrics['samples'][number]): StorageDrive {
  const { kind, label, value } = sample;
  if (kind === 'DISK_PERCENT' || kind === 'DISK_BYTES') {
    if (!drive.mounts.some((mount) => mount.path === label)) return drive;
    const field = kind === 'DISK_PERCENT' ? 'percent' : 'usedBytes';
    return { ...drive, mounts: drive.mounts.map((mount) => (mount.path === label ? { ...mount, [field]: value } : mount)) };
  }
  if (drive.name !== label) return drive;
  if (kind === 'DISK_READ_BPS') return { ...drive, readBps: value };
  if (kind === 'DISK_WRITE_BPS') return { ...drive, writeBps: value };
  if (kind === 'DISK_BUSY_PERCENT') return { ...drive, busyPercent: value };
  return drive;
}

/** The drives with a live batch's newest values (MET-023); the drive list itself changes only with a reload. The input is not changed. */
export function mergeStorage(drives: readonly StorageDrive[], added: NewMetrics): readonly StorageDrive[] {
  return drives.map((drive) => added.samples.reduce(withSample, drive));
}
