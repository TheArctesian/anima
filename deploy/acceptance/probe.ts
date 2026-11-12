// A live probe of a running hub by its origin (criteria 4, 12 and 13; SEC-001, SEC-002, API-003, INT-001 to
// INT-003, D-3, D-4, D-23, D-24, R-100). Run it from the repository root:
//   bun deploy/acceptance/probe.ts https://hub.anima.ing [--key-file <path>] [--ca <pem file>]
// Each check prints `ok <check>` or `FAIL <check>: <reason>`, then a summary line. A reason is fixed text: it may
// name a status code, a header or a K7 error code, never a key, a cookie, a header value or a body (A-2).
// Exit codes: 0 every check passed, 1 a check failed, 2 bad usage, an unusable key or CA file, or no hub answer.
// The key is read only from `--key-file` (an `anima_` API key; a READ key proves criterion 13), never from argv or
// the environment. Every document sent with it is invalid, so nothing can change even if the hub failed to refuse.
// TLS is always verified; `--ca` adds a trusted certificate for a local hub. Plain http is allowed only for a
// loopback host. Redirects are never followed, so the key never leaves the origin.
import { constants } from "node:fs";
import { open, type FileHandle } from "node:fs/promises";
import { parseArgs } from "node:util";
import { familiesVerdict, labelValuesVerdict, labelsVerdict } from "./exposition.ts";
import { ask, fakeCookie, hubOrigin, postGraphql, preflight, proxyConfigured, socketOutcome, type Answer, type Target } from "./hub-requests.ts";
import { parseExposition, type Family } from "./prometheus-text.ts";
import * as refusals from "./refusals.ts";
import { IMMUTABLE, NO_CACHE, cacheVerdict, fixedHeadersVerdict, immutableAssetPath, policyVerdict } from "./static-headers.ts";

const KEY_SHAPE = /^anima_[A-Za-z0-9_-]{43}$/;
const EXPOSITION_TYPE = "text/plain; version=0.0.4; charset=utf-8";
const USAGE = "usage: probe.ts <https://hub origin> [--key-file <path>] [--ca <pem file>]";
/** One deadline for the whole run: a safety net behind the per-request timeouts, with no test of its own. */
const RUN_LIMIT_MS = 120_000;
const NO_ANSWER = "no answer (network, TLS, timeout or too large)";

/** One check's result: `undefined` passed, else the fixed reason it failed. */
interface Check {
  name: string;
  reason: string | undefined;
}

type Verdict = (answer: Answer) => string | undefined;

class ProbeError extends Error {}

function check(name: string, answer: Answer | undefined, verdict: Verdict): Check {
  return { name, reason: answer === undefined ? NO_ANSWER : verdict(answer) };
}

/** The verdict, once the answer is a 200. */
function served(verdict: Verdict): Verdict {
  return (answer) => (answer.status === 200 ? verdict(answer) : `answered status ${String(answer.status)}, not 200`);
}

/** The cache rule and the K7 headers on one hashed asset the page names. */
async function assetChecks(target: Target, index: Answer | undefined): Promise<Check[]> {
  const path = index === undefined ? undefined : immutableAssetPath(index.body);
  if (path === undefined) {
    const reason = index === undefined ? NO_ANSWER : "the page names no file under /_app/immutable/";
    return [{ name: "asset-cache", reason }, { name: "asset-headers", reason }];
  }
  const asset = await ask(target, path);
  return [
    check("asset-cache", asset, served((answer) => cacheVerdict(answer.headers, IMMUTABLE))),
    check("asset-headers", asset, served((answer) =>
      fixedHeadersVerdict(answer.headers) ?? policyVerdict(answer.headers, target.origin))),
  ];
}

async function corsClientCheck(target: Target): Promise<Check> {
  for (const origin of refusals.CLIENT_ORIGINS) {
    const answer = await preflight(target, origin);
    const reason = answer === undefined ? NO_ANSWER : refusals.clientPreflightVerdict(answer.headers, origin);
    if (reason !== undefined) {
      return { name: "cors-client-origins", reason };
    }
  }
  return { name: "cors-client-origins", reason: undefined };
}

/** The checks that need no secret: K7 headers, the cookie and CORS refusals, and the closed `/metrics`. */
async function* signedOutChecks(target: Target, health: Answer): AsyncGenerator<Check> {
  yield check("health", health, served((answer) => (answer.body.trim() === "ok" ? undefined : "the body is not ok")));
  const index = await ask(target, "/");
  yield check("index-csp", index, served((answer) => policyVerdict(answer.headers, target.origin)));
  yield check("index-headers", index, served((answer) => fixedHeadersVerdict(answer.headers)));
  yield check("index-cache", index, served((answer) => cacheVerdict(answer.headers, NO_CACHE)));
  yield* await assetChecks(target, index);
  const foreign = { origin: refusals.FOREIGN_ORIGIN, cookie: fakeCookie() };
  const signedOut = await postGraphql(target, refusals.IDENTITY_QUERY, foreign);
  yield check("graphql-foreign-cookie", signedOut, (answer) =>
    refusals.foreignCookieVerdict(answer.status, answer.headers, answer.body));
  yield { name: "socket-foreign-cookie", reason: refusals.socketVerdict(await socketOutcome(target)) };
  yield await corsClientCheck(target);
  const foreignPreflight = await preflight(target, refusals.FOREIGN_ORIGIN);
  yield check("cors-foreign-origin", foreignPreflight, (answer) => refusals.foreignPreflightVerdict(answer.headers));
  const metrics = await ask(target, "/metrics");
  yield check("metrics-without-token", metrics, (answer) => refusals.metricsRefusedVerdict(answer.status));
}

/** The scrape with the key, then each rule on its text. Lines with a problem are skipped by the rules. */
function expositionChecks(scraped: Answer | undefined): Check[] {
  const scrape = check("metrics-with-key", scraped, served((answer) =>
    answer.headers.get("content-type") === EXPOSITION_TYPE ? undefined : "the content-type is not the K7 text format"));
  // The text is read whenever the scrape answered 200, so a wrong content type fails only its own check.
  const parsed = scraped?.status === 200 ? parseExposition(scraped.body) : undefined;
  const first = parsed?.problems[0];
  const count = String(parsed?.problems.length);
  const problems = first === undefined ? undefined : `line ${String(first.line)}: ${first.problem} (${count} problems)`;
  const rule = (name: string, verdict: (families: Map<string, Family>) => string | undefined): Check => ({
    name,
    reason: parsed === undefined ? "no exposition to read" : verdict(parsed.families),
  });
  return [
    scrape,
    { name: "metrics-text", reason: parsed === undefined ? "no exposition to read" : problems },
    rule("metrics-families", familiesVerdict),
    rule("metrics-labels", labelsVerdict),
    rule("metrics-label-values", labelValuesVerdict),
  ];
}

/** The checks with the operator's key: the exposition, then the READ and key-kind refusals. */
async function* keyChecks(target: Target, key: string): AsyncGenerator<Check> {
  const authorization = { authorization: `Bearer ${key}` };
  yield* expositionChecks(await ask(target, "/metrics", { headers: authorization }));
  const me = await postGraphql(target, refusals.IDENTITY_QUERY, authorization);
  yield check("key-accepted", me, (answer) => refusals.keyAcceptedVerdict(answer.status, answer.body));
  const mutation = await postGraphql(target, refusals.ORDINARY_MUTATION, authorization);
  yield check("read-key-mutation", mutation, (answer) => refusals.readOnlyVerdict(answer.status, answer.body));
  for (const root of refusals.REFUSED_ROOTS) {
    const refused = await postGraphql(target, refusals.refusedRootDocument(root), authorization);
    yield check(`refused-root-${root}`, refused, (answer) => refusals.refusedRootVerdict(answer.status, answer.body));
  }
}

function errorCode(error: unknown): string {
  const code = (error as { code?: unknown } | null)?.code;
  return typeof code === "string" && /^[A-Z0-9_]+$/.test(code) ? code : "read error";
}

/** At most `limit` bytes of an open file; more is refused. The size `fstat` gives is not trusted (S-C6). */
async function boundedText(handle: FileHandle, limit: number): Promise<string | undefined> {
  const buffer = Buffer.alloc(limit + 1);
  let length = 0;
  for (let read = 1; read > 0 && length <= limit; length += read) {
    ({ bytesRead: read } = await handle.read(buffer, length, limit + 1 - length, null));
  }
  return length > limit ? undefined : buffer.subarray(0, length).toString("utf8");
}

/** The text of a small regular file; `what` names it in a fixed message. Warns when others may read a key file. */
async function readSmallFile(path: string, what: "key file" | "CA file", limit: number): Promise<string> {
  const unreadable = `the ${what} is not a regular file of at most ${String(limit)} bytes`;
  try {
    // Non-blocking, so a FIFO in place of the file cannot hang the probe; the stat is of the opened file.
    const handle = await open(path, constants.O_RDONLY | constants.O_NONBLOCK);
    try {
      const stat = await handle.stat();
      if (!stat.isFile()) {
        throw new ProbeError(unreadable);
      }
      if (what === "key file" && (stat.mode & 0o077) !== 0) {
        process.stderr.write("probe: warning: the key file can be read by group or other users; chmod 600 it\n");
      }
      const text = await boundedText(handle, limit);
      if (text === undefined) {
        throw new ProbeError(unreadable);
      }
      return text;
    } finally {
      await handle.close();
    }
  } catch (error) {
    throw error instanceof ProbeError ? error : new ProbeError(`the ${what} cannot be read (${errorCode(error)})`);
  }
}

/** The API key in the file, trimmed; refused unless it has the `anima_` key shape, so no other file is sent. */
async function readKey(path: string): Promise<string> {
  const key = (await readSmallFile(path, "key file", 4096)).trim();
  if (!KEY_SHAPE.test(key)) {
    throw new ProbeError("the key file does not hold one Anima API key (anima_ and 43 base64url characters)");
  }
  return key;
}

async function readCa(path: string): Promise<string> {
  const pem = await readSmallFile(path, "CA file", 1024 * 1024);
  if (!pem.includes("-----BEGIN CERTIFICATE-----")) {
    throw new ProbeError("the CA file holds no PEM certificate");
  }
  return pem;
}

interface Request {
  target: Target;
  keyFile: string | undefined;
}

async function parseRequest(args: string[]): Promise<Request> {
  let parsed;
  try {
    const options = { "key-file": { type: "string" }, ca: { type: "string" } } as const;
    parsed = parseArgs({ args, options, allowPositionals: true, strict: true });
  } catch {
    // The parser's message quotes the argument, and a pasted key may be one, so it is not shown.
    throw new ProbeError(USAGE);
  }
  const origin = parsed.positionals.length === 1 ? hubOrigin(parsed.positionals[0] ?? "") : undefined;
  if (origin === undefined) {
    throw new ProbeError(`${USAGE}; the origin is https://host[:port], http only for a loopback host`);
  }
  const ca = parsed.values.ca === undefined ? undefined : await readCa(parsed.values.ca);
  return { target: { origin, ca }, keyFile: parsed.values["key-file"] };
}

/** Prints each check as it ends; returns how many ran and how many failed. */
async function report(groups: AsyncGenerator<Check>[]): Promise<{ total: number; failed: number }> {
  let total = 0;
  let failed = 0;
  for (const group of groups) {
    for await (const { name, reason } of group) {
      total += 1;
      failed += reason === undefined ? 0 : 1;
      process.stdout.write(reason === undefined ? `ok ${name}\n` : `FAIL ${name}: ${reason}\n`);
    }
  }
  return { total, failed };
}

/** Runs the probe for these arguments and returns the exit code; checks go to stdout, problems to stderr. */
async function main(args: string[]): Promise<number> {
  try {
    if (proxyConfigured(process.env)) {
      throw new ProbeError("a proxy variable (HTTP_PROXY, HTTPS_PROXY or ALL_PROXY) is set; unset it to probe the hub directly");
    }
    const request = await parseRequest(args);
    const key = request.keyFile === undefined ? undefined : await readKey(request.keyFile);
    const health = await ask(request.target, "/health");
    if (health === undefined) {
      throw new ProbeError("the hub cannot be reached (network, TLS or timeout)");
    }
    const groups = [signedOutChecks(request.target, health)];
    if (key !== undefined) {
      groups.push(keyChecks(request.target, key));
    }
    const { total, failed } = await report(groups);
    const scope = key === undefined ? " (no --key-file: the key checks did not run)" : "";
    process.stdout.write(`${String(total)} checks, ${String(failed)} failed${scope}\n`);
    return failed === 0 ? 0 : 1;
  } catch (error) {
    // Only fixed text reaches stderr: a runtime message could quote an argument, a file or an answer (A-2).
    process.stderr.write(`probe: ${error instanceof ProbeError ? error.message : "unexpected failure"}\n`);
    return 2;
  }
}

if (import.meta.main) {
  // A reader that closes the pipe early ends the run with fixed text, never a stack trace.
  process.stdout.on("error", () => {
    process.stderr.write("probe: the output closed\n");
    process.exit(2);
  });
  // One deadline for the whole run: a hub that answers each request just inside its timeout cannot hold it open.
  const deadline = setTimeout(() => {
    process.stderr.write(`probe: the run did not end within ${String(RUN_LIMIT_MS / 1000)} s\n`);
    process.exit(2);
  }, RUN_LIMIT_MS);
  process.exitCode = await main(process.argv.slice(2));
  clearTimeout(deadline);
}
