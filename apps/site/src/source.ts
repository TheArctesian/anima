// Reads a release file at build time from a URL or a local folder (OD-19). The only I/O of the release step.

import { readFile } from "node:fs/promises";
import { join } from "node:path";

/** A release file could not be read; the message names the file and the cause. */
export class ReleaseSourceError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ReleaseSourceError";
  }
}

const TIMEOUT_MS = 15_000;
/** `latest.json` is a few hundred bytes and `SHA256SUMS` a few lines; anything larger is not one. */
const MAX_BYTES = 64 * 1024;

function overLimit(text: string, where: string): string {
  if (text.length > MAX_BYTES) throw new ReleaseSourceError(`${where} is larger than ${MAX_BYTES} bytes.`);
  return text;
}

async function fromUrl(url: string): Promise<string> {
  let response: Response;
  try {
    response = await fetch(url, { signal: AbortSignal.timeout(TIMEOUT_MS), redirect: "error" });
  } catch (error) {
    const cause = error instanceof Error ? error.message : String(error);
    throw new ReleaseSourceError(`Cannot read ${url}: ${cause}`);
  }
  if (!response.ok) throw new ReleaseSourceError(`Cannot read ${url}: the host answered ${response.status}.`);
  return overLimit(await response.text(), url);
}

async function fromFolder(folder: string, path: string): Promise<string> {
  const file = join(folder, path);
  try {
    return overLimit(await readFile(file, "utf8"), file);
  } catch (error) {
    if (error instanceof ReleaseSourceError) throw error;
    const cause = error instanceof Error ? error.message : String(error);
    throw new ReleaseSourceError(`Cannot read ${file}: ${cause}`);
  }
}

/** The text of `path` (for example `linux-x86_64/latest.json`) under `source`, a URL or a folder. */
export function readReleaseFile(source: string, path: string): Promise<string> {
  return /^https?:\/\//i.test(source) ? fromUrl(`${source}/${path}`) : fromFolder(source, path);
}
