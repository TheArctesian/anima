// The build of the site: read and check the release, render the page, then write the output folder (SITE-003, SITE-007, OD-19).
// Everything is read and checked before the output folder is touched, so a failed build leaves no page with wrong links.

import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

import { FONT_FILES, siteFontRules } from "./fonts.ts";
import { renderPage } from "./page.ts";
import { type Artefact, artefactOf, feedVersion, LAYOUTS, PLATFORMS } from "./release.ts";
import type { Settings } from "./settings.ts";
import { readReleaseFile } from "./source.ts";

/** Files of the client that the site reuses, as paths from the repository root to a path in the output folder. */
const CLIENT_STYLES = "apps/client/src/styles";
const REUSED_FILES: readonly (readonly [from: string, to: string])[] = [
  [`${CLIENT_STYLES}/tokens.css`, "tokens.css"],
  ["apps/client/static/logo.png", "logo.png"],
  ["apps/client/static/favicon.png", "favicon.png"],
  ["apps/client/static/apple-touch-icon.png", "apple-touch-icon.png"],
  ...FONT_FILES.map((file) => [`${CLIENT_STYLES}/fonts/${file}`, `fonts/${file}`] as const),
];

/** The newest release as the feeds of the source name it. Throws `ReleaseFeedError` or `ReleaseSourceError`. */
export async function readArtefacts(settings: Settings): Promise<Artefact[]> {
  const base = settings.releaseBase;
  return Promise.all(
    PLATFORMS.map(async (platform) => {
      const layout = LAYOUTS[platform];
      const feed = await readReleaseFile(settings.releaseSource, `${layout.feedFolder}/latest.json`);
      const version = feedVersion(feed, platform, base);
      const sums = await readReleaseFile(settings.releaseSource, `${layout.targetFolder}/${version}/SHA256SUMS`);
      return artefactOf(platform, base, version, sums);
    }),
  );
}

/** The files of the site, by path in the output folder, and the artefacts the page shows. */
async function siteFiles(settings: Settings, root: string) {
  const artefacts = await readArtefacts(settings);
  const files = new Map<string, string | Uint8Array>();
  files.set("index.html", renderPage(artefacts, settings.repository));
  files.set("site.css", await readFile(join(root, "apps/site/src/site.css"), "utf8"));
  files.set("fonts.css", siteFontRules(await readFile(join(root, CLIENT_STYLES, "fonts.css"), "utf8")));
  for (const [from, to] of REUSED_FILES) files.set(to, await readFile(join(root, from)));
  return { files, artefacts };
}

/** Builds the site into `outputFolder`, replacing what is there. Returns the artefacts the page shows. */
export async function buildSite(settings: Settings, root: string, outputFolder: string): Promise<Artefact[]> {
  const { files, artefacts } = await siteFiles(settings, root);
  await rm(outputFolder, { recursive: true, force: true });
  for (const [path, content] of files) {
    const target = join(outputFolder, path);
    await mkdir(dirname(target), { recursive: true });
    await writeFile(target, content);
  }
  return artefacts;
}
