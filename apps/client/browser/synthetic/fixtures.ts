import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// The contract fixtures (K6) the synthetic hub answers from. Later phases replace earlier ones per collection:
// a device list is one phase's, never a mix of two (the same names would show twice).

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
}

function readPhase(name: string): Row | undefined {
  const path = `${FIXTURE_FOLDER}${name}.json`;
  return existsSync(path) ? (JSON.parse(readFileSync(path, 'utf8')) as Row) : undefined;
}

function rows(file: Row, name: string): Row[] | undefined {
  const value = file[name];
  return Array.isArray(value) && value.length > 0 ? (value as Row[]) : undefined;
}

/** Reads `contracts/fixtures/phase*.json`; the newest file that has a collection supplies it. */
export function loadFixtures(): Fixtures {
  const files = PHASES.map(readPhase).filter((file): file is Row => file !== undefined);
  const newest = (name: string): Row[] =>
    files.map((file) => rows(file, name)).findLast((found) => found !== undefined) ?? [];
  const operator = files.map((file) => file.operator).find((found) => found !== undefined);
  return {
    operator: (operator ?? {}) as Row,
    sites: newest('sites'),
    devices: newest('devices'),
    checks: newest('checks'),
    sessions: newest('sessions'),
    metrics: newest('metrics'),
  };
}
