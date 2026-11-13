import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { CONTENT_SECURITY_POLICY } from "../src/page.ts";

interface Header {
  key: string;
  value: string;
}
interface Config {
  framework: unknown;
  cleanUrls: boolean;
  headers: { source: string; headers: Header[] }[];
  [key: string]: unknown;
}

const config = JSON.parse(readFileSync(join(import.meta.dir, "../vercel.json"), "utf8")) as Config;
const headers = new Map((config.headers[0]?.headers ?? []).map((header) => [header.key, header.value]));

describe("SITE-007, SITE-008 and SITE-011: the Vercel configuration is static and sends the page policy", () => {
  test("it is a static deployment: no framework, no function, no rewrite, no environment value", () => {
    expect(config.framework).toBeNull();
    expect(config.cleanUrls).toBe(true);
    for (const key of ["functions", "rewrites", "redirects", "crons", "env", "build"]) expect(config[key]).toBeUndefined();
  });

  test("SITE-008 the header policy is the meta policy plus frame-ancestors, for every route", () => {
    expect(config.headers[0]?.source).toBe("/(.*)");
    expect(headers.get("Content-Security-Policy")).toBe(`${CONTENT_SECURITY_POLICY}; frame-ancestors 'none'`);
  });

  test("the other security headers are set", () => {
    expect(headers.get("X-Content-Type-Options")).toBe("nosniff");
    expect(headers.get("Referrer-Policy")).toBe("no-referrer");
    for (const feature of ["camera", "microphone", "geolocation", "payment", "usb"]) expect(headers.get("Permissions-Policy")).toContain(`${feature}=()`);
  });
});
