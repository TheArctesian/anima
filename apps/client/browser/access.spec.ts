import { graphql } from 'graphql';
import { expect, test } from '@playwright/test';
import { OPERATOR } from './synthetic/access';
import { loadFixtures } from './synthetic/fixtures';
import { fieldFromFixtures, loadSchema } from './synthetic/graphql-schema';
import { newHubState } from './synthetic/model';
import { rootValue, type CallContext } from './synthetic/resolvers';

// The other half of the K7 cookie model, which a page cannot show: a browser always sends its own Origin. These run in Node
// against the hub's resolvers, with no browser.

const SIGN_IN = 'mutation SignIn($input: LoginInput!) { signInBrowser(input: $input) { expiresAt } }';

async function signInFrom(origin: string | null): Promise<{ code: unknown; issued: boolean }> {
  const state = newHubState(loadFixtures(), () => new Date());
  const call: CallContext = { state, caller: 'signed-out', origin, cookie: null, reply: {} };
  const result = await graphql({
    schema: loadSchema(),
    source: SIGN_IN,
    rootValue: rootValue(state),
    contextValue: call,
    variableValues: { input: { username: OPERATOR.username, password: OPERATOR.password, totpCode: OPERATOR.code } },
    fieldResolver: fieldFromFixtures,
  });
  return { code: result.errors?.[0]?.extensions?.code, issued: call.reply.issuedToken !== undefined };
}

test('CLI-001: signInBrowser from another Origin, or none, is FORBIDDEN and sets no cookie', async () => {
  for (const origin of ['https://example.net', 'null', null]) {
    expect(await signInFrom(origin)).toEqual({ code: 'FORBIDDEN', issued: false });
  }
});

test('CLI-001: signInBrowser from the hub Origin with good credentials issues the cookie', async () => {
  expect(await signInFrom('https://hub.anima.ing')).toEqual({ code: undefined, issued: true });
});
