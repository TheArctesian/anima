// What the site says about the newest release (SITE-002, SITE-003, OD-19). Pure: the text of the feed and of
// SHA256SUMS come in as parameters, and a feed that does not match what the hub writes is an error, never a guess.
// The shapes are the ones the hub writes: `latest.json` (BLD-065, BLD-067, UPD-009) and `SHA256SUMS` (UPD-010).

export type Platform = "macos" | "linux";

/** One platform of INS-001: where its feed and sums are, and the names its files have. */
interface PlatformLayout {
  readonly label: string;
  readonly format: string;
  /** The feed folder the hub writes (`<folder>/latest.json`, BLD-067). */
  readonly feedFolder: string;
  /** The platform keys the feed lists (UPD-009). */
  readonly feedKeys: readonly string[];
  /** The build target folder that holds `<version>/SHA256SUMS` (BLD-060). */
  readonly targetFolder: string;
  /** The ending of the updater file the feed names (BLD-065). */
  readonly updaterSuffix: string;
  /** The stable name for people at the root of the release host (UPD-010). */
  readonly stableName: string;
  /** The name of the same file in `SHA256SUMS`. */
  readonly versionedName: (version: string) => string;
}

export const LAYOUTS: Readonly<Record<Platform, PlatformLayout>> = {
  macos: {
    label: "macOS",
    format: "Disk image (.dmg)",
    feedFolder: "darwin-aarch64",
    feedKeys: ["darwin-aarch64", "darwin-x86_64"],
    targetFolder: "darwin-universal",
    updaterSuffix: ".app.tar.gz",
    stableName: "Anima.dmg",
    versionedName: (version) => `Anima_${version}_universal.dmg`,
  },
  linux: {
    label: "Linux",
    format: "AppImage",
    feedFolder: "linux-x86_64",
    feedKeys: ["linux-x86_64"],
    targetFolder: "linux-x86_64",
    updaterSuffix: ".AppImage",
    stableName: "Anima-x86_64.AppImage",
    versionedName: (version) => `Anima_${version}_amd64.AppImage`,
  },
};

export const PLATFORMS: readonly Platform[] = ["macos", "linux"];

/** What the page shows for one artefact. */
export interface Artefact {
  readonly platform: Platform;
  readonly label: string;
  readonly format: string;
  readonly version: string;
  /** The SHA-256 of the file, lower-case hex. */
  readonly sha256: string;
  /** The link that always gives the newest published file (SITE-002). */
  readonly downloadUrl: string;
  readonly fileName: string;
  /** The `SHA256SUMS` file of this version (SITE-003). */
  readonly sumsUrl: string;
}

/** The feed or the sums do not match the shape the hub writes. The message says what and where. */
export class ReleaseFeedError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ReleaseFeedError";
  }
}

const SEMVER = /^(?:0|[1-9]\d{0,8})\.(?:0|[1-9]\d{0,8})\.(?:0|[1-9]\d{0,8})$/;
const SHA256_LINE = /^([0-9a-f]{64}) {2}(\S+)$/;
const FILE_NAME = /^[\w.-]+$/;
const FEED_FIELDS = ["version", "notes", "pub_date", "platforms"];
const ENTRY_FIELDS = ["url", "signature"];

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseObject(text: string, where: string): Record<string, unknown> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new ReleaseFeedError(`${where} is not valid JSON.`);
  }
  if (!isObject(parsed)) throw new ReleaseFeedError(`${where} is not a JSON object.`);
  return parsed;
}

function onlyFields(object: Record<string, unknown>, allowed: readonly string[], where: string): void {
  const extra = Object.keys(object).filter((key) => !allowed.includes(key));
  if (extra.length > 0) throw new ReleaseFeedError(`${where} has an unknown field: ${extra.join(", ")}.`);
}

function versionOf(feed: Record<string, unknown>, where: string): string {
  const version = feed["version"];
  if (typeof version !== "string" || !SEMVER.test(version)) {
    throw new ReleaseFeedError(`${where} has no version of the form major.minor.patch.`);
  }
  return version;
}

/** One platform entry: a URL of the updater file under `<base>/<target>/<version>/`, an optional signature, nothing else. */
function checkEntry(entry: unknown, folderUrl: string, suffix: string, where: string): void {
  if (!isObject(entry)) throw new ReleaseFeedError(`${where} is not an object.`);
  onlyFields(entry, ENTRY_FIELDS, where);
  const url = entry["url"];
  if (typeof url !== "string" || !url.startsWith(folderUrl)) {
    throw new ReleaseFeedError(`${where} does not name a file under ${folderUrl} (another host or folder).`);
  }
  const name = url.slice(folderUrl.length);
  if (!FILE_NAME.test(name) || !name.endsWith(suffix)) {
    throw new ReleaseFeedError(`${where} does not name an updater file that ends with ${suffix}.`);
  }
  const signature = entry["signature"];
  if (signature !== undefined && typeof signature !== "string") {
    throw new ReleaseFeedError(`${where} has a signature that is not text.`);
  }
}

/** The version of a feed text, after the strict check of its shape (UPD-009, BLD-065). */
export function feedVersion(text: string, platform: Platform, base: string): string {
  const layout = LAYOUTS[platform];
  const where = `The ${layout.label} feed (${layout.feedFolder}/latest.json)`;
  const feed = parseObject(text, where);
  onlyFields(feed, FEED_FIELDS, where);
  const version = versionOf(feed, where);
  const platforms = feed["platforms"];
  if (!isObject(platforms)) throw new ReleaseFeedError(`${where} has no platforms.`);
  const keys = Object.keys(platforms);
  const wanted = layout.feedKeys;
  if (keys.length !== wanted.length || !wanted.every((key) => keys.includes(key))) {
    throw new ReleaseFeedError(`${where} must list exactly ${wanted.join(" and ")}.`);
  }
  const folderUrl = `${base}/${layout.targetFolder}/${version}/`;
  for (const key of wanted) checkEntry(platforms[key], folderUrl, layout.updaterSuffix, `${where}, entry ${key}`);
  return version;
}

/** The SHA-256 that `sums` lists for `fileName`; every line must be `<64 hex>  <name>` (UPD-010). */
export function checksumOf(sums: string, fileName: string, platform: Platform, version: string): string {
  const where = `SHA256SUMS of ${LAYOUTS[platform].label} ${version}`;
  const listed = new Map<string, string>();
  for (const line of sums.split("\n").filter((text) => text !== "")) {
    const match = SHA256_LINE.exec(line);
    const [, sha256, name] = match ?? [];
    if (sha256 === undefined || name === undefined) {
      throw new ReleaseFeedError(`${where} has a line that is not "<64 hex digits>  <file name>".`);
    }
    if (listed.has(name)) throw new ReleaseFeedError(`${where} lists ${name} twice.`);
    listed.set(name, sha256);
  }
  const found = listed.get(fileName);
  if (found === undefined) throw new ReleaseFeedError(`${where} does not list ${fileName}.`);
  return found;
}

/** The artefact of one platform, from the version its feed names and the text of that version's `SHA256SUMS`. */
export function artefactOf(platform: Platform, base: string, version: string, sums: string): Artefact {
  const layout = LAYOUTS[platform];
  return {
    platform,
    label: layout.label,
    format: layout.format,
    version,
    sha256: checksumOf(sums, layout.versionedName(version), platform, version),
    downloadUrl: `${base}/${layout.stableName}`,
    fileName: layout.stableName,
    sumsUrl: `${base}/${layout.targetFolder}/${version}/SHA256SUMS`,
  };
}
