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
  read_only?: boolean;
  depends_on?: Record<string, unknown>;
  healthcheck?: { test: string[]; interval?: string; retries?: number };
}
interface Compose {
  services: Record<string, Container>;
  networks: Record<string, { internal?: boolean; ipam?: { config?: { subnet: string }[] } } | null>;
  volumes: Record<string, unknown>;
}

interface Block { first: number; size: number; contains: (address: number) => boolean }

function ipv4(text: string): number {
  const parts = text.split(".").map(Number);
  if (parts.length !== 4 || parts.some((part) => !Number.isInteger(part) || part < 0 || part > 255)) throw new Error(`not an IPv4 address: ${text}`);
  return parts.reduce((sum, part) => sum * 256 + part, 0);
}

function cidr(text: string): Block {
  const [address, prefixText] = text.split("/");
  const prefix = Number(prefixText);
  if (address === undefined || !Number.isInteger(prefix) || prefix < 0 || prefix > 32) throw new Error(`not a CIDR: ${text}`);
  const size = 2 ** (32 - prefix);
  const first = ipv4(address);
  if (first % size !== 0) throw new Error(`${text} has host bits set`);
  return { first, size, contains: (value) => value >= first && value < first + size };
}

/** The text of a compose entry with `${NAME:-default}` replaced by its default; other `${...}` become opaque. */
function resolveDefault(text: string): string {
  return text.replace(/\$\{[A-Z_]+:-([^}]*)\}/g, "$1");
}

/** The deny entries that are literal or have a default, as blocks. Required variables (`:?`) are opaque and skipped. */
function denyEntries(hub: Container): Block[] {
  return (hub.environment?.["ANIMA_PROBE_DENY"] ?? "")
    .split(",")
    .map(resolveDefault)
    .filter((entry) => !entry.includes("${"))
    .map(cidr);
}

/** How many addresses of `subnet` some entry covers. */
function coveredCount(entries: Block[], subnet: Block): number {
  let count = 0;
  for (let host = 0; host < subnet.size; host += 1) {
    if (entries.some((entry) => entry.contains(subnet.first + host))) count += 1;
  }
  return count;
}

const compose = Bun.YAML.parse(readFileSync(join(ROOT, "deploy", "compose.yaml"), "utf8")) as Compose;

describe("hub configuration", () => {
  test("every ANIMA_* variable the hub receives is read by config.rs or listed in the conventions.md environment table", () => {
    const source = readFileSync(CONFIG_PATH, "utf8");
    const conventions = readFileSync(join(ROOT, "contracts", "conventions.md"), "utf8");
    // Phase 2 variables (K7) are frozen in phase-2-contracts.md until conventions.md absorbs them.
    const phase2 = readFileSync(join(ROOT, "docs", "programme", "phase-2-contracts.md"), "utf8");
    // Phase 3 variables (K7) are frozen in phase-3-contracts.md for the same reason.
    const phase3 = readFileSync(join(ROOT, "docs", "programme", "phase-3-contracts.md"), "utf8");
    const known = new Set([...source.matchAll(/"(ANIMA_[A-Z_]+)"/g), ...conventions.matchAll(/`(ANIMA_[A-Z_]+)`/g), ...phase2.matchAll(/`(ANIMA_[A-Z_]+)`/g), ...phase3.matchAll(/`(ANIMA_[A-Z_]+)`/g)].map((match) => match[1]));
    for (const name of ["hub", "migrate"]) {
      for (const key of Object.keys(compose.services[name]?.environment ?? {})) {
        expect(known.has(key) ? key : `${name}.${key} is in neither config.rs nor conventions.md`).toBe(key);
      }
    }
  });

  test("NET-001 ANIMA_PROBE_DENY covers every network the hub joins, notify minus exactly the Apprise address", () => {
    const hub = compose.services["hub"] as Container;
    const apprise = compose.services["apprise"] as Container;
    const appriseAddress = ipv4(String((apprise.networks as Record<string, { ipv4_address?: string }>)["notify"]?.ipv4_address));
    const entries = denyEntries(hub);
    const attached = Array.isArray(hub.networks) ? hub.networks : Object.keys(hub.networks ?? {});
    expect(attached.sort()).toEqual(["database", "gateway", "localapi", "notify"]);
    for (const name of attached) {
      const subnet = cidr(resolveDefault(compose.networks[name]?.ipam?.config?.[0]?.subnet ?? `${name} has no fixed subnet`));
      const carved = name === "notify";
      // Every address of the subnet is denied, except the Apprise address on notify.
      expect(coveredCount(entries, subnet)).toBe(subnet.size - (carved ? 1 : 0));
    }
    expect(entries.some((entry) => entry.contains(appriseAddress))).toBe(false);
  });

  test("NET-001 the notify carve-out leaves only 172.30.3.10 allowed, with no overlapping blocks", () => {
    const entries = denyEntries(compose.services["hub"] as Container);
    const subnet = cidr("172.30.3.0/24");
    const allowed = [];
    for (let host = 0; host < subnet.size; host += 1) {
      if (!entries.some((entry) => entry.contains(subnet.first + host))) allowed.push(host);
    }
    expect(allowed).toEqual([10]);
    // The block sizes add up to the subnet minus one: no block hides behind a wider one.
    const inside = entries.filter((entry) => subnet.contains(entry.first));
    expect(inside.length).toBe(8);
    expect(inside.reduce((sum, entry) => sum + entry.size, 0)).toBe(subnet.size - 1);
  });

  test("NET-001 the deny list holds the VPS public address and the hub VPN address as /32", () => {
    const deny = (compose.services["hub"]?.environment?.["ANIMA_PROBE_DENY"] ?? "").split(",");
    expect(deny).toContain("${ANIMA_VPS_PUBLIC_ADDRESS:?set to the VPS public IPv4}/32");
    expect(deny).toContain("${ANIMA_HUB_ADDRESS:?set ANIMA_HUB_ADDRESS to the hub's VPN address}/32");
  });

  test("K7 the hub gets the web, origin and recording variables with the values of plan Section 6", () => {
    const environment = compose.services["hub"]?.environment ?? {};
    expect(environment["ANIMA_PUBLIC_ORIGIN"]).toBe("${ANIMA_PUBLIC_ORIGIN:-https://hub.anima.ing}");
    expect(environment["ANIMA_WEB_ROOT"]).toBe("/usr/share/anima/web");
    expect(environment["ANIMA_RECORDING_ROOT"]).toBe("/var/lib/anima/recordings");
    expect(environment["ANIMA_RECORDING_MAX_BYTES"]).toBe("${ANIMA_RECORDING_MAX_BYTES:-2147483648}");
  });

  test("D-19 only the hub mounts the recordings volume, read-write, at ANIMA_RECORDING_ROOT", () => {
    const hub = compose.services["hub"] as Container;
    const root = hub.environment?.["ANIMA_RECORDING_ROOT"];
    expect(hub.volumes).toContain(`recordings:${root}`);
    expect(compose.volumes).toHaveProperty("recordings");
    const users = Object.entries(compose.services).filter(([, service]) => JSON.stringify(service.volumes ?? []).includes("recordings")).map(([name]) => name);
    expect(users).toEqual(["hub"]);
  });

  test("SEC-001 the hub root file system stays read-only with the recordings volume", () => {
    expect(compose.services["hub"]?.read_only).toBe(true);
  });

  test("D-13 the hub joins notify and does not wait for Apprise", () => {
    const hub = compose.services["hub"] as Container;
    expect(Object.keys(hub.networks as object)).toContain("notify");
    expect(Object.keys(hub.depends_on ?? {})).not.toContain("apprise");
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

  test("SEC-005 R-031 the image builds only the hub binary with one unchanged cargo line, in the rust stage", () => {
    expect(builds).toEqual(["RUN cargo build --release --locked --bin anima-hub"]);
    const rust = dockerfile.slice(dockerfile.indexOf("AS build"), dockerfile.indexOf("\nFROM debian"));
    expect(rust).toContain(builds[0]);
  });

  test("SEC-005 the build never enables test targets or extra features", () => {
    for (const flag of ["--all-targets", "--tests", "--features", "--all-features", "test-support"]) {
      expect(dockerfile).not.toContain(flag);
    }
  });

  test("SEC-005 every workspace member is in the hub build context, so cargo can load the workspace", () => {
    const manifest = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
    const members = [...(/^members = \[(.*)\]$/m.exec(manifest)?.[1] ?? "").matchAll(/"([^"]+)"/g)].map((match) => match[1] ?? "");
    const copied = dockerfile.split("\n").filter((line) => line.startsWith("COPY ") && !line.includes("--from")).map((line) => line.split(" ")[1] ?? "");
    expect(members.length).toBeGreaterThan(0);
    for (const member of members) {
      expect(copied.some((source) => member === source || member.startsWith(`${source}/`))).toBe(true);
    }
  });
});

const hubDockerfile = readFileSync(join(ROOT, "deploy", "hub", "Dockerfile"), "utf8");
  const stage = (name: string): string[] => {
  const lines = hubDockerfile.split("\n");
  const start = lines.findIndex((line) => new RegExp(`^FROM .* AS ${name}$`).test(line));
  const end = lines.findIndex((line, index) => index > start && line.startsWith("FROM "));
  return lines.slice(start, end === -1 ? undefined : end);
};
const finalStage = hubDockerfile.split("\n").slice(hubDockerfile.split("\n").map((line) => line.startsWith("FROM ")).lastIndexOf(true));

describe("hub image web stage and recordings folder", () => {

  test("D-2 the web stage builds the client and the final stage copies only its output to /usr/share/anima/web", () => {
    const web = stage("web");
    expect(web[0]).toMatch(/^FROM (--platform=\$BUILDPLATFORM )?oven\/bun:1\.3\.14-\S+ AS web$/);
    expect(web).toContain("RUN bun run --cwd apps/client build");
    expect(finalStage.filter((line) => line.includes("--from=web"))).toEqual([
      "COPY --from=web /src/apps/client/build /usr/share/anima/web",
    ]);
  });

  test("E-5 the web stage runs one frozen install limited to the client", () => {
    expect(stage("web").filter((line) => line.includes("bun install"))).toEqual([
      "RUN bun install --frozen-lockfile --filter @anima/client",
    ]);
  });

  test("D-13 E-3 the final stage seeds the recordings folder for 10001:10001 with mode 0700 and changes nothing else", () => {
    const commands = finalStage.join("\n").replace(/\\\n/g, " ").split("\n");
    const run = commands.find((line) => line.startsWith("RUN mkdir")) ?? "";
    expect(run.split(/\s*&&\s*/)).toEqual([
      "RUN mkdir -p /var/lib/anima/recordings",
      "chown 10001:10001 /var/lib/anima/recordings",
      "chmod 0700 /var/lib/anima/recordings",
    ]);
    expect(commands.filter((line) => /\b(chmod|chown)\b/.test(line))).toEqual([run]);
    expect(commands.indexOf(run)).toBeLessThan(commands.indexOf("USER 10001:10001"));
  });
});

describe("hub image build context", () => {
  test("E-5 every workspace manifest the frozen install reads is copied by the web stage and allowed by the dockerignore", () => {
    const root = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as { workspaces: string[] };
    const manifests = ["package.json"];
    for (const glob of root.workspaces) {
      for (const found of new Bun.Glob(`${glob}/package.json`).scanSync({ cwd: ROOT })) manifests.push(found);
    }
    const members = [...new Set(manifests)].filter((path) => !path.includes("node_modules"));
    expect(members).toContain("apps/client/package.json");
    expect(members).toContain("deploy/dns/package.json");
    const web = stage("web").join("\n");
    const allowed = readFileSync(join(ROOT, "deploy", "hub", "Dockerfile.dockerignore"), "utf8").split("\n");
    for (const manifest of members) {
      expect(web).toContain(`COPY ${manifest} `);
      const dir = manifest.slice(0, manifest.lastIndexOf("/") + 1);
      expect(allowed.some((line) => line === `!${manifest}` || (dir !== "" && line === `!${dir}`) || line === `!${dir}**`)).toBe(true);
    }
    expect(web).toContain("COPY package.json bun.lock ");
    expect(allowed).toContain("!bun.lock");
    expect(allowed).toContain("!apps/client/");
    const lastAllow = allowed.findLastIndex((line) => line.startsWith("!"));
    for (const excluded of ["**/node_modules", "**/.svelte-kit", "**/build", "**/dist", "**/.env*", "**/secrets", "**/.DS_Store"]) {
      expect(allowed.indexOf(excluded)).toBeGreaterThan(lastAllow);
    }
  });

  test("SEC-001 Dockerfile.dockerignore is an allow-list", () => {
    const lines = readFileSync(join(ROOT, "deploy", "hub", "Dockerfile.dockerignore"), "utf8")
      .split("\n")
      .filter((line) => line !== "" && !line.startsWith("#"));
    expect(lines[0]).toBe("*");
    const allowed = lines.filter((line) => line.startsWith("!"));
    // The Rust build, the lock files and the web stage's manifests and client; nothing else (SEC-001).
    expect(allowed.filter((line) => !/^!(Cargo\.(toml|lock)|bun\.lock|package\.json|crates\/|apps\/client\/|apps\/desktop\/(src-tauri\/|package\.json)|(deploy(\/dns|\/headscale)?|design|quality)\/package\.json)$/.test(line))).toEqual([]);
    expect(allowed).toContain("!crates/");
  });
});
