import type { DeviceStatus } from '../card/card';

/** The one status a device shows (DEV-006): the hub's five, and Warning, which only the precedence below produces. */
export type CombinedStatus = DeviceStatus | 'WARNING';

/** One check as the precedence reads it. A paused check is not enabled. */
export interface CheckState {
  readonly name: string;
  readonly status: DeviceStatus;
  readonly paused: boolean;
}

export type SecurityWarning = 'HOST_KEY_CHANGED' | 'CERTIFICATE_CHANGED';

/** A certificate or domain registration and the whole days left before it expires. */
export interface Expiry {
  readonly kind: 'CERTIFICATE' | 'DOMAIN';
  readonly name: string;
  readonly daysLeft: number;
}

/** Everything DEV-006 reads. The caller fills in what the hub tells it and leaves the rest empty. */
export interface StatusFacts {
  /** The device is inside a maintenance window, or its manual flag is set. */
  readonly maintenance: boolean;
  readonly checks: readonly CheckState[];
  readonly securityWarnings: readonly SecurityWarning[];
  readonly expiries: readonly Expiry[];
  /** The shortest NOT-006 threshold in days (default 7): an expiry fewer days away is a warning. */
  readonly warnWithinDays: number;
  /** The names of the metrics over their threshold. */
  readonly metricsOverThreshold: readonly string[];
}

/** Why a status applied, in words the badge's HelpTip can show as they are. */
export interface StatusReason {
  readonly status: CombinedStatus;
  readonly text: string;
}

export interface CombinedResult {
  /** The first status in the precedence that applies. */
  readonly status: CombinedStatus;
  /** Every reason that applied, in precedence order; the first belongs to `status`. */
  readonly reasons: readonly StatusReason[];
}

const WARNING_TEXT: Record<SecurityWarning, string> = {
  HOST_KEY_CHANGED: 'The SSH host key has changed',
  CERTIFICATE_CHANGED: 'The certificate has changed',
};

function enabled(facts: StatusFacts): readonly CheckState[] {
  return facts.checks.filter((check) => !check.paused);
}

function reasons(status: CombinedStatus, texts: readonly string[]): StatusReason[] {
  return texts.map((text) => ({ status, text }));
}

function days(count: number): string {
  if (count < 0) return 'has expired';
  return count === 1 ? 'expires in 1 day' : `expires in ${String(count)} days`;
}

function maintenanceReasons(facts: StatusFacts): StatusReason[] {
  return reasons('MAINTENANCE', facts.maintenance ? ['The device is in a maintenance window'] : []);
}

function downReasons(facts: StatusFacts): StatusReason[] {
  return reasons('DOWN', enabled(facts).filter((check) => check.status === 'DOWN').map((check) => `${check.name} is down`));
}

function warningReasons(facts: StatusFacts): StatusReason[] {
  const soon = facts.expiries.filter((expiry) => expiry.daysLeft < facts.warnWithinDays);
  const label = (expiry: Expiry): string => `The ${expiry.kind === 'CERTIFICATE' ? 'certificate' : 'domain'} ${expiry.name} ${days(expiry.daysLeft)}`;
  return reasons('WARNING', [...facts.securityWarnings.map((warning) => WARNING_TEXT[warning]), ...soon.map(label)]);
}

function degradedReasons(facts: StatusFacts): StatusReason[] {
  const slow = enabled(facts).filter((check) => check.status === 'DEGRADED').map((check) => `${check.name} is degraded`);
  return reasons('DEGRADED', [...slow, ...facts.metricsOverThreshold.map((metric) => `${metric} is over its threshold`)]);
}

/** Up or Unknown, which apply only when nothing above does. */
function restingReason(facts: StatusFacts): StatusReason {
  const checks = enabled(facts);
  if (checks.some((check) => check.status === 'UP')) return { status: 'UP', text: 'At least one check is up' };
  const text = checks.length === 0 && facts.checks.length > 0 ? 'Every check is paused' : 'No check has a result yet';
  return { status: 'UNKNOWN', text };
}

/**
 * DEV-006: Maintenance, Down, Warning, Degraded, Up, Unknown; the first that applies wins. Every reason that
 * applied is returned in that order, so the badge's HelpTip can list them. Pure: the same facts give the same result.
 */
export function combinedStatus(facts: StatusFacts): CombinedResult {
  const applied = [maintenanceReasons, downReasons, warningReasons, degradedReasons].flatMap((rule) => rule(facts));
  const [first] = applied;
  if (first === undefined) {
    const resting = restingReason(facts);
    return { status: resting.status, reasons: [resting] };
  }
  return { status: first.status, reasons: applied };
}
