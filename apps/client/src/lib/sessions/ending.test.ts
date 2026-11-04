import { describe, expect, it } from 'vitest';
import { ending, recordedEnding, type SessionEndReason, type TunnelClose } from './ending';

const opened = (closeCode: number | undefined, gatewayError?: number): TunnelClose => ({ closeCode, gatewayError, opened: true });

/** The K4 §5 table, plus the codes the client sees when it closed first after a gateway `error`. */
const table: readonly [string, TunnelClose, SessionEndReason | undefined, string][] = [
  ['1000', opened(1000), 'REMOTE_CLOSED', 'The device closed the session.'],
  ['1001', opened(1001), 'HUB_SHUTDOWN', 'The hub shut down.'],
  ['1002', opened(1002), 'PROTOCOL_ERROR', 'The hub ended the session because Anima sent an instruction it could not read.'],
  ['1011 with 0x0301', opened(1011, 0x0301), 'AUTH_FAILED', 'The device refused the sign-in.'],
  ['1011 with 0x0303', opened(1011, 0x0303), 'AUTH_FAILED', 'The device refused the sign-in.'],
  ['1011 with 0x0300', opened(1011, 0x0300), 'GATEWAY_ERROR', 'The gateway lost the connection to the device.'],
  ['1011 with 0x0308', opened(1011, 0x0308), 'GATEWAY_ERROR', 'The gateway lost the connection to the device.'],
  ['1011 with no error', opened(1011), 'GATEWAY_ERROR', 'The gateway lost the connection to the device.'],
  ['4001 with 0x0303', opened(4001, 0x0303), 'HOST_KEY_MISMATCH', "The device's host key has changed, so Anima did not connect."],
  ['4002', opened(4002), 'TIMEOUT', 'The session timed out because the hub heard nothing from Anima.'],
  ['1000 after the client closed on 0x0301', opened(1000, 0x0301), 'AUTH_FAILED', 'The device refused the sign-in.'],
  ['1005 after the client closed on 0x0300', opened(1005, 0x0300), 'GATEWAY_ERROR', 'The gateway lost the connection to the device.'],
  ['1006 after open', opened(1006), undefined, 'The connection to the hub was lost.'],
  ['no close code', opened(undefined), undefined, 'The connection to the hub was lost.'],
  ['1006 before open', { closeCode: 1006, gatewayError: undefined, opened: false }, undefined, 'Could not open the session.'],
];

describe('ending (K4 §5, D-4, RA-007)', () => {
  for (const [name, close, reason, text] of table) {
    it(`K4 §5: ${name} gives ${String(reason)}`, () => {
      expect(ending(close)).toEqual({ reason, opened: close.opened, text });
    });
  }

  it('D-4: the recorded endReason wins over the provisional one', () => {
    // 1000 alone reads as the device; the hub recorded that the operator ended it elsewhere.
    expect(recordedEnding(ending(opened(1000)), 'OPERATOR')).toEqual({ reason: 'OPERATOR', opened: true, text: 'You ended the session.' });
    // The client closed first on 0x0303, so it guessed AUTH_FAILED; the hub knows it was the host key.
    expect(recordedEnding(ending(opened(1000, 0x0303)), 'HOST_KEY_MISMATCH').reason).toBe('HOST_KEY_MISMATCH');
  });

  it('D-4: a tunnel that never opened says so, with the recorded reason', () => {
    const provisional = ending({ closeCode: 1006, gatewayError: undefined, opened: false });
    expect(recordedEnding(provisional, 'GATEWAY_ERROR').text).toBe('Could not open the session. The gateway lost the connection to the device.');
  });

  it('D-4: no recorded reason keeps the provisional ending', () => {
    const provisional = ending(opened(4002));
    expect(recordedEnding(provisional, null)).toBe(provisional);
    expect(recordedEnding(provisional, undefined)).toBe(provisional);
  });

  it('R-035: every text is fixed English and names a cause', () => {
    const reasons: SessionEndReason[] = ['OPERATOR', 'REMOTE_CLOSED', 'GATEWAY_ERROR', 'AUTH_FAILED', 'HOST_KEY_MISMATCH', 'PROTOCOL_ERROR', 'TIMEOUT', 'HUB_SHUTDOWN'];
    const texts = reasons.map((reason) => recordedEnding(ending(opened(1000)), reason).text);
    expect(new Set(texts).size).toBe(reasons.length);
    for (const text of texts) expect(text).not.toContain('Aborted');
  });
});
