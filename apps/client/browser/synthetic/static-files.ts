import { existsSync, readFileSync, statSync } from 'node:fs';
import { extname, join, normalize, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { HUB_HOST } from './access';
import { inlineScriptHashes } from './inline-scripts';

// The web client as the hub serves it (K7 "HTTP surface", WP-43 A-2): the production build with the static file headers.

export const BUILD_FOLDER = fileURLToPath(new URL('../../build/', import.meta.url));
export const BUILD_MISSING = 'run `bun run --cwd apps/client build` first';

const TYPES: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.wasm': 'application/wasm',
  '.txt': 'text/plain; charset=utf-8',
  '.webmanifest': 'application/manifest+json',
};

/** The K7 `Content-Security-Policy` value for a page whose inline scripts hash to `hashes`. */
export function contentSecurityPolicy(hashes: readonly string[]): string {
  const scripts = hashes.map((hash) => ` ${hash}`).join('');
  return (
    `default-src 'self'; script-src 'self'${scripts}; connect-src 'self' wss://${HUB_HOST}; ` +
    `img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; font-src 'self'; ` +
    `object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`
  );
}

const IMMUTABLE = 'public, max-age=31536000, immutable';

export interface StaticAnswer {
  readonly status: number;
  readonly headers: Record<string, string>;
  readonly body: Buffer;
}

function securityHeaders(policy: string, cache: string, contentType: string): Record<string, string> {
  return {
    'content-type': contentType,
    'content-security-policy': policy,
    'x-content-type-options': 'nosniff',
    'referrer-policy': 'no-referrer',
    'cross-origin-opener-policy': 'same-origin',
    'strict-transport-security': 'max-age=31536000',
    'cache-control': cache,
  };
}

function decodeURIComponentOrNull(text: string): string | null {
  try {
    return decodeURIComponent(text);
  } catch {
    // A malformed escape names no file.
    return null;
  }
}

/** The file under the build folder for a URL path, or `undefined` when none is there or the path climbs out of it. */
function fileFor(pathname: string): string | undefined {
  const decoded = URL.canParse(`https://x${pathname}`) ? decodeURIComponentOrNull(pathname) : null;
  if (decoded === null) return undefined;
  const relative = normalize(decoded).replace(/^[/\\]+/, '');
  if (relative.split(sep).some((part) => part.startsWith('.'))) return undefined;
  const path = join(BUILD_FOLDER, relative);
  return existsSync(path) && statSync(path).isFile() ? path : undefined;
}

export interface StaticSite {
  answer(pathname: string): StaticAnswer;
}

/** Reads the build once. Throws `BUILD_MISSING` when there is none, so a spec never runs against nothing. */
export function staticSite(): StaticSite {
  const indexPath = join(BUILD_FOLDER, 'index.html');
  if (!existsSync(indexPath)) throw new Error(BUILD_MISSING);
  const index = readFileSync(indexPath);
  const policy = contentSecurityPolicy(inlineScriptHashes(index.toString('utf8')));
  const page = (status: number): StaticAnswer => ({
    status,
    headers: securityHeaders(policy, 'no-cache', TYPES['.html'] ?? ''),
    body: index,
  });
  return {
    answer(pathname) {
      if (pathname === '/' || pathname === '/index.html') return page(200);
      const file = fileFor(pathname);
      if (file === undefined) {
        // A missing build file is a 404; any other unknown path is a client route (SPA fallback).
        return pathname.startsWith('/_app/') ? { ...page(404), body: Buffer.from('') } : page(200);
      }
      const cache = pathname.startsWith('/_app/immutable/') ? IMMUTABLE : 'no-cache';
      const type = TYPES[extname(file)] ?? 'application/octet-stream';
      return { status: 200, headers: securityHeaders(policy, cache, type), body: readFileSync(file) };
    },
  };
}
