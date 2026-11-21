import { describe, expect, it } from 'vitest';
import { askGateway, ENTER_AGAIN_MS, gatewayCode, gatewayError, hubDidNotAnswer, isGatewayAnswer, leaveGateway, mayEnterGatewayNow } from './gateway';
import { fakeGateway, hubAlone } from './gateway.fixture';

const ORIGIN = 'https://hub.example.net';

describe('the answers of the gateway', () => {
  it('HMD-059: a refusal of the gateway is read by its code, and an answer of a hub is not one', () => {
    expect(gatewayCode({ error: { code: 'sign_in_needed', message: 'x' } })).toBe('sign_in_needed');
    expect(gatewayCode({ error: { code: 'hub_not_ready' } })).toBe('hub_not_ready');
    for (const body of [null, 'text', [], {}, { error: 'no' }, { error: { code: 'other' } }, { errors: [{ message: 'm' }] }, { data: {} }]) {
      expect(gatewayCode(body), JSON.stringify(body)).toBeNull();
    }
  });

  it('HMD-059: a browser with no session of the gateway is signed out, as a hub that refuses the cookie says it', () => {
    expect(gatewayError('sign_in_needed').code).toBe('UNAUTHENTICATED');
    expect(hubDidNotAnswer('sign_in_needed')).toBe(false);
  });

  it('HMD-063: a hub that starts or does not answer is not reached, in plain words', () => {
    for (const code of ['hub_not_ready', 'hub_failed'] as const) {
      expect(gatewayError(code).code).toBe('UNAVAILABLE');
      expect(hubDidNotAnswer(code)).toBe(true);
    }
    expect(gatewayError('hub_not_ready').message).toBe('Your hub is starting. Try again in a few seconds.');
    expect(gatewayError('too_many_requests').code).toBe('UNAVAILABLE');
  });

  it('only a gateway answers its unknown path with 404 and its code', () => {
    expect(isGatewayAnswer(404, { error: { code: 'no_such_route' } })).toBe(true);
    expect(isGatewayAnswer(200, { error: { code: 'no_such_route' } })).toBe(false);
    expect(isGatewayAnswer(404, { error: { code: 'sign_in_needed' } })).toBe(false);
    expect(isGatewayAnswer(404, 'Not found')).toBe(false);
  });
});

describe('the question whether a gateway is in front', () => {
  it('HMD-059: a gateway says yes, with or without a session, and the question goes to its own path only', async () => {
    for (const session of [true, false]) {
      const gateway = fakeGateway({ session });
      expect(await askGateway(gateway.send, ORIGIN)).toBe(true);
      expect(gateway.requests).toEqual([{ path: '/_door/client', method: 'GET' }]);
    }
  });

  it('a hub alone, and no answer at all, say no', async () => {
    expect(await askGateway(hubAlone, ORIGIN)).toBe(false);
    expect(await askGateway(() => Promise.reject(new TypeError('no network')), ORIGIN)).toBe(false);
  });
});

describe('the sign-out of the gateway', () => {
  it('HMD-062: it is one POST to the sign-out of the gateway, and the redirect that is not followed counts as done', async () => {
    const gateway = fakeGateway({ session: true });
    expect(await leaveGateway(gateway.send, ORIGIN)).toBe(true);
    expect(gateway.requests).toEqual([{ path: '/_door/sign-out', method: 'POST' }]);
    expect(gateway.state.session).toBe(false);
  });

  it('a refusal and no answer are not a sign-out', async () => {
    expect(await leaveGateway(() => Promise.resolve(new Response('{}', { status: 403 })), ORIGIN)).toBe(false);
    expect(await leaveGateway(() => Promise.reject(new TypeError('no network')), ORIGIN)).toBe(false);
  });
});

describe('the entry by itself', () => {
  it('HMD-063: the browser enters by itself once, and not again within a minute, so a hub that takes no session makes no loop', () => {
    expect(mayEnterGatewayNow(null, 1_000)).toBe(true);
    expect(mayEnterGatewayNow(1_000, 1_000 + ENTER_AGAIN_MS - 1)).toBe(false);
    expect(mayEnterGatewayNow(1_000, 1_000 + ENTER_AGAIN_MS)).toBe(true);
    expect(mayEnterGatewayNow(5_000, 1_000), 'a clock that went back').toBe(true);
  });
});
