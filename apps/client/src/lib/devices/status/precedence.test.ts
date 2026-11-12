import { describe, expect, it } from 'vitest';
import { combinedStatus, type CombinedStatus, type StatusFacts } from './precedence';

const QUIET: StatusFacts = {
  maintenance: false,
  checks: [],
  securityWarnings: [],
  expiries: [],
  warnWithinDays: 7,
  metricsOverThreshold: [],
};

/** Facts that make exactly one rule apply, in the order of DEV-006. */
const TRIGGERS: readonly (readonly [CombinedStatus, Partial<StatusFacts>])[] = [
  ['MAINTENANCE', { maintenance: true }],
  ['DOWN', { checks: [{ name: 'ping', status: 'DOWN', paused: false }] }],
  ['WARNING', { securityWarnings: ['HOST_KEY_CHANGED'] }],
  ['DEGRADED', { metricsOverThreshold: ['CPU'] }],
  ['UP', { checks: [{ name: 'web', status: 'UP', paused: false }] }],
];

function facts(...parts: readonly Partial<StatusFacts>[]): StatusFacts {
  const checks = parts.flatMap((part) => part.checks ?? []);
  return Object.assign({}, QUIET, ...parts, { checks }) as StatusFacts;
}

describe('DEV-006 combined status precedence', () => {
  it('DEV-006: each rule alone gives its own status and its own reason', () => {
    for (const [status, part] of TRIGGERS) {
      const result = combinedStatus(facts(part));
      expect(result.status).toBe(status);
      expect(result.reasons.length).toBeGreaterThan(0);
    }
  });

  const pairs = TRIGGERS.slice(0, 4).flatMap((first, index) =>
    TRIGGERS.slice(index + 1, 4).map(([lower, b]) => [first[0], lower, first[1], b] as const),
  );
  it.each(pairs)('DEV-006: %s beats %s and both reasons are listed in order', (higher, lower, a, b) => {
    const result = combinedStatus(facts(a, b));
    expect(result.status).toBe(higher);
    expect(result.reasons.map((reason) => reason.status)).toEqual([higher, lower]);
  });

  it.each(TRIGGERS.slice(0, 4))('DEV-006: %s beats Up even when a check is Up', (status, part) => {
    const up = { checks: [{ name: 'web', status: 'UP', paused: false }] } as const;
    expect(combinedStatus(facts(up, part)).status).toBe(status);
  });

  it('DEV-006: Up beats Unknown; no enabled check with a result is Unknown', () => {
    expect(combinedStatus(QUIET).status).toBe('UNKNOWN');
    const paused = [{ name: 'web', status: 'UP', paused: true }] as const;
    const result = combinedStatus(facts({ checks: paused }));
    expect(result.status).toBe('UNKNOWN');
    expect(result.reasons[0]?.text).toBe('Every check is paused');
  });

  it('DEV-006: a paused Down check is not enabled and does not make the device Down', () => {
    const checks = [
      { name: 'ping', status: 'DOWN', paused: true },
      { name: 'web', status: 'UP', paused: false },
    ] as const;
    expect(combinedStatus(facts({ checks })).status).toBe('UP');
  });

  it('DEV-006: Degraded when any check is Degraded', () => {
    const checks = [{ name: 'web', status: 'DEGRADED', paused: false }] as const;
    expect(combinedStatus(facts({ checks })).status).toBe('DEGRADED');
  });

  it('DEV-006 and NOT-006: an expiry inside the shortest threshold is a warning, one outside it is not', () => {
    const near = { expiries: [{ kind: 'CERTIFICATE', name: 'nas.lan', daysLeft: 6 }] } as const;
    const far = { expiries: [{ kind: 'DOMAIN', name: 'example.org', daysLeft: 7 }] } as const;
    expect(combinedStatus(facts(near)).status).toBe('WARNING');
    expect(combinedStatus(facts(far)).status).toBe('UNKNOWN');
    expect(combinedStatus(facts(near)).reasons[0]?.text).toBe('The certificate nas.lan expires in 6 days');
  });

  it('DEV-006: every reason of one rule is kept, after the rules above it', () => {
    const result = combinedStatus(
      facts({ securityWarnings: ['HOST_KEY_CHANGED', 'CERTIFICATE_CHANGED'], metricsOverThreshold: ['Disk'] }, { checks: [{ name: 'ping', status: 'DOWN', paused: false }] }),
    );
    expect(result.reasons.map((reason) => `${reason.status}:${reason.text}`)).toEqual([
      'DOWN:ping is down',
      'WARNING:The SSH host key has changed',
      'WARNING:The certificate has changed',
      'DEGRADED:Disk is over its threshold',
    ]);
  });
});
