/** The sign-in page path. */
export const SIGN_IN_PATH = '/sign-in';

/** The share viewer's page (D-7). */
const SHARED_PATH = '/shared';

/**
 * The routes that need no sign-in (API-003, E-3). `/shared` is also bare, so the guard never runs there; listing it
 * here too is defence in depth.
 */
const SIGNED_OUT_ROUTES = [SIGN_IN_PATH, SHARED_PATH] as const;

/**
 * The signed-out routes that render only their page: no operator hub, socket, guard or shell, so a share viewer calls
 * no operator operation (SEC-002, D-7). `/sign-in` is signed out but keeps the shell: its mutation needs the hub.
 */
const BARE_ROUTES = [SHARED_PATH] as const;

/** Whether `route` is `parent` or below it by whole segment: `/shared/x` is below `/shared`, `/sharedx` is not. */
function within(route: string, parent: string): boolean {
  return route === parent || route.startsWith(`${parent}/`);
}

/**
 * The path the router matches for `pathname`: below the `base` path and percent-decoded as SvelteKit decodes it
 * (`%25` stays, and reserved escapes such as `%2F` stay), so `/%73hared` counts as `/shared` exactly when the router
 * shows the share page there. A path the router cannot decode stays as it is; the router shows its error page for it.
 * `page.route.id` would carry the router's answer itself, but it also names layout groups, and every router stand-in
 * of the layout's tests gives a URL only; the copy is one line.
 */
export function routePath(pathname: string, base: string): string {
  const below = base !== '' && within(pathname, base) ? pathname.slice(base.length) : pathname;
  try {
    return below.split('%25').map(decodeURI).join('%25') || '/';
  } catch {
    // A malformed escape: the router refuses it too, so the undecoded text decides.
    return below || '/';
  }
}

/** Whether the route path (see `routePath`) needs no sign-in (API-003, E-3). */
export function isSignedOut(route: string): boolean {
  return SIGNED_OUT_ROUTES.some((parent) => within(route, parent));
}

/** Whether the route path (see `routePath`) renders bare, with no operator hub (SEC-002, D-7, E-3). */
export function isBare(route: string): boolean {
  return BARE_ROUTES.some((parent) => within(route, parent));
}
