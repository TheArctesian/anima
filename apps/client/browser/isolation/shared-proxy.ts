import { startDenyProxy, type DenyProxy } from './deny-proxy';

// One proxy per engine and per `playwright test` run, so the check can tell which engine made an attempt. The runner's main
// process loads the config first and starts them; each worker loads the config again and finds the addresses in the
// environment it inherited.

const HANDLES = Symbol.for('anima.denyProxies');

const addressVariable = (engine: string): string => `ANIMA_DENY_PROXY_${engine.toUpperCase()}`;

function started(): Map<string, DenyProxy> {
  const known = Reflect.get(globalThis, HANDLES) as Map<string, DenyProxy> | undefined;
  if (known !== undefined) return known;
  const fresh = new Map<string, DenyProxy>();
  Reflect.set(globalThis, HANDLES, fresh);
  return fresh;
}

/** The address of `engine`'s proxy in this run, starting it in the first process that asks. */
export async function runProxyUrl(engine: string): Promise<string> {
  const inherited = process.env[addressVariable(engine)];
  if (inherited !== undefined) return inherited;
  const proxy = await startDenyProxy();
  started().set(engine, proxy);
  process.env[addressVariable(engine)] = proxy.url;
  return proxy.url;
}

/** The proxies this process started, by engine: only the runner's main process has any. */
export function startedProxies(): ReadonlyMap<string, DenyProxy> {
  return started();
}
