import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

// Vitest resolves the server build of svelte unless told otherwise, so runes and
// createSubscriber effects do nothing in DOM tests. The `browser` condition applies in test
// mode only; the SvelteKit build chooses its own conditions.
export default defineConfig(({ mode }) => ({
  plugins: [sveltekit()],
  resolve: mode === 'test' ? { conditions: ['browser'] } : {},
  test: { include: ['src/**/*.test.ts'] },
}));
