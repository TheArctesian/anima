import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { chmodSync } from "node:fs";
import { join } from "node:path";
import { HARDEN_TIMEOUT_MS, fakeHost, removeHosts, type Host } from "./vps-host.ts";

setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

const DROP_IN = "etc/ssh/sshd_config.d/00-anima.conf";
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

describe("harden.sh ssh", () => {
  test.each([
    ["no authorized_keys", { ANIMA_ADMIN_USER: "nokeys" }],
    ["root", { ANIMA_ADMIN_USER: "root" }],
    ["uid 0 under another name", { ANIMA_ADMIN_USER: "toor" }],
    ["a user only in group sudo, not wheel", { FAKE_GROUPS: "admin sudo users" }],
    ["a user that does not exist", { ANIMA_ADMIN_USER: "ghost" }],
    ["a relative home directory", { ANIMA_ADMIN_USER: "relhome" }],
    ["keys owned by another user", { FAKE_OWNER: "4242" }],
    ["a name that could inject sshd config", { ANIMA_ADMIN_USER: "admin\nPermitRootLogin yes" }],
    ["an unset ANIMA_ADMIN_USER", { ANIMA_ADMIN_USER: "" }],
  ])("VPS-001: refuses %s", (_, env) => {
    const run = host.run(["ssh"], env);
    expect(run.code).toBe(2);
    expect(host.exists(DROP_IN)).toBe(false);
  });

  test("VPS-001: refuses an authorized_keys with no key in it", () => {
    host.write("home/admin/.ssh/authorized_keys", "# no keys yet\n\n", 0o600);
    expect(host.run(["ssh"]).code).toBe(2);
  });

  test.each([
    ["home/admin/.ssh/authorized_keys", 0o620],
    ["home/admin/.ssh", 0o770],
    ["home/admin", 0o757],
  ])("VPS-001: refuses when %s is writable by others (StrictModes)", (path, mode) => {
    chmodSync(join(host.root, path), mode);
    const run = host.run(["ssh"]);
    expect(run.code).toBe(2);
    expect(run.out).toContain("writable");
  });

  test("VPS-001: installs the drop-in with AllowUsers, validates it and reloads sshd", () => {
    const run = host.run(["ssh"]);
    expect(run.code).toBe(0);
    expect(host.read(DROP_IN)).toContain("\nAllowUsers admin anima-probe@172.30.0.10\n");
    expect(run.calls).toEqual(expect.arrayContaining(["SSHD -t", "SSHD -T", "SYSTEMCTL reload sshd.service"]));
    expect(run.out).not.toContain("AAAA");
  });

  // The files an AlmaLinux 10 VPS may carry: the permitrootlogin migration (01- or 25-),
  // Red Hat's defaults (50-redhat.conf, X11Forwarding yes) and cloud-init's.
});

describe("harden.sh ssh against other drop-ins", () => {
  test.each([
    ["01-permitrootlogin.conf", "PermitRootLogin yes\n"],
    ["25-permitrootlogin.conf", "PermitRootLogin yes\n"],
    ["50-redhat.conf", "X11Forwarding yes\nChallengeResponseAuthentication no\n"],
    ["50-cloud-init.conf", "PasswordAuthentication yes\n"],
  ])("VPS-001: wins over %s", (file, text) => {
    host.write(`etc/ssh/sshd_config.d/${file}`, text);
    expect(host.run(["ssh"]).code).toBe(0);
  });

  test("VPS-001: refuses and restores when another drop-in adds AllowUsers", () => {
    host.write("etc/ssh/sshd_config.d/50-cloud-init.conf", "PasswordAuthentication yes\nAllowUsers intruder\n");
    const run = host.run(["ssh"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain('unexpected "allowusers intruder"');
    expect(host.exists(DROP_IN)).toBe(false);
    expect(run.calls).not.toContain("SYSTEMCTL reload sshd.service");
  });

  test("VPS-001: restores the previous drop-in when an earlier file overrides a value", () => {
    host.write(DROP_IN, "# previous\n");
    host.write("etc/ssh/sshd_config.d/0-early.conf", "PermitRootLogin yes\n");
    const run = host.run(["ssh"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain('expected "permitrootlogin no"');
    expect(host.read(DROP_IN)).toBe("# previous\n");
  });

  test.each([
    ["sshd -t fails", { FAKE_SSHD_SYNTAX: "1" }],
    ["systemctl reload sshd fails", { FAKE_RELOAD: "1" }],
  ])("VPS-001: removes the drop-in when %s", (_, env) => {
    expect(host.run(["ssh"], env).code).toBe(1);
    expect(host.exists(DROP_IN)).toBe(false);
  });
});

// The bypass a provider image could carry: sshd -T alone still shows the hardened values.
const PROVIDER_MATCH = "Match Address *\n  PasswordAuthentication yes\n  PermitRootLogin yes\n  AllowUsers root\n";
const PUBLIC_ROOT = "sshd -T -C user=root,host=x,addr=198.51.100.7";

describe("harden.sh ssh and check (Match blocks)", () => {
  test.each([
    ["a drop-in", "etc/ssh/sshd_config.d/90-provider.conf", "Match Address *"],
    ["sshd_config itself", "etc/ssh/sshd_config", "Match All"],
    ["a file included by a relative path", "etc/ssh/sshd_config.d/50-extra.conf", "  match user root"],
  ])("VPS-001: ssh refuses an active Match line in %s", (_, file, match) => {
    if (file.endsWith("50-extra.conf")) {
      host.write("etc/ssh/sshd_config.d/50-extra.conf", "Include nested.conf\n");
      host.write("etc/ssh/nested.conf", `${match}\n  PermitRootLogin yes\n`);
    } else {
      const before = host.exists(file) ? host.read(file) : "";
      host.write(file, `${before}${match}\n  PermitRootLogin yes\n`);
    }
    const run = host.run(["ssh"]);
    expect(run.code).toBe(2);
    expect(run.out).toContain("a Match block can override the hardening");
    expect(host.exists(DROP_IN)).toBe(false);
  });

  test.each([
    ["Match=Address *"],
    ['"Match" Address *'],
    ["\tMATCH all"],
  ])("VPS-001: ssh refuses a Match line in another form: %p", (match) => {
    host.write("etc/ssh/sshd_config.d/90-provider.conf", `${match}\n  PermitRootLogin yes\n`);
    const run = host.run(["ssh"]);
    expect(run.code).toBe(2);
    expect(run.out).toContain("90-provider.conf:1: a Match block can override the hardening");
  });
});

describe("harden.sh ssh and check (Include forms)", () => {
  test.each([
    ["Include=/etc/ssh/provider.conf"],
    ['"Include" /etc/ssh/provider.conf'],
    ['Include "/etc/ssh/provider dir/x.conf"'],
    ["include = /etc/ssh/provider.conf"],
  ])("VPS-001: ssh refuses an Include it cannot follow literally: %p", (include) => {
    host.write("etc/ssh/sshd_config.d/50-provider.conf", `${include}\n`);
    host.write("etc/ssh/provider.conf", "PasswordAuthentication yes\n");
    const run = host.run(["ssh"]);
    expect(run.code).toBe(2);
    expect(run.out).toContain("50-provider.conf:1: an Include this check cannot follow");
    expect(host.exists(DROP_IN)).toBe(false);
  });

  test("VPS-001: ssh follows a plain Include with several patterns and mixed case", () => {
    host.write("etc/ssh/sshd_config.d/50-provider.conf", "  INCLUDE /etc/ssh/a.conf b.conf\n");
    host.write("etc/ssh/b.conf", "Match User root\n");
    expect(host.run(["ssh"]).out).toContain("/etc/ssh/b.conf:1: a Match block");
  });

  test("VPS-001: ssh accepts the stock commented-out Match example", () => {
    expect(host.read("etc/ssh/sshd_config")).toContain("#Match User anoncvs");
    expect(host.run(["ssh"]).code).toBe(0);
  });

  test("VPS-001: check reports the provider's Match block and the values it turns back on", () => {
    host.harden();
    host.write("etc/ssh/sshd_config.d/90-provider.conf", PROVIDER_MATCH);
    const run = host.run(["check"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain("VIOLATION: sshd: /etc/ssh/sshd_config.d/90-provider.conf:1: a Match block");
    expect(run.out).not.toContain("VIOLATION: sshd -T: ");
    expect(run.out).toContain(`VIOLATION: ${PUBLIC_ROOT}: expected "permitrootlogin no"`);
    expect(run.out).toContain(`VIOLATION: ${PUBLIC_ROOT}: expected "passwordauthentication no"`);
    expect(run.out).toContain(`VIOLATION: ${PUBLIC_ROOT}: expected "allowusers admin"`);
  });
});

describe("harden.sh ssh and check (sshd -T -C and include limits)", () => {
  test("VPS-001: check compares sshd -T -C for root and the admin user on the public and VPN addresses", () => {
    host.harden();
    const run = host.run(["check"], { VPS_PUBLIC_IPV4: "203.0.113.9" });
    expect(run.calls).toEqual(expect.arrayContaining([
      "SSHD -T -C user=root,host=x,addr=203.0.113.9",
      "SSHD -T -C user=admin,host=x,addr=203.0.113.9",
      "SSHD -T -C user=admin,host=x,addr=100.64.0.2",
    ]));
  });

  test("VPS-001: ssh restores the drop-in when only sshd -T -C shows a weaker value", () => {
    const run = host.run(["ssh"], { FAKE_SSHD_C_CONF: "PasswordAuthentication yes" });
    expect(run.code).toBe(1);
    expect(run.out).toContain(`${PUBLIC_ROOT}: expected "passwordauthentication no"`);
    expect(host.exists(DROP_IN)).toBe(false);
    expect(run.calls).not.toContain("SYSTEMCTL reload sshd.service");
  });

  test("VPS-001: check stops at includes that nest too deep", () => {
    host.harden();
    host.write("etc/ssh/sshd_config.d/50-chain.conf", "Include /etc/ssh/chain/1.conf\n");
    for (let level = 1; level <= 20; level += 1) host.write(`etc/ssh/chain/${level}.conf`, `Include /etc/ssh/chain/${level + 1}.conf\n`);
    const run = host.run(["check"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain("VIOLATION: sshd: includes nest deeper than 16");
  }, 60_000);

  test("VPS-001: a file that includes itself twice is read once, not exponentially often", () => {
    host.harden();
    const loop = "Include /etc/ssh/sshd_config.d/50-loop.conf\n";
    host.write("etc/ssh/sshd_config.d/50-loop.conf", `${loop}${loop}`);
    const started = Date.now();
    expect(host.run(["ssh"]).code).toBe(0);
    expect(Date.now() - started).toBeLessThan(20_000);
  }, 60_000);

  test("VPS-001: check reports a VPS_PUBLIC_IPV4 that is not an address", () => {
    host.harden();
    expect(host.run(["check"], { VPS_PUBLIC_IPV4: "x;y" }).out).toContain("VIOLATION: VPS_PUBLIC_IPV4 is not an IPv4 address");
  });
});

describe("harden.sh check (admin and sshd)", () => {
  test("VPS-001: check reports sshd values that are not in force", () => {
    const run = host.run(["check"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain('VIOLATION: sshd -T: expected "passwordauthentication no"');
  });

  test("VPS-001: check reports an admin user without keys", () => {
    host.harden();
    const run = host.run(["check"], { ANIMA_ADMIN_USER: "nokeys" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("VIOLATION: nokeys has no ~/.ssh/authorized_keys");
  });
});
