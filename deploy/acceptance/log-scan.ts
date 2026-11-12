// Scans a hub log for the never-log shapes of K7 (criterion 22, LOG-007, SEC-009). Run it on the live log
// or a file, from the repository root, with pipefail so a failed `docker compose` is not an empty clean log:
//   bash -o pipefail -c 'docker compose logs --no-color hub | bun deploy/acceptance/log-scan.ts [--needles <file>]'
//   bun deploy/acceptance/log-scan.ts [--needles <file>] <log file>
// It prints `line <n>: <kind>` for each kind found on a line, then `scanned <n> lines, <m> hits`. It never
// prints the matched text, the line or a needle, and no error message holds input (A-3).
// The hub log is tracing JSON whose `?` fields are Rust Debug strings escaped again, so each line is read in
// several views: as written; with JSON and Debug escapes undone, once per pass up to four passes; and that
// last view percent-decoded. A kind found in any view counts once.
// The needles file holds one exact value per line (surrounding white space and blank lines are ignored); a line
// that contains one in any view (so also JSON- or Debug-escaped up to four times, or percent-encoded), or for a URL
// in its normal form with a lower-case host, is a `needle` hit. Other spellings (non-ASCII percent-encoding, a
// different case in the path) are not found. The file stays on the machine; give it for values only the operator
// knows (channel URLs, ntfy topics on a server whose host does not say `ntfy`, generic webhook URLs, SMTP
// passwords).
// Exit codes: 0 no hit, 1 one or more hits, 2 bad usage, an empty or unreadable log or needles file, or closed
// output. What each shape skips on purpose is listed in leak-shapes.ts.
import { createReadStream } from "node:fs";
import { readFile } from "node:fs/promises";
import { parseArgs } from "node:util";
import { SHAPE_KINDS, shapeKinds, type ShapeKind } from "./leak-shapes.ts";

/** The stable name of a finding: a shape kind, or `needle` for an operator value. */
export type LeakKind = ShapeKind | "needle";

/** One finding: a 1-based line number and the kind of value found there. */
export interface Leak {
  line: number;
  kind: LeakKind;
}

const ESC = String.fromCharCode(0x1b);
const BEL = String.fromCharCode(0x07);
// CSI sequences (colours, cursor) and OSC sequences (titles, links) that `docker compose logs` may add.
const TERMINAL_CODES = new RegExp(`${ESC}\\[[0-9;?]*[ -/]*[@-~]|${ESC}\\][^${BEL}${ESC}]*(?:${BEL}|${ESC}\\\\)`, "g");
// JSON escapes and Rust Debug escapes (`\u{1f600}`, `\'`, `\0`).
const ESCAPE = /\\(?:u([0-9A-Fa-f]{4})|u\{([0-9A-Fa-f]{1,6})\}|(["\\/'bfnrt0]))/g;
const ESCAPED: Readonly<Record<string, string>> = {
  '"': '"', "\\": "\\", "/": "/", "'": "'", b: "\b", f: "\f", n: "\n", r: "\r", t: "\t", 0: "\0",
};
const KIND_ORDER: readonly LeakKind[] = [...SHAPE_KINDS, "needle"];
const PERCENT = /%([0-9A-Fa-f]{2})/g;
const MAX_PASSES = 4;

/** The line as the scanner reads it: no trailing carriage return, no terminal colour or control sequences. */
export function plainLine(raw: string): string {
  return raw.replace(/\r$/, "").replace(TERMINAL_CODES, "");
}

function unescapeOnce(text: string): string {
  return text.replace(ESCAPE, (escape, unit?: string, point?: string, single?: string) => {
    const code = Number.parseInt(unit ?? point ?? "", 16);
    if (single !== undefined) {
      return ESCAPED[single] ?? escape;
    }
    return code <= 0x10ffff ? String.fromCodePoint(code) : escape;
  });
}

/** ASCII percent escapes only, so decoding never fails on a stray `%` or a broken UTF-8 sequence. */
function percentDecodeOnce(text: string): string {
  return text.replace(PERCENT, (escape, hex: string) => {
    const code = Number.parseInt(hex, 16);
    return code < 0x80 ? String.fromCharCode(code) : escape;
  });
}

function settle(text: string, needed: string, pass: (text: string) => string, views: string[]): string {
  let current = text;
  for (let round = 0; round < MAX_PASSES && current.includes(needed); round += 1) {
    const next = pass(current);
    if (next === current) {
      break;
    }
    views.push(next);
    current = next;
  }
  return current;
}

/** The views of one plain line: as written, each unescape pass, then the last of those percent-decoded. */
export function lineViews(line: string): string[] {
  const views = [line];
  const unescaped = settle(line, "\\", unescapeOnce, views);
  settle(unescaped, "%", percentDecodeOnce, views);
  return views;
}

const needleCache = new WeakMap<readonly string[], string[]>();

function urlForm(value: string): string | undefined {
  try {
    return new URL(value).href;
  } catch {
    return undefined;
  }
}

/** Each needle as written and, for a URL, its normal form (lower-case host); escaped spellings meet the views. */
function needleForms(needles: readonly string[]): string[] {
  let forms = needleCache.get(needles);
  if (forms === undefined) {
    const urls = needles.map(urlForm).filter((href) => href !== undefined);
    forms = [...new Set([...needles, ...urls, ...urls.map((href) => href.replace(/\/$/, ""))])];
    needleCache.set(needles, forms);
  }
  return forms;
}

/** Every leak kind on one raw log line, in any of its views, each once and in a fixed order. */
export function leakKinds(raw: string, needles: readonly string[]): LeakKind[] {
  const views = lineViews(plainLine(raw));
  const found = new Set<LeakKind>(views.flatMap(shapeKinds));
  const forms = needleForms(needles);
  if (views.some((view) => forms.some((form) => view.includes(form)))) {
    found.add("needle");
  }
  return KIND_ORDER.filter((kind) => found.has(kind));
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

/** Writes one line of output; a reader that goes away surfaces as a stdout error event (see stopOnClosedOutput). */
function report(text: string): void {
  process.stdout.write(`${text}\n`);
}

async function scanStream(stream: AsyncIterable<string | Uint8Array>, needles: readonly string[]): Promise<number> {
  let number = 0;
  let hits = 0;
  try {
    for await (const raw of streamLines(stream)) {
      number += 1;
      for (const kind of leakKinds(raw, needles)) {
        hits += 1;
        report(`line ${String(number)}: ${kind}`);
      }
    }
  } catch (error) {
    throw new ScanError(`the log cannot be read after line ${String(number)} (${errorCode(error)})`);
  }
  if (number === 0) {
    throw new ScanError("the log is empty");
  }
  report(`scanned ${String(number)} lines, ${String(hits)} hits`);
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

/** A reader that closes the pipe early may surface as an asynchronous stream error; it ends the run the same way. */
function stopOnClosedOutput(): void {
  process.stdout.on("error", (error) => {
    process.stderr.write(`log-scan: the output closed (${errorCode(error)})\n`);
    process.exit(2);
  });
}

if (import.meta.main) {
  stopOnClosedOutput();
  process.exitCode = await main(process.argv.slice(2));
}
