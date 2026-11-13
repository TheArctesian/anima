// deploy/bin/preflight, checked on plain inputs (NET-003, DB-002). Nothing here needs Docker.
import { describe, expect, test } from "bun:test";
import { chmodSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { finish } from "./finished-process.ts";
import { SCRIPT, names, preflight, secrets, withFolder, write } from "./preflight-fixture.ts";

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

describe("preflight secrets", () => {
  test("DB-002 accepts a folder with every secret file and names the one that is missing", async () => {
    await withFolder(async (folder) => {
      for (const name of names.slice(1)) write(folder, name);
      const missing = await secrets(folder);
      expect(missing.ok).toBe(false);
      expect(missing.message).toContain("postgres_password");
      write(folder, "postgres_password", "");
      expect((await secrets(folder)).ok).toBe(false);
      write(folder, "postgres_password");
      expect((await secrets(folder)).ok).toBe(true);
      write(folder, "cloudflare_dns_token", "");
      expect((await secrets(folder)).message).toContain("cloudflare_dns_token");
    });
  });

  test.each([0o600, 0o444, 0o640, 0o400])("DB-002 refuses a generated file with mode %o and says to rerun make-secrets", async (mode) => {
    await withFolder(async (folder) => {
      for (const name of names) write(folder, name);
      write(folder, "hub_password", "secret-value-1", mode);
      const result = await secrets(folder);
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
      const result = await secrets(folder);
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

describe("preflight host (C-8)", () => {
  test("C-8 host checks the .env settings as the operator, with no Docker and no secrets folder", async () => {
    await withFolder(async (bin) => {
      writeFileSync(join(bin, "docker"), "#!/bin/sh\necho docker was asked >&2\nexit 1\n");
      chmodSync(join(bin, "docker"), 0o755);
      const path = `${bin}:${process.env["PATH"] ?? ""}`;
      const env = { PATH: path, ANIMA_HUB_ADDRESS: "100.64.0.5", ANIMA_VPS_PUBLIC_ADDRESS: "31.220.53.226" };
      const ok = await finish(["sh", SCRIPT, "host"], { env });
      expect(ok.stderr).toBe("");
      expect(ok.stdout).toBe("preflight: host ok\n");
      const refused = await finish(["sh", SCRIPT, "host"], { env: { ...env, ANIMA_HUB_ADDRESS: "10.0.0.1" } });
      expect(refused.exitCode).toBe(1);
      expect(refused.stderr).toContain("not in 100.64.0.0/10");
      const asked = await finish(["sh", SCRIPT, "docker"], { env });
      expect(asked.stderr).toContain("docker was asked");
    });
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
      const result = await secrets(folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain(name);
      expect(result.message).toContain("Release publishing");
    });
  });

  test.each([0o600, 0o444, 0o400])("R-108 refuses the R2 token file with mode %o and prints no value", async (mode) => {
    await withFolder(async (folder) => {
      for (const name of names) write(folder, name);
      write(folder, "r2_secret_access_key", "r2-value-1", mode);
      const result = await secrets(folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("mode 0440");
      expect(result.message).not.toContain("r2-value-1");
    });
  });
});
