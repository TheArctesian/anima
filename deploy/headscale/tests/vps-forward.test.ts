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

describe("VPS forwarding for the gateway (R-050, VPS-002)", () => {
  for (const bridge of ["br-1a2b3c4d5e6f", "docker0"]) {
    test(`R-050 a packet from ${bridge} out of tailscale0 to the Mac (VNC 5900) is dropped by neither chain`, () => {
      expect(forwarded(toMac(bridge))).toEqual({ prerouting: "accept", forward: "accept" });
    });

    test(`R-050 a packet from ${bridge} out of tailscale0 to a site device (RDP 3389) is dropped by neither chain`, () => {
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
});
