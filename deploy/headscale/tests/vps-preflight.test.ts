import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { HARDEN_TIMEOUT_MS, fakeHost, removeHosts, VPN_SESSION, type Host } from "./vps-host.ts";

setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

// What must hold before the firewall and the Docker settings go on: Docker 28 or newer
// (WP-04 preflight) and no interface beside the bridges in Docker's range (NET-002, C-3).
const vpn = { SSH_CONNECTION: VPN_SESSION };
const applied = (calls: string[]): boolean => calls.some((call) => call.startsWith("NFT -f ") || call.startsWith("SYSTEMD_RUN --unit="));
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

describe("harden.sh needs Docker 28 or newer (D-10, WP-04 preflight)", () => {
  test.each([["docker"], ["firewall"]])("%s refuses Docker 27 and changes nothing", (command) => {
    const run = host.run([command], { ...vpn, FAKE_DOCKER_VERSION: "27.5.1" });
    expect(run.code).toBe(2);
    expect(run.out).toContain("Docker 27.5.1 is older than 28");
    expect(applied(run.calls)).toBe(false);
    expect(host.exists("etc/docker/daemon.json")).toBe(false);
  });

  test.each([["docker"], ["firewall"]])("%s refuses when the Docker version cannot be read", (command) => {
    expect(host.run([command], { ...vpn, FAKE_DOCKER_DOWN: "1" }).code).toBe(2);
  });

  test.each([["docker"], ["firewall"]])("%s goes ahead with Docker 28", (command) => {
    expect(host.run([command], { ...vpn, FAKE_DOCKER_VERSION: "28.0.0" }).code).toBe(0);
  });
});

describe("harden.sh keeps Docker's range for the bridges (NET-002, WP-04 C-3)", () => {
  const withAddress = (line: string): Record<string, string> => ({
    FAKE_ADDRS: `1: lo    inet 127.0.0.1/8 scope host lo\\n${line}`,
  });

  test.each([
    ["a provider private network", "2: eth1    inet 172.20.0.5/16 brd 172.20.255.255 scope global eth1", "eth1 holds 172.20.0.5/16"],
    ["the low end of the range", "2: ens4    inet 172.16.0.2/24 scope global ens4", "ens4 holds 172.16.0.2/24"],
    ["the high end, on a link-named device", "2: eth0@if7    inet 172.31.255.1/24 scope global eth0", "eth0 holds 172.31.255.1/24"],
  ])("check reports and firewall refuses %s", (_, line, message) => {
    host.harden();
    const check = host.run(["check"], withAddress(line));
    expect(check.code).toBe(1);
    expect(check.out).toContain(`VIOLATION: interface ${message} in 172.16.0.0/12`);
    const firewall = host.run(["firewall"], { ...vpn, ...withAddress(line) });
    expect(firewall.code).toBe(2);
    expect(firewall.out).toContain("ask the programme before applying the firewall");
    expect(applied(firewall.calls)).toBe(false);
  });

  test.each([
    ["Docker's bridges", "2: docker0    inet 172.17.0.1/16 scope global docker0\\n3: br-0a1b    inet 172.30.0.1/24 scope global br-0a1b"],
    ["addresses just outside the range", "2: eth1    inet 172.15.0.5/16 scope global eth1\\n3: eth2    inet 172.32.0.5/16 scope global eth2"],
  ])("leaves %s alone", (_, lines) => {
    host.harden();
    expect(host.run(["check"], withAddress(lines)).out).not.toContain("172.16.0.0/12");
    expect(host.run(["firewall"], { ...vpn, ...withAddress(lines) }).code).toBe(0);
  });
});
