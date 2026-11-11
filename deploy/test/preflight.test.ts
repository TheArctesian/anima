// deploy/bin/preflight, checked on plain inputs (NET-003, DB-002). Nothing here needs Docker.
import { describe, expect, test } from "bun:test";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
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

describe("preflight public", () => {
  test.each(["31.220.53.226", "8.8.8.8", "100.128.0.1", "172.32.0.1"])("NET-003 accepts the public address %s", (ip) => {
    expect(preflight("public", ip).ok).toBe(true);
  });

  test.each(["100.64.0.1", "100.127.255.255", "192.0.2.1", "198.18.0.1", "198.51.100.1", "203.0.113.5", "10.0.0.1", "172.16.0.1", "172.31.255.1", "192.168.1.1", "127.0.0.1", "169.254.1.1", "0.0.0.0"])(
    "NET-003 refuses %s as not public",
    (ip) => {
      const result = preflight("public", ip);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("not a public address");
    },
  );

  test.each(["", "abc", "31.220.53", "31.220.53.256", "31.220.053.226", "31.220.53.226/32"])("NET-003 refuses %p as an invalid address", (ip) => {
    expect(preflight("public", ip).ok).toBe(false);
  });
});

describe("preflight docker", () => {
  test.each(["28.0.0", "29.9.2", "100.1"])("NET-003 accepts Docker %s", (version) => {
    expect(preflight("docker", version).ok).toBe(true);
  });

  test.each(["27.5.1", "20.10.24", "", "dev"])("NET-003 refuses Docker %p", (version) => {
    expect(preflight("docker", version).ok).toBe(false);
  });
});

const names = ["postgres_password", "owner_password", "hub_password", "owner_database_url", "hub_database_url", "anima_key", "cloudflare_dns_token", "r2_access_key_id", "r2_secret_access_key"];
const modeOf = (name: string) => (name === "cloudflare_dns_token" ? 0o400 : 0o440);

function write(folder: string, name: string, content = "x", mode = modeOf(name)): void {
  rmSync(join(folder, name), { force: true });
  writeFileSync(join(folder, name), content);
  chmodSync(join(folder, name), mode);
}

function withFolder(body: (folder: string) => void): void {
  const folder = mkdtempSync(join(tmpdir(), "anima-preflight-"));
  try {
    body(folder);
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
}

describe("preflight secrets", () => {
  test("DB-002 accepts a folder with every secret file and names the one that is missing", () => {
    withFolder((folder) => {
      for (const name of names.slice(1)) write(folder, name);
      const missing = preflight("secrets", folder);
      expect(missing.ok).toBe(false);
      expect(missing.message).toContain("postgres_password");
      write(folder, "postgres_password", "");
      expect(preflight("secrets", folder).ok).toBe(false);
      write(folder, "postgres_password");
      expect(preflight("secrets", folder).ok).toBe(true);
      write(folder, "cloudflare_dns_token", "");
      expect(preflight("secrets", folder).message).toContain("cloudflare_dns_token");
    });
  });

  test.each([0o600, 0o444, 0o640, 0o400])("DB-002 refuses a generated file with mode %o and says to rerun make-secrets", (mode) => {
    withFolder((folder) => {
      for (const name of names) write(folder, name);
      write(folder, "hub_password", "secret-value-1", mode);
      const result = preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("hub_password");
      expect(result.message).toContain("mode 0440");
      expect(result.message).toContain("make-secrets");
      expect(result.message).not.toContain("secret-value-1");
    });
  });

  test.each([0o440, 0o600, 0o444])("DB-002 refuses the Cloudflare token with mode %o and prints no value", (mode) => {
    withFolder((folder) => {
      for (const name of names) write(folder, name);
      write(folder, "cloudflare_dns_token", "token-value-1", mode);
      const result = preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("mode 0400");
      expect(result.message).toContain("make-secrets");
      expect(result.message).not.toContain("token-value-1");
    });
  });
});

describe("preflight all", () => {
  test("NET-003 refuses a private VPS public address before it looks at Docker", () => {
    const result = Bun.spawnSync(["sh", SCRIPT, "all"], {
      stdin: "ignore",
      env: { PATH: process.env["PATH"] ?? "", ANIMA_HUB_ADDRESS: "100.64.0.5", ANIMA_VPS_PUBLIC_ADDRESS: "10.0.0.1" },
    });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString() + result.stdout.toString()).toContain("not a public address");
  });
});

describe("preflight R2 token files (R-108)", () => {
  test.each(["r2_access_key_id", "r2_secret_access_key"])("R-108 names a missing %s and points at the README, not make-secrets", (name) => {
    withFolder((folder) => {
      for (const other of names.filter((n) => n !== name)) write(folder, other);
      const result = preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain(name);
      expect(result.message).toContain("Release publishing");
    });
  });

  test.each([0o600, 0o444, 0o400])("R-108 refuses the R2 token file with mode %o and prints no value", (mode) => {
    withFolder((folder) => {
      for (const name of names) write(folder, name);
      write(folder, "r2_secret_access_key", "r2-value-1", mode);
      const result = preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("mode 0440");
      expect(result.message).not.toContain("r2-value-1");
    });
  });
});
