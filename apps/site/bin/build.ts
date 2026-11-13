// Builds the site into apps/site/dist. Settings: see apps/site/README.md.
import { join } from "node:path";

import { buildSite } from "../src/build.ts";
import { ReleaseFeedError } from "../src/release.ts";
import { readSettings, SettingsError } from "../src/settings.ts";
import { ReleaseSourceError } from "../src/source.ts";

const site = join(import.meta.dir, "..");
const root = join(site, "../..");
const output = process.env["ANIMA_SITE_OUTPUT"] ?? "dist";

try {
  const settings = readSettings(process.env);
  const artefacts = await buildSite(settings, root, join(site, output));
  console.log(`Built apps/site/${output} from ${settings.releaseSource}`);
  for (const artefact of artefacts) console.log(`  ${artefact.label} ${artefact.version} ${artefact.sha256}`);
} catch (error) {
  if (error instanceof ReleaseFeedError || error instanceof ReleaseSourceError || error instanceof SettingsError) {
    console.error(`The site was not built. ${error.message}`);
    console.error("Nothing was written. Fix the release source, or set ANIMA_SITE_RELEASE_SOURCE (see apps/site/README.md).");
    process.exit(1);
  }
  throw error;
}
