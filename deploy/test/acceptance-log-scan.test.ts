// deploy/acceptance/log-scan.ts (criterion 22, LOG-007, SEC-009, A-3). Every value here is synthetic and built at
// run time, so no secret shape sits in the repository. Each kind has a leaking line that is caught and near misses
// that are not; spawned runs prove the output holds only line numbers and kinds.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { findLeaks, leakKinds, parseNeedles, plainLine, type LeakKind } from "../acceptance/log-scan.ts";

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

interface Case {
  kind: LeakKind;
  leaks: string[];
  nearMisses: string[];
}

const CASES: Case[] = [
  {
    kind: "api-key",
    leaks: [`{"level":"INFO","fields":{"key":"${apiKey}"}}`, `Bearer ${apiKey}.`],
    nearMisses: [`anima_${fake(42)}`, `anima_${fake(44)}`, "anima_check_up 1", `anima_${fake(6)} (display prefix)`],
  },
  {
    kind: "agent-token",
    leaks: [`token=${agentToken}`],
    nearMisses: [`anima_agent_${fake(42)}`, `anima_agent_${fake(44)}`, "anima_agent_ prefix refused"],
  },
  {
    kind: "enrolment-code",
    leaks: [`{"code":"${enrolmentCode}"}`],
    nearMisses: [`anima_enrol_${fake(21)}`, `anima_enrol_${fake(23)}`],
  },
  {
    kind: "browser-cookie",
    leaks: [`set-cookie: __Host-anima=${fake(12)}; Secure; HttpOnly`, `cookie: theme=dark; __Host-anima=x1`],
    nearMisses: ["set-cookie: __Host-anima=; Max-Age=0; Path=/", 'cookie: __Host-anima=""', "__Host-anima="],
  },
  {
    kind: "authorization-header",
    leaks: [
      `Authorization: Bearer ${fake(16)}`,
      `{"authorization": "basic ${fake(12)}"}`,
      String.raw`{\"authorization\":\"Bearer ${fake(10)}\"}`,
      `authorization=Bearer ${fake(9)}`,
    ],
    nearMisses: ["authorization header missing", "Authorization: Bearer", "Authorization: Bearer ***", "Bearer realm"],
  },
  {
    kind: "bearer-value",
    leaks: [`GET /push/${bearer} 200`, `{"token":"${bearer}"}`],
    nearMisses: [
      "request_id=7f1c2a3b-4d5e-4f60-8a9b-0c1d2e3f4a5b",
      `sha256 ${"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"}`,
      `host key SHA256:${bearer}`,
      `integrity sha256-${bearer}`,
      `test log_007_the_whole_flow_holds_no_code_xyz ok`,
      `blob ${bearer}+Ab/cd==`,
      `run ${fake(42)} and ${fake(44)}`,
      `lower ${fake(43, "abcdefg_hijk")}`,
    ],
  },
  {
    kind: "webhook-url",
    leaks: [
      `POST https://discord.com/api/webhooks/123456789/${fake(60)}`,
      "url=https://hooks.slack.com/services/T0000/B0000/XXXXXXXX",
      "https://example.webhook.office.com/webhookb2/abc@def/IncomingWebhook/x/y",
    ],
    nearMisses: [
      "origin https://discord.com",
      "https://discord.com/api/webhooks/123456789",
      "https://discord.com/api/webhooks/",
      "https://hooks.slack.com/services/",
    ],
  },
  {
    kind: "apprise-url",
    leaks: [`apprises://apprise.local/${fake(20)}`, `urls=tgram://${fake(20)}/12345`, "mailtos://smtp.example?to=x"],
    nearMisses: ["apprise://", "server https://apprise.local:8000", "mode STATELESS_URLS"],
  },
  {
    kind: "ntfy-topic-url",
    leaks: ["POST https://ntfy.sh/anima-alerts-x7 200", "https://ntfy.example.org:8443/ops"],
    nearMisses: ["server https://ntfy.sh", "server https://ntfy.sh/", "kind NTFY"],
  },
  {
    kind: "ntfy-token",
    leaks: [`token ${ntfyToken}`],
    nearMisses: [`tk_${fake(10)}`, `tk_${fake(30, "a9B8c7D6e5F4")}`, "tokenStored true"],
  },
  {
    kind: "smtp-auth",
    leaks: [`C: AUTH PLAIN ${"AHVzZXIAcGFzcw=="}`, `AUTH LOGIN ${"dXNlcg=="}`],
    nearMisses: ["S: 250-AUTH PLAIN LOGIN XOAUTH2 OAUTHBEARER", "C: AUTH LOGIN", "AUTH CRAM-MD5"],
  },
  {
    kind: "secret-field",
    leaks: [
      "password=hunter2x",
      '{"smtp_password":"pw-123"}',
      "client_secret: abc123",
      "PASSWD = x",
      String.raw`{\"password\":\"q\"}`,
    ],
    nearMisses: [
      "password=",
      "password=&user=x",
      '"password": ""',
      '{"passwordStored":true}',
      "secret: [REDACTED]",
      "password: null",
      '{"reason":"SECRET_UNREADABLE"}',
      "secrets: 3",
    ],
  },
  {
    kind: "pem-private-key",
    leaks: ["-----BEGIN OPENSSH PRIVATE KEY-----", '{"k":"-----BEGIN PRIVATE KEY-----\\nMII"}'],
    nearMisses: ["-----BEGIN CERTIFICATE-----", "-----BEGIN PUBLIC KEY-----", "-----END PRIVATE KEY"],
  },
  {
    kind: "url-credentials",
    leaks: ["postgres://anima:pw@db:5432/anima", "https://bob:x@example.org/"],
    nearMisses: ["ssh://okita@host", "https://hub.anima.ing:8443/graphql", "mail to ops@example.com", "https://a:@b/"],
  },
];

describe("each kind", () => {
  for (const { kind, leaks, nearMisses } of CASES) {
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

describe("needles", () => {
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
    const units = ["x", "http://", "a://b:", "x:", "AUTH PLAIN ", "password=", "anima_", "https://ntfy", `${ESC}[`, "a+"];
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

describe("spawned scan", () => {
  beforeAll(writeFixtures);

  afterAll(() => {
    rmSync(dir, { recursive: true, force: true });
  });

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

  test("an unreadable log, needles file or bad usage exits 2 without echoing input", () => {
    const missing = run([join(dir, "missing.log")]);
    const folder = run([join(dir, "folder")]);
    const noNeedles = run(["--needles", join(dir, "no-needles"), join(dir, "clean.log")]);
    const emptyNeedles = run(["--needles", join(dir, "empty-needles"), join(dir, "clean.log")]);
    const unknown = run([`--${apiKey}`, join(dir, "clean.log")]);
    const twoLogs = run([join(dir, "clean.log"), apiKey]);
    for (const result of [missing, folder, noNeedles, emptyNeedles, unknown, twoLogs]) {
      expect(result.code).toBe(2);
      expect(result.out).not.toContain("scanned");
      expect(result.err).toMatch(/^log-scan: [^\n]+\n$/);
      expectNoSecret(result);
      expect(result.err).not.toContain(dir);
    }
  });
});
