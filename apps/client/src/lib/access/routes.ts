/** The sign-in page path. */
export const SIGN_IN_PATH = '/sign-in';

/** Whether `route` is `parent` or below it by whole segment: `/sign-in/x` is below `/sign-in`, `/sign-inx` is not. */
function within(route: string, parent: string): boolean {
  return route === parent || route.startsWith(`${parent}/`);
}

/**
 * The path the router matches for `pathname`: below the `base` path and percent-decoded as SvelteKit decodes it
 * (`%25` stays, and reserved escapes such as `%2F` stay), so `/sign-%69n` counts as `/sign-in` exactly when the router
 * shows the sign-in page there. A path the router cannot decode stays as it is; the router shows its error page for it.
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

/**
 * Whether the route path (see `routePath`) needs no sign-in: only the sign-in page and below it (API-003, E-3). Every
 * other route, `/shared` included, needs sign-in (R-111).
 */
export function isSignedOut(route: string): boolean {
  return within(route, SIGN_IN_PATH);
}
