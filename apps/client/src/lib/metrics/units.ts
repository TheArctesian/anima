/** A reading as text; `null` (no value yet) is a dash, never 0 (MET-007). */
export type UnitFormatter = (value: number | null) => string;

/** MET-004: a metric strictly above this is degraded. */
export const DEGRADED_ABOVE = 85;

const UNKNOWN = '—';
const IEC = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'] as const;

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

/** Load average with two decimals: "0.42". */
export const formatLoad: UnitFormatter = (value) => (known(value) ? value.toFixed(2) : UNKNOWN);
