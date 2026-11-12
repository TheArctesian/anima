import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseNftables } from "../src/nftables.ts";
import { HARDEN_TIMEOUT_MS, VPS_DIR, fakeHost, removeHosts, type Host } from "./vps-host.ts";

// VP-13, VPS-001: sshd admits the hub's unprivileged user anima-probe, by key, only from the hub
// container's address 172.30.0.10, beside the admin user. harden.sh ssh and check accept exactly
// that entry and report any other user or host part.
setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

const DROP_IN = "etc/ssh/sshd_config.d/00-anima.conf";
const PROBE = "anima-probe@172.30.0.10";
const readVps = (name: string): string => readFileSync(join(VPS_DIR, name), "utf8");
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

describe("the anima-probe entry (VP-13)", () => {
  test("VP-13 the drop-in allows the admin and anima-probe from the hub address, with keys only", () => {
    const lines = readVps("sshd-anima.conf").split("\n").filter((line) => !line.startsWith("#"));
    expect(lines.filter((line) => line.startsWith("AllowUsers"))).toEqual([`AllowUsers \${ANIMA_ADMIN_USER} ${PROBE}`]);
    expect(lines).toContain("PasswordAuthentication no");
    expect(lines).toContain("AuthenticationMethods publickey");
  });

  test("VP-13 the probe address is the firewall's hub_source, compose's ANIMA_HUB_IP and sshd.sh's PROBE_ALLOW", () => {
    const address = PROBE.split("@")[1];
    expect(parseNftables(readVps("nftables.conf")).commands).toContain(`define hub_source = ${String(address)}`);
    const compose = readFileSync(join(VPS_DIR, "..", "..", "compose.yaml"), "utf8");
    expect(compose).toContain(`ipv4_address: \${ANIMA_HUB_IP:-${String(address)}}`);
    const sshd = readVps("sshd.sh");
    expect(sshd).toMatch(/^PROBE_USER=anima-probe$/m);
    expect(sshd).toContain(`PROBE_ALLOW=$PROBE_USER@${String(address)}\n`);
  });

  test("VP-13 ssh installs the drop-in and check finds no AllowUsers problem", () => {
    expect(host.run(["ssh"]).code).toBe(0);
    expect(host.read(DROP_IN)).toContain(`\nAllowUsers admin ${PROBE}\n`);
    const run = host.run(["check"]);
    expect(run.out).not.toContain("allowusers");
    expect(run.calls).toContain("SSHD -T -C user=anima-probe,host=x,addr=172.30.0.10");
  });

  test("VP-13 ssh refuses anima-probe as the admin user", () => {
    const run = host.run(["ssh"], { ANIMA_ADMIN_USER: "anima-probe" });
    expect(run.code).toBe(2);
    expect(run.out).toContain("anima-probe is the hub's user (VP-13)");
    expect(host.exists(DROP_IN)).toBe(false);
  });
});

describe("other AllowUsers entries stay refused (VP-13, VPS-001)", () => {
  test.each([
    ["anima-probe"],
    ["anima-probe@*"],
    ["anima-probe@172.30.0.0/24"],
    ["anima-probe@172.30.0.11"],
    ["anima-probe@100.64.0.2"],
    ["intruder@172.30.0.10"],
    ["*@172.30.0.10"],
  ])("VP-13 ssh refuses and restores when another drop-in adds AllowUsers %s", (entry) => {
    host.write("etc/ssh/sshd_config.d/50-cloud-init.conf", `AllowUsers ${entry}\n`);
    const run = host.run(["ssh"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain(`unexpected "allowusers ${entry}"`);
    expect(host.exists(DROP_IN)).toBe(false);
    expect(run.calls).not.toContain("SYSTEMCTL reload sshd.service");
  });

  test("VP-13 check reports a drop-in without the probe entry", () => {
    host.harden();
    host.write(DROP_IN, host.read(DROP_IN).replace(` ${PROBE}`, ""));
    const run = host.run(["check"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain(`VIOLATION: sshd -T: expected "allowusers ${PROBE}"`);
  });

  test("VP-13 check reports a probe entry with a wider host part", () => {
    host.harden();
    host.write(DROP_IN, host.read(DROP_IN).replace(PROBE, "anima-probe@172.30.0.*"));
    const run = host.run(["check"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain('VIOLATION: sshd -T: unexpected "allowusers anima-probe@172.30.0.*"');
  });
});
