import { describe, expect, test } from "bun:test";

import { artefactOf, feedVersion, ReleaseFeedError } from "../src/release.ts";
import { RELEASE_BASE } from "../src/settings.ts";

const SHA = "ab".repeat(32);
const MAC_FOLDER = `${RELEASE_BASE}/darwin-universal/1.2.3/`;
const LINUX_FOLDER = `${RELEASE_BASE}/linux-x86_64/1.2.3/`;

function entry(folder: string, name: string): Record<string, string> {
  return { url: `${folder}${name}`, signature: "signature" };
}

function macFeed(overrides: Record<string, unknown> = {}): string {
  const updater = entry(MAC_FOLDER, "Anima.app.tar.gz");
  const platforms = { "darwin-aarch64": updater, "darwin-x86_64": updater };
  return JSON.stringify({ version: "1.2.3", notes: "n", pub_date: "2026-11-12T10:00:00Z", platforms, ...overrides });
}

function linuxFeed(platforms: Record<string, unknown> = { "linux-x86_64": entry(LINUX_FOLDER, "Anima_1.2.3_amd64.AppImage") }): string {
  return JSON.stringify({ version: "1.2.3", platforms });
}

const MAC_SUMS = `${"cd".repeat(32)}  Anima.app.tar.gz\n${SHA}  Anima_1.2.3_universal.dmg\n`;
const LINUX_SUMS = `${SHA}  Anima_1.2.3_amd64.AppImage\n`;

describe("SITE-002 and SITE-003: the release the page shows", () => {
  test("SITE-002 a valid feed gives the stable-name download link for each platform", () => {
    const mac = artefactOf("macos", RELEASE_BASE, feedVersion(macFeed(), "macos", RELEASE_BASE), MAC_SUMS);
    const linux = artefactOf("linux", RELEASE_BASE, feedVersion(linuxFeed(), "linux", RELEASE_BASE), LINUX_SUMS);
    expect(mac.downloadUrl).toBe("https://releases.anima.ing/Anima.dmg");
    expect(linux.downloadUrl).toBe("https://releases.anima.ing/Anima-x86_64.AppImage");
  });

  test("SITE-003 the version, the checksum and the SHA256SUMS link come from the feed and the sums", () => {
    const mac = artefactOf("macos", RELEASE_BASE, feedVersion(macFeed(), "macos", RELEASE_BASE), MAC_SUMS);
    expect(mac.version).toBe("1.2.3");
    expect(mac.sha256).toBe(SHA);
    expect(mac.sumsUrl).toBe("https://releases.anima.ing/darwin-universal/1.2.3/SHA256SUMS");
  });

  test("SITE-003 an artefact missing from SHA256SUMS is a build error", () => {
    const only = `${"cd".repeat(32)}  Anima.app.tar.gz\n`;
    expect(() => artefactOf("macos", RELEASE_BASE, "1.2.3", only)).toThrow(/does not list Anima_1.2.3_universal.dmg/);
  });

  test("SITE-003 a checksum of the wrong length is a build error", () => {
    const short = `${"ab".repeat(31)}  Anima_1.2.3_amd64.AppImage\n`;
    expect(() => artefactOf("linux", RELEASE_BASE, "1.2.3", short)).toThrow(ReleaseFeedError);
    const upper = `${"AB".repeat(32)}  Anima_1.2.3_amd64.AppImage\n`;
    expect(() => artefactOf("linux", RELEASE_BASE, "1.2.3", upper)).toThrow(ReleaseFeedError);
  });

  test("SITE-003 a name listed twice is a build error", () => {
    expect(() => artefactOf("linux", RELEASE_BASE, "1.2.3", LINUX_SUMS + LINUX_SUMS)).toThrow(/twice/);
  });

  test("SITE-003 a version that is not major.minor.patch is a build error", () => {
    for (const version of ["1.2", "v1.2.3", "1.2.3-rc1", "01.2.3", "latest", 7]) {
      expect(() => feedVersion(macFeed({ version }), "macos", RELEASE_BASE)).toThrow(/version/);
    }
  });

  test("SITE-003 a feed that lacks a platform key or lists another is a build error", () => {
    const updater = entry(MAC_FOLDER, "Anima.app.tar.gz");
    expect(() => feedVersion(macFeed({ platforms: { "darwin-aarch64": updater } }), "macos", RELEASE_BASE)).toThrow(/exactly/);
    expect(() => feedVersion(linuxFeed({ "linux-x86_64": entry(LINUX_FOLDER, "Anima_1.2.3_amd64.AppImage"), "linux-arm64": updater }), "linux", RELEASE_BASE)).toThrow(/exactly/);
  });

  test("SITE-008 an entry on another host is a build error", () => {
    const other = { "linux-x86_64": { url: "https://example.org/linux-x86_64/1.2.3/Anima_1.2.3_amd64.AppImage" } };
    expect(() => feedVersion(linuxFeed(other), "linux", RELEASE_BASE)).toThrow(/another host or folder/);
  });

  test("SITE-008 an entry in a folder of another version is a build error", () => {
    const stale = { "linux-x86_64": entry(`${RELEASE_BASE}/linux-x86_64/1.2.2/`, "Anima_1.2.2_amd64.AppImage") };
    expect(() => feedVersion(linuxFeed(stale), "linux", RELEASE_BASE)).toThrow(ReleaseFeedError);
  });

  test("an unknown field, a wrong updater file or text that is not a JSON object is a build error", () => {
    expect(() => feedVersion(macFeed({ extra: 1 }), "macos", RELEASE_BASE)).toThrow(/unknown field/);
    const wrong = { "linux-x86_64": entry(LINUX_FOLDER, "Anima.dmg") };
    expect(() => feedVersion(linuxFeed(wrong), "linux", RELEASE_BASE)).toThrow(/ends with/);
    expect(() => feedVersion("[]", "linux", RELEASE_BASE)).toThrow(/JSON object/);
    expect(() => feedVersion("{", "linux", RELEASE_BASE)).toThrow(/not valid JSON/);
  });
});
