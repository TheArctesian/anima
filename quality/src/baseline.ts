import type { Violation } from "./report.ts";

export const SIGNALS = ["file-lines", "folder-files"] as const;
export type Signal = (typeof SIGNALS)[number];
/** Path (file or folder) -> signals it exceeds and the value recorded. */
export type Baseline = Record<string, Partial<Record<Signal, number>>>;

export class BaselineShapeError extends Error {
  constructor(message: string) {
    super(`baseline.json: ${message}`);
    this.name = "BaselineShapeError";
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseEntry(path: string, entry: unknown): Baseline[string] {
  if (!isRecord(entry)) throw new BaselineShapeError(`${path}: entry must be an object`);
  const signals: Baseline[string] = {};
  for (const [signal, value] of Object.entries(entry)) {
    if (!SIGNALS.includes(signal as Signal)) {
      throw new BaselineShapeError(`${path}: unknown signal ${signal}`);
    }
    if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
      throw new BaselineShapeError(`${path}: ${signal} must be a non-negative integer`);
    }
    signals[signal as Signal] = value;
  }
  return signals;
}

export function parseBaseline(text: string): Baseline {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new BaselineShapeError("not valid JSON");
  }
  if (!isRecord(parsed)) throw new BaselineShapeError("top level must be an object");
  const baseline: Baseline = {};
  for (const [path, entry] of Object.entries(parsed)) {
    baseline[path] = parseEntry(path, entry);
  }
  return baseline;
}

export function baselineValue(baseline: Baseline, path: string, signal: Signal): number | undefined {
  return Object.hasOwn(baseline, path) ? baseline[path]?.[signal] : undefined;
}

/** CQ-011: a new path, a new signal or a larger value is a violation; removals are not. */
export function growth(base: Baseline, current: Baseline): Violation[] {
  const violations: Violation[] = [];
  for (const [path, signals] of Object.entries(current)) {
    for (const signal of SIGNALS) {
      const value = signals[signal];
      if (value === undefined) continue;
      const before = baselineValue(base, path, signal) ?? 0;
      if (value > before) {
        violations.push({ path, signal, detail: `${value} > ${before}` });
      }
    }
  }
  return violations;
}
