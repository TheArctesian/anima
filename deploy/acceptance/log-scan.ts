// Scans a hub log for the never-log shapes of K7 (criterion 22, LOG-007, SEC-009). Run it on the live log
// or a file, from the repository root:
//   docker compose logs --no-color hub | bun deploy/acceptance/log-scan.ts [--needles <file>]
//   bun deploy/acceptance/log-scan.ts [--needles <file>] <log file>
// It prints `line <n>: <kind>` for each kind found on a line, then `scanned <n> lines, <m> hits`. It never
// prints the matched text, the line or a needle, and no error message holds input (A-3). The needles file holds
// one exact value per line (surrounding white space and blank lines are ignored); a line that contains one is a
// `needle` hit. The file stays on the machine; give it for values only the operator knows (channel URLs, ntfy
// topics on a server whose host does not say `ntfy`, generic webhook URLs, SMTP passwords).
// Exit codes: 0 no hit, 1 one or more hits, 2 bad usage or an unreadable log or needles file.
//
// Not flagged, on purpose (false positives would bury real hits):
// - A bare 43-character base64url run (a 32-byte bearer: push token, plain API token) needs an upper-case
//   letter, a lower-case letter and a digit. Snake-case names, hex digests (SHA-256 hex is 64 characters and
//   has no upper case) and words are skipped; a random value lacks a digit about once in 1,500 draws.
// - A run that sits inside a longer base64url or standard base64 run (`+` beside it), or that follows
//   `SHA256:`/`sha256-`, is a fingerprint or digest, not a bearer. UUIDs are 36 characters and never match.
// - `__Host-anima=;` (and a quoted empty value) is the cookie cleared by logout.
// - `password=`, `secret: ""`, and the placeholders null, true, false, `***` and `[REDACTED]` hold no value.
// - A Discord webhook URL without its token segment, a Slack hooks URL without a path, `apprise://` with
//   nothing after it, an ntfy server origin without a topic, and URLs with a user but no password.
import { createReadStream } from "node:fs";
import { readFile } from "node:fs/promises";
import { parseArgs } from "node:util";

/** The stable name of a leak shape; the operator records it with the line number. */
export type LeakKind =
  | "api-key"
  | "agent-token"
  | "enrolment-code"
  | "browser-cookie"
  | "authorization-header"
  | "bearer-value"
  | "webhook-url"
  | "apprise-url"
  | "ntfy-topic-url"
  | "ntfy-token"
  | "smtp-auth"
  | "secret-field"
  | "pem-private-key"
  | "url-credentials"
  | "needle";

/** One finding: a 1-based line number and the kind of value found there. */
export interface Leak {
  line: number;
  kind: LeakKind;
}

interface Shape {
  kind: LeakKind;
  pattern: RegExp;
  /** Keeps a match only when it really holds a value; absent means every match counts. */
  holdsValue?: (match: RegExpExecArray, line: string) => boolean;
}

const B64URL = String.raw`A-Za-z0-9_\-`;
const ESC = String.fromCharCode(0x1b);
const BEL = String.fromCharCode(0x07);
// CSI sequences (colours, cursor) and OSC sequences (titles, links) that `docker compose logs` may add.
const TERMINAL_CODES = new RegExp(`${ESC}\\[[0-9;?]*[ -/]*[@-~]|${ESC}\\][^${BEL}${ESC}]*(?:${BEL}|${ESC}\\\\)`, "g");
const PLACEHOLDER = /^(?:null|none|true|false|\*+|\[redacted\]|<redacted>|redacted)$/i;
const APPRISE_SCHEMES =
  "apprises?|discord|slack|tgram|mailtos?|ntfys?|pover|pushover|gotifys?|matrixs?|msteams|pbul|jsons?|xmls?|" +
  "forms?|signals?|rockets?|mmosts?|zulip|join|ifttt|twilio";

function isBearerValue(match: RegExpExecArray, line: string): boolean {
  const value = match[0];
  const before = line.slice(Math.max(0, match.index - 7), match.index);
  return /[A-Z]/.test(value) && /[a-z]/.test(value) && /\d/.test(value) && !/sha256[:-]$/i.test(before);
}

function isRealValue(match: RegExpExecArray): boolean {
  const value = match[1] ?? "";
  return value !== "" && !PLACEHOLDER.test(value);
}

const SHAPES: readonly Shape[] = [
  { kind: "agent-token", pattern: new RegExp(`anima_agent_[${B64URL}]{43}(?![${B64URL}])`, "g") },
  { kind: "enrolment-code", pattern: new RegExp(`anima_enrol_[${B64URL}]{22}(?![${B64URL}])`, "g") },
  { kind: "api-key", pattern: new RegExp(`anima_[${B64URL}]{43}(?![${B64URL}])`, "g") },
  { kind: "browser-cookie", pattern: /__Host-anima=[^;\s"',]/g },
  {
    kind: "authorization-header",
    pattern: /authorization\\?["']?\s*[:=]\s*\\?["']?(?:bearer|basic)\s+[A-Za-z0-9._~+/=-]{8,}/gi,
  },
  {
    kind: "bearer-value",
    pattern: new RegExp(`(?<![${B64URL}+])[${B64URL}]{43}(?![${B64URL}+])`, "g"),
    holdsValue: isBearerValue,
  },
  {
    kind: "webhook-url",
    pattern:
      /discord(?:app)?\.com\/api\/webhooks\/\d+\/[A-Za-z0-9_-]|hooks\.slack\.com\/(?:services|workflows|triggers)\/[A-Za-z0-9]|webhook\.office\.com\/webhookb2\/[A-Za-z0-9]/gi,
  },
  {
    kind: "apprise-url",
    pattern: new RegExp(`(?<![A-Za-z0-9+.-])(?:${APPRISE_SCHEMES})://[^\\s"'<>]`, "gi"),
  },
  { kind: "ntfy-topic-url", pattern: /https?:\/\/[A-Za-z0-9.-]{0,253}ntfy[A-Za-z0-9.-]{0,253}(?::\d+)?\/[A-Za-z0-9_-]/gi },
  { kind: "ntfy-token", pattern: /(?<![A-Za-z0-9_])tk_[A-Za-z0-9]{29}(?![A-Za-z0-9_])/g },
  {
    kind: "smtp-auth",
    pattern: /\bAUTH\s+(?:PLAIN|LOGIN|XOAUTH2|OAUTHBEARER|CRAM-MD5)\s+(?=\S*[a-z+/=])[A-Za-z0-9+/]{4,}={0,2}(?![\w+/=-])/g,
  },
  {
    kind: "secret-field",
    pattern: /(?<![A-Za-z])(?:password|passwd|passphrase|secret)\\?["']?\s*[:=][ \t]*\\?["']?([^\s"',;&}\\]*)/gi,
    holdsValue: isRealValue,
  },
  { kind: "pem-private-key", pattern: /-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY(?: BLOCK)?-----/g },
  { kind: "url-credentials", pattern: /(?<![A-Za-z0-9+.-])[A-Za-z][A-Za-z0-9+.-]{0,31}:\/\/[^\s/?#@:"'<>]+:[^\s/?#@"'<>]+@/g },
];

/** The line as the scanner reads it: no trailing carriage return, no terminal colour or control sequences. */
export function plainLine(raw: string): string {
  return raw.replace(/\r$/, "").replace(TERMINAL_CODES, "");
}

function hasShape(shape: Shape, line: string): boolean {
  for (const match of line.matchAll(shape.pattern)) {
    if (shape.holdsValue === undefined || shape.holdsValue(match as RegExpExecArray, line)) {
      return true;
    }
  }
  return false;
}

/** Every leak kind on one raw log line, each once, in a fixed order; needles are matched literally. */
export function leakKinds(raw: string, needles: readonly string[]): LeakKind[] {
  const line = plainLine(raw);
  const kinds = SHAPES.filter((shape) => hasShape(shape, line)).map((shape) => shape.kind);
  if (needles.some((needle) => line.includes(needle))) {
    kinds.push("needle");
  }
  return kinds;
}

/** Every leak in a log, numbered from line 1. */
export function findLeaks(lines: Iterable<string>, needles: readonly string[]): Leak[] {
  const leaks: Leak[] = [];
  let number = 0;
  for (const raw of lines) {
    number += 1;
    for (const kind of leakKinds(raw, needles)) {
      leaks.push({ line: number, kind });
    }
  }
  return leaks;
}

/** The values of a needles file: one per line, trimmed, blank lines dropped. */
export function parseNeedles(text: string): string[] {
  return text
    .split("\n")
    .map((value) => value.trim())
    .filter((value) => value !== "");
}

/** The lines of a stream as they arrive, so a large log is never held whole. */
async function* streamLines(stream: AsyncIterable<string | Uint8Array>): AsyncGenerator<string> {
  const decoder = new TextDecoder();
  let rest = "";
  for await (const chunk of stream) {
    const text = typeof chunk === "string" ? chunk : decoder.decode(chunk, { stream: true });
    const end = text.lastIndexOf("\n");
    if (end === -1) {
      rest += text;
      continue;
    }
    yield* (rest + text.slice(0, end)).split("\n");
    rest = text.slice(end + 1);
  }
  rest += decoder.decode();
  if (rest !== "") {
    yield rest;
  }
}

class ScanError extends Error {}

interface Request {
  logPath: string | undefined;
  needlesPath: string | undefined;
}

function parseRequest(args: string[]): Request {
  let parsed;
  try {
    parsed = parseArgs({ args, options: { needles: { type: "string" } }, allowPositionals: true, strict: true });
  } catch {
    // The parser's message quotes the argument; the argument may be a value, so it is not shown.
    throw new ScanError("usage: log-scan.ts [--needles <file>] [<log file> | -]");
  }
  if (parsed.positionals.length > 1) {
    throw new ScanError("usage: log-scan.ts [--needles <file>] [<log file> | -]");
  }
  const path = parsed.positionals[0];
  return { logPath: path === "-" ? undefined : path, needlesPath: parsed.values.needles };
}

async function readNeedles(path: string | undefined): Promise<string[]> {
  if (path === undefined) {
    return [];
  }
  let text: string;
  try {
    text = await readFile(path, "utf8");
  } catch (error) {
    throw new ScanError(`the needles file cannot be read (${errorCode(error)})`);
  }
  const needles = parseNeedles(text);
  if (needles.length === 0) {
    throw new ScanError("the needles file holds no values");
  }
  return needles;
}

function errorCode(error: unknown): string {
  const code = (error as { code?: unknown } | null)?.code;
  return typeof code === "string" && /^[A-Z0-9_]+$/.test(code) ? code : "read error";
}

async function scanStream(stream: AsyncIterable<string | Uint8Array>, needles: readonly string[]): Promise<number> {
  let number = 0;
  let hits = 0;
  try {
    for await (const raw of streamLines(stream)) {
      number += 1;
      for (const kind of leakKinds(raw, needles)) {
        hits += 1;
        process.stdout.write(`line ${String(number)}: ${kind}\n`);
      }
    }
  } catch (error) {
    throw new ScanError(`the log cannot be read after line ${String(number)} (${errorCode(error)})`);
  }
  process.stdout.write(`scanned ${String(number)} lines, ${String(hits)} hits\n`);
  return hits;
}

/** Runs the scan for these arguments and returns the exit code; output goes to stdout, problems to stderr. */
async function main(args: string[]): Promise<number> {
  try {
    const request = parseRequest(args);
    const needles = await readNeedles(request.needlesPath);
    if (request.logPath === undefined && process.stdin.isTTY) {
      throw new ScanError("no log on stdin: pipe the log in or give a file");
    }
    const stream = request.logPath === undefined ? process.stdin : createReadStream(request.logPath);
    return (await scanStream(stream, needles)) === 0 ? 0 : 1;
  } catch (error) {
    // Only fixed text reaches stderr: a runtime message could quote the log or a needle (A-3).
    const reason = error instanceof ScanError ? error.message : `unexpected failure (${errorCode(error)})`;
    process.stderr.write(`log-scan: ${reason}\n`);
    return 2;
  }
}

if (import.meta.main) {
  process.exitCode = await main(process.argv.slice(2));
}
