// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
  namespace App {
    /** State a page passes to the next one with `goto(url, { state })`; it is not in the URL, so a reload does not replay it. */
    interface PageState {
      /** The hub's message when a new device was created but its credential was not stored (WP-13 H-1). Never the credential. */
      credentialError?: string;
    }
  }
}

export {};
