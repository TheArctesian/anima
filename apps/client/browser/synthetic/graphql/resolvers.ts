import { GraphQLError } from 'graphql';
import { BEARER_TOKEN, COOKIE_MAX_AGE_SECONDS, HUB_ORIGIN, OPERATOR, type Caller } from '../access';
import type { Row } from '../fixtures';
import { vpnFields } from '../vpn';
import { allDevices, channelView, connection, checkView, deviceOf, eventRows, sessionRows, type HubState } from '../model';

/** What one GraphQL call brings to its resolvers, and what they hand back for the answer's headers. */
export interface CallContext {
  readonly state: HubState;
  readonly caller: Caller;
  readonly origin: string | null;
  /** The browser session token of the request's cookie, for `logout`. */
  readonly cookie: string | null;
  readonly reply: { issuedToken?: string; cleared?: boolean };
}

interface Page {
  readonly first?: number | null;
}

type Args<T> = T & Page;

function refuse(code: string, message: string): never {
  throw new GraphQLError(message, { extensions: { code } });
}

function operatorView(state: HubState): Row {
  return { id: state.fixtures.operator.id ?? '0190a000-0000-7000-8000-000000000001', username: OPERATOR.username, totpEnabled: true };
}

interface LoginInput {
  readonly username: string;
  readonly password: string;
  readonly totpCode?: string | null;
}

function checkCredentials({ username, password, totpCode }: LoginInput): void {
  const right = username === OPERATOR.username && password === OPERATOR.password && totpCode === OPERATOR.code;
  if (!right) refuse('UNAUTHENTICATED', 'The user name, password or code is not right.');
}

const expiry = (state: HubState): string => new Date(state.now().getTime() + COOKIE_MAX_AGE_SECONDS * 1000).toISOString();

const SSH = 'SSH';

function startSession(state: HubState, { input }: { input: { deviceId: string; protocol: string } }): Row {
  const device = deviceOf(state, input.deviceId);
  if (device === null) refuse('NOT_FOUND', 'The device does not exist.');
  const now = state.now();
  const sessionId = `0190a000-0000-7000-8014-${String(100 + state.started.length).padStart(12, '0')}`;
  state.started.unshift({
    id: sessionId,
    deviceId: input.deviceId,
    deviceName: device.name,
    protocol: input.protocol,
    operatorName: OPERATOR.username,
    startedAt: now.toISOString(),
    endedAt: null,
    endReason: null,
    clientAddr: '100.64.0.9',
  });
  const ticket = state.tickets.issue(sessionId, input.protocol === SSH ? '/terminal' : '/tunnel', now);
  return { sessionId, token: ticket.token, tunnelPath: ticket.tunnelPath, expiresAt: ticket.expiresAt.toISOString(), wakeSent: false };
}

function endSession(state: HubState, { id }: { id: string }): Row {
  const session = sessionRows(state).find((found) => found.id === id);
  if (session === undefined) refuse('NOT_FOUND', 'The session does not exist.');
  return { ...session, endedAt: state.now().toISOString(), endReason: session.endedAt ?? 'OPERATOR' };
}

/** The Query and Mutation fields of the synthetic hub. Anything not here answers from the schema's empty values. */
export function rootValue(state: HubState): Record<string, unknown> {
  return {
    me: () => operatorView(state),
    sites: () => state.fixtures.sites,
    tags: () => [...new Set(state.fixtures.devices.flatMap((device) => (device.tags ?? []) as string[]))],
    device: ({ id }: { id: string }) => deviceOf(state, id),
    devices: (page: Page) => connection(allDevices(state), page),
    check: ({ id }: { id: string }) => {
      const row = state.fixtures.checks.find((found) => found.id === id);
      return row === undefined ? null : checkView(state, row);
    },
    events: (page: Page) => connection(eventRows(state), page),
    sessions: ({ live, deviceId, ...page }: Args<{ live?: boolean | null; deviceId?: string | null }>) => {
      const rows = sessionRows(state).filter(
        (row) => (live !== true || row.endedAt === null) && (deviceId == null || row.deviceId === deviceId),
      );
      return connection(rows, page);
    },
    retention: () => state.fixtures.retention,
    checkDefaults: () => state.fixtures.checkDefaults,
    notificationChannels: () => state.fixtures.channels.map((row) => channelView(state, row)),
    notificationSettings: () => state.fixtures.notificationSettings,
    apiKeys: () => state.fixtures.apiKeys,
    login: ({ input }: { input: LoginInput }) => {
      checkCredentials(input);
      return { token: BEARER_TOKEN, expiresAt: expiry(state), operator: operatorView(state) };
    },
    signInBrowser: ({ input }: { input: LoginInput }, context: CallContext) => {
      if (context.origin !== HUB_ORIGIN) refuse('FORBIDDEN', 'Sign-in works only from the hub address.');
      checkCredentials(input);
      context.reply.issuedToken = state.browserSessions.issue();
      return { operator: operatorView(state), expiresAt: expiry(state) };
    },
    logout: (_args: unknown, context: CallContext) => {
      if (context.cookie !== null) state.browserSessions.revoke(context.cookie);
      context.reply.cleared = true;
      return true;
    },
    startSession: (args: { input: { deviceId: string; protocol: string } }) => startSession(state, args),
    endSession: (args: { id: string }) => endSession(state, args),
    ...vpnFields(state.now),
  };
}
