// deploy/bin/preflight, checked on plain inputs (NET-003, DB-002). Nothing here needs Docker.
import { describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SCRIPT = join(import.meta.dir, "..", "bin", "preflight");

function preflight(...args: string[]): { ok: boolean; message: string } {
  const result = Bun.spawnSync(["sh", SCRIPT, ...args], { stdin: "ignore" });
  return { ok: result.exitCode === 0, message: result.stderr.toString() };
}

describe("preflight address", () => {
  test.each(["100.64.0.1", "100.100.1.2", "100.127.255.255"])("NET-003 accepts the VPN address %s", (ip) => {
    expect(preflight("address", ip).ok).toBe(true);
  });

  test.each(["0.0.0.0", "127.0.0.1", "10.0.0.1", "100.63.255.255", "100.128.0.0", "203.0.113.5", "::1"])(
    "NET-003 refuses %s",
    (ip) => {
      expect(preflight("address", ip).ok).toBe(false);
    },
  );

  test.each(["100.064.0.1", "100.64.00.1", "100.64.0.01"])("NET-003 refuses %s with a leading zero", (ip) => {
    const result = preflight("address", ip);
    expect(result.ok).toBe(false);
    expect(result.message).toContain("leading zero");
  });

  test.each(["", "abc", "100.64.0", "100.64.0.1.2", "100.64.0.256", "100..0.1", "100.64.0.1/10"])(
    "NET-003 refuses %p as not an IPv4 address",
    (ip) => {
      const result = preflight("address", ip);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("not an IPv4 address");
    },
  );
});

describe("preflight docker", () => {
  test.each(["28.0.0", "29.9.2", "100.1"])("NET-003 accepts Docker %s", (version) => {
    expect(preflight("docker", version).ok).toBe(true);
  });

  test.each(["27.5.1", "20.10.24", "", "dev"])("NET-003 refuses Docker %p", (version) => {
    expect(preflight("docker", version).ok).toBe(false);
  });
});

describe("preflight secrets", () => {
  const names = ["postgres_password", "owner_password", "hub_password", "owner_database_url", "hub_database_url", "anima_key", "cloudflare_dns_token"];

  test("DB-002 accepts a folder with every secret file and names the one that is missing", () => {
    const folder = mkdtempSync(join(tmpdir(), "anima-preflight-"));
    try {
      for (const name of names.slice(1)) writeFileSync(join(folder, name), "x");
      const missing = preflight("secrets", folder);
      expect(missing.ok).toBe(false);
      expect(missing.message).toContain("postgres_password");
      writeFileSync(join(folder, "postgres_password"), "");
      expect(preflight("secrets", folder).ok).toBe(false);
      writeFileSync(join(folder, "postgres_password"), "x");
      expect(preflight("secrets", folder).ok).toBe(true);
      writeFileSync(join(folder, "cloudflare_dns_token"), "");
      expect(preflight("secrets", folder).message).toContain("cloudflare_dns_token");
    } finally {
      rmSync(folder, { recursive: true, force: true });
    }
  });
});
