import { beforeEach, describe, expect, test } from "bun:test";
import { chmodSync } from "node:fs";
import { join } from "node:path";
import { fakeHost, type Host } from "./vps-host.ts";

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
    expect(host.read(DROP_IN)).toContain("\nAllowUsers admin\n");
    expect(run.calls).toEqual(expect.arrayContaining(["SSHD -t", "SSHD -T", "SYSTEMCTL reload sshd.service"]));
    expect(run.out).not.toContain("AAAA");
  });

  // The files an AlmaLinux 10 VPS may carry: the permitrootlogin migration (01- or 25-),
  // Red Hat's defaults (50-redhat.conf, X11Forwarding yes) and cloud-init's.
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

describe("harden.sh check (admin and sshd)", () => {
  test("VPS-001: check reports sshd values that are not in force", () => {
    const run = host.run(["check"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain('VIOLATION: sshd: expected "passwordauthentication no"');
  });

  test("VPS-001: check reports an admin user without keys", () => {
    host.harden();
    const run = host.run(["check"], { ANIMA_ADMIN_USER: "nokeys" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("VIOLATION: nokeys has no ~/.ssh/authorized_keys");
  });
});
