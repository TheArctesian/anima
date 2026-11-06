/**
 * A minimal packet evaluator for the rule forms in `vps/nftables.conf`: interface name
 * matches (with a trailing `*` wildcard), `fib daddr type local`, `ip daddr` and
 * `ip6 daddr` prefixes, and the `drop` and `accept` verdicts. A rule using anything else
 * throws, so a new rule form fails the tests loudly instead of being guessed.
 */
import type { Chain } from "../src/nftables.ts";

export interface Packet {
  iifname: string;
  oifname?: string;
  /** Destination IPv4 address. */
  daddr: string;
  /** Whether `fib daddr type local` is true: the destination is one of this host's addresses. */
  daddrIsLocal: boolean;
}

export type Verdict = "accept" | "drop";

const TOKEN = /"[^"]*"|!=|[{},]|[^\s{},"]+/g;

function ipv4(address: string): number {
  const parts = address.split(".").map(Number);
  if (parts.length !== 4 || parts.some((part) => !Number.isInteger(part) || part < 0 || part > 255)) {
    throw new Error(`not an IPv4 address: ${address}`);
  }
  return parts.reduce((sum, part) => sum * 256 + part, 0);
}

function inPrefix4(address: string, cidr: string): boolean {
  const [base = "", bits = "32"] = cidr.split("/");
  const size = 2 ** (32 - Number(bits));
  return Math.floor(ipv4(address) / size) === Math.floor(ipv4(base) / size);
}

const matchesName = (pattern: string, name: string): boolean =>
  pattern.endsWith("*") ? name.startsWith(pattern.slice(0, -1)) : pattern === name;

/** Read `"a"` or `{ "a", "b" }` starting at tokens[at]; returns the names and the next index. */
function readNames(tokens: string[], at: number): [string[], number] {
  const first = tokens[at];
  if (first === undefined) throw new Error("missing interface name");
  if (first.startsWith('"')) return [[first.slice(1, -1)], at + 1];
  if (first !== "{") throw new Error(`unsupported interface operand: ${first}`);
  const names: string[] = [];
  let i = at + 1;
  for (; tokens[i] !== "}"; i += 1) {
    const token = tokens[i];
    if (token === undefined) throw new Error("unclosed interface set");
    if (token === ",") continue;
    if (!token.startsWith('"')) throw new Error(`unsupported set member: ${token}`);
    names.push(token.slice(1, -1));
  }
  return [names, i + 1];
}

interface Condition {
  matches: boolean;
  /** Index of the first token after the condition. */
  next: number;
}

function interfaceCondition(tokens: string[], at: number, packet: Packet): Condition {
  const negated = tokens[at + 1] === "!=";
  const [names, next] = readNames(tokens, at + (negated ? 2 : 1));
  const actual = tokens[at] === "iifname" ? packet.iifname : packet.oifname;
  // A rule on oifname cannot match a packet that has no output interface yet.
  const hit = actual !== undefined && names.some((name) => matchesName(name, actual));
  return { matches: actual !== undefined && hit !== negated, next };
}

function addressCondition(tokens: string[], at: number, packet: Packet): Condition {
  // The packets are IPv4, so an ip6 rule never matches them.
  const matches = tokens[at] === "ip" && inPrefix4(packet.daddr, tokens[at + 2] ?? "");
  return { matches, next: at + 3 };
}

function condition(tokens: string[], at: number, packet: Packet, rule: string): Condition {
  const word = tokens[at];
  if (word === "iifname" || word === "oifname") return interfaceCondition(tokens, at, packet);
  if (word === "fib" && tokens.slice(at, at + 4).join(" ") === "fib daddr type local") {
    return { matches: packet.daddrIsLocal, next: at + 4 };
  }
  if ((word === "ip" || word === "ip6") && tokens[at + 1] === "daddr" && tokens[at + 2] !== undefined) {
    return addressCondition(tokens, at, packet);
  }
  throw new Error(`unsupported match "${word ?? ""}" in rule: ${rule}`);
}

/** Whether the packet meets every condition of the rule, and the verdict (the last token). */
function ruleMatches(rule: string, packet: Packet): { matches: boolean; verdict: Verdict } {
  const tokens = rule.match(TOKEN) ?? [];
  const verdict = tokens.pop();
  if (verdict !== "drop" && verdict !== "accept") throw new Error(`unsupported verdict in rule: ${rule}`);
  let matches = true;
  for (let at = 0; at < tokens.length; ) {
    const result = condition(tokens, at, packet, rule);
    matches &&= result.matches;
    at = result.next;
  }
  return { matches, verdict };
}

/** The verdict of one chain: the first matching rule's, or the chain policy (default accept). */
export function evaluateChain(chain: Chain, packet: Packet): Verdict {
  for (const rule of chain.rules) {
    const result = ruleMatches(rule, packet);
    if (result.matches) return result.verdict;
  }
  if (chain.policy === undefined || chain.policy === "accept") return "accept";
  if (chain.policy === "drop") return "drop";
  throw new Error(`unsupported chain policy: ${chain.policy}`);
}
