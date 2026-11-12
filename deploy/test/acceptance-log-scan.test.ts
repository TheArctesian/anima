// deploy/acceptance/log-scan.ts (criterion 22, LOG-007, SEC-009, A-3). Every value here is synthetic and built at
// run time, so no secret shape sits in the repository. Each kind has a leaking line that is caught and near misses
// that are not; spawned runs prove the output holds only line numbers and kinds.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { findLeaks, leakKinds, lineViews, parseNeedles, plainLine, type LeakKind } from "../acceptance/log-scan.ts";

const SCRIPT = join(import.meta.dir, "..", "acceptance", "log-scan.ts");
const ESC = String.fromCharCode(0x1b);

/** A synthetic base64url run of this length with upper case, lower case, digits, `_` and `-`. */
function fake(length: number, seed = "qR7_vK2-mZ9xT4"): string {
  return seed.repeat(Math.ceil(length / seed.length)).slice(0, length);
}

const apiKey = `anima_${fake(43)}`;
const agentToken = `anima_agent_${fake(43, "Lp3Wq8_zN5-yHc")}`;
const enrolmentCode = `anima_enrol_${fake(22, "Ue6-Ji1_Ok4")}`;
const bearer = fake(43, "Gh5sD2-Fj8_kQw");
const ntfyToken = `tk_${fake(29, "a9B8c7D6e5F4")}`;

// Each kind: lines that leak it, then near misses that must stay clean of every kind.
const CASES: [LeakKind, string[], string[]][] = [
  ["api-key", [`{"level":"INFO","fields":{"key":"${apiKey}"}}`, `Bearer ${apiKey}.`],
    [`anima_${fake(42)}`, `anima_${fake(44)}`, "anima_check_up 1", `anima_${fake(6)} (display prefix)`]],
  ["agent-token", [`token=${agentToken}`], [`anima_agent_${fake(42)}`, `anima_agent_${fake(44)}`, "anima_agent_ prefix"]],
  ["enrolment-code", [`{"code":"${enrolmentCode}"}`], [`anima_enrol_${fake(21)}`, `anima_enrol_${fake(23)}`]],
  ["browser-cookie", [`set-cookie: __Host-anima=${fake(12)}; Secure; HttpOnly`, `cookie: theme=dark; __Host-anima=x1`],
    ["set-cookie: __Host-anima=; Max-Age=0; Path=/", 'cookie: __Host-anima=""', "__Host-anima="]],
  ["authorization-header", [`Authorization: Bearer ${fake(16)}`, `{"authorization": "basic ${fake(12)}"}`,
    String.raw`{\"authorization\":\"Bearer ${fake(10)}\"}`, `authorization=Bearer ${fake(9)}`],
    ["authorization header missing", "Authorization: Bearer", "Authorization: Bearer ***", "Bearer realm"]],
  ["bearer-value", [`GET /push/${bearer} 200`, `{"token":"${bearer}"}`],
    ["request_id=7f1c2a3b-4d5e-4f60-8a9b-0c1d2e3f4a5b", `sha256 ${"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"}`,
      `host key SHA256:${bearer}`, `integrity sha256-${bearer}`, `test log_007_the_whole_flow_holds_no_code_xyz ok`,
      `blob ${bearer}+Ab/cd==`, `run ${fake(42)} and ${fake(44)}`, `lower ${fake(43, "abcdefg_hijk")}`]],
  ["webhook-url", [`POST https://discord.com/api/webhooks/123456789/${fake(60)}`,
    `POST https://discord.com/api/v10/webhooks/123456789/${fake(60)}`,
    "url=https://hooks.slack.com/services/T0000/B0000/XXXXXXXX",
    "https://example.webhook.office.com/webhookb2/abc@def/IncomingWebhook/x/y"],
    ["origin https://discord.com", "https://discord.com/api/webhooks/123456789", "https://discord.com/api/webhooks/",
      "https://hooks.slack.com/services/"]],
  ["apprise-url", [`apprises://apprise.local/${fake(20)}`, `urls=tgram://${fake(20)}/12345`, "mailtos://smtp.example?to=x"],
    ["apprise://", "server https://apprise.local:8000", "mode STATELESS_URLS"]],
  ["ntfy-topic-url", ["POST https://ntfy.sh/anima-alerts-x7 200", "https://ntfy.example.org:8443/ops"],
    ["server https://ntfy.sh", "server https://ntfy.sh/", "kind NTFY"]],
  ["ntfy-token", [`token ${ntfyToken}`], [`tk_${fake(10)}`, `tk_${fake(30, "a9B8c7D6e5F4")}`, "tokenStored true"]],
  ["smtp-auth", [`C: AUTH PLAIN ${"AHVzZXIAcGFzcw=="}`, `AUTH LOGIN ${"dXNlcg=="}`],
    ["S: 250-AUTH PLAIN LOGIN XOAUTH2 OAUTHBEARER", "C: AUTH LOGIN", "AUTH CRAM-MD5"]],
  ["secret-field", ["password=hunter2x", '{"smtp_password":"pw-123"}', "client_secret: abc123", '{"smtpPassword":"pw-123"}',
    "clientSecret=abc123", '{"webhookSecret": "s3"}', "PASSWD = x", String.raw`{\"password\":\"q\"}`],
    ["password=", "password=&user=x", '"password": ""', '{"passwordStored":true}', "secret: [REDACTED]", "password: null",
      '{"reason":"SECRET_UNREADABLE"}', "secrets: 3", "mypassword=x"]],
  ["pem-private-key", ["-----BEGIN OPENSSH PRIVATE KEY-----", '{"k":"-----BEGIN PRIVATE KEY-----\\nMII"}'],
    ["-----BEGIN CERTIFICATE-----", "-----BEGIN PUBLIC KEY-----", "-----END PRIVATE KEY"]],
  ["url-credentials", ["postgres://anima:pw@db:5432/anima", "https://bob:x@example.org/", "https://:pw@hooks.example.net/x",
    `https://${fake(30)}@hooks.example.net/x`],
    ["ssh://okita@host", "https://hub.anima.ing:8443/graphql", "mail to ops@example.com", "https://a:@b/",
      `https://${fake(30, "abcdefgh_ij")}@h/`]],
];

describe("each kind", () => {
  for (const [kind, leaks, nearMisses] of CASES) {
    test(`catches a leaking ${kind} line`, () => {
      for (const line of leaks) {
        expect({ line, kinds: leakKinds(line, []) }).toEqual({ line, kinds: expect.arrayContaining([kind]) });
      }
    });

    test(`passes ${kind} near misses`, () => {
      for (const line of nearMisses) {
        expect({ line, kinds: leakKinds(line, []) }).toEqual({ line, kinds: [] });
      }
    });
  }

  test("a prefixed token is reported by its own kind only", () => {
    expect(leakKinds(agentToken, [])).toEqual(["agent-token"]);
    expect(leakKinds(enrolmentCode, [])).toEqual(["enrolment-code"]);
    expect(leakKinds(apiKey, [])).toEqual(["api-key"]);
  });

  test("a line with two kinds reports both, once each", () => {
    expect(leakKinds(`${apiKey} ${apiKey} password=x`, [])).toEqual(["api-key", "secret-field"]);
  });
});

// The hub's process log is tracing JSON; a `?` field is a Rust Debug string, escaped again (M-3).
function tracingLine(message: string): string {
  return JSON.stringify({ level: "WARN", fields: { message }, target: "anima_hub::notifications" });
}

describe("escaped and encoded views", () => {
  test("a value after an escaped newline or tab, or in a percent-encoded path, is found", () => {
    expect(leakKinds(tracingLine(`reply body:\n${bearer}`), [])).toEqual(["bearer-value"]);
    expect(leakKinds(tracingLine(`a\t${bearer}`), [])).toEqual(["bearer-value"]);
    expect(leakKinds(tracingLine(`url=https%3A%2F%2Fhub%2Fpush%2F${bearer}`), [])).toEqual(["bearer-value"]);
    const webhook = `https%3A%2F%2Fdiscord.com%2Fapi%2Fwebhooks%2F123%2F${fake(20)}`;
    expect(leakKinds(tracingLine(webhook), [])).toContain("webhook-url");
  });

  test("a Debug string of a JSON body, escaped twice, is found", () => {
    const debugged = (body: object) => tracingLine(`reply ${JSON.stringify(JSON.stringify(body))}`);
    expect(leakKinds(debugged({ password: "pw-123" }), [])).toEqual(["secret-field"]);
    expect(leakKinds(debugged({ authorization: `Bearer ${fake(16)}` }), [])).toEqual(["authorization-header"]);
  });

  test("Debug unicode escapes are undone and broken escapes never throw", () => {
    expect(lineViews(String.raw`a\u{41}B %41`)).toEqual([String.raw`a\u{41}B %41`, "aAB %41", "aAB A"]);
    const broken = String.raw`\u{110000} \x %zz %E2%82`;
    expect(lineViews(broken)).toEqual([broken]);
  });

  test("unescaping stops after four passes", () => {
    expect(lineViews("\\".repeat(64))).toHaveLength(5);
  });
});

describe("Apprise schemes the hub accepts", () => {
  test("every scheme in the hub's allow list is detected", () => {
    const rust = readFileSync(join(import.meta.dir, "../../crates/hub/src/notifications/channels/apprise_url.rs"), "utf8");
    const list = /const SCHEMES: \[&str; \d+\] = \[([^\]]*)\]/.exec(rust)?.[1] ?? "";
    const schemes = [...list.matchAll(/"([a-z0-9]+)"/g)].map((match) => String(match[1]));
    expect(schemes.length).toBeGreaterThanOrEqual(8);
    for (const scheme of schemes) {
      const line = `apprise ${scheme}://${fake(12)}/x`;
      expect({ line, kinds: leakKinds(line, []) }).toEqual({ line, kinds: expect.arrayContaining(["apprise-url"]) });
    }
  });
});

describe("needles", () => {
  test("a needle with a quote or backslash is found JSON-escaped once and twice", () => {
    const value = String.raw`pa"ss\word`;
    expect(leakKinds(tracingLine(`smtp ${value}`), [value])).toEqual(["needle"]);
    expect(leakKinds(tracingLine(JSON.stringify(`smtp ${value}`)), [value])).toEqual(["needle"]);
  });

  test("a URL needle is found in its normal form with a lower-case host", () => {
    const line = tracingLine("POST https://hooks.example.net/in/abc failed");
    expect(leakKinds(line, ["https://Hooks.Example.NET/in/abc"])).toEqual(["needle"]);
    expect(leakKinds("origin https://hooks.example.net", ["https://Hooks.Example.NET"])).toEqual(["needle"]);
  });

  test("a needle is found percent-encoded and Debug-escaped", () => {
    const encoded = "target=https%3A%2F%2Fhooks.example.net%2Fin%2Fabc";
    expect(leakKinds(encoded, ["https://hooks.example.net/in/abc"])).toEqual(["needle"]);
    expect(leakKinds(String.raw`topic \'ops\u{2019}room\'`, ["'ops’room'"])).toEqual(["needle"]);
  });

  test("a line holding a needle is a needle hit", () => {
    expect(leakKinds("channel opsroom-42 sent", ["opsroom-42"])).toEqual(["needle"]);
    expect(leakKinds("channel opsroom-4 sent", ["opsroom-42"])).toEqual([]);
  });

  test("the needles file is read one trimmed value per line, blank lines dropped", () => {
    expect(parseNeedles("alpha\r\n\n  beta  \n\n")).toEqual(["alpha", "beta"]);
  });
});

describe("terminal codes and line ends", () => {
  test("colour codes and a carriage return are removed before matching", () => {
    const coloured = `${ESC}[36mhub-1  |${ESC}[0m ${apiKey.slice(0, 20)}${ESC}[1;31m${apiKey.slice(20)}\r`;
    expect(plainLine(coloured)).toBe(`hub-1  | ${apiKey}`);
    expect(leakKinds(coloured, [])).toEqual(["api-key"]);
  });

  test("an OSC link sequence is removed", () => {
    const link = `${ESC}]8;;https://x${ESC}\\password=q${ESC}]8;;${String.fromCharCode(7)}`;
    expect(leakKinds(link, [])).toEqual(["secret-field"]);
  });

  test("leaks are numbered from line 1", () => {
    const lines = ["clean", `x ${apiKey}`, "clean", "password=x"];
    expect(findLeaks(lines, [])).toEqual([
      { line: 2, kind: "api-key" },
      { line: 4, kind: "secret-field" },
    ]);
  });
});

describe("property: an embedded API key is found whatever surrounds it", () => {
  test("random clean prefixes and suffixes", () => {
    let state = 20_261_111;
    const next = () => (state = (state * 1_103_515_245 + 12_345) % 2_147_483_648);
    const separators = [" ", '"', "=", ":", ",", "(", "\t", "'", "/", "[", "}"];
    const words = ["hub", "GET", "200", "device", "id=4", "{", "ok", "--"];
    for (let round = 0; round < 500; round += 1) {
      const pick = <T>(list: T[]) => list[next() % list.length] as T;
      const around = () => Array.from({ length: next() % 6 }, () => pick(words)).join(" ");
      const line = `${around()}${pick(separators)}${apiKey}${pick(separators)}${around()}`;
      expect({ line, kinds: leakKinds(line, []) }).toEqual({ line, kinds: expect.arrayContaining(["api-key"]) });
    }
  });
});

describe("hostile lines", () => {
  // Patterns start at a literal or a run boundary and bound their repeats, so time grows with the line, not its square.
  test("long adversarial lines scan in linear time", () => {
    const units = ["x", "http://", "a://b:", "x:", "AUTH PLAIN ", "password=", "anima_", "https://ntfy", `${ESC}[`, "a+"]
      .concat(["\\\\", "\\u{", "%25", '\\"', "authorization: ", "secret:  "]);
    for (const unit of units) {
      const line = unit.repeat(Math.ceil(100_000 / unit.length));
      const started = performance.now();
      leakKinds(line, []);
      expect({ unit, slow: performance.now() - started > 2_000 }).toEqual({ unit, slow: false });
    }
  });
});

let dir = "";
const needle = "my-own-ntfy-topic-q9";
const secrets = [apiKey, agentToken, enrolmentCode, bearer, ntfyToken, needle, "hunter2x"];

function writeFixtures() {
  dir = mkdtempSync(join(tmpdir(), "log-scan-"));
  writeFileSync(join(dir, "clean.log"), "start\r\nrequest_id=7f1c2a3b-4d5e-4f60-8a9b-0c1d2e3f4a5b\r\nstop\r\n");
  const leaking = [
    "start",
    `{"key":"${apiKey}"}`,
    `agent ${agentToken}`,
    `${ESC}[33mcode ${enrolmentCode}${ESC}[0m\r`,
    `push /push/${bearer}`,
    `ntfy ${ntfyToken} topic ${needle}`,
    "password=hunter2x",
  ];
  writeFileSync(join(dir, "leaking.log"), `${leaking.join("\n")}\n`);
  writeFileSync(join(dir, "needles"), `${needle}\n`);
  writeFileSync(join(dir, "empty-needles"), "\n  \n");
  mkdirSync(join(dir, "folder"));
}

function run(args: string[], stdin?: string) {
  const result = Bun.spawnSync(["bun", SCRIPT, ...args], { stdin: stdin === undefined ? "ignore" : Buffer.from(stdin) });
  return { code: result.exitCode, out: result.stdout.toString(), err: result.stderr.toString() };
}

function expectNoSecret(output: { out: string; err: string }) {
  for (const secret of secrets) {
    expect(output.out).not.toContain(secret);
    expect(output.err).not.toContain(secret);
  }
}

beforeAll(writeFixtures);

afterAll(() => {
  rmSync(dir, { recursive: true, force: true });
});

describe("spawned scan", () => {
  test("a clean log exits 0 with the line count", () => {
    expect(run([join(dir, "clean.log")])).toEqual({ code: 0, out: "scanned 3 lines, 0 hits\n", err: "" });
  });

  test("a leaking file exits 1 and prints only line numbers and kinds", () => {
    const result = run(["--needles", join(dir, "needles"), join(dir, "leaking.log")]);
    expect(result.code).toBe(1);
    expect(result.out).toBe(
      [
        "line 2: api-key",
        "line 3: agent-token",
        "line 4: enrolment-code",
        "line 5: bearer-value",
        "line 6: ntfy-token",
        "line 6: needle",
        "line 7: secret-field",
        "scanned 7 lines, 7 hits",
        "",
      ].join("\n"),
    );
    expect(result.err).toBe("");
    expectNoSecret(result);
  });

  test("stdin is scanned line by line, also with a value split across read chunks", () => {
    const padding = "x".repeat(70_000);
    const result = run(["-"], `${padding} ${apiKey}\nclean\n${padding}${"y".repeat(65_536)} ${bearer}`);
    expect(result).toEqual({ code: 1, out: "line 1: api-key\nline 3: bearer-value\nscanned 3 lines, 2 hits\n", err: "" });
  });

  test("a large log streams through", () => {
    const big = `${"hub-1 | {\"level\":\"INFO\",\"route\":\"/graphql\"}\n".repeat(100_000)}${apiKey}\n`;
    expect(run([], big)).toEqual({ code: 1, out: "line 100001: api-key\nscanned 100001 lines, 1 hits\n", err: "" });
  });

});

describe("spawned scan refusals", () => {
  test("an unreadable log, needles file or bad usage exits 2 without echoing input", () => {
    const missing = run([join(dir, "missing.log")]);
    const folder = run([join(dir, "folder")]);
    const noNeedles = run(["--needles", join(dir, "no-needles"), join(dir, "clean.log")]);
    const emptyNeedles = run(["--needles", join(dir, "empty-needles"), join(dir, "clean.log")]);
    const unknown = run([`--${apiKey}`, join(dir, "clean.log")]);
    const twoLogs = run([join(dir, "clean.log"), apiKey]);
    const emptyLog = run(["-"], "");
    const emptyFile = run(["/dev/null"]);
    expect(emptyLog.err).toBe("log-scan: the log is empty\n");
    for (const result of [missing, folder, noNeedles, emptyNeedles, unknown, twoLogs, emptyLog, emptyFile]) {
      expect(result.code).toBe(2);
      expect(result.out).not.toContain("scanned");
      expect(result.err).toMatch(/^log-scan: [^\n]+\n$/);
      expectNoSecret(result);
      expect(result.err).not.toContain(dir);
    }
  });

  test("a reader that closes the output early ends the scan with exit 2 and fixed text", () => {
    writeFileSync(join(dir, "many.log"), "password=hunter2x\n".repeat(40_000));
    const pipeline = 'bun "$0" "$1" | head -n 1 >/dev/null; echo "status ${PIPESTATUS[0]}"';
    const result = Bun.spawnSync(["bash", "-c", pipeline, SCRIPT, join(dir, "many.log")], { stdin: "ignore" });
    expect(result.stdout.toString()).toBe("status 2\n");
    expect(result.stderr.toString()).toBe("log-scan: the output closed (EPIPE)\n");
  });
});
