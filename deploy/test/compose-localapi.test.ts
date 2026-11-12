// What deploy/compose.yaml gives the LocalAPI proxy (D-4, VPN-005, NET-003, SEC-001). Nothing here starts Docker.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readConfig } from "../localapi/server.ts";

const ROOT = join(import.meta.dir, "..", "..");

interface Container {
  networks?: string[] | Record<string, unknown>;
  environment?: Record<string, string>;
  depends_on?: Record<string, unknown>;
  volumes?: { type?: string; source?: string; target?: string; read_only?: boolean; bind?: unknown }[];
  read_only?: boolean;
  cap_drop?: string[];
  security_opt?: string[];
  cap_add?: string[];
  restart?: string;
  healthcheck?: { test: string[] };
  secrets?: string[];
  build?: string;
  image?: string;
}
interface Compose {
  services: Record<string, Container>;
  networks: Record<string, { internal?: boolean; ipam?: { config?: { subnet: string }[] } } | null>;
}

const compose = Bun.YAML.parse(readFileSync(join(ROOT, "deploy", "compose.yaml"), "utf8")) as Compose;
const localapi = compose.services["localapi"] as Container;
const networksOf = (service: Container): string[] => (Array.isArray(service.networks) ? service.networks : Object.keys(service.networks ?? {}));
const dockerfile = readFileSync(join(ROOT, "deploy", "localapi", "Dockerfile"), "utf8");
const defaults = readConfig({});

describe("localapi service (D-4)", () => {
  test("SEC-001 localapi is hardened and adds no capability", () => {
    expect(localapi.read_only).toBe(true);
    expect(localapi.cap_drop).toEqual(["ALL"]);
    expect(localapi.security_opt).toContain("no-new-privileges:true");
    expect(localapi.cap_add).toBeUndefined();
    expect(localapi.restart).toBe("unless-stopped");
    expect(localapi.secrets).toBeUndefined();
  });

  test("D-4 localapi shares a network only with the hub, and that network is internal", () => {
    expect(networksOf(localapi)).toEqual(["localapi"]);
    expect(compose.networks["localapi"]?.internal).toBe(true);
    const members = Object.entries(compose.services).filter(([, s]) => networksOf(s).includes("localapi")).map(([name]) => name);
    expect(members.sort()).toEqual(["hub", "localapi"]);
  });

  test("D-4 the only mount is the tailscale socket directory, read-only", () => {
    expect(localapi.volumes).toEqual([{ type: "bind", source: "/var/run/tailscale", target: "/run/tailscale", read_only: true }]);
  });

  test("D-4 the default socket path lies inside the mount", () => {
    const mount = localapi.volumes?.[0];
    expect(defaults.socketPath).toBe(join(mount?.target ?? "", "tailscaled.sock"));
  });

  test("VPN-005 the hub reaches the proxy at localapi:<port the proxy listens on>, and does not wait for it", () => {
    const hub = compose.services["hub"] as Container;
    expect(hub.environment?.["ANIMA_TAILSCALE_LOCALAPI"]).toBe(`\${ANIMA_TAILSCALE_LOCALAPI:-localapi:${String(defaults.port)}}`);
    expect(Object.keys(hub.depends_on ?? {})).not.toContain("localapi");
    expect(networksOf(hub)).toContain("localapi");
  });

  test("NET-001 the probe deny list contains the localapi subnet", () => {
    const subnet = compose.networks["localapi"]?.ipam?.config?.[0]?.subnet;
    expect(subnet).toBe("${ANIMA_LOCALAPI_SUBNET:-172.30.2.0/24}");
    expect((compose.services["hub"]?.environment?.["ANIMA_PROBE_DENY"] ?? "").split(",")).toContain(subnet ?? "missing");
  });

  test("SEC-001 the image is pinned, runs as a numeric non-root user and installs nothing", () => {
    expect(dockerfile).toMatch(/^FROM oven\/bun:1\.\d+\.\d+-distroless$/m);
    expect(dockerfile).toMatch(/^USER (?!0(?::|$))\d+:\d+$/m);
    expect(dockerfile).not.toMatch(/^RUN /m);
    expect(dockerfile.split("\n").filter((line) => line.startsWith("COPY"))).toEqual(["COPY allow.ts server.ts ./"]);
    expect(localapi.build).toBe("./localapi");
  });

  test("D-4 the healthcheck needs no shell", () => {
    expect(localapi.healthcheck?.test[0]).toBe("CMD");
  });
});
