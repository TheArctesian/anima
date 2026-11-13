import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { buildSite } from "../src/build.ts";
import { ANALYTICS_SCRIPT, CONTENT_SECURITY_POLICY } from "../src/page.ts";
import { readSettings, RELEASE_BASE, REPOSITORY } from "../src/settings.ts";
import { type Appearance, contrastOf } from "./token-colours.ts";

const root = join(import.meta.dir, "../../..");
let output = "";
let html = "";
let allText = "";

async function filesUnder(folder: string): Promise<string[]> {
  const entries = await readdir(folder, { withFileTypes: true, recursive: true });
  return entries.filter((entry) => entry.isFile()).map((entry) => join(entry.parentPath, entry.name));
}

beforeAll(async () => {
  output = await mkdtemp(join(tmpdir(), "anima-site-"));
  const settings = readSettings({ ANIMA_SITE_RELEASE_SOURCE: join(root, "apps/site/fixtures/releases"), ANIMA_SITE_PREVIEW: "1" });
  await buildSite(settings, root, output);
  html = await readFile(join(output, "index.html"), "utf8");
  const textFiles = (await filesUnder(output)).filter((file) => /\.(html|css)$/.test(file));
  allText = (await Promise.all(textFiles.map((file) => readFile(file, "utf8")))).join("\n");
});

afterAll(async () => {
  await rm(output, { recursive: true, force: true });
});

describe("SITE-003, SITE-006, SITE-007, SITE-008: what the built site refers to", () => {
  test("SITE-008 the only hosts in the built files are the release host and the repository", () => {
    const hosts = new Set([...allText.matchAll(/https?:\/\/([^/\s"'<>)]+)/gi)].map((match) => match[1]));
    expect([...hosts].sort()).toEqual([new URL(REPOSITORY).host, new URL(RELEASE_BASE).host].sort());
    expect(allText).not.toMatch(/http:\/\//i);
  });

  test("SITE-006 no hub address, name or API path appears", () => {
    expect(allText).not.toMatch(/hub\.anima\.ing|hs\.anima\.ing|graphql|\/api\b|headscale/i);
  });

  test("SITE-007 and SITE-008 the only script is the same-origin analytics script, with no inline code", () => {
    const scripts = [...html.matchAll(/<script\b[^>]*>[\s\S]*?<\/script>/gi)].map((match) => match[0]);
    expect(scripts).toEqual([ANALYTICS_SCRIPT]);
    expect(ANALYTICS_SCRIPT).toMatch(/^<script defer src="\/_vercel\/insights\/script\.js"><\/script>$/);
  });

  test("SITE-007 and SITE-008 the page has no form, frame, inline style or event handler", () => {
    expect(html).not.toMatch(/<form|<iframe|<object|<embed|<input|<button/i);
    expect(html).not.toMatch(/\sstyle=|<style|\son[a-z]+=/i);
  });

  test("SITE-008 the page carries the content security policy and the stylesheets are local", () => {
    expect(html).toContain(`<meta http-equiv="Content-Security-Policy" content="${CONTENT_SECURITY_POLICY}">`);
    expect(CONTENT_SECURITY_POLICY).toContain("default-src 'self'");
    expect(CONTENT_SECURITY_POLICY).toContain("script-src 'self'");
    expect(CONTENT_SECURITY_POLICY).toContain("connect-src 'self'");
    expect(CONTENT_SECURITY_POLICY).not.toMatch(/unsafe|https?:|\*/);
    const links = [...html.matchAll(/<link [^>]*href="([^"]+)"/g)].map((match) => match[1] ?? "");
    expect(links.length).toBeGreaterThan(3);
    for (const href of links) expect(href).not.toMatch(/^(?:[a-z]+:)?\/\//i);
  });

  test("SITE-008 each font file the CSS loads is in the output", async () => {
    const css = await readFile(join(output, "fonts.css"), "utf8");
    const files = [...css.matchAll(/url\("\.\/([^"]+)"\)/g)].map((match) => match[1] ?? "");
    expect(files.length).toBe(3);
    for (const file of files) expect(await Bun.file(join(output, file)).exists()).toBe(true);
  });

  test("SITE-002 and SITE-003 the install links, versions and checksums are those of the fixture feed", () => {
    expect(html).toContain(`href="${RELEASE_BASE}/Anima.dmg"`);
    expect(html).toContain(`href="${RELEASE_BASE}/Anima-x86_64.AppImage"`);
    expect(html).toContain(`href="${RELEASE_BASE}/darwin-universal/0.1.12/SHA256SUMS"`);
    expect(html).toContain(`href="${RELEASE_BASE}/linux-x86_64/0.1.12/SHA256SUMS"`);
    expect(html).toContain("1f".repeat(32));
    expect(html).toContain("c3".repeat(32));
  });

  test("SITE-004 the verification commands are on the page for each platform", () => {
    expect(html).toContain(`echo "${"1f".repeat(32)}  Anima.dmg" | shasum -a 256 -c`);
    expect(html).toContain("codesign --verify");
    expect(html).toContain("xcrun stapler validate Anima.dmg");
    expect(html).toContain(`echo "${"c3".repeat(32)}  Anima-x86_64.AppImage" | sha256sum -c`);
  });

  test("SITE-005 the page says that Anima needs a hub the operator runs and is not hosted", () => {
    expect(html).toContain("Anima is not a hosted service today");
    expect(html).toMatch(/Your own hub/);
    expect(html).toMatch(/private network \(VPN\)/);
  });

  test("SITE-012 the page says nothing about a paid hosted option", () => {
    expect(html).not.toMatch(/\b(?:pricing|price|paid|subscription|subscribe|plan|free trial|premium)\b/i);
  });
});

describe("SITE-009: accessibility smoke checks", () => {
  test("the document has a language, a title, a description, a viewport and one h1", () => {
    expect(html).toMatch(/<html lang="en"/);
    expect(html).toMatch(/<title>[^<]{10,}<\/title>/);
    expect(html).toMatch(/<meta name="description" content="[^"]{30,}"/);
    expect(html).toContain('<meta name="viewport" content="width=device-width, initial-scale=1">');
    expect(html.match(/<h1[ >]/g)).toHaveLength(1);
  });

  test("the page has a header, one nav, one main and a footer, and a skip link that reaches main", () => {
    for (const landmark of ["header", "nav", "main", "footer"]) expect(html.match(new RegExp(`<${landmark}[ >]`, "g"))).toHaveLength(1);
    expect(html).toContain('<a class="skip" href="#main">');
    expect(html).toContain('<main id="main"');
  });

  test("ids are unique, each in-page link has a target and headings do not skip a level", () => {
    const ids = [...html.matchAll(/\sid="([^"]+)"/g)].map((match) => match[1] ?? "");
    expect(new Set(ids).size).toBe(ids.length);
    for (const [, target] of html.matchAll(/href="#([^"]+)"/g)) expect(ids).toContain(target ?? "");
    const levels = [...html.matchAll(/<h([1-6])[ >]/g)].map((match) => Number(match[1]));
    levels.forEach((level, index) => expect(level - (levels[index - 1] ?? level)).toBeLessThanOrEqual(1));
  });

  test("an image has an alt attribute and a size, and a link text makes sense on its own", () => {
    for (const [image] of html.matchAll(/<img [^>]*>/g)) expect(image).toMatch(/alt="[^"]*"[^>]*width="\d+" height="\d+"/);
    const texts = [...html.matchAll(/<a [^>]*>([\s\S]*?)<\/a>/g)].map((match) => (match[1] ?? "").replace(/<[^>]+>/g, "").trim().toLowerCase());
    for (const text of texts) {
      expect(text.length).toBeGreaterThan(0);
      expect(text).not.toMatch(/^(?:click here|here|more|read more|link)$/);
    }
  });

  test("a scrollable code block can take focus and has a name", () => {
    for (const [block] of html.matchAll(/<pre [^>]*>/g)) expect(block).toMatch(/tabindex="0"[^>]*aria-label="[^"]+"/);
  });

  test("the CSS has no raw colour, duration or font family (TOK-003) and respects reduced motion", async () => {
    const css = await readFile(join(output, "site.css"), "utf8");
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(|\b\d+m?s\b|@keyframes/i);
    expect(css).toContain("prefers-reduced-motion: reduce");
    expect(css).toContain("prefers-contrast: more");
    expect(css).toContain(":focus-visible");
  });

});

describe("UI-002 and SITE-009: contrast of the colours the page uses", () => {
  const TEXT_PAIRS: readonly (readonly [string, string])[] = [
    ["text", "surface-base"],
    ["text", "surface-sunken"],
    ["text", "surface-raised"],
    ["text-secondary", "surface-base"],
    ["text-secondary", "surface-sunken"],
    ["text-secondary", "surface-raised"],
    ["accent", "surface-base"],
    ["accent", "surface-sunken"],
    ["accent-hover", "surface-base"],
    ["accent-hover", "surface-hover"],
    ["on-accent", "accent"],
    ["on-accent", "accent-hover"],
    ["status-up-text", "status-up-tint"],
    ["status-maintenance-text", "status-maintenance-tint"],
  ];

  for (const appearance of ["dark", "light"] as const satisfies readonly Appearance[]) {
    test(`UI-002 every text colour pair the page uses reaches 4.5:1 in ${appearance}`, () => {
      for (const [foreground, background] of TEXT_PAIRS) {
        expect({ foreground, background, ok: contrastOf(foreground, background, appearance) >= 4.5 }).toEqual({ foreground, background, ok: true });
      }
    });

    test(`UI-002 the border of a code block reaches 3:1 against the page in ${appearance}`, () => {
      expect(contrastOf("border-strong", "surface-base", appearance)).toBeGreaterThanOrEqual(3);
    });
  }
});
