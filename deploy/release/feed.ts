// The release feed (UPD-009, UPD-010, DNS-007): what latest.json lists, which stable names the
// release carries and what SHA256SUMS says. Pure: the date and the file bytes are parameters.
import { createHash } from "node:crypto";

export interface UpdateFile {
  /** File name of the updater bundle in the release. */
  file: string;
  /** Text of the `.sig` file made by the Tauri updater key. */
  signature: string;
}

/** The updater files of the platforms whose build passed in this run. */
export interface BuiltPlatforms {
  macos?: UpdateFile | undefined;
  linux?: UpdateFile | undefined;
}

export interface Feed {
  version: string;
  notes: string;
  pub_date: string;
  platforms: Record<string, { signature: string; url: string }>;
}

export class NoPlatformsError extends Error {
  constructor() {
    super("no platform built: a release without an updater file must not be published");
    this.name = "NoPlatformsError";
  }
}

/** The macOS bundle is universal, so both Mac architectures get the same file. */
const FEED_KEYS = {
  macos: ["darwin-aarch64", "darwin-x86_64"],
  linux: ["linux-x86_64"],
} as const;

/** A stable name for people, chosen by the file's extension. */
const STABLE_NAMES: readonly [suffix: string, name: string][] = [
  [".dmg", "Anima.dmg"],
  [".AppImage", "Anima-x86_64.AppImage"],
];

function usable(update: UpdateFile | undefined): update is UpdateFile {
  return update !== undefined && update.file !== "" && update.signature.trim() !== "";
}

export function buildFeed(input: { version: string; baseUrl: string; date: Date; built: BuiltPlatforms }): Feed {
  const platforms: Feed["platforms"] = {};
  for (const [platform, keys] of Object.entries(FEED_KEYS)) {
    const update = input.built[platform as keyof BuiltPlatforms];
    if (!usable(update)) continue;
    const entry = {
      signature: update.signature.trim(),
      url: `${input.baseUrl}/${encodeURIComponent(update.file)}`,
    };
    for (const key of keys) platforms[key] = entry;
  }
  if (Object.keys(platforms).length === 0) throw new NoPlatformsError();
  return {
    version: input.version,
    notes: `Anima ${input.version}`,
    pub_date: input.date.toISOString().replace(/\.\d{3}Z$/, "Z"),
    platforms,
  };
}

/** The stable name for a release file, or undefined when the file has none. */
export function stableName(file: string): string | undefined {
  return STABLE_NAMES.find(([suffix]) => file.endsWith(suffix))?.[1];
}

export interface NamedBytes {
  name: string;
  bytes: Uint8Array;
}

/** `sha256sum` format: `<hex>  <name>`, one line per file, sorted by name. */
export function checksums(files: readonly NamedBytes[]): string {
  return [...files]
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
    .map(({ name, bytes }) => `${createHash("sha256").update(bytes).digest("hex")}  ${name}\n`)
    .join("");
}
