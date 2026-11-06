import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseNftables, type Chain } from "../src/nftables.ts";
import { evaluateChain, type Packet, type Verdict } from "./nft-eval.ts";

const ruleset = parseNftables(readFileSync(join(import.meta.dir, "..", "vps", "nftables.conf"), "utf8"));

function chainNamed(name: string): Chain {
  const chain = ruleset.chains.find((candidate) => candidate.table === "inet anima_filter" && candidate.name === name);
  if (!chain) throw new Error(`no chain ${name}`);
  return chain;
}

/** What the VPS does with a packet: raw prerouting first, then forward if prerouting let it through. */
function forwarded(packet: Packet): { prerouting: Verdict; forward: Verdict } {
  const prerouting = evaluateChain(chainNamed("prerouting"), packet);
  const forward = prerouting === "drop" ? "drop" : evaluateChain(chainNamed("forward"), packet);
  return { prerouting, forward };
}

const toMac = (iifname: string): Packet => ({ iifname, oifname: "tailscale0", daddr: "100.64.0.2", daddrIsLocal: false });
const toSiteDevice = (iifname: string): Packet => ({ iifname, oifname: "tailscale0", daddr: "10.10.0.5", daddrIsLocal: false });

// The forward chain has no port rule, so a packet carries no port: VNC 5900 and RDP 3389 pass as any port does.
describe("VPS forwarding for the gateway (R-050, VPS-002)", () => {
  for (const bridge of ["br-1a2b3c4d5e6f", "docker0"]) {
    test(`R-050 a packet from ${bridge} out of tailscale0 to the Mac is dropped by neither chain`, () => {
      expect(forwarded(toMac(bridge))).toEqual({ prerouting: "accept", forward: "accept" });
    });

    test(`R-050 a packet from ${bridge} out of tailscale0 to a site device is dropped by neither chain`, () => {
      expect(forwarded(toSiteDevice(bridge))).toEqual({ prerouting: "accept", forward: "accept" });
    });
  }

  test("VPS-002 R-050 a packet from the public interface out of tailscale0 is dropped by forward", () => {
    for (const packet of [toMac("eth0"), toSiteDevice("eth0")]) {
      expect(forwarded(packet)).toEqual({ prerouting: "accept", forward: "drop" });
    }
  });

  test("R-050 a packet from a bridge to the host's own VPN address is still dropped in prerouting (WP-04 limitation)", () => {
    for (const iifname of ["br-1a2b3c4d5e6f", "docker0"]) {
      const packet: Packet = { iifname, daddr: "100.64.0.1", daddrIsLocal: true };
      expect(forwarded(packet).prerouting).toBe("drop");
    }
  });

  test("R-050 the evaluator refuses a rule form it does not understand", () => {
    const unknown: Chain = { table: "inet x", name: "c", rules: ["meta mark 1 drop"], policy: "accept" };
    expect(() => evaluateChain(unknown, toMac("docker0"))).toThrow("unsupported match");
  });

  test("R-050 the evaluator takes the first matching rule, the chain policy, and refuses what it does not know", () => {
    const packet = toMac("docker0");
    const chain = (rules: string[], policy: string): Chain => ({ table: "inet x", name: "c", rules, policy });
    expect(evaluateChain(chain(['iifname "docker0" accept', 'iifname "docker0" drop'], "drop"), packet)).toBe("accept");
    expect(evaluateChain(chain(['iifname "docker0" drop', 'iifname "docker0" accept'], "accept"), packet)).toBe("drop");
    expect(evaluateChain(chain(['iifname "eth0" accept'], "drop"), packet)).toBe("drop");
    expect(() => evaluateChain(chain([], "queue"), packet)).toThrow("unsupported chain policy");
    expect(() => evaluateChain(chain(["iifname docker0 jump x"], "accept"), packet)).toThrow("unsupported verdict");
    // nft gives a packet with no output interface (prerouting) an empty oifname.
    const noOif: Packet = { iifname: "docker0", daddr: "100.64.0.2", daddrIsLocal: false };
    expect(evaluateChain(chain(['oifname != "tailscale0" drop'], "accept"), noOif)).toBe("drop");
    expect(evaluateChain(chain(['oifname "tailscale0" drop'], "accept"), noOif)).toBe("accept");
  });
});
