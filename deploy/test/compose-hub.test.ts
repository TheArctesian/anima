// What deploy/compose.yaml passes to the hub: environment, networks and mounts (DNS-002, NET-001, NET-003).
// Nothing here starts Docker.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..", "..");
const CONFIG_PATH = join(ROOT, "crates", "hub", "src", "config.rs");

interface Container {
  networks?: string[] | Record<string, { ipv4_address?: string } | null>;
  environment?: Record<string, string>;
  volumes?: unknown[];
  healthcheck?: { test: string[]; interval?: string; retries?: number };
}
interface Compose {
  services: Record<string, Container>;
  networks: Record<string, { internal?: boolean; ipam?: { config?: { subnet: string }[] } } | null>;
}

const compose = Bun.YAML.parse(readFileSync(join(ROOT, "deploy", "compose.yaml"), "utf8")) as Compose;

describe("hub configuration", () => {
  test("every ANIMA_* variable the hub receives is read by config.rs or listed in the conventions.md environment table", () => {
    const source = readFileSync(CONFIG_PATH, "utf8");
    const conventions = readFileSync(join(ROOT, "contracts", "conventions.md"), "utf8");
    const known = new Set([...source.matchAll(/"(ANIMA_[A-Z_]+)"/g), ...conventions.matchAll(/`(ANIMA_[A-Z_]+)`/g)].map((match) => match[1]));
    for (const name of ["hub", "migrate"]) {
      for (const key of Object.keys(compose.services[name]?.environment ?? {})) {
        expect(known.has(key) ? key : `${name}.${key} is in neither config.rs nor conventions.md`).toBe(key);
      }
    }
  });

  test("NET-001 ANIMA_PROBE_DENY names the subnet of every network the hub is attached to, and the VPS public /32", () => {
    const hub = compose.services["hub"] as Container;
    const attached = Array.isArray(hub.networks) ? hub.networks : Object.keys(hub.networks ?? {});
    const subnets = attached.map((name) => {
      const config = compose.networks[name]?.ipam?.config;
      return config?.[0]?.subnet ?? `${name} has no fixed subnet`;
    });
    expect(subnets.length).toBe(2);
    const deny = hub.environment?.["ANIMA_PROBE_DENY"] ?? "";
    expect(deny.split(",").sort()).toEqual([...subnets, "${ANIMA_VPS_PUBLIC_ADDRESS:?set to the VPS public IPv4}/32"].sort());
    expect(subnets).toContain("${ANIMA_DATABASE_SUBNET:-172.30.1.0/24}");
    expect(subnets).toContain("${ANIMA_HUB_SUBNET:-172.30.0.0/24}");
  });

  test("DNS-002 the hub reads certificate and key from the one tls.pem on the certs volume", () => {
    const environment = compose.services["hub"]?.environment ?? {};
    expect(environment["ANIMA_TLS_CERT_FILE"]).toBe("/certs/hub/tls.pem");
    expect(environment["ANIMA_TLS_KEY_FILE"]).toBe("/certs/hub/tls.pem");
  });

  test("NET-003 the hub listens on its fixed gateway address and reaches guacd by name", () => {
    const hub = compose.services["hub"] as Container;
    expect(hub.environment?.["ANIMA_LISTEN"]).toBe("${ANIMA_HUB_IP:-172.30.0.10}:8443");
    expect(hub.environment?.["ANIMA_GUACD"]).toBe("guacd:4822");
    const gateway = (hub.networks as Record<string, { ipv4_address?: string }>)["gateway"];
    expect(gateway?.ipv4_address).toBe("${ANIMA_HUB_IP:-172.30.0.10}");
  });

  test("DNS-002 the hub mounts the certs volume read-only", () => {
    expect(compose.services["hub"]?.volumes).toContain("certs:/certs:ro");
  });

  test("NET-003 the permitted networks default is exactly the VPN range", () => {
    const value = compose.services["hub"]?.environment?.["ANIMA_PERMITTED_NETWORKS"];
    expect(value).toBe("${ANIMA_PERMITTED_NETWORKS:-100.64.0.0/10}");
  });

  test("GW-001 guacd writes its font cache to /tmp", () => {
    expect(compose.services["guacd"]?.environment?.["XDG_CACHE_HOME"]).toBe("/tmp");
  });

  test("GW-002 the gateway network keeps its egress and fixed subnet", () => {
    expect(compose.networks["gateway"]?.internal).not.toBe(true);
  });
});

describe("healthchecks", () => {
  test("DNS-002 certs may take 15 minutes to become healthy before the hub gives up", () => {
    const check = compose.services["certs"]?.healthcheck;
    expect((check?.retries ?? 0) * 10).toBeGreaterThanOrEqual(900);
    expect(check?.interval).toBe("10s");
  });

  test("SEC-001 the hub healthcheck is the token-free anima-hub health probe", () => {
    const check = compose.services["hub"]?.healthcheck;
    expect(check?.test).toEqual(["CMD", "anima-hub", "health"]);
  });

  test("GW-001 guacd reports healthy within a minute, not after the image's 5-minute interval", () => {
    const check = compose.services["guacd"]?.healthcheck;
    expect(check?.test.join(" ")).toContain("4822");
    expect(check?.interval).toBe("30s");
  });
});

describe("hub image build", () => {
  const dockerfile = readFileSync(join(ROOT, "deploy", "hub", "Dockerfile"), "utf8");
  const builds = dockerfile.split("\n").filter((line) => line.includes("cargo build"));

  test("SEC-005 the image builds only the hub binary, so no test-support code ships", () => {
    expect(builds).toEqual(["RUN cargo build --release --locked --bin anima-hub"]);
  });

  test("SEC-005 the build never enables test targets or extra features", () => {
    for (const flag of ["--all-targets", "--tests", "--features", "--all-features", "test-support"]) {
      expect(dockerfile).not.toContain(flag);
    }
  });
});
