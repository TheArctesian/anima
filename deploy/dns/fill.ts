// Fills the address placeholders of anima.ing.records.json after checking each value.
// Usage, from the repository root: bun deploy/dns/fill.ts [--out FILE] [RECORD_NAME ...]
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { isPublicAddress, isVpnAddress } from './address.ts';

/** A value or argument that must not be written into the records. Messages never contain the value. */
export class FillError extends Error {
  override name = 'FillError';
}

export type Env = Readonly<Record<string, string | undefined>>;

interface Zone {
  zone: string;
  records: { name: string }[];
}

const PLACEHOLDERS: Readonly<Record<string, { accepts: (value: string) => boolean; rule: string }>> = {
  VPS_PUBLIC_IPV4: {
    accepts: isPublicAddress,
    rule: 'a public IPv4 dotted quad (not private, CGNAT, loopback, link-local, documentation or reserved)',
  },
  HUB_VPN_IPV4: { accepts: isVpnAddress, rule: 'an IPv4 dotted quad in 100.64.0.0/10' },
};

function checkedValue(variable: string, env: Env): string {
  const placeholder = PLACEHOLDERS[variable];
  if (placeholder === undefined) throw new FillError(`unknown placeholder \${${variable}}`);
  const value = env[variable];
  if (value === undefined || value === '') throw new FillError(`${variable} is unset or empty`);
  if (!placeholder.accepts(value)) throw new FillError(`${variable} must be ${placeholder.rule}`);
  return value;
}

function fillRecord(record: unknown, env: Env): unknown {
  const text = JSON.stringify(record).replace(/\$\{([^}]*)\}/g, (_match, variable: string) =>
    checkedValue(variable, env),
  );
  return JSON.parse(text);
}

/**
 * Returns the records JSON with every placeholder in the selected records filled from `env`.
 * `names` selects records by name; empty selects all. Throws FillError when a value fails its check.
 */
export function fillRecords(template: string, names: readonly string[], env: Env): string {
  const zone = JSON.parse(template) as Zone;
  for (const name of names) {
    if (!zone.records.some((record) => record.name === name)) throw new FillError(`no record named ${name}`);
  }
  const selected = names.length === 0 ? zone.records : zone.records.filter((record) => names.includes(record.name));
  const records = selected.map((record) => fillRecord(record, env));
  return `${JSON.stringify({ zone: zone.zone, records }, null, 2)}\n`;
}

function run(args: readonly string[], env: Env): number {
  const out = args[0] === '--out' ? args[1] : undefined;
  if (args[0] === '--out' && (out === undefined || out === '')) {
    console.error('fill: --out needs a file name');
    return 2;
  }
  const names = out === undefined ? args : args.slice(2);
  try {
    const filled = fillRecords(readFileSync(join(import.meta.dir, 'anima.ing.records.json'), 'utf8'), names, env);
    if (out === undefined) process.stdout.write(filled);
    else writeFileSync(out, filled);
    return 0;
  } catch (error) {
    if (!(error instanceof FillError)) throw error;
    console.error(`fill: ${error.message}; nothing written`);
    return 1;
  }
}

if (import.meta.main) process.exitCode = run(Bun.argv.slice(2), process.env);
