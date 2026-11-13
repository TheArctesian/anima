import { describe, expect, test } from "bun:test";

import { readSettings, RELEASE_BASE, REPOSITORY, SettingsError } from "../src/settings.ts";

describe("SITE-003 and SITE-011: build settings", () => {
  test("SITE-003 the defaults are the release host of the spec and the public repository", () => {
    expect(readSettings({})).toEqual({ releaseBase: RELEASE_BASE, releaseSource: RELEASE_BASE, repository: REPOSITORY });
  });

  test("the feed source can be a folder or a URL, and does not change the link base", () => {
    expect(readSettings({ ANIMA_SITE_RELEASE_SOURCE: "fixtures/releases" }).releaseSource).toBe("fixtures/releases");
    const local = readSettings({ ANIMA_SITE_RELEASE_SOURCE: "http://127.0.0.1:9/" });
    expect(local.releaseSource).toBe("http://127.0.0.1:9");
    expect(local.releaseBase).toBe(RELEASE_BASE);
  });

  test("the release base must be an https origin with no path", () => {
    for (const base of ["http://releases.example.org", "https://releases.example.org/x", "not a url", "https://a.b?x=1"]) {
      expect(() => readSettings({ ANIMA_SITE_RELEASE_BASE: base })).toThrow(SettingsError);
    }
  });

  test("an empty repository setting shows no repository link", () => {
    expect(readSettings({ ANIMA_SITE_REPOSITORY: "" }).repository).toBeUndefined();
  });
});
