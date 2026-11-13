import { describe, expect, test } from "bun:test";

import { readSettings, RELEASE_BASE, REPOSITORY, SettingsError } from "../src/settings.ts";

const SOURCE = "https://releases.example.org";

describe("SITE-003, SITE-011 and OD-19: build settings", () => {
  test("SITE-003 the base defaults to the release host of the spec and the repository to the public one", () => {
    expect(readSettings({ ANIMA_SITE_RELEASE_SOURCE: RELEASE_BASE })).toEqual({ releaseBase: RELEASE_BASE, releaseSource: RELEASE_BASE, repository: REPOSITORY });
  });

  test("an unset or empty source is an error that names the setting", () => {
    expect(() => readSettings({})).toThrow(/ANIMA_SITE_RELEASE_SOURCE is not set/);
    expect(() => readSettings({ ANIMA_SITE_RELEASE_SOURCE: "" })).toThrow(SettingsError);
  });

  test("a local folder is a source only in a preview, and never on Vercel", () => {
    const folder = { ANIMA_SITE_RELEASE_SOURCE: "fixtures/releases" };
    expect(() => readSettings(folder)).toThrow(/local preview/);
    expect(readSettings({ ...folder, ANIMA_SITE_PREVIEW: "1" }).releaseSource).toBe("fixtures/releases");
    expect(() => readSettings({ ...folder, ANIMA_SITE_PREVIEW: "1", VERCEL: "1" })).toThrow(/never on Vercel/);
  });

  test("a source URL does not change the link base", () => {
    const settings = readSettings({ ANIMA_SITE_RELEASE_SOURCE: "http://127.0.0.1:9/" });
    expect(settings.releaseSource).toBe("http://127.0.0.1:9");
    expect(settings.releaseBase).toBe(RELEASE_BASE);
  });

  test("the release base must be an https origin with no path", () => {
    for (const base of ["http://releases.example.org", `${SOURCE}/x`, "not a url", "https://a.b?x=1"]) {
      expect(() => readSettings({ ANIMA_SITE_RELEASE_SOURCE: SOURCE, ANIMA_SITE_RELEASE_BASE: base })).toThrow(SettingsError);
    }
  });

  test("an empty repository setting shows no repository link", () => {
    expect(readSettings({ ANIMA_SITE_RELEASE_SOURCE: SOURCE, ANIMA_SITE_REPOSITORY: "" }).repository).toBeUndefined();
  });
});
