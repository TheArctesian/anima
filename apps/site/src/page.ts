// The one page of the site as HTML text (SITE-001, SITE-007, SITE-008). Pure: the release and the settings come in as parameters.

import type { Artefact } from "./release.ts";
import { afterInstall, escapeHtml, footer, hero, install, needs, planned, PURPOSE, today, verify, whatItIs } from "./sections.ts";

/**
 * The policy that makes SITE-008 true in the page itself: every kind of content comes from this origin, there is no
 * script, no form and no frame, and a style comes only from a file. `frame-ancestors` is not allowed in a `meta`
 * element; the host sets it (see apps/site/README.md).
 */
export const CONTENT_SECURITY_POLICY =
  "default-src 'self'; script-src 'none'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'";

const TITLE = "Anima: a self-hosted place for your computers, services and data";
const DESCRIPTION = `${PURPOSE} Anima is free software that you run yourself. Download it for macOS or Linux.`;

/** The fonts the page uses, preloaded so the text does not move when they arrive (SITE-009). */
export const PRELOADED_FONTS = ["fonts/IBMPlexSans-Regular.woff2", "fonts/IBMPlexSans-Bold.woff2", "fonts/FiraCode-Regular.woff2"];

function head(): string {
  const preloads = PRELOADED_FONTS.map((font) => `<link rel="preload" href="${font}" as="font" type="font/woff2" crossorigin>`);
  return `<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta http-equiv="Content-Security-Policy" content="${CONTENT_SECURITY_POLICY}">
    <title>${escapeHtml(TITLE)}</title>
    <meta name="description" content="${escapeHtml(DESCRIPTION)}">
    <meta name="color-scheme" content="dark light">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Anima">
    <meta property="og:title" content="${escapeHtml(TITLE)}">
    <meta property="og:description" content="${escapeHtml(DESCRIPTION)}">
    <link rel="icon" href="favicon.png" type="image/png">
    <link rel="apple-touch-icon" href="apple-touch-icon.png">
    ${preloads.join("\n    ")}
    <link rel="stylesheet" href="tokens.css">
    <link rel="stylesheet" href="fonts.css">
    <link rel="stylesheet" href="site.css">
  </head>`;
}

function header(repository: string | undefined): string {
  const source = repository === undefined ? "" : `\n          <li><a href="${escapeHtml(repository)}">Source</a></li>`;
  return `<header class="page-header">
      <div class="page bar">
        <a class="brand" href="#top"><img src="logo.png" alt="" width="32" height="32"><span>Anima</span></a>
        <nav aria-label="Page sections">
          <ul>
            <li><a href="#install">Install</a></li>
            <li><a href="#verify">Verify</a></li>${source}
          </ul>
        </nav>
      </div>
    </header>`;
}

/** The whole document. `data-theme="system"` makes the token CSS follow `prefers-color-scheme` (UI-005). */
export function renderPage(artefacts: readonly Artefact[], repository: string | undefined): string {
  return `<!doctype html>
<html lang="en" data-theme="system">
  ${head()}
  <body>
    <a class="skip" href="#main">Skip to the content</a>
    ${header(repository)}
    <main id="main" class="page">
      ${hero(repository)}
      ${whatItIs()}
      ${today()}
      ${planned()}
      ${needs()}
      ${install(artefacts)}
      ${verify(artefacts)}
      ${afterInstall()}
    </main>
    ${footer(repository)}
  </body>
</html>
`;
}
