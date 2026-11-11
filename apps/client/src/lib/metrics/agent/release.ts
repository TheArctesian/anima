import releaseFile from './release.json';

/** The platforms an agent binary is built for (R-092 D-5), as they appear in `anima-agent-<target>`. */
export const AGENT_TARGETS = [
  { target: 'linux-x86_64', label: 'Linux (x86_64)' },
  { target: 'linux-aarch64', label: 'Linux (aarch64)' },
  { target: 'macos-aarch64', label: 'macOS (Apple silicon)' },
] as const;

export type AgentTarget = (typeof AGENT_TARGETS)[number]['target'];

/** One published agent version and the SHA-256 of each target's binary (WP-54 D-10). */
export interface AgentRelease {
  readonly version: string;
  readonly sha256: Readonly<Partial<Record<AgentTarget, string>>>;
}

const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;
const SHA256 = /^[0-9a-f]{64}$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isTarget(name: string): name is AgentTarget {
  return AGENT_TARGETS.some((each) => each.target === name);
}

function parseTargets(targets: unknown): AgentRelease['sha256'] | null {
  if (!isRecord(targets)) return null;
  const sha256: Partial<Record<AgentTarget, string>> = {};
  for (const [name, entry] of Object.entries(targets)) {
    if (!isTarget(name) || !isRecord(entry) || typeof entry['sha256'] !== 'string' || !SHA256.test(entry['sha256'])) return null;
    sha256[name] = entry['sha256'];
  }
  return Object.keys(sha256).length === 0 ? null : sha256;
}

/**
 * The release a `release.json` describes, or `null` for "no release": the development form, or any file whose shape
 * is wrong. A malformed file never makes a command (MET-019).
 *
 * The release build writes (T6d) either `{"release": null}` or
 * `{"release": {"version": "1.2.3", "targets": {"linux-x86_64": {"sha256": "<64 lowercase hex>"}, ...}}}`;
 * a target without an entry has no download.
 */
export function parseRelease(file: unknown): AgentRelease | null {
  if (!isRecord(file) || !isRecord(file['release'])) return null;
  const { version, targets } = file['release'];
  if (typeof version !== 'string' || !SEMVER.test(version)) return null;
  const sha256 = parseTargets(targets);
  return sha256 === null ? null : { version, sha256 };
}

/** The release this client was built with (WP-54 D-10 c). */
export const BUILT_RELEASE: AgentRelease | null = parseRelease(releaseFile);

/** The target a browser's platform text names, or `null` when it cannot be known (MET-019). Browsers hide the CPU of a Mac. */
export function targetOfPlatform(platform: string, userAgent: string): AgentTarget | null {
  const text = `${platform} ${userAgent}`.toLowerCase();
  if (text.includes('android') || text.includes('iphone') || text.includes('ipad')) return null;
  if (text.includes('mac')) return 'macos-aarch64';
  if (!text.includes('linux')) return null;
  return /aarch64|arm64|armv8/.test(text) ? 'linux-aarch64' : 'linux-x86_64';
}
