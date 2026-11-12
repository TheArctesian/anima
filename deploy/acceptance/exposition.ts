// The K7 rules for the hub's Prometheus exposition (K7 "Prometheus series", INT-001, INT-002, D-24), as pure
// verdicts on a parsed text. A verdict is `undefined` when the rule holds, else a fixed reason that never quotes
// the text: a label value is the operator's own device or check name.
import type { Family } from "./prometheus-text.ts";

const CHECK_LABELS = ["check_id", "check_name", "check_type", "device_id", "device_name", "site"] as const;
const DEVICE_LABELS = ["device_id", "device_name", "site"] as const;

/** Each K7 family and its exact label names (D-24); every one is a gauge. */
export const K7_FAMILIES: Readonly<Record<string, readonly string[]>> = {
  anima_check_status: CHECK_LABELS,
  anima_check_up: CHECK_LABELS,
  anima_check_response_time_seconds: CHECK_LABELS,
  anima_check_uptime_ratio: [...CHECK_LABELS, "window"],
  anima_check_certificate_days_left: CHECK_LABELS,
  anima_check_domain_days_left: CHECK_LABELS,
  anima_device_status: DEVICE_LABELS,
};

/** The three `window` values of `anima_check_uptime_ratio` (CHK-011). */
export const WINDOWS: readonly string[] = ["1d", "7d", "30d"];

/** The label names K7 states; only these may be named in a reason, any other name is the hub's text. */
const K7_LABEL_NAMES: Readonly<Record<string, true>> = Object.fromEntries(
  [...CHECK_LABELS, "window"].map((label) => [label, true]),
);

/** The labels that hold a hub ID (a UUID): never a changing value. */
const ID_LABELS = new Set(["check_id", "device_id", "site"]);
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

// Shapes no label may hold (K7: no status, time, address or message; D-24: nothing that changes often). A
// heuristic: the exact D-24 label names are the hard rule (S-C7).
const VOLATILE: readonly RegExp[] = [
  /\d{4}-\d{2}-\d{2}/, // a date, with or without a time
  /\d{1,2}:\d{2}(?::\d{2})?/, // a time of day
  /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$/, // a bare number: a count, a Unix time
  /^\d+(?:\.\d+)?(?:ns|us|µs|ms|s|m|h|d|w|y)$/, // a duration, or epoch seconds with `s`
  /^\d{1,3}(?:\.\d{1,3}){3}(?:\/\d{1,2})?(?::\d+)?$/, // an IPv4 address or CIDR
  /^\[?[0-9a-fA-F]*:[0-9a-fA-F]*:[0-9a-fA-F:.]*\]?(?:\/\d{1,3})?$/, // an IPv6 address or CIDR
  /^\[?[A-Za-z0-9.:-]+\]?:\d{1,5}$/, // host:port
  /^(?:up|down|degraded|maintenance|unknown|paused)$/i, // a status
];

/** Every K7 family is declared as a gauge, and no other family is. */
export function familiesVerdict(families: ReadonlyMap<string, Family>): string | undefined {
  for (const name of Object.keys(K7_FAMILIES)) {
    const family = families.get(name);
    if (family === undefined) {
      return `the family ${name} is missing`;
    }
    if (family.type !== "gauge") {
      return `the family ${name} is not a gauge`;
    }
  }
  const extra = [...families.keys()].some((name) => !(name in K7_FAMILIES));
  return extra ? "the text has a family K7 does not list" : undefined;
}

/** Every sample of a K7 family has exactly its D-24 label names, and `window` only its three values. */
export function labelsVerdict(families: ReadonlyMap<string, Family>): string | undefined {
  for (const [name, expected] of Object.entries(K7_FAMILIES)) {
    for (const sample of families.get(name)?.samples ?? []) {
      const names = sample.labels.map(([label]) => label);
      if (names.length !== expected.length || !expected.every((label) => names.includes(label))) {
        return `a sample of ${name} has other labels than D-24 states`;
      }
      const window = sample.labels.find(([label]) => label === "window")?.[1];
      if (window !== undefined && !WINDOWS.includes(window)) {
        return "a window label is not 1d, 7d or 30d";
      }
    }
  }
  return undefined;
}

/** Whether one label value is fit for its label: an ID label holds a UUID, no label a volatile shape. */
export function steadyLabel(label: string, value: string): boolean {
  if (ID_LABELS.has(label)) {
    return UUID.test(value);
  }
  return label === "window" ? WINDOWS.includes(value) : !VOLATILE.some((shape) => shape.test(value));
}

/** No label value of any sample looks like a time, a changing number, an address or a status. */
export function labelValuesVerdict(families: ReadonlyMap<string, Family>): string | undefined {
  for (const family of families.values()) {
    for (const sample of family.samples) {
      const unsteady = sample.labels.find(([label, value]) => !steadyLabel(label, value));
      if (unsteady !== undefined) {
        const label = unsteady[0] in K7_LABEL_NAMES ? unsteady[0] : "label";
        return `a ${label} value is not steady: an ID that is not a UUID, or a time, number, address or status`;
      }
    }
  }
  return undefined;
}
