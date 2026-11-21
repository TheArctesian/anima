/** An answer of the pool in the form of `contracts/account-usage.md`, for the tests. */
export const POOL_ANSWER = {
  account: 'cus_test',
  period: '2026-11',
  until: 1_795_161_600,
  currency: 'usd',
  lines: [
    { size: 'solo', plan: 'hobby', kind: 'running', seconds: 40_000, hours: '11.1', amount: 300 },
    { size: 'solo', plan: 'hobby', kind: 'running', seconds: 5_000, hours: '1.3', amount: 112 },
    { size: 'solo', plan: 'hobby', kind: 'asleep', seconds: 1_800, hours: '0.5', amount: 0 },
  ],
  total: 412,
  prices_set: true,
  hubs: [{ id: 'h1', state: 'running', size: 'solo', plan: 'hobby', disk_limit_bytes: 10_000_000_000, disk_used_bytes: 2_500_000_000 }],
};

/** The one hub of `POOL_ANSWER` with other members. */
export const withHub = (hub: Record<string, unknown>) => ({ ...POOL_ANSWER, hubs: [{ ...POOL_ANSWER.hubs[0], ...hub }] });
