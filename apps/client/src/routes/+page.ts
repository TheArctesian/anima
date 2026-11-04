import { redirect } from '@sveltejs/kit';

/*
 * A `load` redirect: the build is a static single-page app (adapter-static, ssr off), so there is
 * no server to answer with a 307; the client router follows a `load` redirect before it renders
 * anything. A `goto` in `onMount` or a `<meta refresh>` would flash the blank page first.
 */
export function load(): never {
  redirect(307, '/devices');
}
