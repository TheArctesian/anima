// Headscale config template and Compose file (WP-03 T2; VPN-003, VPS-002, VPS-003,
// DNS-006, NET-005; decisions D-5 to D-9).
import { describe, expect, test } from 'bun:test';
import { parseGoDuration, seconds } from '../src/go-duration.ts';
import { days, parsePrometheusDuration } from '../src/prometheus-duration.ts';

const root = new URL('../', import.meta.url);
const template = await Bun.file(new URL('config/config.yaml', root)).text();
const composeText = await Bun.file(new URL('compose.yaml', root)).text();

const sample: Record<string, string> = {
  VPS_PUBLIC_IPV4: '203.0.113.10',
  ACME_EMAIL: 'operator@example.org',
};

type Tree = Record<string, unknown>;

/** Renders like `envsubst` limited to the given variables: other text is untouched. */
function render(text: string, values: Record<string, string>): string {
  return text.replace(/\$\{([A-Za-z_][A-Za-z0-9_]*)\}/g, (whole, name: string) => values[name] ?? whole);
}

function at(tree: unknown, path: string): unknown {
  let node = tree;
  for (const key of path.split('.')) {
    node = typeof node === 'object' && node !== null ? (node as Tree)[key] : undefined;
  }
  return node;
}

function leafPaths(tree: unknown, prefix = ''): string[] {
  if (typeof tree !== 'object' || tree === null || Array.isArray(tree)) {
    return [prefix];
  }
  const entries = Object.entries(tree as Tree);
  if (entries.length === 0) {
    return [prefix];
  }
  return entries.flatMap(([key, value]) => leafPaths(value, prefix === '' ? key : `${prefix}.${key}`));
}

interface Listener {
  host: string;
  port: number;
}

function parseListener(address: string): Listener {
  const match = /^(?:\[([^\]]*)\]|([^:]*)):(\d+)$/.exec(address);
  if (match === null) {
    throw new Error(`not a host:port listener: ${address}`);
  }
  return { host: match[1] ?? match[2] ?? '', port: Number(match[3]) };
}

const isLoopback = (host: string): boolean => /^127\.\d+\.\d+\.\d+$/.test(host) || host === '::1';

const config: unknown = Bun.YAML.parse(render(template, sample));
const compose: unknown = Bun.YAML.parse(composeText);
const service = at(compose, 'services.headscale');

// Every listener key of Headscale v0.29.7 and its protocol. A key containing "listen"
// that is not here fails the inventory test, so a new listener cannot slip past.
const listenerProtocols: Record<string, 'tcp' | 'udp'> = {
  listen_addr: 'tcp',
  metrics_listen_addr: 'tcp',
  grpc_listen_addr: 'tcp',
  'derp.server.stun_listen_addr': 'udp',
  tls_letsencrypt_listen: 'tcp',
};

function listeners(): { key: string; protocol: 'tcp' | 'udp'; listener: Listener }[] {
  return Object.entries(listenerProtocols).flatMap(([key, protocol]) => {
    const value = at(config, key);
    return typeof value === 'string' && value !== '' ? [{ key, protocol, listener: parseListener(value) }] : [];
  });
}

describe('placeholders (VPS-003, LOG-007)', () => {
  test('every ${…} placeholder in the template is in the documented header list', () => {
    const used = new Set([...template.matchAll(/\$\{([^}]*)\}/g)].map((m) => m[1]));
    const header = template.slice(0, template.indexOf('\nserver_url:'));
    for (const name of used) {
      expect(header).toMatch(new RegExp(`^#\\s+${name}\\s`, 'm'));
    }
    expect([...used].sort()).toEqual(Object.keys(sample).sort());
  });

  test('no placeholder or $ is left after rendering the documented variables', () => {
    expect(render(template, sample)).not.toContain('$');
  });

  test('rendered values land where the decisions put them (D-6, D-7)', () => {
    expect(at(config, 'derp.server.ipv4')).toBe('203.0.113.10');
    expect(at(config, 'acme_email')).toBe('operator@example.org');
  });
});

describe('config: control server, expiry and names (VPN-003, DNS-006, NET-005)', () => {
  test('VPN-003: node.expiry parses as exactly 180 days', () => {
    const expiry = at(config, 'node.expiry');
    expect(typeof expiry).toBe('string');
    expect(parsePrometheusDuration(expiry as string)).toBe(days(180));
  });

  test('server_url is https://hs.anima.ing', () => {
    expect(at(config, 'server_url')).toBe('https://hs.anima.ing');
  });

  test('DNS-006: base_domain is net.anima.ing, not the server_url host and not its parent', () => {
    const base = at(config, 'dns.base_domain');
    const host = new URL(at(config, 'server_url') as string).hostname;
    expect(base).toBe('net.anima.ing');
    expect(host).not.toBe(base);
    expect(host.endsWith(`.${String(base)}`)).toBe(false);
  });

  test('DNS-006: MagicDNS on, split DNS only, no pushed resolvers or extra records (D-5, D-8)', () => {
    expect(at(config, 'dns.magic_dns')).toBe(true);
    expect(at(config, 'dns.override_local_dns')).toBe(false);
    expect(at(config, 'dns.nameservers.global')).toEqual([]);
    expect(at(config, 'dns.nameservers.split')).toEqual({});
    expect(at(config, 'dns.search_domains')).toEqual([]);
    expect(at(config, 'dns.extra_records')).toEqual([]);
  });

  test('NET-005: node addresses come sequentially from the Tailscale ranges', () => {
    expect(at(config, 'prefixes.v4')).toBe('100.64.0.0/10');
    expect(at(config, 'prefixes.v6')).toBe('fd7a:115c:a1e0::/48');
    expect(at(config, 'prefixes.allocation')).toBe('sequential');
  });

  test('Go durations pass the bounds Headscale enforces at start (else fatal)', () => {
    const go = (key: string): number | null => parseGoDuration(String(at(config, key)));
    const interval = go('node.routes.ha.probe_interval') ?? 0;
    const timeout = go('node.routes.ha.probe_timeout') ?? 0;
    expect(go('node.ephemeral.inactivity_timeout')).toBeGreaterThan(seconds(65));
    expect(interval).toBeGreaterThanOrEqual(seconds(2));
    expect(timeout).toBeGreaterThanOrEqual(seconds(1));
    expect(timeout).toBeLessThan(interval);
  });

  test('VPN-004: policy is the file beside the config (T1 path)', () => {
    expect(at(config, 'policy.mode')).toBe('file');
    expect(at(config, 'policy.path')).toBe('/etc/headscale/policy.hujson');
  });
});

describe('config: listeners and ports (VPS-002, VPS-003)', () => {
  test('VPS-003: every key naming a listener is a known Headscale listener', () => {
    const named = leafPaths(config).filter((path) => path.includes('listen'));
    expect(named.filter((path) => !(path in listenerProtocols))).toEqual([]);
  });

  test('VPS-003: gRPC and metrics are set explicitly and on loopback', () => {
    for (const key of ['grpc_listen_addr', 'metrics_listen_addr']) {
      expect(isLoopback(parseListener(at(config, key) as string).host)).toBe(true);
    }
    expect(at(config, 'grpc_allow_insecure')).toBe(false);
  });

  test('VPS-002: no listener uses a wildcard address (0.0.0.0, [::], empty host)', () => {
    const wildcards = listeners().filter(({ listener }) => ['', '0.0.0.0', '::'].includes(listener.host));
    expect(wildcards.map(({ key }) => key)).toEqual([]);
  });

  test('VPS-003: every listener except listen_addr and STUN is loopback', () => {
    const exposed = listeners().filter(({ listener }) => !isLoopback(listener.host));
    expect(exposed.map(({ key }) => key).sort()).toEqual(['derp.server.stun_listen_addr', 'listen_addr']);
  });

  test('VPS-002: public listeners are exactly 443/tcp and 3478/udp on the rendered VPS address', () => {
    const ports = listeners()
      .filter(({ listener }) => !isLoopback(listener.host))
      .map(({ protocol, listener }) => `${listener.host}:${String(listener.port)}/${protocol}`);
    expect(ports.sort()).toEqual(['203.0.113.10:3478/udp', '203.0.113.10:443/tcp']);
  });

  test('VPS-003: the CLI socket is a unix socket not open to other users', () => {
    expect(at(config, 'unix_socket')).toBe('/var/run/headscale/headscale.sock');
    const mode = at(config, 'unix_socket_permission');
    expect(mode).toMatch(/^0[0-7]{3}$/);
    expect(Number.parseInt(mode as string, 8) & 0o007).toBe(0);
  });

  test('VPS-003: no OIDC and no forwarded-for trust', () => {
    expect(at(config, 'oidc')).toBeUndefined();
    expect(at(config, 'trusted_proxies')).toEqual([]);
  });
});

describe('config: TLS, DERP and storage (D-6, D-7, VPS-006)', () => {
  test('VPS-003: Let\'s Encrypt TLS-ALPN-01 on :443, no HTTP-01 listener, no static certs', () => {
    expect(at(config, 'acme_url')).toBe('https://acme-v02.api.letsencrypt.org/directory');
    expect(at(config, 'tls_letsencrypt_hostname')).toBe('hs.anima.ing');
    expect(at(config, 'tls_letsencrypt_challenge_type')).toBe('TLS-ALPN-01');
    expect(parseListener(at(config, 'listen_addr') as string).port).toBe(443);
    expect(at(config, 'tls_letsencrypt_listen')).toBeUndefined();
    expect(at(config, 'tls_cert_path')).toBe('');
    expect(at(config, 'tls_key_path')).toBe('');
    expect(at(config, 'tls_letsencrypt_cache_dir')).toBe('/var/lib/headscale/cache');
  });

  test('D-7: embedded DERP on, verifying clients, STUN 3478, no external DERP maps', () => {
    expect(at(config, 'derp.server.enabled')).toBe(true);
    expect(at(config, 'derp.server.region_id')).toBe(999);
    expect(at(config, 'derp.server.verify_clients')).toBe(true);
    expect(parseListener(at(config, 'derp.server.stun_listen_addr') as string).port).toBe(3478);
    expect(at(config, 'derp.server.automatically_add_embedded_derp_region')).toBe(true);
    expect(at(config, 'derp.urls')).toEqual([]);
    expect(at(config, 'derp.paths')).toEqual([]);
    expect(at(config, 'derp.auto_update_enabled')).toBe(false);
  });

  test('VPS-006: database and keys live in the lib volume', () => {
    expect(at(config, 'database.type')).toBe('sqlite');
    expect(at(config, 'database.sqlite.path')).toBe('/var/lib/headscale/db.sqlite');
    expect(at(config, 'database.sqlite.write_ahead_log')).toBe(true);
    expect(at(config, 'noise.private_key_path')).toBe('/var/lib/headscale/noise_private.key');
    expect(at(config, 'derp.server.private_key_path')).toMatch(/^\/var\/lib\/headscale\//);
  });

  test('LOG-007: log level info (trace logs the plaintext pre-auth key), no SQL debug', () => {
    expect(at(config, 'log.level')).toBe('info');
    expect(at(config, 'log.format')).toBe('json');
    expect(at(config, 'database.debug')).toBe(false);
    expect(at(config, 'database.gorm.parameterized_queries')).toBe(true);
  });

  test('outbound features are off: update check, logtail, taildrop, client auto-update', () => {
    expect(at(config, 'disable_check_updates')).toBe(true);
    expect(at(config, 'auto_update.enabled')).toBe(false);
    expect(at(config, 'logtail.enabled')).toBe(false);
    expect(at(config, 'taildrop.enabled')).toBe(false);
  });
});

// Leaf keys of the v0.29.7 config-example.yaml. A misspelt key is ignored by Headscale
// and its default applies (node.expiry would default to "never"), so unknown keys fail.
const knownKeys = new Set([
  'server_url', 'listen_addr', 'metrics_listen_addr', 'grpc_listen_addr', 'grpc_allow_insecure',
  'trusted_proxies', 'noise.private_key_path', 'prefixes.v4', 'prefixes.v6', 'prefixes.allocation',
  'derp.server.enabled', 'derp.server.region_id', 'derp.server.region_code', 'derp.server.region_name',
  'derp.server.verify_clients', 'derp.server.stun_listen_addr', 'derp.server.private_key_path',
  'derp.server.automatically_add_embedded_derp_region', 'derp.server.ipv4', 'derp.server.ipv6',
  'derp.urls', 'derp.paths', 'derp.auto_update_enabled', 'derp.update_frequency',
  'disable_check_updates', 'node.expiry', 'node.ephemeral.inactivity_timeout',
  'node.routes.ha.probe_interval', 'node.routes.ha.probe_timeout', 'database.type', 'database.debug',
  'database.gorm.prepare_stmt', 'database.gorm.parameterized_queries',
  'database.gorm.skip_err_record_not_found', 'database.gorm.slow_threshold', 'database.sqlite.path',
  'database.sqlite.write_ahead_log', 'database.sqlite.wal_autocheckpoint', 'acme_url', 'acme_email',
  'tls_letsencrypt_hostname', 'tls_letsencrypt_cache_dir', 'tls_letsencrypt_challenge_type',
  'tls_letsencrypt_listen', 'tls_cert_path', 'tls_key_path', 'log.level', 'log.format', 'policy.mode',
  'policy.path', 'dns.magic_dns', 'dns.base_domain', 'dns.override_local_dns', 'dns.nameservers.global',
  'dns.nameservers.split', 'dns.search_domains', 'dns.extra_records', 'unix_socket',
  'unix_socket_permission', 'logtail.enabled', 'taildrop.enabled', 'auto_update.enabled',
]);

test('every config key exists in Headscale v0.29.7', () => {
  expect(leafPaths(config).filter((path) => !knownKeys.has(path))).toEqual([]);
});

describe('compose (VPS-002, VPS-003, D-9)', () => {
  test('Headscale is the only service, with exactly the reviewed keys (no environment, user, pid, entrypoint)', () => {
    // Guards against HEADSCALE_* environment overrides (viper AutomaticEnv) and against
    // user, pid or entrypoint changes that the review did not see.
    expect(Object.keys(at(compose, 'services') as Tree)).toEqual(['headscale']);
    expect(Object.keys(service as Tree).sort()).toEqual([
      'cap_add', 'cap_drop', 'command', 'healthcheck', 'image', 'logging', 'network_mode',
      'read_only', 'restart', 'security_opt', 'tmpfs', 'volumes',
    ]);
  });

  test('image is Headscale 0.29.7 pinned by tag and digest', () => {
    expect(at(service, 'image')).toMatch(/^docker\.io\/headscale\/headscale:0\.29\.7@sha256:[0-9a-f]{64}$/);
    expect(at(service, 'command')).toBe('serve');
    expect(at(service, 'restart')).toBe('always');
  });

  test('VPS-002: host network and no Docker-published ports', () => {
    expect(at(service, 'network_mode')).toBe('host');
    expect(at(service, 'ports')).toBeUndefined();
  });

  test('VPS-003: read-only root, no new privileges, only NET_BIND_SERVICE', () => {
    expect(at(service, 'read_only')).toBe(true);
    expect(at(service, 'privileged')).toBeUndefined();
    expect(at(service, 'security_opt')).toEqual(['no-new-privileges:true']);
    expect(at(service, 'cap_drop')).toEqual(['ALL']);
    expect(at(service, 'cap_add')).toEqual(['NET_BIND_SERVICE']);
  });

  test('VPS-003: config is mounted read-only; only lib and the root-only socket folder are writable', () => {
    expect(at(service, 'volumes')).toEqual(['./config:/etc/headscale:ro', './lib:/var/lib/headscale']);
    expect(at(service, 'tmpfs')).toEqual(['/var/run/headscale:mode=0700']);
  });

  test('health check uses the CLI; logs rotate by size', () => {
    expect(at(service, 'healthcheck.test')).toEqual(['CMD', 'headscale', 'health']);
    expect(at(service, 'logging.driver')).toBe('json-file');
    expect(at(service, 'logging.options.max-size')).toMatch(/^\d+[km]$/);
    expect(at(service, 'logging.options.max-file')).toMatch(/^\d+$/);
  });
});
