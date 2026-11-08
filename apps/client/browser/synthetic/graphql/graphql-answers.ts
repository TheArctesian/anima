import type { BrowserContext, Route } from '@playwright/test';
import { graphql, parse, Kind, type DocumentNode, type ExecutionResult, type GraphQLSchema } from 'graphql';
import {
  callerOf,
  clearedCookie,
  COOKIE_MAX_AGE_SECONDS,
  HUB_ORIGIN,
  SESSION_COOKIE,
  sessionCookie,
  SIGNED_OUT_FIELDS,
  type RequestFacts,
} from '../access';
import { fieldFromFixtures } from './graphql-schema';
import type { HubState } from '../model';
import type { Recording } from '../recorder';
import { rootValue, type CallContext } from './resolvers';

/** What `POST /graphql` needs: the context holds the cookie jar. */
export interface GraphqlParts {
  readonly context: BrowserContext;
  readonly schema: GraphQLSchema;
  readonly state: HubState;
  readonly recording: Recording;
}

interface GraphqlBody {
  readonly query: string;
  readonly variables?: Record<string, unknown> | null;
  readonly operationName?: string | null;
}

interface Operation {
  /** The operation's name, or `null` for an unnamed one. */
  readonly name: string | null;
  /** The root fields, `__typename` aside. */
  readonly fields: string[];
}

function operationOf(document: DocumentNode, operationName: string | null | undefined): Operation {
  const operations = document.definitions.filter((definition) => definition.kind === Kind.OPERATION_DEFINITION);
  const operation = operations.find((found) => operationName == null || found.name?.value === operationName) ?? operations[0];
  const fields = operation?.selectionSet.selections.flatMap((selection) => (selection.kind === Kind.FIELD ? [selection.name.value] : [])) ?? [];
  return { name: operation?.name?.value ?? null, fields: fields.filter((name) => name !== '__typename') };
}

/** A signed-out request may run exactly one of `login`, `signInBrowser` and `joinShare` (K7). */
function allowedSignedOut(fields: readonly string[]): boolean {
  return fields.length === 1 && SIGNED_OUT_FIELDS.includes(fields[0] ?? '');
}

function unauthenticated(): ExecutionResult {
  return { data: null, errors: [{ message: 'Sign in first.', extensions: { code: 'UNAUTHENTICATED' } }] } as unknown as ExecutionResult;
}

export async function requestFacts(context: BrowserContext, route: Route): Promise<RequestFacts> {
  const headers = await route.request().allHeaders();
  const jar = await context.cookies(HUB_ORIGIN);
  return {
    origin: headers.origin ?? null,
    authorization: headers.authorization ?? null,
    cookie: jar.find((found) => found.name === SESSION_COOKIE)?.value ?? null,
  };
}

/** Keeps the browser's cookie jar in step with what the hub says; the header is sent too, as the real hub does. */
async function applyCookieEffects(context: BrowserContext, reply: CallContext['reply'], now: Date): Promise<Record<string, string>> {
  if (reply.issuedToken !== undefined) {
    const expires = Math.floor(now.getTime() / 1000) + COOKIE_MAX_AGE_SECONDS;
    await context.addCookies([
      { name: SESSION_COOKIE, value: reply.issuedToken, url: HUB_ORIGIN, httpOnly: true, secure: true, sameSite: 'Strict', expires },
    ]);
    return { 'set-cookie': sessionCookie(reply.issuedToken) };
  }
  if (reply.cleared === true) {
    await context.clearCookies({ name: SESSION_COOKIE });
    return { 'set-cookie': clearedCookie() };
  }
  return {};
}

async function run(parts: GraphqlParts, body: GraphqlBody, call: CallContext): Promise<ExecutionResult> {
  const result = await graphql({
    schema: parts.schema,
    source: body.query,
    rootValue: rootValue(parts.state),
    contextValue: call,
    variableValues: body.variables ?? undefined,
    operationName: body.operationName ?? undefined,
    fieldResolver: fieldFromFixtures,
  });
  return result as ExecutionResult;
}

/** Answers one `POST /graphql` in the shape `lib/hub/request.ts` reads: a JSON body with `data` and `errors`. */
export async function answerGraphql(parts: GraphqlParts, route: Route): Promise<void> {
  const facts = await requestFacts(parts.context, route);
  const caller = callerOf(facts, parts.state.browserSessions);
  const body = JSON.parse(route.request().postData() ?? '{}') as GraphqlBody;
  const document = parse(body.query);
  const { name, fields } = operationOf(document, body.operationName);
  parts.recording.record({
    channel: 'graphql',
    operation: name ?? fields[0] ?? null,
    variables: body.variables ?? null,
    hadAuthorization: facts.authorization !== null,
    hadCookie: facts.cookie !== null,
    origin: facts.origin,
    signedIn: caller !== 'signed-out',
  });
  const call: CallContext = { state: parts.state, caller, origin: facts.origin, cookie: facts.cookie, reply: {} };
  const result = caller === 'signed-out' && !allowedSignedOut(fields) ? unauthenticated() : await run(parts, body, call);
  const headers = await applyCookieEffects(parts.context, call.reply, parts.state.now());
  await route.fulfill({ status: 200, contentType: 'application/graphql-response+json', headers, body: JSON.stringify(result) });
}
