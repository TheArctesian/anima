// What the hub must refuse, and how a refusal looks, as pure verdicts on one answer (SEC-002, API-003, D-3, D-4,
// D-23, D-24, R-100). A verdict is `undefined` when the hub refused as it should, else a fixed reason. A reason
// may name a status code or one of the K7 error codes, never a value or body the hub sent (A-2).

/** The desktop web view origins: a copy of `CLIENT_ORIGINS` in `crates/hub/src/lib.rs` (a test keeps them equal). */
export const CLIENT_ORIGINS: readonly string[] = ["tauri://localhost", "http://tauri.localhost"];

/** An origin the hub must never trust. */
export const FOREIGN_ORIGIN = "https://evil.example";

/**
 * The roots an API key may never select: a copy of `API_KEY_REFUSED_ROOTS` in `crates/hub/src/fleet/access/guard.rs`, in its
 * order, with the VPN roots (VP-7, VP-10) and without the recording roots (R-113); a test keeps the lists equal (D-23,
 * R-100, R-107).
 */
export const REFUSED_ROOTS: readonly string[] = [
  "createApiKey",
  "revokeApiKey",
  "apiKeys",
  "signInBrowser",
  "logout",
  "startSession",
  "openFiles",
  "setSshCredential",
  "clearSshCredential",
  "setRdpCredential",
  "clearRdpCredential",
  "setVncCredential",
  "clearVncCredential",
  "enrolAgent",
  "revokeAgentToken",
  "acceptHostKey",
  "acceptServerCertificate",
  "acceptWebCertificate",
  "updateDevice",
  "deleteDevice",
  "setRetention",
  "enrolBuilder",
  "revokeBuilder",
  "createRelease",
  "cancelBuildJob",
  "createVpnInvite",
  "createRouterKey",
  "registerVpnNode",
  "renewVpnKey",
  "approveSiteRoute",
];

/** The refused roots that are query fields; every other one is a mutation field. */
export const QUERY_ROOTS: readonly string[] = ["apiKeys"];

/**
 * An argument no field has: a document that selects it fails validation, so it can never reach a resolver, even if
 * the guard that must refuse it first were gone. The guard refuses by root name before validation.
 */
const NO_SUCH_ARGUMENT = "animaProbeOnly: true";

/** A synthetic check ID that no hub holds. */
const NO_SUCH_CHECK = "00000000-0000-7000-8000-000000000000";

/** The document that asks for one refused root, with an argument that makes it invalid. */
export function refusedRootDocument(root: string): string {
  return `${QUERY_ROOTS.includes(root) ? "query" : "mutation"} { ${root}(${NO_SUCH_ARGUMENT}) }`;
}

/** An ordinary mutation a READ key may not run: a check that does not exist, with an argument that makes it invalid. */
export const ORDINARY_MUTATION = `mutation { setCheckPaused(id: "${NO_SUCH_CHECK}", paused: true, ${NO_SUCH_ARGUMENT}) { id } }`;

/** A query that needs a signed-in identity and reads nothing an answer could leak. */
export const IDENTITY_QUERY = "{ me { __typename } }";

/** The text of the READ refusal (`access::Error::ReadOnly`); a refused root must be refused by its kind instead. */
export const READ_ONLY_MESSAGE = "This token may only read.";

/** The K7 error codes; only these are named in a reason. */
const K7_CODES = new Set(["UNAUTHENTICATED", "FORBIDDEN", "NOT_FOUND", "VALIDATION", "CONFLICT", "UNAVAILABLE", "INTERNAL"]);

interface GraphqlAnswer {
  data: unknown;
  errors: { code: unknown; message: unknown }[];
}

/** The data and the errors (code and message) of a GraphQL JSON answer, or undefined when it is not one. */
function graphqlAnswer(body: string): GraphqlAnswer | undefined {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return undefined;
  }
  if (typeof parsed !== "object" || parsed === null) {
    return undefined;
  }
  const { data, errors } = parsed as { data?: unknown; errors?: unknown };
  const list = Array.isArray(errors) ? (errors as unknown[]) : [];
  return {
    data: data ?? null,
    errors: list.map((error) => {
      const { message, extensions } = (error ?? {}) as { message?: unknown; extensions?: { code?: unknown } };
      return { code: extensions?.code, message };
    }),
  };
}

/** How a reason names the first error code: a K7 code by name, anything else generically. */
function firstCode(answer: GraphqlAnswer): string {
  const code = answer.errors[0]?.code;
  if (code === undefined) {
    return answer.errors.length === 0 ? "no error" : "an error without a code";
  }
  return typeof code === "string" && K7_CODES.has(code) ? code : "an unknown code";
}

/** The hub answered a GraphQL request with `code` as its first error and no data. */
function refusedWith(status: number, body: string, code: string): { answer?: GraphqlAnswer; reason?: string } {
  const answer = graphqlAnswer(body);
  if (status !== 200 || answer === undefined) {
    return { reason: `answered status ${String(status)}, not a GraphQL answer` };
  }
  if (answer.errors[0]?.code !== code) {
    return { answer, reason: `answered ${firstCode(answer)}, not ${code}` };
  }
  return answer.data === null ? { answer } : { answer, reason: `answered ${code} but with data` };
}

/** A foreign-origin request with a fake cookie is signed out: refused outright, or UNAUTHENTICATED with no data. */
export function foreignCookieVerdict(status: number, headers: Headers, body: string): string | undefined {
  const allowed = headers.get("access-control-allow-origin");
  if (allowed === "*" || allowed === FOREIGN_ORIGIN) {
    return "the foreign origin got CORS permission";
  }
  if (headers.has("set-cookie")) {
    return "the hub set a cookie for the foreign origin";
  }
  if (status === 401 || status === 403) {
    return undefined;
  }
  return refusedWith(status, body, "UNAUTHENTICATED").reason;
}

/** The hub took the key: `me` answers an operator. */
export function keyAcceptedVerdict(status: number, body: string): string | undefined {
  const answer = graphqlAnswer(body);
  const me = (answer?.data as { me?: { __typename?: unknown } } | null)?.me;
  if (status === 200 && me?.__typename === "Operator") {
    return undefined;
  }
  return answer === undefined ? `answered status ${String(status)}` : `the hub did not take the key (${firstCode(answer)})`;
}

/** A READ key's ordinary mutation is FORBIDDEN; anything else means the key may write. */
export function readOnlyVerdict(status: number, body: string): string | undefined {
  const { answer, reason } = refusedWith(status, body, "FORBIDDEN");
  const code = answer?.errors[0]?.code;
  if (reason === undefined || answer === undefined || code === "FORBIDDEN" || code === "UNAUTHENTICATED") {
    return reason;
  }
  return `not refused (${firstCode(answer)}): the key is a WRITE key, or the hub lets a READ key write; use a READ key`;
}

/** A refused root is FORBIDDEN by the key-kind rule, not only by the READ rule. */
export function refusedRootVerdict(status: number, body: string): string | undefined {
  const { answer, reason } = refusedWith(status, body, "FORBIDDEN");
  if (reason !== undefined) {
    return reason;
  }
  return answer?.errors[0]?.message === READ_ONLY_MESSAGE ? "refused only as a read, not by the key-kind rule" : undefined;
}

/** A preflight from a client origin is allowed for `POST` from exactly that origin. */
export function clientPreflightVerdict(headers: Headers, origin: string): string | undefined {
  if (headers.get("access-control-allow-origin") !== origin) {
    return `the preflight from ${origin} is not allowed for that origin`;
  }
  const methods = (headers.get("access-control-allow-methods") ?? "").split(",").map((method) => method.trim());
  return methods.includes("POST") ? undefined : `the preflight from ${origin} does not allow POST`;
}

/** A preflight from a foreign origin gets no CORS permission. */
export function foreignPreflightVerdict(headers: Headers): string | undefined {
  const allowed = headers.get("access-control-allow-origin");
  if (allowed === "*" || allowed === FOREIGN_ORIGIN) {
    return "the foreign origin is allowed";
  }
  return headers.get("access-control-allow-credentials") === "true" ? "the foreign origin may send credentials" : undefined;
}

/** `/metrics` with no token answers 401. */
export function metricsRefusedVerdict(status: number): string | undefined {
  return status === 401 ? undefined : `answered status ${String(status)}, not 401`;
}

/** What a `graphql-transport-ws` socket did after `connection_init`. */
export type SocketOutcome =
  | { kind: "not-opened" }
  | { kind: "closed"; code: number }
  | { kind: "message"; acknowledged: boolean }
  | { kind: "silent" };

/** A foreign-origin socket with a fake cookie opens, then is closed with 4403 at `connection_init` (D-3). */
export function socketVerdict(outcome: SocketOutcome): string | undefined {
  switch (outcome.kind) {
    case "not-opened":
      return "the socket never opened, so no 4403 at connection_init";
    case "closed":
      return outcome.code === 4403 ? undefined : `closed with ${String(outcome.code)}, not 4403`;
    case "message":
      return outcome.acknowledged ? "connection_init was acknowledged" : "answered connection_init with a message, not 4403";
    case "silent":
      return "no answer to connection_init";
  }
}
