/**
 * The builder removes its own secrets from each log line before it sends the line to the hub (BLD-049, LOG-007): the
 * builder token and any other value it knows, presigned upload URLs, and key material. The hub redacts again before it
 * stores a line; this is the first of the two passes, so it errs towards removing too much.
 */

const REDACTED = '[redacted]';
/** A known value shorter than this is not looked for: replacing `a` everywhere would ruin the log and hide nothing. */
const SHORTEST_SECRET = 8;

/** A URL whose query carries a signature or a credential: a presigned upload URL (BLD-060). */
const SIGNED_URL = /\bhttps?:\/\/\S*[?&](?:X-Amz-Signature|X-Amz-Credential|X-Amz-Security-Token|Signature|sig|token)=\S*/gi;

/**
 * minisign key and signature text: the base64 key lines of a secret key (`RWRTY0Iy` is `EdScB2`, `RWQAAEIy` is
 * `Ed\0\0B2`), and base64 of a whole minisign file, which starts with `untrusted comment:` (`dW50cnVzdGVkIGNvbW1lbnQ6`),
 * as Tauri keeps keys and signatures.
 */
const MINISIGN = /(?:RWRTY0Iy|RWQAAEIy|dW50cnVzdGVkIGNvbW1lbnQ6)[A-Za-z0-9+/=]*/g;

/** An assignment of a signing key or its password, as a build script could echo it. */
const KEY_ASSIGNMENT = /\b(TAURI_SIGNING_PRIVATE_KEY(?:_PASSWORD)?|TAURI_PRIVATE_KEY(?:_PASSWORD)?)\s*[=:]\s*\S+/g;

const BLOCK_START = /-----BEGIN [A-Z0-9 ]*PRIVATE KEY( BLOCK)?-----/;
const BLOCK_END = /-----END [A-Z0-9 ]*PRIVATE KEY( BLOCK)?-----/;

function withoutKnown(line: string, secrets: readonly string[]): string {
  let text = line;
  for (const secret of secrets) {
    if (secret.length >= SHORTEST_SECRET) text = text.replaceAll(secret, REDACTED);
  }
  return text;
}

/** `line` with every known secret, signed URL, minisign text and key assignment replaced by `[redacted]`. */
export function cleanLine(line: string, secrets: readonly string[]): string {
  return withoutKnown(line, secrets)
    .replace(SIGNED_URL, REDACTED)
    .replace(MINISIGN, REDACTED)
    .replace(KEY_ASSIGNMENT, `$1=${REDACTED}`);
}

/** Cleans the lines of one job in order: a private key block is removed whole, from its first line to its last. */
export class LineCleaner {
  #inBlock = false;
  readonly #secrets: () => readonly string[];

  /** `secrets` is read for each line, so a token stored later is still removed. */
  constructor(secrets: () => readonly string[]) {
    this.#secrets = secrets;
  }

  clean(line: string): string {
    if (this.#inBlock || BLOCK_START.test(line)) {
      this.#inBlock = !BLOCK_END.test(line);
      return REDACTED;
    }
    return cleanLine(line, this.#secrets());
  }
}
