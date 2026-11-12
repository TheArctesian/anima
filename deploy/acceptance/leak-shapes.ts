// The never-log shapes of K7 that deploy/acceptance/log-scan.ts looks for in one view of a log line (criterion 22,
// LOG-007, SEC-009). Each pattern starts at a literal or a run boundary and bounds its repeats, so the time to test
// a line grows with its length, not its square.
//
// Not flagged, on purpose (false positives would bury real hits):
// - A bare 43-character base64url run (a 32-byte bearer: push token, plain API token) needs an upper-case
//   letter, a lower-case letter and a digit. Snake-case names, hex digests (SHA-256 hex is 64 characters and
//   has no upper case) and words are skipped; a random value lacks a digit about once in 1,500 draws.
// - A run that sits inside a longer base64url or standard base64 run (`+` beside it), or that follows
//   `SHA256:`/`sha256-`, is a fingerprint or digest, not a bearer. UUIDs are 36 characters and never match.
// - `__Host-anima=;` (and a quoted empty value) is the cookie cleared by logout.
// - `password=`, `secret: ""`, and the placeholders null, true, false, `***` and `[REDACTED]` hold no value. A
//   field name only counts at a word start or a camelCase hump (`smtpPassword`), so `passwordStored` and
//   `SECRET_UNREADABLE` are names, not values.
// - A Discord webhook URL without its token segment, a Slack hooks URL without a path, `apprise://` with
//   nothing after it, an ntfy server origin without a topic, and a URL whose only user info is a short name
//   (`ssh://okita@host`). A user of 20 or more characters with upper case, lower case and a digit is a token.

/** The stable name of a leak shape; the operator records it with the line number. */
export type ShapeKind =
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
  | "url-credentials";

interface Shape {
  kind: ShapeKind;
  pattern: RegExp;
  /** Keeps a match only when it really holds a value; absent means every match counts. */
  holdsValue?: (match: RegExpExecArray, line: string) => boolean;
}

const B64URL = String.raw`A-Za-z0-9_\-`;
const PLACEHOLDER = /^(?:null|none|true|false|\*+|\[redacted\]|<redacted>|redacted)$/i;
// Every scheme the hub accepts (crates/hub/src/notifications/channels/apprise_url.rs; a test keeps the lists in
// step), then the other Apprise schemes an operator might paste.
const APPRISE_SCHEMES =
  "apprises?|discord|tgram|slack|pover|pbul|gchat|pagerduty|opsgenie|" +
  "mailtos?|ntfys?|pushover|gotifys?|matrixs?|msteams|jsons?|xmls?|forms?|signals?|rockets?|mmosts?|zulip|join|" +
  "ifttt|twilio";
// User info characters: anything up to the end of the authority.
const USER = String.raw`[^\s/?#@:"'<>]`;

function isRandom(value: string): boolean {
  return /[A-Z]/.test(value) && /[a-z]/.test(value) && /\d/.test(value);
}

function isBearerValue(match: RegExpExecArray, line: string): boolean {
  const before = line.slice(Math.max(0, match.index - 7), match.index);
  return isRandom(match[0]) && !/sha256[:-]$/i.test(before);
}

function isRealValue(value: string | undefined): boolean {
  return value !== undefined && value !== "" && !PLACEHOLDER.test(value);
}

function isSecretField(match: RegExpExecArray, line: string): boolean {
  const before = line.charAt(match.index - 1);
  const atWordStart = !/[A-Za-z]/.test(before) || (/^[A-Z]/.test(match[1] ?? "") && /[a-z0-9]/.test(before));
  return atWordStart && isRealValue(match[2]);
}

function isUrlCredential(match: RegExpExecArray): boolean {
  const tokenUser = match[3];
  return match[2] !== undefined || (tokenUser !== undefined && isRandom(tokenUser));
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
      /discord(?:app)?\.com\/api\/(?:v\d+\/)?webhooks\/\d+\/[A-Za-z0-9_-]|hooks\.slack\.com\/(?:services|workflows|triggers)\/[A-Za-z0-9]|webhook\.office\.com\/webhookb2\/[A-Za-z0-9]/gi,
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
    pattern: /(password|passwd|passphrase|secret)\\?["']?\s*[:=][ \t]*\\?["']?([^\s"',;&}\\]*)/gi,
    holdsValue: isSecretField,
  },
  { kind: "pem-private-key", pattern: /-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY(?: BLOCK)?-----/g },
  {
    kind: "url-credentials",
    pattern: new RegExp(
      `(?<![A-Za-z0-9+.-])[A-Za-z][A-Za-z0-9+.-]{0,31}://(?:(${USER}*):([^\\s/?#@"'<>]+)|(${USER}{20,}))@`,
      "g",
    ),
    holdsValue: isUrlCredential,
  },
];

/** Every shape kind, in the order findings are reported. */
export const SHAPE_KINDS: readonly ShapeKind[] = SHAPES.map((shape) => shape.kind);

function hasShape(shape: Shape, line: string): boolean {
  for (const match of line.matchAll(shape.pattern)) {
    if (shape.holdsValue === undefined || shape.holdsValue(match as RegExpExecArray, line)) {
      return true;
    }
  }
  return false;
}

/** Every shape kind in one view of a line, in a fixed order. */
export function shapeKinds(line: string): ShapeKind[] {
  return SHAPES.filter((shape) => hasShape(shape, line)).map((shape) => shape.kind);
}
