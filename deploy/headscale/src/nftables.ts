/**
 * Reads the structure of an nftables file so the tests can check what the VPS firewall
 * opens (VPS-002). It accepts the subset `vps/nftables.conf` uses: top-level commands,
 * tables, chains and one rule per line. Anything else is refused, never guessed.
 */

export interface Chain {
  table: string;
  name: string;
  hook?: string;
  policy?: string;
  rules: string[];
}

export interface Ruleset {
  /** Statements outside any table block, such as `delete table inet x`. */
  commands: string[];
  chains: Chain[];
}

export class NftParseError extends Error {
  constructor(line: number, reason: string) {
    super(`line ${String(line)}: ${reason}`);
    this.name = "NftParseError";
  }
}

const TABLE = /^table\s+(\S+)\s+(\S+)\s*\{$/;
const CHAIN = /^chain\s+(\S+)\s*\{$/;

function stripComment(line: string): string {
  let quoted = false;
  for (let i = 0; i < line.length; i += 1) {
    if (line[i] === '"') quoted = !quoted;
    if (line[i] === "#" && !quoted) return line.slice(0, i);
  }
  return line;
}

function balanced(statement: string): boolean {
  return statement.split("{").length === statement.split("}").length;
}

function addStatements(chain: Chain, text: string, line: number): void {
  for (const piece of text.split(";").map((part) => part.trim())) {
    if (piece === "") continue;
    if (!balanced(piece)) throw new NftParseError(line, "multi-line sets are not supported");
    if (piece.startsWith("type ")) chain.hook = /\bhook\s+(\w+)/.exec(piece)?.[1];
    else if (piece.startsWith("policy ")) chain.policy = piece.slice("policy ".length).trim();
    else chain.rules.push(piece);
  }
}

/** Parse the text of an nftables file; throws NftParseError on anything unexpected. */
export function parseNftables(text: string): Ruleset {
  const ruleset: Ruleset = { commands: [], chains: [] };
  let table: string | undefined;
  let chain: Chain | undefined;
  text.split("\n").forEach((raw, index) => {
    const line = index + 1;
    const statement = stripComment(raw).trim();
    if (statement === "") return;
    const tableMatch = TABLE.exec(statement);
    const chainMatch = CHAIN.exec(statement);
    if (statement === "}") {
      if (chain) chain = undefined;
      else if (table !== undefined) table = undefined;
      else throw new NftParseError(line, "unmatched }");
    } else if (chain) {
      addStatements(chain, statement, line);
    } else if (table !== undefined) {
      if (!chainMatch?.[1]) throw new NftParseError(line, `unsupported table statement: ${statement}`);
      chain = { table, name: chainMatch[1], rules: [] };
      ruleset.chains.push(chain);
    } else if (tableMatch?.[1] && tableMatch[2]) {
      table = `${tableMatch[1]} ${tableMatch[2]}`;
    } else {
      if (!balanced(statement)) throw new NftParseError(line, "unbalanced braces");
      ruleset.commands.push(statement);
    }
  });
  if (table !== undefined) throw new NftParseError(text.split("\n").length, "unclosed block");
  return ruleset;
}

/** The input interface a rule requires, or undefined if it matches every interface. */
export function ruleInterface(rule: string): string | undefined {
  return /\biifname\s+"([^"]+)"/.exec(rule)?.[1] ?? /\biif\s+"?([\w.-]+)"?/.exec(rule)?.[1];
}

/** Destination ports a rule matches, as `port/protocol`. */
export function rulePorts(rule: string): string[] {
  const ports: string[] = [];
  for (const [, protocol, value] of rule.matchAll(/\b(tcp|udp)\s+dport\s+(\{[^}]*\}|\d+)/g)) {
    for (const port of (value ?? "").replace(/[{}]/g, "").split(",")) {
      ports.push(`${port.trim()}/${protocol ?? ""}`);
    }
  }
  return ports;
}

const isAccept = (rule: string): boolean => /\baccept$/.test(rule);

/** Ports a chain accepts from any interface. */
export function publicPorts(chain: Chain): string[] {
  return chain.rules
    .filter((rule) => isAccept(rule) && ruleInterface(rule) === undefined)
    .flatMap(rulePorts);
}

/** Ports a chain accepts only from the given interface. */
export function interfacePorts(chain: Chain, name: string): string[] {
  return chain.rules
    .filter((rule) => isAccept(rule) && ruleInterface(rule) === name)
    .flatMap(rulePorts);
}

const EXPECTED_ACCEPT = [/^ct state established,related accept$/, /^iif "?lo"? accept$/, /^icmp(v6)? type /];

/** Accept rules that open no port and are not connection state, loopback or ICMP. */
export function unexplainedAccepts(chain: Chain): string[] {
  return chain.rules.filter(
    (rule) => isAccept(rule) && rulePorts(rule).length === 0 && !EXPECTED_ACCEPT.some((re) => re.test(rule)),
  );
}
