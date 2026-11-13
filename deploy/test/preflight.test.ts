// deploy/bin/preflight, checked on plain inputs (NET-003, DB-002). Nothing here needs Docker.
import { describe, expect, test } from "bun:test";
import { chmodSync, chownSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { finish } from "./finished-process.ts";

const SCRIPT = join(import.meta.dir, "..", "bin", "preflight");

/** The test's own user and group stand in for root and the hub's group 10001. */
const OWNERS = { ANIMA_SECRETS_OWNER: String(process.getuid?.()), ANIMA_SECRETS_GROUP: String(process.getgid?.()) };

async function preflight(...args: string[]): Promise<{ ok: boolean; message: string }> {
  const result = await finish(["sh", SCRIPT, ...args], { env: { ...process.env, ...OWNERS } });
  return { ok: result.exitCode === 0, message: result.stderr };
}

describe("preflight address", () => {
  test.each(["100.64.0.1", "100.100.1.2", "100.127.255.255"])("NET-003 accepts the VPN address %s", async (ip) => {
    expect((await preflight("address", ip)).ok).toBe(true);
  });

  test.each(["0.0.0.0", "127.0.0.1", "10.0.0.1", "100.63.255.255", "100.128.0.0", "203.0.113.5", "::1"])(
    "NET-003 refuses %s",
    async (ip) => {
      expect((await preflight("address", ip)).ok).toBe(false);
    },
  );

  test.each(["100.064.0.1", "100.64.00.1", "100.64.0.01"])("NET-003 refuses %s with a leading zero", async (ip) => {
    const result = await preflight("address", ip);
    expect(result.ok).toBe(false);
    expect(result.message).toContain("leading zero");
  });

  test.each(["", "abc", "100.64.0", "100.64.0.1.2", "100.64.0.256", "100..0.1", "100.64.0.1/10"])(
    "NET-003 refuses %p as not an IPv4 address",
    async (ip) => {
      const result = await preflight("address", ip);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("not an IPv4 address");
    },
  );
});

describe("preflight public", () => {
  test.each(["31.220.53.226", "8.8.8.8", "100.128.0.1", "172.32.0.1"])("NET-003 accepts the public address %s", async (ip) => {
    expect((await preflight("public", ip)).ok).toBe(true);
  });

  test.each(["100.64.0.1", "100.127.255.255", "192.0.2.1", "198.18.0.1", "198.51.100.1", "203.0.113.5", "10.0.0.1", "172.16.0.1", "172.31.255.1", "192.168.1.1", "127.0.0.1", "169.254.1.1", "0.0.0.0"])(
    "NET-003 refuses %s as not public",
    async (ip) => {
      const result = await preflight("public", ip);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("not a public address");
    },
  );

  test.each(["", "abc", "31.220.53", "31.220.53.256", "31.220.053.226", "31.220.53.226/32"])("NET-003 refuses %p as an invalid address", async (ip) => {
    expect((await preflight("public", ip)).ok).toBe(false);
  });
});

describe("preflight docker", () => {
  test.each(["28.0.0", "29.9.2", "100.1"])("NET-003 accepts Docker %s", async (version) => {
    expect((await preflight("docker", version)).ok).toBe(true);
  });

  test.each(["27.5.1", "20.10.24", "", "dev"])("NET-003 refuses Docker %p", async (version) => {
    expect((await preflight("docker", version)).ok).toBe(false);
  });
});

const names = ["postgres_password", "owner_password", "hub_password", "owner_database_url", "hub_database_url", "anima_key", "cloudflare_dns_token", "r2_access_key_id", "r2_secret_access_key"];
const modeOf = (name: string) => (name === "cloudflare_dns_token" ? 0o400 : 0o440);

function write(folder: string, name: string, content = "x", mode = modeOf(name)): void {
  rmSync(join(folder, name), { force: true });
  writeFileSync(join(folder, name), content);
  chmodSync(join(folder, name), mode);
}

async function withFolder(body: (folder: string) => Promise<void>): Promise<void> {
  const folder = mkdtempSync(join(tmpdir(), "anima-preflight-"));
  try {
    await body(folder);
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
}

describe("preflight secrets", () => {
  test("DB-002 accepts a folder with every secret file and names the one that is missing", async () => {
    await withFolder(async (folder) => {
      for (const name of names.slice(1)) write(folder, name);
      const missing = await preflight("secrets", folder);
      expect(missing.ok).toBe(false);
      expect(missing.message).toContain("postgres_password");
      write(folder, "postgres_password", "");
      expect((await preflight("secrets", folder)).ok).toBe(false);
      write(folder, "postgres_password");
      expect((await preflight("secrets", folder)).ok).toBe(true);
      write(folder, "cloudflare_dns_token", "");
      expect((await preflight("secrets", folder)).message).toContain("cloudflare_dns_token");
    });
  });

  test.each([0o600, 0o444, 0o640, 0o400])("DB-002 refuses a generated file with mode %o and says to rerun make-secrets", async (mode) => {
    await withFolder(async (folder) => {
      for (const name of names) write(folder, name);
      write(folder, "hub_password", "secret-value-1", mode);
      const result = await preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("hub_password");
      expect(result.message).toContain("mode 0440");
      expect(result.message).toContain("make-secrets");
      expect(result.message).not.toContain("secret-value-1");
    });
  });

  test.each([0o440, 0o600, 0o444])("DB-002 refuses the Cloudflare token with mode %o and prints no value", async (mode) => {
    await withFolder(async (folder) => {
      for (const name of names) write(folder, name);
      write(folder, "cloudflare_dns_token", "token-value-1", mode);
      const result = await preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("mode 0400");
      expect(result.message).toContain("make-secrets");
      expect(result.message).not.toContain("token-value-1");
    });
  });
});

async function networks(env: Record<string, string>): Promise<{ ok: boolean; message: string }> {
  const result = await finish(["sh", SCRIPT, "networks"], { env: { PATH: process.env["PATH"] ?? "", ...env } });
  return { ok: result.exitCode === 0, message: result.stderr };
}

describe("preflight networks (VP-13)", () => {
  test("VP-13 accepts the compose defaults and other subnets inside 172.30.0.0/16", async () => {
    expect(await networks({})).toEqual({ ok: true, message: "" });
    expect((await networks({ ANIMA_DATABASE_SUBNET: "172.30.9.0/24", ANIMA_ACME_SUBNET: "172.30.200.0/28" })).ok).toBe(true);
  });

  test.each([
    ["ANIMA_HUB_SUBNET", "10.0.0.0/24"],
    ["ANIMA_DATABASE_SUBNET", "172.31.1.0/24"],
    ["ANIMA_LOCALAPI_SUBNET", "172.16.0.0/12"],
    ["ANIMA_ACME_SUBNET", "172.17.0.0/16"],
    ["ANIMA_HUB_IP_RANGE", "192.168.0.0/25"],
  ])("VP-13 refuses %s %s outside 172.30.0.0/16, which the VPS firewall keeps off the VPN", async (name, value) => {
    const result = await networks({ [name]: value });
    expect(result.ok).toBe(false);
    expect(result.message).toContain(`${name} '${value}' is not inside 172.30.0.0/16`);
  });

  test.each([["172.30.1.0"], ["172.30.1.0/x"], ["172.30.01.0/24"]])("VP-13 refuses the subnet %p as not a network", async (value) => {
    expect((await networks({ ANIMA_ACME_SUBNET: value })).ok).toBe(false);
  });

  test("VP-13 refuses guacd on the hub's address", async () => {
    const result = await networks({ ANIMA_GUACD_IP: "172.30.0.10" });
    expect(result.ok).toBe(false);
    expect(result.message).toContain("ANIMA_GUACD_IP is ANIMA_HUB_IP");
  });

  test.each([
    [{ ANIMA_HUB_IP: "172.30.0.130" }, "ANIMA_HUB_IP"],
    [{ ANIMA_GUACD_IP: "172.30.0.255" }, "ANIMA_GUACD_IP"],
    [{ ANIMA_HUB_IP_RANGE: "172.30.0.0/24" }, "ANIMA_HUB_IP"],
    [{ ANIMA_HUB_IP_RANGE: "172.30.0.16/28" }, "ANIMA_GUACD_IP"],
  ])("VP-13 refuses a fixed address inside the dynamic range: %p", async (env, name) => {
    const result = await networks(env);
    expect(result.ok).toBe(false);
    expect(result.message).toContain(`${name} is inside ANIMA_HUB_IP_RANGE`);
  });
});

describe("preflight all", () => {
  test("NET-003 refuses a private VPS public address before it looks at Docker", async () => {
    const result = await finish(["sh", SCRIPT, "all"], {
      env: { PATH: process.env["PATH"] ?? "", ANIMA_HUB_ADDRESS: "100.64.0.5", ANIMA_VPS_PUBLIC_ADDRESS: "10.0.0.1" },
    });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr + result.stdout).toContain("not a public address");
  });
});

describe("preflight R2 token files (R-108)", () => {
  test.each(["r2_access_key_id", "r2_secret_access_key"])("R-108 names a missing %s and points at the README, not make-secrets", async (name) => {
    await withFolder(async (folder) => {
      for (const other of names.filter((n) => n !== name)) write(folder, other);
      const result = await preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain(name);
      expect(result.message).toContain("Release publishing");
    });
  });

  test.each([0o600, 0o444, 0o400])("R-108 refuses the R2 token file with mode %o and prints no value", async (mode) => {
    await withFolder(async (folder) => {
      for (const name of names) write(folder, name);
      write(folder, "r2_secret_access_key", "r2-value-1", mode);
      const result = await preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("mode 0440");
      expect(result.message).not.toContain("r2-value-1");
    });
  });
});

describe("preflight updater key pair (R-116, D-44)", () => {
  const pair = ["updater/updater_key", "updater/updater_key_password"];
  function withPair(folder: string): void {
    mkdirSync(join(folder, "updater"), { recursive: true });
    chmodSync(join(folder, "updater"), 0o750);
    for (const name of [...names, ...pair]) write(folder, name);
  }

  test.each([[pair], [pair.slice(0, 1)], [pair.slice(1)]])("D-44 warns but passes when %p is absent, for the hub starts with publishing off", async (absent) => {
    await withFolder(async (folder) => {
      withPair(folder);
      for (const name of absent) rmSync(join(folder, name));
      const result = await preflight("secrets", folder);
      expect(result.ok).toBe(true);
      expect(result.message).toContain("warning: no updater key pair");
      expect(result.message).toContain("publishing off");
    });
  });

  test("D-44 is silent with the pair, accepts an empty password for a key with none and refuses an empty key", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      expect(await preflight("secrets", folder)).toEqual({ ok: true, message: "" });
      write(folder, "updater/updater_key_password", "");
      expect(await preflight("secrets", folder)).toEqual({ ok: true, message: "" });
      write(folder, "updater/updater_key", "");
      const result = await preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("updater_key is empty");
    });
  });

  test.each([
    ["updater/updater_key", 0o600],
    ["updater/updater_key", 0o444],
    ["updater/updater_key_password", 0o600],
    ["updater/updater_key_password", 0o400],
  ])("D-44 refuses %s with mode %o and prints no value", async (name, mode) => {
    await withFolder(async (folder) => {
      withPair(folder);
      write(folder, name, "updater-value-1", mode);
      const result = await preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain(name);
      expect(result.message).toContain("mode 0440");
      expect(result.message).not.toContain("updater-value-1");
    });
  });

  test.skipIf(process.getuid?.() === 0)("S-3 refuses a folder it cannot search and says to run with sudo, never that the pair is absent", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      chmodSync(join(folder, "updater"), 0o640);
      const result = await preflight("secrets", folder);
      chmodSync(join(folder, "updater"), 0o750);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("run preflight with sudo");
      expect(result.message).not.toContain("no updater key pair");
    });
  });

  test("S-3 refuses an updater folder that is not 0750 and names it", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      chmodSync(join(folder, "updater"), 0o755);
      const result = await preflight("secrets", folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("mode 0750");
    });
  });

  // Another group the test's user is in, so one file can leave the stand-in hub group.
  const otherGroup = (process.getgroups?.() ?? []).find((group) => group !== process.getgid?.());

  test.skipIf(otherGroup === undefined).each(["updater", "updater/updater_key", "updater/updater_key_password", "r2_secret_access_key"])(
    "S-3 refuses %s outside the hub's group and prints no value",
    async (name) => {
      await withFolder(async (folder) => {
        withPair(folder);
        write(folder, "updater/updater_key", "updater-value-1");
        chownSync(join(folder, name), process.getuid?.() ?? 0, otherGroup ?? 0);
        const result = await preflight("secrets", folder);
        expect(result.ok).toBe(false);
        expect(result.message).toContain(`${name} is not`);
        expect(result.message).toContain(`group ${OWNERS.ANIMA_SECRETS_GROUP}`);
        expect(result.message).not.toContain("updater-value-1");
      });
    },
  );
});
