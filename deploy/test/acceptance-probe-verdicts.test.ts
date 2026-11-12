// deploy/acceptance/ pure parts (criteria 4, 12, 13; SEC-001, SEC-002, D-3, D-23, D-24, R-100, A-2): the probe's
// copies of hub lists pinned to the Rust source, and each verdict on right and wrong answers. No reason may quote
// what the hub sent.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { K7_FAMILIES, WINDOWS, familiesVerdict } from "../acceptance/exposition.ts";
import { hubOrigin, proxyConfigured } from "../acceptance/hub-requests.ts";
import type { Family } from "../acceptance/prometheus-text.ts";
import * as refusals from "../acceptance/refusals.ts";
import { fixedHeadersVerdict, immutableAssetPath, policyVerdict } from "../acceptance/static-headers.ts";

const source = (path: string) => readFileSync(join(import.meta.dir, "../..", path), "utf8");
const quoted = (text: string) => [...text.matchAll(/"([^"]+)"/g)].map((match) => String(match[1]));

describe("the probe's copies of the hub's lists", () => {
  test("SEC-002 G-4: the client origins are CLIENT_ORIGINS of lib.rs", () => {
    const list = /pub const CLIENT_ORIGINS: \[&str; \d+\] = \[([^\]]*)\]/.exec(source("crates/hub/src/lib.rs"))?.[1];
    expect(quoted(list ?? "")).toEqual([...refusals.CLIENT_ORIGINS]);
  });

  test("D-23 R-100: the refused roots are API_KEY_REFUSED_ROOTS of guard.rs", () => {
    const guard = source("crates/hub/src/access/guard.rs");
    const list = quoted(/const API_KEY_REFUSED_ROOTS: \[&str; \d+\] = \[([^\]]*)\]/.exec(guard)?.[1] ?? "");
    expect(list.length).toBeGreaterThan(20);
    expect(list).toEqual([...refusals.REFUSED_ROOTS]);
  });

  test("D-23: each refused root is a field of the root type its document selects it from", () => {
    const schema = source("contracts/schema.graphql");
    const fields = (type: string) => {
      const block = new RegExp(`^type ${type} \\{([\\s\\S]*?)^\\}`, "m").exec(schema)?.[1] ?? "";
      return [...block.matchAll(/^ {2}(\w+)[(:]/gm)].map((match) => String(match[1]));
    };
    for (const root of refusals.REFUSED_ROOTS) {
      const query = refusals.QUERY_ROOTS.includes(root);
      expect({ root, found: fields(query ? "Query" : "Mutation").includes(root) }).toEqual({ root, found: true });
      expect(refusals.refusedRootDocument(root)).toStartWith(query ? "query {" : "mutation {");
    }
  });

  test("D-23: the READ refusal text is the hub's ReadOnly error", () => {
    const text = /#\[error\("([^"]+)"\)\]\s*ReadOnly,/.exec(source("crates/hub/src/access/error.rs"))?.[1];
    expect(text).toBe(refusals.READ_ONLY_MESSAGE);
  });

  test("INT-001 CHK-011: the families and windows are those render.rs and model.rs write", () => {
    const render = source("crates/hub/src/checks/exposition/render.rs");
    expect(new Set(render.match(/"anima_[a-z_]+"/g)?.map((name) => name.slice(1, -1)))).toEqual(new Set(Object.keys(K7_FAMILIES)));
    const model = source("crates/hub/src/checks/exposition/model.rs");
    expect([...model.matchAll(/\(UptimeWindow::\w+, "(\w+)"\)/g)].map((match) => match[1])).toEqual([...WINDOWS]);
  });
});

describe("origins, assets and headers", () => {
  test("only https, or http on a loopback host, with nothing but an origin", () => {
    for (const ok of ["https://hub.anima.ing", "https://hub.anima.ing:8443/", "http://127.0.0.1:9", "http://localhost:1", "http://[::1]:2"]) {
      expect({ ok, url: hubOrigin(ok) !== undefined }).toEqual({ ok, url: true });
    }
    const refused = ["http://hub.anima.ing", "http://10.0.0.1", "https://u:p@hub.example", "https://hub.example/graphql",
      "https://hub.example/?x", "https://hub.example#x", "ftp://hub.example", "hub.anima.ing", "", "ws://127.0.0.1"];
    for (const bad of refused) {
      expect({ bad, url: hubOrigin(bad) }).toEqual({ bad, url: undefined });
    }
  });

  test("the asset is a hashed path on the hub's own origin, never another host", () => {
    expect(immutableAssetPath('<script>import("./_app/immutable/entry/start.x1.js")</script>')).toBe("/_app/immutable/entry/start.x1.js");
    expect(immutableAssetPath('<link href="https://cdn.example/_app/immutable/a.js">')).toBeUndefined();
    expect(immutableAssetPath('<link href="/_app/immutable/../../etc/a.js">')).toBeUndefined();
    expect(immutableAssetPath("<p>no assets</p>")).toBeUndefined();
  });

  test("SEC-001: a CSP with a directive twice, an extra directive or a wide script-src fails", () => {
    const origin = new URL("https://hub.example");
    const good = "default-src 'self'; script-src 'self' 'sha256-AbC+/9='; connect-src 'self' wss://hub.example; img-src 'self' data: blob:; "
      + "style-src 'self' 'unsafe-inline'; font-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
    const verdict = (policy: string) => policyVerdict(new Headers({ "content-security-policy": policy }), origin);
    expect(verdict(good)).toBeUndefined();
    expect(verdict(`${good}; default-src *`)).toBe("the CSP names a directive twice");
    expect(verdict(`${good}; worker-src *`)).toBe("the CSP has a directive K7 does not list");
    expect(verdict(good.replace("'sha256-AbC+/9='", "'sha256-AbC+/9=' https:"))).toContain("not a script hash");
    expect(verdict(good.replace("wss://hub.example", "wss://evil.example"))).toContain("connect-src");
    expect(verdict(good.replace(" 'sha256-AbC+/9='", ""))).toContain("script-src");
    // Only ASCII white space separates tokens: a no-break space keeps `'self'\u00a0https:` one (invalid) source.
    expect(verdict(good.replace("script-src 'self'", "script-src 'self'\u00a0https:"))).toContain("script-src");
    expect(verdict(good.replace("font-src", "font\u00a0src"))).toBe("the CSP has a malformed directive name");
    expect(verdict(good.replace("font-src 'self'", "font-src\t'self'"))).toBeUndefined();
  });

  test("SEC-001: each fixed header fails alone when it is missing or wrong", () => {
    const fixed: [string, string][] = [
      ["x-content-type-options", "nosniff"],
      ["referrer-policy", "no-referrer"],
      ["cross-origin-opener-policy", "same-origin"],
      ["strict-transport-security", "max-age=31536000"],
    ];
    expect(fixedHeadersVerdict(new Headers(fixed))).toBeUndefined();
    for (const [name, value] of fixed) {
      const missing = new Headers(fixed.filter(([other]) => other !== name));
      const wrong = new Headers(fixed.map(([other, text]) => [other, other === name ? `${text}x` : text]));
      expect(fixedHeadersVerdict(missing)).toBe(`the ${name} header is missing`);
      expect(fixedHeadersVerdict(wrong)).toBe(`the ${name} header is not ${value}`);
    }
  });

});

describe("refusals, families and proxies", () => {
  test("SEC-002: a preflight must allow POST for the client origin, and no credentials for a foreign one", () => {
    const origin = String(refusals.CLIENT_ORIGINS[0]);
    const allow = (methods: string) => new Headers({ "access-control-allow-origin": origin, "access-control-allow-methods": methods });
    expect(refusals.clientPreflightVerdict(allow("GET, POST"), origin)).toBeUndefined();
    expect(refusals.clientPreflightVerdict(allow("GET"), origin)).toBe(`the preflight from ${origin} does not allow POST`);
    expect(refusals.foreignPreflightVerdict(new Headers())).toBeUndefined();
    expect(refusals.foreignPreflightVerdict(new Headers({ "access-control-allow-credentials": "true" })))
      .toBe("the foreign origin may send credentials");
  });

  test("INT-001: a K7 family that is not a gauge fails", () => {
    const families = new Map<string, Family>(Object.keys(K7_FAMILIES).map((name) => [name, { name, type: "gauge", help: true, samples: [] }]));
    expect(familiesVerdict(families)).toBeUndefined();
    families.set("anima_check_up", { name: "anima_check_up", type: "counter", help: true, samples: [] });
    expect(familiesVerdict(families)).toBe("the family anima_check_up is not a gauge");
  });

  test("S-S2: a proxy variable in either case counts; an empty one does not", () => {
    for (const name of ["HTTP_PROXY", "https_proxy", "ALL_PROXY", "Http_Proxy"]) {
      expect({ name, set: proxyConfigured({ [name]: "http://127.0.0.1:3128" }) }).toEqual({ name, set: true });
    }
    expect(proxyConfigured({ HTTPS_PROXY: "", NO_PROXY: "*", PATH: "/bin" })).toBe(false);
  });

  test("D-3: a socket passes only when it opens and then closes with 4403", () => {
    expect(refusals.socketVerdict({ kind: "not-opened" })).toBe("the socket never opened, so no 4403 at connection_init");
    expect(refusals.socketVerdict({ kind: "closed", code: 4403 })).toBeUndefined();
    expect(refusals.socketVerdict({ kind: "closed", code: 1000 })).toBe("closed with 1000, not 4403");
    expect(refusals.socketVerdict({ kind: "message", acknowledged: true })).toBe("connection_init was acknowledged");
    expect(refusals.socketVerdict({ kind: "silent" })).toBe("no answer to connection_init");
  });

  test("D-23: a mutation that is not FORBIDDEN names a WRITE key; a body never reaches a reason", () => {
    const secret = "pve-01-secret-name";
    const answer = (code: string) => JSON.stringify({ data: null, errors: [{ message: secret, extensions: { code } }] });
    expect(refusals.readOnlyVerdict(200, answer("NOT_FOUND"))).toContain("WRITE key");
    expect(refusals.readOnlyVerdict(200, answer("UNAUTHENTICATED"))).toBe("answered UNAUTHENTICATED, not FORBIDDEN");
    expect(refusals.refusedRootVerdict(200, answer(secret))).toBe("answered an unknown code, not FORBIDDEN");
    expect(refusals.refusedRootVerdict(500, secret)).toBe("answered status 500, not a GraphQL answer");
    const withData = JSON.stringify({ data: { deleteDevice: "x" }, errors: [{ message: secret, extensions: { code: "FORBIDDEN" } }] });
    expect(refusals.refusedRootVerdict(200, withData)).toBe("answered FORBIDDEN but with data");
    expect(refusals.foreignCookieVerdict(403, new Headers(), secret)).toBeUndefined();
  });
});

