import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const androidVersion = process.env.ANIMA_ANDROID_BUILD_VERSION;

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  compilerOptions: { runes: true },
  kit: {
    adapter: adapter({ fallback: 'index.html' }),
    ...(androidVersion ? { version: { name: androidVersion } } : {}),
  },
};

export default config;
