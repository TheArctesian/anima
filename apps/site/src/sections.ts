// The sections of the page as HTML text (SITE-001, SITE-004, SITE-005, SITE-012). Pure: the release comes in as a parameter.
// The text says nothing about a paid hosted option (SITE-012) and names no hub (SITE-006).

import type { Artefact, Platform } from "./release.ts";

const ESCAPES: Readonly<Record<string, string>> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

/** Text that is safe inside an element or a quoted attribute. */
export function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (character) => ESCAPES[character] ?? character);
}

export const PURPOSE = "Everything that computes for you, or holds data about you, in one place that you own.";

export function hero(repository: string | undefined): string {
  const source = repository === undefined ? "" : `\n        <a class="button secondary" href="${escapeHtml(repository)}">Source code on GitHub</a>`;
  return `<section class="hero" aria-labelledby="top">
        <h1 id="top">Anima</h1>
        <p class="lede">${PURPOSE}</p>
        <p>Anima is a self-hosted application for the computers you own, the services you rent and the data that is held about you. You run it on your own machine, and you own what it keeps.</p>
        <p class="actions">
        <a class="button" href="#install">Install Anima</a>${source}
        </p>
      </section>`;
}

export function whatItIs(): string {
  return `<section aria-labelledby="what">
        <h2 id="what">What Anima is</h2>
        <p>Anima joins three kinds of resource in one place. Today it covers the first kind. The others are planned.</p>
        <dl class="kinds">
          <div><dt>Computers you own <span class="tag">Available now</span></dt><dd>The machines at home and at the office. Anima connects you to them, checks that they are up and records how they are used.</dd></div>
          <div><dt>Services you rent <span class="tag planned">Planned</span></dt><dd>Hosting, domains and databases that you pay for. Anima would show their state and read from them. It would not run them.</dd></div>
          <div><dt>Data held about you <span class="tag planned">Planned</span></dt><dd>Health, money, music and other data that services keep about you. Anima would bring a copy home.</dd></div>
        </dl>
      </section>`;
}

export function today(): string {
  return `<section aria-labelledby="today">
        <h2 id="today">What it does today</h2>
        <ul>
          <li><strong>Devices.</strong> A list of the machines on a home or small private network, with their state.</li>
          <li><strong>Remote access through your hub.</strong> SSH terminals, RDP and VNC desktops, and the web interfaces of your devices, inside the application.</li>
          <li><strong>Uptime checks</strong> for each device, with maintenance windows.</li>
          <li><strong>Metrics.</strong> The resource use of each device over time.</li>
          <li><strong>Notifications</strong> when a check fails.</li>
          <li><strong>A command line and an MCP server.</strong> Scripts and coding agents use the same functions as the application.</li>
          <li><strong>A desktop application</strong> for macOS and Linux that updates itself.</li>
        </ul>
      </section>`;
}

export function planned(): string {
  return `<section aria-labelledby="planned">
        <h2 id="planned">Planned</h2>
        <p>These are not built. There are no dates.</p>
        <ul>
          <li>An overview of the services you rent.</li>
          <li>A home copy of the data that services keep about you.</li>
          <li>Notes, documents, reminders and a library of your own.</li>
          <li>Jobs and builds that run on your own machines, and a model service on your own hardware.</li>
          <li>A copy of the hub on a second machine, so that one failure does not stop it.</li>
          <li>An iOS application.</li>
        </ul>
      </section>`;
}

export function needs(): string {
  return `<section aria-labelledby="need">
        <h2 id="need">What you need</h2>
        <p>Anima is not a hosted service today. You run it yourself. There is no account to create, and this site does not connect to anything of yours.</p>
        <ul>
          <li><strong>Your own hub.</strong> The hub is the server part of Anima. You run it on a machine that you control.</li>
          <li><strong>A private network (VPN)</strong> between your devices and the hub. The hub is not on the public internet.</li>
          <li><strong>The application</strong> on a Mac or a Linux computer, from the downloads below.</li>
        </ul>
      </section>`;
}

function card(artefact: Artefact): string {
  const { label, format, version, sha256, downloadUrl, sumsUrl } = artefact;
  const note = artefact.platform === "macos" ? "For Apple silicon and Intel. Signed with a Developer ID and notarized by Apple." : "For x86-64 computers.";
  return `<article class="card" aria-labelledby="get-${artefact.platform}">
            <h3 id="get-${artefact.platform}">${label}</h3>
            <p>${format}. ${note}</p>
            <p><a class="button" href="${escapeHtml(downloadUrl)}">Download Anima for ${label}</a></p>
            <dl class="facts">
              <div><dt>Version</dt><dd>${escapeHtml(version)}</dd></div>
              <div><dt>SHA-256</dt><dd><code>${escapeHtml(sha256)}</code></dd></div>
            </dl>
            <p><a href="${escapeHtml(sumsUrl)}">SHA256SUMS of ${escapeHtml(version)} (${label})</a></p>
          </article>`;
}

export function install(artefacts: readonly Artefact[]): string {
  return `<section aria-labelledby="install">
        <h2 id="install">Install</h2>
        <p>Each download link always gives the newest published release. The version and the SHA-256 below are those of the release when this page was built.</p>
        <div class="cards">
          ${artefacts.map(card).join("\n          ")}
        </div>
        <h3>Steps</h3>
        <ul>
          <li><strong>macOS.</strong> Open <code>Anima.dmg</code>, drag Anima to Applications, then open it.</li>
          <li><strong>Linux.</strong> Make the file executable with <code>chmod +x Anima-x86_64.AppImage</code>, then run it.</li>
        </ul>
      </section>`;
}

function byPlatform(artefacts: readonly Artefact[], platform: Platform): Artefact {
  const found = artefacts.find((artefact) => artefact.platform === platform);
  if (found === undefined) throw new Error(`No artefact for ${platform}.`);
  return found;
}

export function verify(artefacts: readonly Artefact[]): string {
  const mac = byPlatform(artefacts, "macos");
  const linux = byPlatform(artefacts, "linux");
  return `<section aria-labelledby="verify">
        <h2 id="verify">Verify a download</h2>
        <p>Compare the file with the SHA-256 that the release lists. Each command prints <code>OK</code> when the file matches.</p>
        <h3>macOS</h3>
        <p>The disk image has a Developer ID signature and is notarized. Check the hash, the signature and the notarization ticket:</p>
        <pre tabindex="0" aria-label="Commands to verify the macOS download"><code>echo "${mac.sha256}  ${mac.fileName}" | shasum -a 256 -c
codesign --verify --verbose=2 ${mac.fileName}
xcrun stapler validate ${mac.fileName}</code></pre>
        <h3>Linux</h3>
        <p>Check the hash:</p>
        <pre tabindex="0" aria-label="Command to verify the Linux download"><code>echo "${linux.sha256}  ${linux.fileName}" | sha256sum -c</code></pre>
        <p>The hash must also match the line for the file in the <code>SHA256SUMS</code> file of the release, linked above.</p>
      </section>`;
}

export function afterInstall(): string {
  return `<section aria-labelledby="after">
        <h2 id="after">After you install</h2>
        <ol>
          <li>Start Anima, enter the address of your hub and sign in.</li>
          <li>The application includes the <code>anima</code> command line. You install it from Settings.</li>
          <li>The application looks for updates and installs them. It asks you before it restarts.</li>
        </ol>
      </section>`;
}

export function footer(repository: string | undefined): string {
  const source =
    repository === undefined
      ? ""
      : `
        <p><a href="${escapeHtml(repository)}">Source code on GitHub</a> and <a href="${escapeHtml(`${repository}/blob/HEAD/LICENSE`)}">the licence text</a>.</p>`;
  return `<footer class="page-footer">
      <div class="page">
        <p>Anima is free software under the GNU Affero General Public License, version 3 or later (AGPL-3.0-or-later).</p>${source}
        <p>This site counts page views with Vercel Web Analytics, which sets no cookies. It has no other tracker and loads nothing from another site.</p>
      </div>
    </footer>`;
}
