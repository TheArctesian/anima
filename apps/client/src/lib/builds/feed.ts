// The release feed the builder proposes (UPD-009, UPD-010, BLD-063, DNS-007): what latest.json lists, which stable
// names the release carries and what SHA256SUMS says. No signature: the hub signs the updater file it checked and fills
// each entry's `signature` (R-116). Pure: the date and the file digests are parameters.

export interface UpdateFile {
  /** File name of the updater bundle in the release. */
  readonly file: string;
}

/** The updater files of the platforms whose build passed. */
export interface BuiltPlatforms {
  readonly macos?: UpdateFile | undefined;
  readonly linux?: UpdateFile | undefined;
}

export interface Feed {
  version: string;
  notes: string;
  pub_date: string;
  platforms: Record<string, { url: string }>;
}

export class NoPlatformsError extends Error {
  constructor() {
    super('no platform built: a release without an updater file must not be published');
    this.name = 'NoPlatformsError';
  }
}

/** The macOS bundle is universal, so both Mac architectures get the same file. */
const FEED_KEYS = {
  macos: ['darwin-aarch64', 'darwin-x86_64'],
  linux: ['linux-x86_64'],
} as const;

/** A stable name for people, chosen by the file's extension. */
const STABLE_NAMES: readonly (readonly [suffix: string, name: string])[] = [
  ['.dmg', 'Anima.dmg'],
  ['.AppImage', 'Anima-x86_64.AppImage'],
];

function usable(update: UpdateFile | undefined): update is UpdateFile {
  return update !== undefined && update.file !== '';
}

export function buildFeed(input: { version: string; baseUrl: string; date: Date; built: BuiltPlatforms }): Feed {
  const platforms: Feed['platforms'] = {};
  for (const platform of ['macos', 'linux'] as const) {
    const update = input.built[platform];
    if (!usable(update)) continue;
    const entry = { url: `${input.baseUrl}/${encodeURIComponent(update.file)}` };
    for (const key of FEED_KEYS[platform]) platforms[key] = entry;
  }
  if (Object.keys(platforms).length === 0) throw new NoPlatformsError();
  return {
    version: input.version,
    notes: `Anima ${input.version}`,
    pub_date: input.date.toISOString().replace(/\.\d{3}Z$/, 'Z'),
    platforms,
  };
}

/** The stable name for a release file, or undefined when the file has none. */
export function stableName(file: string): string | undefined {
  return STABLE_NAMES.find(([suffix]) => file.endsWith(suffix))?.[1];
}

/** A file by its name and the lower-case hex SHA-256 of its bytes. */
export interface NamedDigest {
  readonly name: string;
  readonly sha256: string;
}

/** `sha256sum` format: `<hex>  <name>`, one line per file, sorted by name. */
export function checksums(files: readonly NamedDigest[]): string {
  return [...files]
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
    .map(({ name, sha256 }) => `${sha256}  ${name}\n`)
    .join('');
}
