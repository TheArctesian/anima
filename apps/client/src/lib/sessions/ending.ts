import type { SessionEndReason } from './pending';

export type { SessionEndReason } from './pending';

/** What the tunnel saw when it ended (the `TunnelState` fields `ending` reads). */
export interface TunnelClose {
  readonly closeCode: number | undefined;
  /** The last Guacamole gateway `error` status; the terminal tunnel has none. */
  readonly gatewayError?: number | undefined;
  readonly opened: boolean;
  /** The reason in the terminal tunnel's `end` message (K4 §8.4), which is the hub's own word for the close. */
  readonly endReason?: SessionEndReason;
}

/** Why a session ended, as the operator reads it. `reason` is undefined when the close alone does not tell. */
export interface SessionEnding {
  readonly reason: SessionEndReason | undefined;
  readonly opened: boolean;
  /** Plain English naming the cause; never text from the hub or the gateway (R-035, LOG-007). */
  readonly text: string;
}

/** K4 §5 close codes. */
const CLOSE = { normal: 1000, goingAway: 1001, protocolError: 1002, noStatus: 1005, internalError: 1011, hostKey: 4001, idle: 4002, serverCertificate: 4003 };

/** Gateway statuses: 0x0301 unauthorized and 0x0303 forbidden are a refused sign-in (K4 §5). */
const UNAUTHORIZED = 0x0301;
const FORBIDDEN = 0x0303;

const REASON_TEXT: Readonly<Record<SessionEndReason, string>> = {
  OPERATOR: 'You ended the session.',
  REMOTE_CLOSED: 'The device closed the session.',
  GATEWAY_ERROR: 'The hub lost the connection to the device.',
  AUTH_FAILED: 'The device refused the sign-in, or no credential is stored for it.',
  HOST_KEY_MISMATCH: "The device's host key has changed, so Anima did not connect.",
  SERVER_CERTIFICATE_MISMATCH: "The device's RDP certificate has changed, so Anima did not connect.",
  PROTOCOL_ERROR: 'The app sent something the hub does not accept, so the hub closed the session.',
  TIMEOUT: 'The session timed out because the hub heard nothing from Anima.',
  HUB_SHUTDOWN: 'The hub shut down.',
  SERVER_CERTIFICATE_MISMATCH: "The device's RDP certificate has changed, so Anima did not connect.",
};

const NOT_OPENED = 'Could not open the session.';
const LOST = 'The connection to the hub was lost.';
/** For a reason this client does not know yet (a newer hub); never the reason's own name or text. */
const UNKNOWN_REASON = 'The session ended for a reason this version of Anima does not know.';

/** The reason a close code gives by itself, before any gateway error is considered. */
const BY_CODE: ReadonlyMap<number, SessionEndReason> = new Map([
  [CLOSE.hostKey, 'HOST_KEY_MISMATCH'],
  [CLOSE.serverCertificate, 'SERVER_CERTIFICATE_MISMATCH'],
  [CLOSE.idle, 'TIMEOUT'],
  [CLOSE.goingAway, 'HUB_SHUTDOWN'],
  [CLOSE.protocolError, 'PROTOCOL_ERROR'],
]);

function gatewayReason(status: number): SessionEndReason {
  return status === UNAUTHORIZED || status === FORBIDDEN ? 'AUTH_FAILED' : 'GATEWAY_ERROR';
}

/**
 * The provisional reason for a tunnel that opened (K4 §5). The client closes the socket itself when a gateway
 * `error` arrives, so the code may then be 1000 or 1005 instead of 1011: the error status decides, except that
 * 4001 is a host key change and 4003 a certificate change even though the hub sends each after `error` 0x0303.
 */
function openedReason({ closeCode, gatewayError }: TunnelClose): SessionEndReason | undefined {
  const byCode = closeCode === undefined ? undefined : BY_CODE.get(closeCode);
  if (byCode !== undefined) return byCode;
  if (gatewayError !== undefined) return gatewayReason(gatewayError);
  if (closeCode === CLOSE.internalError) return 'GATEWAY_ERROR';
  // 1000 is OPERATOR or REMOTE_CLOSED; an operator's close also closes the tab, so an ended panel means the device.
  if (closeCode === CLOSE.normal || closeCode === CLOSE.noStatus) return 'REMOTE_CLOSED';
  return undefined;
}

/** Whether `value` is a `SessionEndReason` this client knows. */
export function isEndReason(value: unknown): value is SessionEndReason {
  return typeof value === 'string' && Object.hasOwn(REASON_TEXT, value);
}

/** The text for a known reason; the lookup is checked because the hub may send a value newer than the schema. */
function reasonText(reason: SessionEndReason): string {
  return Object.hasOwn(REASON_TEXT, reason) ? REASON_TEXT[reason] : UNKNOWN_REASON;
}

function textFor(reason: SessionEndReason | undefined, opened: boolean): string {
  if (!opened) return reason === undefined ? NOT_OPENED : `${NOT_OPENED} ${reasonText(reason)}`;
  return reason === undefined ? LOST : reasonText(reason);
}

/**
 * The provisional ending from the tunnel alone (D-4). A tunnel that never opened (a refusal before the upgrade
 * arrives as 1006 with no status) has no reason yet: "Could not open the session". The terminal tunnel's `end`
 * message names the reason itself, so it wins over the close code (K4 §8.6).
 */
export function ending(close: TunnelClose): SessionEnding {
  const reason = close.endReason ?? (close.opened ? openedReason(close) : undefined);
  return { reason, opened: close.opened, text: textFor(reason, close.opened) };
}

/** The ending once `endSession(id)` has answered: the hub's recorded `endReason` wins when there is one (D-4). */
export function recordedEnding(provisional: SessionEnding, endReason: SessionEndReason | null | undefined): SessionEnding {
  if (endReason === null || endReason === undefined) return provisional;
  return { reason: endReason, opened: provisional.opened, text: textFor(endReason, provisional.opened) };
}
