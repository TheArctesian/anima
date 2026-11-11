/**
 * A minimal packet evaluator for the rule forms in `vps/nftables.conf`: interface name
 * matches (with a trailing `*` wildcard), `fib daddr type local`, `ip saddr` and `ip daddr`
 * prefixes or prefix sets, `ip6` matches (never true: the packets are IPv4), `ct state`,
 * `tcp`/`udp dport`, `icmp type` (by name), `limit rate` (never exceeded), `$name` defines,
 * and the `drop` and `accept` verdicts. An `icmpv6 type` rule never matches (the packets are
 * IPv4). Any other rule form throws, and so does a rule that needs a packet field the test did
 * not give, so nothing is guessed.
 */
import type { Chain, parseNftables } from "../src/nftables.ts";

export interface Packet {
  iifname: string;
  oifname?: string;
  /** Source IPv4 address; required by `ip saddr` rules. */
  saddr?: string;
  /** Destination IPv4 address. */
  daddr: string;
  /** Whether `fib daddr type local` is true: the destination is one of this host's addresses. */
  daddrIsLocal: boolean;
  /** Connection tracking state; required by `ct state` rules. */
  ctState?: "new" | "established" | "related" | "invalid";
  /** Layer 4 protocol; required by port and ICMP rules. */
  protocol?: "tcp" | "udp" | "icmp";
  dport?: number;
  /** ICMP type by its nft name, such as `echo-request`; required by `icmp type` rules. */
  icmpType?: string;
}

export type Verdict = "accept" | "drop";

type Ruleset = ReturnType<typeof parseNftables>;

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

/** Read `a`, `a,b` or `{ a, b }` starting at tokens[at]; returns the values and the next index. */
function readValues(tokens: string[], at: number): [string[], number] {
  const first = tokens[at];
  if (first === undefined) throw new Error("missing operand");
  if (first !== "{") {
    const values = [first];
    let i = at + 1;
    for (; tokens[i] === "," && tokens[i + 1] !== undefined; i += 2) values.push(tokens[i + 1] ?? "");
    return [values, i];
  }
  const values: string[] = [];
  let i = at + 1;
  for (; tokens[i] !== "}"; i += 1) {
    const token = tokens[i];
    if (token === undefined) throw new Error("unclosed set");
    if (token !== ",") values.push(token);
  }
  return [values, i + 1];
}

/** Read an optional `!=`, then the values; returns whether it was negated. */
function readOperand(tokens: string[], at: number): { negated: boolean; values: string[]; next: number } {
  const negated = tokens[at] === "!=";
  const [values, next] = readValues(tokens, negated ? at + 1 : at);
  return { negated, values, next };
}

interface Condition {
  matches: boolean;
  /** Index of the first token after the condition. */
  next: number;
}

function required<T>(value: T | undefined, field: string, rule: string): T {
  if (value === undefined) throw new Error(`the packet has no ${field}, which this rule needs: ${rule}`);
  return value;
}

function interfaceCondition(tokens: string[], at: number, packet: Packet): Condition {
  const { negated, values, next } = readOperand(tokens, at + 1);
  if (values.some((value) => !value.startsWith('"'))) throw new Error(`unsupported interface operand: ${values.join(" ")}`);
  const actual = tokens[at] === "oifname" ? packet.oifname : packet.iifname;
  // As in nft, a packet with no output interface yet (prerouting) has an empty oifname:
  // `oifname "x"` does not match it and `oifname != "x"` does.
  const hit = values.some((value) => matchesName(value.slice(1, -1), actual ?? ""));
  return { matches: hit !== negated, next };
}

function addressCondition(tokens: string[], at: number, packet: Packet, rule: string): Condition {
  const { negated, values, next } = readOperand(tokens, at + 2);
  // The packets are IPv4, so an ip6 match never holds.
  if (tokens[at] === "ip6") return { matches: false, next };
  const field = tokens[at + 1];
  if (field !== "saddr" && field !== "daddr") throw new Error(`unsupported ip field "${field ?? ""}" in rule: ${rule}`);
  const address = required(packet[field], field, rule);
  const hit = values.some((cidr) => inPrefix4(address, cidr));
  return { matches: hit !== negated, next };
}

function ctCondition(tokens: string[], at: number, packet: Packet, rule: string): Condition {
  if (tokens[at + 1] !== "state") throw new Error(`unsupported ct match in rule: ${rule}`);
  const { negated, values, next } = readOperand(tokens, at + 2);
  const hit = values.includes(required(packet.ctState, "ctState", rule));
  return { matches: hit !== negated, next };
}

function protocolCondition(tokens: string[], at: number, packet: Packet, rule: string): Condition {
  const protocol = tokens[at];
  const field = tokens[at + 1];
  const { negated, values, next } = readOperand(tokens, at + 2);
  const actual = required(packet.protocol, "protocol", rule);
  if (actual !== protocol) return { matches: false, next };
  if (actual === "icmp" && field === "type") {
    const hit = values.includes(required(packet.icmpType, "icmpType", rule));
    return { matches: hit !== negated, next };
  }
  if (field !== "dport") throw new Error(`unsupported ${actual} match "${field ?? ""}" for this packet in rule: ${rule}`);
  const hit = values.map(Number).includes(required(packet.dport, "dport", rule));
  return { matches: hit !== negated, next };
}

/** `limit rate N/unit [burst N packets]`: the tests never exceed a rate, so it always holds. */
function limitCondition(tokens: string[], at: number, rule: string): Condition {
  if (tokens[at + 1] !== "rate" || tokens[at + 2] === undefined) throw new Error(`unsupported limit in rule: ${rule}`);
  const next = tokens[at + 3] === "burst" ? at + 6 : at + 3;
  return { matches: true, next };
}

function condition(tokens: string[], at: number, packet: Packet, rule: string): Condition {
  const word = tokens[at] ?? "";
  if (word === "iifname" || word === "oifname" || word === "iif") return interfaceCondition(tokens, at, packet);
  if (word === "fib" && tokens.slice(at, at + 4).join(" ") === "fib daddr type local") {
    return { matches: packet.daddrIsLocal, next: at + 4 };
  }
  if (word === "ip" || word === "ip6") return addressCondition(tokens, at, packet, rule);
  if (word === "ct") return ctCondition(tokens, at, packet, rule);
  if (["tcp", "udp", "icmp", "icmpv6"].includes(word)) return protocolCondition(tokens, at, packet, rule);
  if (word === "limit") return limitCondition(tokens, at, rule);
  throw new Error(`unsupported match "${word}" in rule: ${rule}`);
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
    if (rule.includes("$")) throw new Error(`unresolved define in rule: ${rule}`);
    const result = ruleMatches(rule, packet);
    if (result.matches) return result.verdict;
  }
  if (chain.policy === undefined || chain.policy === "accept") return "accept";
  if (chain.policy === "drop") return "drop";
  throw new Error(`unsupported chain policy: ${chain.policy}`);
}

/** The values of the file's top-level `define name = value` lines. */
function defines(ruleset: Ruleset): Map<string, string> {
  const values = new Map<string, string>();
  for (const command of ruleset.commands) {
    const match = /^define\s+(\w+)\s*=\s*(.+)$/.exec(command);
    if (match?.[1] && match[2]) values.set(match[1], match[2].trim());
  }
  return values;
}

/** A chain of the ruleset with every `$name` replaced by its define, as nft does on load. */
export function resolvedChain(ruleset: Ruleset, table: string, name: string): Chain {
  const chain = ruleset.chains.find((candidate) => candidate.table === table && candidate.name === name);
  if (!chain) throw new Error(`no chain ${name} in ${table}`);
  const values = defines(ruleset);
  const rules = chain.rules.map((rule) =>
    rule.replace(/\$(\w+)/g, (_, key: string) => {
      const value = values.get(key);
      if (value === undefined) throw new Error(`undefined variable $${key} in rule: ${rule}`);
      return value;
    }),
  );
  return { ...chain, rules };
}
