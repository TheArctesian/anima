/** A reading as text; `null` (no value yet) is a dash, never 0 (MET-007). */
export type UnitFormatter = (value: number | null) => string;

const UNKNOWN = '—';
const IEC = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'] as const;
const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

function known(value: number | null): value is number {
  return value !== null && Number.isFinite(value);
}

/** Whole percent: "42 %", the style of the device card (MET-004). */
export const formatPercent: UnitFormatter = (value) => (known(value) ? `${String(Math.round(value))} %` : UNKNOWN);

/** IEC bytes with one decimal above 1 KiB: "512 B", "1.5 GiB". */
export const formatBytes: UnitFormatter = (value) => {
  if (!known(value)) return UNKNOWN;
  let scaled = Math.abs(value);
  let step = 0;
  while (scaled >= 1024 && step < IEC.length - 1) {
    scaled /= 1024;
    step += 1;
  }
  const sign = value < 0 ? '-' : '';
  const digits = step === 0 ? String(Math.round(scaled)) : scaled.toFixed(1);
  return `${sign}${digits} ${IEC[step]}`;
};

/** IEC bytes per second: "1.5 MiB/s". */
export const formatBytesPerSecond: UnitFormatter = (value) => (known(value) ? `${formatBytes(value)}/s` : UNKNOWN);

/** Seconds as the two largest units: "3 d 4 h", "5 h 12 min", "42 min", "30 s". */
export const formatUptime: UnitFormatter = (value) => {
  if (!known(value) || value < 0) return UNKNOWN;
  const seconds = Math.floor(value);
  if (seconds >= DAY) return `${String(Math.floor(seconds / DAY))} d ${String(Math.floor((seconds % DAY) / HOUR))} h`;
  if (seconds >= HOUR) return `${String(Math.floor(seconds / HOUR))} h ${String(Math.floor((seconds % HOUR) / MINUTE))} min`;
  if (seconds >= MINUTE) return `${String(Math.floor(seconds / MINUTE))} min`;
  return `${String(seconds)} s`;
};

/** Load average with two decimals: "0.42". */
export const formatLoad: UnitFormatter = (value) => (known(value) ? value.toFixed(2) : UNKNOWN);
