/** An answer of the pool in the form of `contracts/account-usage.md`, for the tests. */
export const POOL_ANSWER = {
  at: '2026-11-20T18:00:00Z',
  month: '2026-11',
  plan: { name: 'Hobby', charged: true },
  awakeSeconds: 45_000,
  asleepSeconds: 1_800,
  amount: { minor: 412, currency: 'USD' },
  owes: false,
  disk: { limitBytes: 10_000_000_000, usedBytes: 2_500_000_000 },
};
