import type { ConnectionTestResult } from './connection-test';

type ReachabilityFlags = Pick<ConnectionTestResult, 'reachable' | 'authenticated' | 'hostKeyChanged'>;

/** A hub connection test answer: the nullable fields K1 always returns default to `null`. */
export function testResult(partial: Partial<ConnectionTestResult> & ReachabilityFlags): ConnectionTestResult {
  return { hostKey: null, message: null, ...partial };
}
