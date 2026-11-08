import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { BUILD_FOLDER } from '../synthetic/static-files';

// Where the desktop updater lives in the production build, so a spec can tell whether a page asked for it (UPD-001). The
// build hashes chunk names, so the files are found by what they hold, not by name.

const IMMUTABLE = join(BUILD_FOLDER, '_app', 'immutable');
/** The updater plugin's command prefix: only the chunk that bundles `@tauri-apps/plugin-updater` has it. */
const PLUGIN_COMMAND = 'plugin:updater|';

function scriptsUnder(folder: string): string[] {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) return scriptsUnder(path);
    return entry.name.endsWith('.js') ? [path] : [];
  });
}

/**
 * The URL paths of the chunk that bundles the updater plugin and of every chunk that names it (`$lib/desktop/updater`, the one
 * that loads it). The layout's chunk does not name the plugin, only the updater module, so it is not in the list.
 */
export function updaterChunkPaths(): string[] {
  const scripts = scriptsUnder(IMMUTABLE).map((path) => ({ path, text: readFileSync(path, 'utf8') }));
  const plugin = scripts.filter((script) => script.text.includes(PLUGIN_COMMAND));
  const namesPlugin = (text: string): boolean => plugin.some((found) => text.includes(found.path.slice(found.path.lastIndexOf('/') + 1)));
  const holding = [...plugin, ...scripts.filter((script) => !plugin.includes(script) && namesPlugin(script.text))];
  return holding.map((script) => script.path.slice(BUILD_FOLDER.length - 1));
}
