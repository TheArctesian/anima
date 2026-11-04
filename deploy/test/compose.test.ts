// Rules for deploy/compose.yaml, checked on the parsed file (DB-002, SEC-001, NET-003).
// Nothing here starts Docker, and no test writes a file.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..", "..");
const COMPOSE_PATH = join(ROOT, "deploy", "compose.yaml");

type Dict = Record<string, unknown>;
interface Container {
  image?: string;
  ports?: string[];
  networks?: string[] | Dict;
  secrets?: (string | { source: string; target?: string })[];
  environment?: Record<string, string>;
  depends_on?: Record<string, { condition: string }>;
  logging?: { driver: string; options: Record<string, string> };
  read_only?: boolean;
  cap_drop?: string[];
  security_opt?: string[];
  command?: string[];
  cap_add?: string[];
  restart?: string;
  tmpfs?: string[];
  volumes?: (string | { type?: string; bind?: { selinux?: string } })[];
  healthcheck?: { test: string[]; interval?: string; retries?: number };
  init?: boolean;
  build?: { additional_contexts?: Record<string, string> };
}
interface Compose {
  services: Record<string, Container>;
  networks: Record<string, { internal?: boolean; driver_opts?: Record<string, string> } | null>;
  volumes: Record<string, unknown>;
  secrets: Record<string, { file: string }>;
}

const compose = Bun.YAML.parse(readFileSync(COMPOSE_PATH, "utf8")) as Compose;
const services = Object.entries(compose.services);

function networksOf(service: Container): string[] {
  const networks = service.networks ?? [];
  return Array.isArray(networks) ? networks : Object.keys(networks);
}

function secretsOf(service: Container): string[] {
  return (service.secrets ?? []).map((secret) => (typeof secret === "string" ? secret : secret.source)).sort();
}

describe("published ports (NET-003, DB-001, GW-002)", () => {
  test("DB-001 postgres publishes no port", () => {
    expect(compose.services["postgres"]?.ports).toBeUndefined();
  });

  test("GW-002 guacd publishes no port", () => {
    expect(compose.services["guacd"]?.ports).toBeUndefined();
  });

  test("NET-003 the hub is the only service with a port, and it binds ANIMA_HUB_ADDRESS", () => {
    const published = services.filter(([, service]) => service.ports !== undefined);
    expect(published.map(([name]) => name)).toEqual(["hub"]);
    const ports = compose.services["hub"]?.ports ?? [];
    expect(ports).toHaveLength(1);
    expect(ports[0]).toMatch(/^\$\{ANIMA_HUB_ADDRESS:\?[^}]+\}:443:8443$/);
  });
});

describe("networks (DB-001, GW-002, GW-003)", () => {
  test("DB-001 the database network is internal", () => {
    expect(compose.networks["database"]?.internal).toBe(true);
  });

  test("GW-002 guacd shares a network only with the hub", () => {
    const guacd = networksOf(compose.services["guacd"] as Container);
    expect(guacd).toEqual(["gateway"]);
    for (const [name, service] of services) {
      if (name === "guacd") continue;
      const shared = networksOf(service).filter((network) => guacd.includes(network));
      expect(shared.length > 0 ? name : null).toBe(shared.length > 0 ? "hub" : null);
    }
  });

  test("DB-001 only postgres, migrate and the hub join the database network", () => {
    const members = services.filter(([, service]) => networksOf(service).includes("database")).map(([name]) => name);
    expect(members.sort()).toEqual(["hub", "migrate", "postgres"]);
  });

  test("DNS-003 only certs joins the acme network", () => {
    const members = services.filter(([, service]) => networksOf(service).includes("acme")).map(([name]) => name);
    expect(members).toEqual(["certs"]);
  });
});

describe("secrets (DB-002, SEC-004, DNS-003)", () => {
  const expected: Record<string, string[]> = {
    postgres: ["hub_password", "owner_password", "postgres_password"],
    migrate: ["owner_database_url"],
    certs: ["cloudflare_dns_token"],
    hub: ["anima_key", "hub_database_url"],
    guacd: [],
  };

  for (const [name, secrets] of Object.entries(expected)) {
    test(`DB-002 ${name} has exactly its secrets`, () => {
      expect(secretsOf(compose.services[name] as Container)).toEqual(secrets);
    });
  }

  test("DB-002 the hub reads hub_database_url as /run/secrets/database_url", () => {
    const entry = compose.services["hub"]?.secrets?.find((secret) => typeof secret !== "string" && secret.source === "hub_database_url");
    expect(entry).toEqual({ source: "hub_database_url", target: "database_url" });
  });

  test("SEC-004 every secret is a file under ./secrets/", () => {
    expect(Object.keys(compose.secrets).length).toBeGreaterThan(0);
    for (const [name, secret] of Object.entries(compose.secrets)) {
      expect(secret.file).toBe(`./secrets/${name}`);
    }
  });

  test("SEC-004 every service secret is declared", () => {
    for (const [, service] of services) {
      for (const secret of secretsOf(service)) expect(compose.secrets).toHaveProperty(secret);
    }
  });

  test("LOG-007 no environment value is a password, token or URL with credentials", () => {
    for (const [name, service] of services) {
      for (const [key, value] of Object.entries(service.environment ?? {})) {
        const label = `${name}.${key}`;
        if (/(PASSWORD|TOKEN|SECRET|KEY)/.test(key)) expect(`${label}:${key}`).toMatch(/_FILE$/);
        expect(`${label}=${value}`).not.toMatch(/:\/\/[^/\s:@]+:[^/\s@]+@/);
      }
    }
  });
});

describe("start order (DB-003)", () => {
  test("DB-003 the hub waits for postgres, migrate, certs and guacd", () => {
    const depends = compose.services["hub"]?.depends_on ?? {};
    expect(Object.fromEntries(Object.entries(depends).map(([name, value]) => [name, value.condition]))).toEqual({
      postgres: "service_healthy",
      migrate: "service_completed_successfully",
      certs: "service_healthy",
      guacd: "service_started",
    });
  });

  test("DB-003 migrate waits for a healthy postgres and runs the migrate command", () => {
    const migrate = compose.services["migrate"] as Container;
    expect(migrate.depends_on?.["postgres"]?.condition).toBe("service_healthy");
    expect(migrate.command).toEqual(["migrate"]);
    expect(migrate.environment?.["ANIMA_DATABASE_URL_FILE"]).toBe("/run/secrets/owner_database_url");
  });
});

describe("container limits (SEC-001)", () => {
  test("every service rotates its log at 10m x 10 (spec Section 3.8)", () => {
    for (const [, service] of services) {
      expect(service.logging).toEqual({ driver: "json-file", options: { "max-size": "10m", "max-file": "10" } });
    }
  });

  test("every service has a read-only root, no capability and no new privileges", () => {
    for (const [, service] of services) {
      expect(service.read_only).toBe(true);
      expect(service.cap_drop).toEqual(["ALL"]);
      expect(service.security_opt).toContain("no-new-privileges:true");
    }
  });
});

describe("host integration (AlmaLinux host firewall)", () => {
  test("every bind mount uses the long syntax with the SELinux z label", () => {
    let binds = 0;
    for (const [name, container] of services) {
      for (const volume of container.volumes ?? []) {
        if (typeof volume === "string") {
          expect(`${name}:${volume}`).not.toMatch(/^[^:]+:(\.|\/)/);
          continue;
        }
        if (volume.type !== "bind") continue;
        binds += 1;
        expect(volume.bind?.selinux).toBe("z");
      }
    }
    expect(binds).toBe(3);
  });

  test("NET-003 no network sets a bridge name (the forward chain relies on the default br-* names)", () => {
    for (const network of Object.values(compose.networks)) {
      expect(JSON.stringify(network ?? {})).not.toContain("bridge.name");
    }
  });

  test("preflight checks exactly the secret files that compose declares", () => {
    const script = readFileSync(join(ROOT, "deploy", "bin", "preflight"), "utf8");
    const loop = /for name in ([^;]*?); do/s.exec(script)?.[1] ?? "";
    const listed = loop.replaceAll("\\", " ").split(/\s+/).filter((word) => word !== "");
    expect(listed.sort()).toEqual(Object.keys(compose.secrets).sort());
  });
});

describe("hardening and images (SEC-001)", () => {
  test("SEC-001 only certs adds a capability, and only CHOWN", () => {
    for (const [name, container] of services) {
      expect(container.cap_add ?? (name === "certs" ? ["missing"] : [])).toEqual(name === "certs" ? ["CHOWN"] : []);
    }
  });

  test("SEC-001 every image has a fixed version tag, and lego is pinned", () => {
    for (const [name, container] of services) {
      if (container.image === undefined) continue;
      const tag = container.image.split(":")[1] ?? "";
      expect(`${name}:${tag}`).toMatch(/^[a-z]+:(?!latest$)[\w.-]+$/);
    }
    expect(compose.services["certs"]?.image).toMatch(/^goacme\/lego:v\d+\.\d+\.\d+$/);
    expect(compose.services["postgres"]?.image).toMatch(/^postgres:17\.(?:1[1-9]|[2-9]\d)-bookworm$/);
  });

  test("SEC-001 the hub Dockerfile pins a dated Debian slim tag", () => {
    const dockerfile = readFileSync(join(ROOT, "deploy", "hub", "Dockerfile"), "utf8");
    expect(dockerfile).toMatch(/^FROM debian:bookworm-\d{8}-slim$/m);
    expect(dockerfile).toMatch(/^FROM rust:\d+\.\d+-bookworm AS build$/m);
  });

  test("SEC-001 Dockerfile.dockerignore is an allow-list", () => {
    const lines = readFileSync(join(ROOT, "deploy", "hub", "Dockerfile.dockerignore"), "utf8")
      .split("\n")
      .filter((line) => line !== "" && !line.startsWith("#"));
    expect(lines[0]).toBe("*");
    expect(lines.filter((line) => line.startsWith("!")).sort()).toEqual(["!Cargo.lock", "!Cargo.toml", "!crates/"]);
  });

  test("GW-001 RA-011 the gateway adds only the font and its licence to guacd", () => {
    const dockerfile = readFileSync(join(ROOT, "deploy", "gateway", "Dockerfile"), "utf8");
    const copies = dockerfile.split("\n").filter((line) => line.startsWith("COPY"));
    expect(copies).toEqual([
      "COPY --from=fonts OpenDyslexicMNerdFontMono-Regular.otf LICENSE-OpenDyslexic.txt /usr/share/fonts/opendyslexic/",
    ]);
    expect(dockerfile).toMatch(/^FROM guacamole\/guacd:\d+\.\d+\.\d+$/m);
    expect(dockerfile.match(/^FROM /gm)).toHaveLength(1);
    expect(compose.services["guacd"]?.build?.additional_contexts?.["fonts"]).toBe("../apps/client/src/styles/fonts");
  });

  test("DNS-002 certs is healthy only when tls.pem exists, runs with init, and keeps lego state", () => {
    const certs = compose.services["certs"] as Container;
    expect(certs.healthcheck?.test.join(" ")).toContain("/certs/hub/tls.pem");
    expect(certs.init).toBe(true);
    expect(certs.volumes).toContain("lego_state:/var/lib/lego");
    expect(compose.volumes).toHaveProperty("lego_state");
  });

  test("DB-006 postgres is healthy only over TCP, so the init-time server does not count", () => {
    const test = compose.services["postgres"]?.healthcheck?.test.join(" ") ?? "";
    expect(test).toContain("pg_isready");
    expect(test).toContain("-h 127.0.0.1");
  });

  test("every long-running service restarts, and migrate does not", () => {
    for (const name of ["postgres", "hub", "guacd", "certs"]) {
      expect(compose.services[name]?.restart).toBe("unless-stopped");
    }
    expect(compose.services["migrate"]?.restart).toBe("no");
  });

  test("GW-002 guacd has a tmpfs for its cache", () => {
    expect(compose.services["guacd"]?.tmpfs).toContain("/tmp");
  });
});

describe("images (DB-002)", () => {
  test("DB-002 the postgres image is a fixed Debian tag, not Alpine (group 999 is postgres only there)", () => {
    const image = compose.services["postgres"]?.image ?? "";
    expect(image).toMatch(/^postgres:\d+\.\d+-bookworm$/);
    expect(image).not.toContain("alpine");
  });

  test("DB-002 the hub image runs as 10001:10001, the owner group of the hub secrets", () => {
    const dockerfile = readFileSync(join(ROOT, "deploy", "hub", "Dockerfile"), "utf8");
    expect(dockerfile).toMatch(/^USER 10001:10001$/m);
    expect(dockerfile).toMatch(/^ENTRYPOINT \["anima-hub"\]$/m);
  });
});

describe("this test file", () => {
  test("it changes nothing: no file write, no process", () => {
    const source = readFileSync(import.meta.path, "utf8");
    // Each word is built from two parts so that this list does not match itself.
    const forbidden = [["write", "File"], ["append", "File"], ["mk", "dir"], ["r", "mSync"], ["un", "link"], ["spa", "wn"], ["child", "_process"]];
    for (const [head, tail] of forbidden) expect(source).not.toContain(`${head}${tail}`);
  });
});
