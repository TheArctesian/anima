import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// The contract fixtures (K6) the synthetic hub answers from. Phase 2 and later are merged by id, the newest file winning per
// id, so every phase's devices show. Phase 1 supplies only the operator: its devices reuse Phase 2 names under other ids,
// and the same name would show twice.

const FIXTURE_FOLDER = fileURLToPath(new URL('../../../../contracts/fixtures/', import.meta.url));
const PHASES = ['phase1', 'phase2', 'phase3'] as const;

export type Row = Record<string, unknown>;

export interface Fixtures {
  readonly operator: Row;
  readonly sites: Row[];
  readonly devices: Row[];
  readonly checks: Row[];
  readonly sessions: Row[];
  readonly metrics: Row[];
  /** The Settings slices (WP-50): the retention periods, the check defaults and the API keys. */
  readonly retention: Row[];
  readonly checkDefaults: Row;
  readonly apiKeys: Row[];
}

function readPhase(name: string): Row | undefined {
  const path = `${FIXTURE_FOLDER}${name}.json`;
  return existsSync(path) ? (JSON.parse(readFileSync(path, 'utf8')) as Row) : undefined;
}

function rows(file: Row, name: string): Row[] {
  const value = file[name];
  return Array.isArray(value) ? (value as Row[]) : [];
}

function mergedById(files: readonly Row[], name: string): Row[] {
  const byId = new Map<unknown, Row>();
  for (const file of files) for (const row of rows(file, name)) byId.set(row.id ?? row.deviceId, row);
  return [...byId.values()];
}

/** Reads `contracts/fixtures/phase*.json`. */
export function loadFixtures(): Fixtures {
  const files = PHASES.map(readPhase).filter((file): file is Row => file !== undefined);
  const operator = files.map((file) => file.operator).find((found) => found !== undefined);
  const later = PHASES.slice(1).map(readPhase).filter((file): file is Row => file !== undefined);
  const collection = (name: string): Row[] => mergedById(later, name);
  return {
    operator: (operator ?? {}) as Row,
    sites: collection('sites'),
    devices: collection('devices'),
    checks: collection('checks'),
    sessions: collection('sessions'),
    metrics: collection('metrics'),
    retention: later.flatMap((file) => rows(file, 'retention')),
    checkDefaults: (later.map((file) => file.checkDefaults).find((found) => found !== undefined) ?? {}) as Row,
    apiKeys: later.flatMap((file) => rows(file, 'apiKeys')),
  };
}
