// The build settings of the site (SITE-003, OD-19, OD-20). Pure: the environment comes in as a parameter.

/** The public base URL of the release bucket `anima-releases` (DNS-007, spec Section 3.9). */
export const RELEASE_BASE = "https://releases.anima.ing";

/** The public repository (checked public on 2026-11-13). Set `ANIMA_SITE_REPOSITORY` to an empty text to show no link. */
export const REPOSITORY = "https://github.com/TheArctesian/anima";

export interface Settings {
  /** Where the page's download links point: an `https` origin with no path. */
  readonly releaseBase: string;
  /** Where the build reads `<folder>/latest.json` and `<folder>/<version>/SHA256SUMS`: a URL or a local folder. */
  readonly releaseSource: string;
  /** The repository link, or `undefined` for none. */
  readonly repository: string | undefined;
}

/** A setting has a value the build cannot use. */
export class SettingsError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SettingsError";
  }
}

function httpsOrigin(text: string, name: string, allowPath: boolean): string {
  let url: URL;
  try {
    url = new URL(text);
  } catch {
    throw new SettingsError(`${name} is not a URL: ${text}`);
  }
  const path = url.pathname.replace(/\/$/, "");
  if (url.protocol !== "https:" || url.search !== "" || url.hash !== "" || url.username !== "" || (!allowPath && path !== "")) {
    throw new SettingsError(`${name} must be an https URL${allowPath ? "" : " with no path"}, with no query or credentials: ${text}`);
  }
  return `${url.origin}${path}`;
}

/** The settings from `ANIMA_SITE_RELEASE_BASE`, `ANIMA_SITE_RELEASE_SOURCE` and `ANIMA_SITE_REPOSITORY`. */
export function readSettings(env: Readonly<Record<string, string | undefined>>): Settings {
  const releaseBase = httpsOrigin(env["ANIMA_SITE_RELEASE_BASE"] ?? RELEASE_BASE, "ANIMA_SITE_RELEASE_BASE", false);
  const source = env["ANIMA_SITE_RELEASE_SOURCE"] ?? releaseBase;
  const releaseSource = /^https?:\/\//i.test(source) ? source.replace(/\/$/, "") : source;
  const repository = env["ANIMA_SITE_REPOSITORY"] ?? REPOSITORY;
  return {
    releaseBase,
    releaseSource,
    repository: repository === "" ? undefined : httpsOrigin(repository, "ANIMA_SITE_REPOSITORY", true),
  };
}
