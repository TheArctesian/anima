import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const contracts = new URL("../../contracts/", import.meta.url);
const read = (path: string): string => readFileSync(new URL(path, contracts), "utf8");
const phase1: unknown = JSON.parse(read("fixtures/phase1.json"));
const phase2: unknown = JSON.parse(read("fixtures/phase2.json"));

const ID_PATTERN = /^0190a000-0000-7000-80[0-9a-f]{2}-[0-9a-f]{12}$/;

/** Enum name to its values, from `enum Name ... { VALUE ... }` blocks of K1. */
function schemaEnums(sdl: string): Map<string, Set<string>> {
  const enums = new Map<string, Set<string>>();
  for (const block of sdl.matchAll(/^enum (\w+)[^{]*\{([^}]*)\}/gm)) {
    const values = (block[2] ?? "")
      .split("\n")
      .map((line) => line.replace(/#.*$/, "").trim())
      .filter((line) => /^[A-Z][A-Z0-9_]*\b/.test(line) && !line.startsWith('"'))
      .map((line) => line.split(/\s/)[0] ?? "");
    enums.set(block[1] ?? "", new Set(values));
  }
  return enums;
}

/** Every value found by following `path`; `*` fans out over arrays. Arrays at the leaf are flattened. */
function valuesAt(node: unknown, path: readonly string[]): unknown[] {
  const [head, ...rest] = path;
  if (head === undefined) return Array.isArray(node) ? (node as unknown[]) : [node];
  if (head === "*") return Array.isArray(node) ? (node as unknown[]).flatMap((n) => valuesAt(n, rest)) : [];
  if (typeof node !== "object" || node === null || !(head in node)) return [];
  return valuesAt((node as Record<string, unknown>)[head], rest);
}

/** Every `id` property anywhere in the document. */
function allIds(node: unknown): string[] {
  if (Array.isArray(node)) return (node as unknown[]).flatMap(allIds);
  if (typeof node !== "object" || node === null) return [];
  return Object.entries(node).flatMap(([key, value]) =>
    key === "id" && typeof value === "string" ? [value] : allIds(value),
  );
}

const referencePaths = [
  "sites.*.wakeRelayDeviceId",
  "devices.*.siteId",
  "devices.*.webInterfaces.*.deviceId",
  "checks.*.deviceId",
  "sessions.*.deviceId",
  "metrics.*.deviceId",
].map((p) => p.split("."));

/** Fixture path to the K1 enum its values must come from. */
const enumPaths: Record<string, string> = {
  "devices.*.rdp.security": "RdpSecurity",
  "devices.*.rdp.resizeMethod": "RdpResizeMethod",
  "devices.*.metricsSource": "MetricsSource",
  "devices.*.accessWarnings": "AccessWarning",
  "devices.*.os": "OperatingSystem",
  "devices.*.gpus.*.vendor": "GpuVendor",
  "devices.*.webInterfaces.*.openIn": "OpenIn",
  "checks.*.type": "CheckType",
  "checks.*.config.snmp.version": "SnmpVersion",
  "sessions.*.protocol": "Protocol",
  "sessions.*.endReason": "SessionEndReason",
  "metrics.*.series.*.kind": "MetricKind",
  "metrics.*.series.*.range": "MetricRange",
};

describe("K6 phase2 fixtures", () => {
  const ids = allIds(phase2);
  const known = new Set([...ids, ...allIds(phase1)]);

  test("every id is unique and has the Phase 1 pattern", () => {
    expect(ids.length).toBeGreaterThan(0);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.filter((id) => !ID_PATTERN.test(id))).toEqual([]);
  });

  test("no id collides with phase1.json", () => {
    const phase1Ids = new Set(allIds(phase1));
    expect(ids.filter((id) => phase1Ids.has(id))).toEqual([]);
  });

  test("every reference resolves to an id in phase2.json or phase1.json", () => {
    const refs = referencePaths.flatMap((path) => valuesAt(phase2, path)).filter((v) => v !== null);
    expect(refs.length).toBeGreaterThan(0);
    expect(refs.filter((ref) => typeof ref !== "string" || !known.has(ref))).toEqual([]);
  });

  test("every enum-valued field holds a value of its K1 enum", () => {
    const enums = schemaEnums(read("schema.graphql"));
    const bad: string[] = [];
    for (const [path, name] of Object.entries(enumPaths)) {
      const allowed = enums.get(name);
      expect(allowed, `enum ${name} in schema.graphql`).toBeDefined();
      const values = valuesAt(phase2, path.split(".")).filter((v) => v !== null);
      expect(values.length, `values at ${path}`).toBeGreaterThan(0);
      for (const value of values) {
        if (typeof value !== "string" || !allowed?.has(value)) bad.push(`${path}: ${String(value)}`);
      }
    }
    expect(bad).toEqual([]);
  });

  test("the fixtures cover each new check type, protocol and the certificate mismatch end", () => {
    const covers = (path: string, wanted: string[]): string[] =>
      wanted.filter((w) => !valuesAt(phase2, path.split(".")).includes(w));
    expect(covers("checks.*.type", ["DOCKER", "SNMP", "TAILSCALE_PING", "SYSTEMD"])).toEqual([]);
    expect(covers("sessions.*.protocol", ["SSH", "RDP", "VNC"])).toEqual([]);
    expect(covers("sessions.*.endReason", ["SERVER_CERTIFICATE_MISMATCH"])).toEqual([]);
  });
});
