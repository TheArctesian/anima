import js from '@eslint/js';
import comments from '@eslint-community/eslint-plugin-eslint-comments';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import svelteParser from 'svelte-eslint-parser';
import tseslint from 'typescript-eslint';

const limits = {
  'max-lines-per-function': ['error', { max: 60, skipBlankLines: true, skipComments: true }],
  'max-params': ['error', 4],
  complexity: ['error', 15],
  'max-depth': ['error', 4],
};

export default tseslint.config(
  {
    ignores: [
      '**/node_modules/**',
      '**/.svelte-kit/**',
      '**/build/**',
      '**/dist/**',
      '**/target/**',
    ],
  },
  js.configs.recommended,
  tseslint.configs.strict,
  tseslint.configs.stylistic,
  svelte.configs.recommended,
  {
    files: ['apps/client/**/*.{ts,js,svelte}', 'quality/**/*.ts', 'design/**/*.ts'],
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
    plugins: { '@eslint-community/eslint-comments': comments },
    rules: {
      ...limits,
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-non-null-assertion': 'error',
      '@eslint-community/eslint-comments/require-description': 'error',
      '@eslint-community/eslint-comments/no-unlimited-disable': 'error',
    },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: { parser: tseslint.parser, extraFileExtensions: ['.svelte'] },
    },
  },
);
