import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const contracts = new URL("../../contracts/", import.meta.url);
const read = (path: string): string => readFileSync(new URL(path, contracts), "utf8");
const phase1: unknown = JSON.parse(read("fixtures/phase1.json"));
const phase2: unknown = JSON.parse(read("fixtures/phase2.json"));
const phase3: unknown = JSON.parse(read("fixtures/phase3.json"));

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

const phase3ReferencePaths = [
  "channels.*.id",
  "deliveries.*.channelId",
  "checkNotifications.*.checkId",
  "checkNotifications.*.channelIds",
  "devices.*.siteId",
  "sessions.*.deviceId",
  "shares.*.sessionId",
  "recordings.*.sessionId",
  "recordings.*.deviceId",
  "streams.guacamole.recordingId",
  "streams.asciicast.recordingId",
  "checks.*.deviceId",
].map((p) => p.split("."));

const phase3EnumPaths: Record<string, string> = {
  "channels.*.kind": "ChannelKind",
  "channels.*.settings.method": "WebhookMethod",
  "channels.*.settings.security": "SmtpSecurity",
  "channels.*.settings.mode": "AppriseMode",
  "deliveries.*.kind": "NotificationKind",
  "apiKeys.*.scope": "TokenScope",
  "devices.*.os": "OperatingSystem",
  "devices.*.gpus.*.vendor": "GpuVendor",
  "sessions.*.protocol": "Protocol",
  "sessions.*.endReason": "SessionEndReason",
  "sessions.*.recording": "RecordingState",
  "shares.*.access": "ShareAccess",
  "recordings.*.protocol": "Protocol",
  "recordings.*.format": "RecordingFormat",
  "streams.guacamole.format": "RecordingFormat",
  "streams.asciicast.format": "RecordingFormat",
  "checks.*.type": "CheckType",
  "retention.*.subject": "RetentionSubject",
};

/** Field names that would hold a secret value; the Phase 3 fixtures carry none (SEC-005). */
const SECRET_KEYS = /^(url|urls|token|password|secret|topic|key|headers|headerValues|hash|keyHash)$/i;

/** Every value at a key `at` or ending in `At`, anywhere in the document; null values are left out. */
function timestamps(node: unknown): unknown[] {
  if (Array.isArray(node)) return (node as unknown[]).flatMap(timestamps);
  if (typeof node !== "object" || node === null) return [];
  return Object.entries(node).flatMap(([key, value]) =>
    (key === "at" || key.endsWith("At")) && value !== null ? [value] : timestamps(value),
  );
}

/** Every property name and every string value of the document, skipping the `_comment`. */
function namesAndStrings(node: unknown): { names: string[]; strings: string[] } {
  if (Array.isArray(node)) {
    const parts = (node as unknown[]).map(namesAndStrings);
    return { names: parts.flatMap((p) => p.names), strings: parts.flatMap((p) => p.strings) };
  }
  if (typeof node === "string") return { names: [], strings: [node] };
  if (typeof node !== "object" || node === null) return { names: [], strings: [] };
  const own = Object.entries(node).filter(([key]) => key !== "_comment");
  const parts = own.map(([, value]) => namesAndStrings(value));
  return {
    names: [...own.map(([key]) => key), ...parts.flatMap((p) => p.names)],
    strings: parts.flatMap((p) => p.strings),
  };
}

describe("K6 phase3 fixtures", () => {
  const ids = allIds(phase3);
  const known = new Set([...ids, ...allIds(phase2), ...allIds(phase1)]);

  test("every id is unique and has the Phase 1 pattern", () => {
    expect(ids.length).toBeGreaterThan(0);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.filter((id) => !ID_PATTERN.test(id))).toEqual([]);
  });

  test("no id collides with phase1.json or phase2.json", () => {
    const earlier = new Set([...allIds(phase1), ...allIds(phase2)]);
    expect(ids.filter((id) => earlier.has(id))).toEqual([]);
  });

  test("every reference resolves to an id in phase3.json, phase2.json or phase1.json", () => {
    const refs = phase3ReferencePaths.flatMap((path) => valuesAt(phase3, path)).filter((v) => v !== null);
    expect(refs.length).toBeGreaterThan(0);
    expect(refs.filter((ref) => typeof ref !== "string" || !known.has(ref))).toEqual([]);
  });

  test("every enum-valued field holds a value of its K1 enum, including @pending enums", () => {
    const enums = schemaEnums(read("schema.graphql"));
    const bad: string[] = [];
    for (const [path, name] of Object.entries(phase3EnumPaths)) {
      const allowed = enums.get(name);
      expect(allowed, `enum ${name} in schema.graphql`).toBeDefined();
      const values = valuesAt(phase3, path.split(".")).filter((v) => v !== null);
      expect(values.length, `values at ${path}`).toBeGreaterThan(0);
      for (const value of values) {
        if (typeof value !== "string" || !allowed?.has(value)) bad.push(`${path}: ${String(value)}`);
      }
    }
    expect(bad).toEqual([]);
  });

  test("SEC-005: no secret-looking field exists, and every URL is an origin", () => {
    const { names, strings } = namesAndStrings(phase3);
    expect(names.filter((name) => SECRET_KEYS.test(name))).toEqual([]);
    const urls = strings.filter((v) => /^[a-z]+:\/\//.test(v));
    expect(urls.length).toBeGreaterThan(0);
    expect(urls.filter((u) => !/^https:\/\/[a-z0-9.-]+\.example\.net$/.test(u))).toEqual([]);
    expect(strings.filter((v) => /anima_[A-Za-z0-9_-]{20,}/.test(v))).toEqual([]);
  });

});

describe("K6 phase3 fixtures: timestamps and coverage", () => {
  test("every timestamp is RFC 3339 UTC with zero-padded fields", () => {
    const stamp = /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/;
    for (const [name, doc] of [["phase1", phase1], ["phase2", phase2], ["phase3", phase3]] as const) {
      const bad = timestamps(doc).filter((v) => typeof v !== "string" || !stamp.test(v) || Number.isNaN(Date.parse(v)));
      expect(bad, name).toEqual([]);
    }
    expect(timestamps(phase3).length).toBeGreaterThan(0);
  });

  test("every NotificationKind has one ok and one failed delivery", () => {
    const kinds = [...(schemaEnums(read("schema.graphql")).get("NotificationKind") ?? [])];
    expect(kinds.length).toBeGreaterThan(0);
    const rows = valuesAt(phase3, ["deliveries"]) as { kind: string; ok: boolean }[];
    const missing = kinds.flatMap((kind) =>
      [true, false].filter((ok) => !rows.some((r) => r.kind === kind && r.ok === ok)).map((ok) => `${kind} ok=${String(ok)}`),
    );
    expect(missing).toEqual([]);
  });

  test("the fixtures cover each channel kind, each delivery outcome and the stopped recording", () => {
    const covers = (path: string, wanted: string[]): string[] =>
      wanted.filter((w) => !valuesAt(phase3, path.split(".")).includes(w));
    expect(covers("channels.*.kind", ["WEBHOOK", "NTFY", "EMAIL", "APPRISE"])).toEqual([]);
    expect(valuesAt(phase3, ["deliveries", "*", "ok"]).includes(false)).toBe(true);
    expect(valuesAt(phase3, ["deliveries", "*", "ok"]).includes(true)).toBe(true);
    expect(covers("devices.*.gpus.*.vendor", ["AMD", "APPLE"])).toEqual([]);
    expect(covers("recordings.*.stoppedReason", ["size_limit"])).toEqual([]);
  });
});
