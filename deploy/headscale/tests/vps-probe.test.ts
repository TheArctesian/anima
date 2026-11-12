import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { HARDEN_TIMEOUT_MS, fakeHost, mutatingCalls, removeHosts, type Host } from "./vps-host.ts";

// VP-13 security review F1 and F6: `harden.sh probe` makes anima-probe, the hub's SSH user, with the
// fixed uid the firewall confines and one key limited to the hub, and `harden.sh check` reports an
// anima-probe that is more than that. Neither prints key material (LOG-007).
setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

const KEY = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIProbeTestOnlyNotARealKey anima-probe";
const OPTIONS = 'restrict,pty,from="172.30.0.10"';
const KEYS = "home/anima-probe/.ssh/authorized_keys";
const PROBE = "anima-probe:x:10002:10002::/home/anima-probe:/bin/bash";
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

/** Write the public key file the operator copied to the VPS; returns the --key option. */
function keyFile(text: string): string {
  const file = join(host.root, "..", "anima-probe.pub");
  writeFileSync(file, text);
  return `--key=${file}`;
}

describe("harden.sh probe makes the hub's SSH user (VP-13 F1)", () => {
  test("VP-13 makes group and user 10002 with a home and bash, and installs the key with the hub's options", () => {
    const run = host.run([keyFile(`${KEY}\n`), "probe"]);
    expect(run.code).toBe(0);
    expect(run.calls).toEqual(expect.arrayContaining([
      "GROUPADD --gid 10002 anima-probe",
      "USERADD --uid 10002 --gid 10002 --create-home --home-dir /home/anima-probe --shell /bin/bash anima-probe",
    ]));
    expect(run.calls.some((call) => call.startsWith("RUNUSER -u anima-probe -- sh -c "))).toBe(true);
    expect(host.read(KEYS)).toBe(`${OPTIONS} ${KEY}\n`);
    expect(statSync(join(host.root, KEYS)).mode & 0o777).toBe(0o600);
    expect(statSync(join(host.root, "home/anima-probe/.ssh")).mode & 0o777).toBe(0o700);
    expect(run.out).not.toContain("AAAA");
  });

  test("VP-13 keeps an existing anima-probe 10002 and replaces its keys", () => {
    host.write(KEYS, `${KEY.replace("Probe", "Old")}\n`, 0o600);
    const run = host.run([keyFile(KEY), "probe"], { FAKE_PROBE: PROBE });
    expect(run.code).toBe(0);
    expect(run.calls.filter((call) => /^(GROUPADD|USERADD) /.test(call))).toEqual([]);
    expect(host.read(KEYS)).toBe(`${OPTIONS} ${KEY}\n`);
  });

  test.each([
    ["another user on uid 10002", "builder:x:10002:10002::/home/builder:/bin/sh"],
    ["anima-probe on another uid", "anima-probe:x:1001:1001::/home/anima-probe:/bin/bash"],
    ["anima-probe in another group", "anima-probe:x:10002:10::/home/anima-probe:/bin/bash"],
    ["anima-probe with another home", "anima-probe:x:10002:10002::/root:/bin/bash"],
  ])("VP-13 refuses %s and changes nothing", (_, entry) => {
    const run = host.run([keyFile(KEY), "probe"], { FAKE_PROBE: entry });
    expect(run.code).toBe(2);
    expect(mutatingCalls(run)).toEqual([]);
    expect(host.exists(KEYS)).toBe(false);
  });

  test.each([
    ["no key file", ""],
    ["an empty file", "\n"],
    ["two keys", `${KEY}\n${KEY}\n`],
    ["a key with options", `command="/bin/sh" ${KEY}`],
    ["a key with its own from=", `from="*" ${KEY}`],
    ["a private key", "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA\n-----END OPENSSH PRIVATE KEY-----\n"],
    ["a key with a carriage return", `${KEY}\r\n`],
  ])("VP-13 refuses %s without printing it", (name, text) => {
    const option = name === "no key file" ? [] : [keyFile(text)];
    const run = host.run([...option, "probe"]);
    expect(run.code).toBe(2);
    expect(mutatingCalls(run)).toEqual([]);
    expect(run.out).not.toContain("AAAA");
    expect(host.exists(KEYS)).toBe(false);
  });

  test("VPS-005 a dry run makes no user and writes no key", () => {
    const run = host.run(["--dry-run", keyFile(KEY), "probe"]);
    expect(run.code).toBe(0);
    expect(mutatingCalls(run)).toEqual([]);
    expect(run.out).toMatch(/\[dry-run\] \S*GROUPADD --gid 10002 anima-probe/);
    expect(host.exists(KEYS)).toBe(false);
  });
});

describe("harden.sh check reports an anima-probe that is more than the hub's user (VP-13 F6)", () => {
  beforeEach(() => {
    host.harden();
    host.write(KEYS, `${OPTIONS} ${KEY}\n`, 0o600);
  });

  test("VP-13 a confined anima-probe, and no anima-probe at all, are no violation", () => {
    expect(host.run(["check"], { FAKE_PROBE: PROBE }).out).toEndWith("ok: no violations\n");
    expect(host.run(["check"]).out).toEndWith("ok: no violations\n");
  });

  test.each([
    ["another uid", { FAKE_PROBE: PROBE.replace(":10002:10002:", ":1001:10002:") }, "anima-probe has uid 1001, not 10002"],
    ["the wheel group", { FAKE_PROBE_GROUPS: "anima-probe wheel" }, "anima-probe is in group wheel; it may be in its own group only"],
    ["another group", { FAKE_PROBE_GROUPS: "anima-probe docker" }, "anima-probe is in group docker"],
    ["a password", { FAKE_PROBE_SHADOW: "anima-probe:$6$salt$hash:20400:0:99999:7:::" }, "anima-probe has a usable password"],
    ["an empty password", { FAKE_PROBE_SHADOW: "anima-probe::20400:0:99999:7:::" }, "anima-probe has a usable password"],
    ["an unreadable shadow entry", { FAKE_PROBE_SHADOW: "unreadable" }, "cannot read the shadow entry of anima-probe"],
  ])("VP-13 check reports %s", (_, env, message) => {
    const run = host.run(["check"], { FAKE_PROBE: PROBE, ...env });
    expect(run.code).toBe(1);
    expect(run.out).toContain(`VIOLATION: ${message}`);
    expect(run.out).not.toContain("$6$");
  });

  test.each([["!!"], ["!"], ["*"], ["!$6$salt$hash"]])("VP-13 check accepts the locked password field %p", (field) => {
    expect(host.run(["check"], { FAKE_PROBE: PROBE, FAKE_PROBE_SHADOW: `anima-probe:${field}:20400::::::` }).code).toBe(0);
  });

  test.each([
    ["etc/sudoers", "root ALL=(ALL) ALL\nanima-probe ALL=(ALL) NOPASSWD: ALL\n"],
    ["etc/sudoers.d/90-cloud", "#10002 ALL=(ALL) ALL\n"],
    ["etc/sudoers.d/probe", "%anima-probe ALL=(ALL) ALL\n"],
  ])("VP-13 check reports a sudo grant in %s", (file, text) => {
    host.write(file, text, 0o440);
    const run = host.run(["check"], { FAKE_PROBE: PROBE });
    expect(run.out).toContain(`VIOLATION: /${file} names anima-probe; it may have no sudo grant`);
  });

  test("VP-13 check ignores comments and other users with similar names in sudoers", () => {
    host.write("etc/sudoers.d/notes", "# anima-probe gets no sudo\nanima-probe2 ALL=(ALL) ALL\n", 0o440);
    expect(host.run(["check"], { FAKE_PROBE: PROBE }).code).toBe(0);
  });

  test.each([
    ["no options", KEY],
    ["no from=", `restrict,pty ${KEY}`],
    ["no restrict", `pty,from="172.30.0.10" ${KEY}`],
    ["a wider from=", `restrict,pty,from="172.30.0.0/24" ${KEY}`],
    ["forwarding turned back on", `restrict,pty,port-forwarding,from="172.30.0.10" ${KEY}`],
  ])("VP-13 check reports an authorized_keys line with %s, without printing it", (_, line) => {
    host.write(KEYS, `${OPTIONS} ${KEY}\n# the hub\n${line}\n`, 0o600);
    const run = host.run(["check"], { FAKE_PROBE: PROBE });
    expect(run.code).toBe(1);
    expect(run.out).toContain(`VIOLATION: anima-probe's authorized_keys line 3 is not limited to ${OPTIONS}`);
    expect(run.out).not.toContain("AAAA");
  });
});
