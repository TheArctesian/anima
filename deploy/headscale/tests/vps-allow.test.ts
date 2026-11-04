import { beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseNftables, publicPorts, unexplainedAccepts, type Chain } from "../src/nftables.ts";
import { HARDEN_TIMEOUT_MS, fakeHost, VPN_SESSION, VPS_DIR, type Host } from "./vps-host.ts";

setDefaultTimeout(HARDEN_TIMEOUT_MS);

// VPS-002, VPS-005: /etc/anima/firewall-allow keeps a port open for another service on the
// VPS. It may add public tcp/udp accepts to the input chain and change nothing else.
const ALLOW = "etc/anima/firewall-allow";
const vpn = { SSH_CONNECTION: VPN_SESSION };
type Ruleset = ReturnType<typeof parseNftables>;
const hooked = (ruleset: Ruleset, hook: string): Chain => ruleset.chains.find((chain) => chain.hook === hook) as Chain;
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

describe("local allow file", () => {
  test("adds exactly its ports as public accepts and leaves the other chains as reviewed", () => {
    host.write(ALLOW, "# blog and mail\n80/tcp\n  8443/tcp  # admin UI\n\n51820/udp\n");
    expect(host.run(["firewall"], vpn).code).toBe(0);
    const base = parseNftables(readFileSync(join(VPS_DIR, "nftables.conf"), "utf8"));
    const rendered = parseNftables(host.checkedRules());
    expect(publicPorts(hooked(rendered, "input")).sort()).toEqual(
      [...publicPorts(hooked(base, "input")), "80/tcp", "8443/tcp", "51820/udp"].sort(),
    );
    expect(unexplainedAccepts(hooked(rendered, "input"))).toEqual([]);
    for (const hook of ["prerouting", "forward"]) expect(hooked(rendered, hook)).toEqual(hooked(base, hook));
    expect(rendered.commands).toEqual(base.commands);
    expect(hooked(rendered, "input").rules.slice(0, 2)).toEqual(hooked(base, "input").rules.slice(0, 2));
  });

  test("without the file the rules are nftables.conf as reviewed", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    expect(host.checkedRules()).toBe(readFileSync(join(VPS_DIR, "nftables.conf"), "utf8"));
  });

  test.each([
    ["22/tcp"],
    ["80/tcp accept"],
    ["80/tcp; flush ruleset"],
    ["0/tcp"],
    ["65536/udp"],
    ["080/tcp"],
    ["80/sctp"],
    ["80"],
    ["1000-2000/tcp"],
    ["100.64.0.1:80/tcp"],
    ['iifname "eth0" accept'],
  ])("refuses the line %p and applies nothing", (line) => {
    host.write(ALLOW, `443/tcp\n${line}\n`);
    const run = host.run(["firewall"], vpn);
    expect(run.code).toBe(1);
    expect(run.out).toContain("/etc/anima/firewall-allow");
    expect(run.calls.some((call) => /^NFT -f |^SYSTEMD_RUN /.test(call))).toBe(false);
  });
});
