// What deploy/compose.yaml gives the optional Apprise service (D-13, NOT-002, SEC-001). Nothing here starts Docker.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..", "..");

interface Container {
  ports?: string[];
  networks?: string[] | Record<string, { ipv4_address?: string } | null>;
  environment?: Record<string, string>;
  depends_on?: Record<string, unknown>;
  volumes?: unknown[];
  tmpfs?: string[];
  profiles?: string[];
  read_only?: boolean;
  cap_drop?: string[];
  cap_add?: string[];
  security_opt?: string[];
  restart?: string;
  user?: string;
  image?: string;
  healthcheck?: { test: string[] };
  secrets?: unknown[];
}
interface Compose {
  services: Record<string, Container>;
  networks: Record<string, { internal?: boolean; ipam?: { config?: { subnet: string }[] } } | null>;
}

const compose = Bun.YAML.parse(readFileSync(join(ROOT, "deploy", "compose.yaml"), "utf8")) as Compose;
const apprise = compose.services["apprise"] as Container;
const networksOf = (service: Container): string[] => (Array.isArray(service.networks) ? service.networks : Object.keys(service.networks ?? {}));

describe("apprise service (D-13)", () => {
  test("SEC-001 the image is a pinned tag with a sha256 digest", () => {
    expect(apprise.image).toMatch(/^caronc\/apprise:\d+\.\d+\.\d+@sha256:[0-9a-f]{64}$/);
  });

  test("D-13 apprise starts only under the apprise profile, so a plain up leaves it out", () => {
    expect(apprise.profiles).toEqual(["apprise"]);
    const others = Object.entries(compose.services).filter(([name, service]) => name !== "apprise" && service.profiles !== undefined);
    expect(others).toEqual([]);
  });

  test("SEC-001 apprise is hardened and adds no capability", () => {
    expect(apprise.read_only).toBe(true);
    expect(apprise.cap_drop).toEqual(["ALL"]);
    expect(apprise.security_opt).toContain("no-new-privileges:true");
    expect(apprise.cap_add).toBeUndefined();
    expect(apprise.restart).toBe("unless-stopped");
  });

  test("SEC-001 apprise runs as a numeric non-root user, so the image needs no SETUID or CHOWN", () => {
    expect(apprise.user).toMatch(/^(?!0(?::|$))\d+:\d+$/);
  });

  test("NOT-002 apprise holds no secret, no volume and publishes no port", () => {
    expect(apprise.secrets).toBeUndefined();
    expect(apprise.volumes).toBeUndefined();
    expect(apprise.ports).toBeUndefined();
  });

  test("NOT-002 apprise is stateless: stateful mode is disabled and every image volume path is a tmpfs", () => {
    expect(apprise.environment?.["APPRISE_STATEFUL_MODE"]).toBe("disabled");
    // The image declares these as volumes; a tmpfs on each stops Docker making anonymous ones.
    const mounted = (apprise.tmpfs ?? []).map((entry) => entry.split(":")[0]);
    for (const path of ["/config", "/attach", "/plugin", "/tmp"]) expect(mounted).toContain(path);
  });

  test("D-13 apprise has only the notify network, at 172.30.3.10 inside its subnet", () => {
    expect(networksOf(apprise)).toEqual(["notify"]);
    const address = (apprise.networks as Record<string, { ipv4_address?: string }>)["notify"]?.ipv4_address;
    expect(address).toBe("172.30.3.10");
    expect(compose.networks["notify"]?.ipam?.config?.[0]?.subnet).toBe("172.30.3.0/24");
  });

  test("D-13 notify has a route out, and exactly the hub and apprise join it", () => {
    expect(compose.networks["notify"]?.internal).not.toBe(true);
    const members = Object.entries(compose.services).filter(([, service]) => networksOf(service).includes("notify")).map(([name]) => name);
    expect(members.sort()).toEqual(["apprise", "hub"]);
  });

  test("D-13 the hub does not depend on apprise", () => {
    expect(Object.keys(compose.services["hub"]?.depends_on ?? {})).not.toContain("apprise");
  });

  test("D-13 the healthcheck asks /status on the port the image listens on", () => {
    expect(apprise.healthcheck?.test.join(" ")).toContain("http://127.0.0.1:8000/status");
  });
});
