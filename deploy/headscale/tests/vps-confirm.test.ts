import { beforeEach, describe, expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { fakeHost, VPN_SESSION, type Host } from "./vps-host.ts";

// The guards around the boot file (S2, S3) and the sessions that may confirm (S4).
const vpn = { SSH_CONNECTION: VPN_SESSION };
const second = { SSH_CONNECTION: "100.64.0.2 51001 100.64.0.1 22" };
const ESTABLISHED = "0 0 100.64.0.1:22 100.64.0.2:51000\n0 0 [fd7a:115c:a1e0::1]:22 [FD7A:115C:A1E0::2]:52000\n";
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

describe("harden.sh firewall and confirm keep the boot file to the Anima include (VPS-002)", () => {
  test.each([["firewall"], ["confirm"]])("%s refuses while the boot file has another active line", (command) => {
    if (command === "confirm") expect(host.run(["firewall"], vpn).code).toBe(0);
    host.write("etc/sysconfig/nftables.conf", '# stock\ninclude "/etc/nftables/extra.nft"\n');
    const run = host.run([command], command === "confirm" ? second : vpn);
    expect(run.code).toBe(2);
    expect(run.out).toContain("/etc/sysconfig/nftables.conf:2: only the Anima include may be active");
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
    expect(host.exists("etc/nftables/anima.nft")).toBe(false);
  });

  test("confirm checks the boot file with the pending rules before it changes anything", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    host.write("etc/sysconfig/nftables.conf", "# stock\n");
    const run = host.run(["confirm"], { ...second, FAKE_NFT_CHECK_BOOT: "1" });
    expect(run.code).toBe(2);
    expect(run.out).toContain("nft -c rejects the boot file with the new rules");
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
    expect(host.exists("etc/nftables/anima.nft")).toBe(false);
  });

  test("confirm checks the installed boot file before it masks firewalld", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    const run = host.run(["confirm"], second);
    expect(run.code).toBe(0);
    const check = run.calls.indexOf(`NFT -c -f ${host.root}/etc/sysconfig/nftables.conf`);
    expect(check).toBeGreaterThan(run.calls.indexOf("SYSTEMCTL enable nftables.service"));
    expect(check).toBeLessThan(run.calls.indexOf("SYSTEMCTL mask firewalld.service"));
  });
});

describe("harden.sh confirm needs a connection opened after firewall (VPS-002)", () => {
  test.each([
    ["the other session that was open over IPv4", "100.64.0.2 51000 100.64.0.1 22", "0 0 100.64.0.1:22 100.64.0.2:51000\n"],
    ["a session open over IPv6, in other case", "fd7a:115c:a1e0::2 52000 fd7a:115c:a1e0::1 22", ESTABLISHED],
    ["an IPv4-mapped session", "100.64.0.2 51001 100.64.0.1 22", "0 0 [::ffff:100.64.0.1]:22 [::ffff:100.64.0.2]:51001\n"],
  ])("refuses %s", (_, connection, established) => {
    expect(host.run(["firewall"], { SSH_CONNECTION: "100.64.0.2 49000 100.64.0.1 22", FAKE_SS_ESTABLISHED: established }).code).toBe(0);
    const run = host.run(["confirm"], { SSH_CONNECTION: connection });
    expect(run.code).toBe(2);
    expect(run.out).toContain("this SSH connection was open before firewall ran");
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
  });

  test("records the established SSH peers and accepts a later connection", () => {
    const run = host.run(["firewall"], { ...vpn, FAKE_SS_ESTABLISHED: ESTABLISHED });
    expect(run.calls).toContain("SS -Htn state established ( sport = :22 )");
    expect(host.read("var/lib/anima/firewall-peers")).toBe("100.64.0.2 51000\nfd7a:115c:a1e0::2 52000\n");
    expect(host.run(["confirm"], second).code).toBe(0);
  });

  test("refuses when firewall left no record of the connections", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    rmSync(join(host.root, "var/lib/anima/firewall-peers"));
    const run = host.run(["confirm"], second);
    expect(run.code).toBe(2);
    expect(run.out).toContain("no recorded SSH connections");
  });
});
