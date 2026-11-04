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

// Token-literal gate for script and markup (TOK-003). Known edge: a string that is only
// a 3/4/6/8-digit hex-looking anchor such as '#add' or '#face' is refused; build the anchor
// from a variable or use a longer id. Prose like 'retry in 5s' also matches the duration rule;
// disable the line with a reason when that is intended. URL fragments such as
// 'https://x.io/#abc123' are refused too; build the URL from parts.
// Do not set no-restricted-syntax for apps/client files in another config object: it
// replaces this list. Extend tokenLiterals instead.
const textRules = [
  [
    String.raw`(?<![\w&])#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})(?![\w-])`,
    'a hex colour',
  ],
  [
    String.raw`\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color-mix)\(`,
    'a colour function',
  ],
  [String.raw`\bcubic-bezier\(`, 'a cubic-bezier() curve'],
  [String.raw`(?:^|[\s:,(])\d*\.?\d+m?s(?:$|[\s;,)])`, 'a CSS duration'],
];
const motionKeys = 'duration|delay|stiffness|damping|mass|bounce|visualDuration|ease|zIndex';
const numeric = String.raw`^[-+]?[\d.]`;
const tokenHint = 'Use a design token (TOK-003): import from $lib/motion/tokens or use a $lib/motion helper.';
const tokenLiterals = [
  ...textRules.flatMap(([re, what]) =>
    [
      ['Literal', 'value'],
      ['TemplateElement', 'value.raw'],
      ['SvelteLiteral', 'value'],
    ].map(([node, field]) => ({
      selector: `${node}[${field}=/${re}/i]`,
      message: `Use a design token (TOK-003): ${what} is not allowed in client code. Read the value from a CSS variable or $lib/motion/tokens.`,
    })),
  ),
  {
    selector: `Property[key.name=/^(?:${motionKeys})$/][value.type='Literal'][value.raw=/${numeric}/]`,
    message: `${tokenHint} A raw number is not allowed for this property.`,
  },
  {
    selector: `Property[key.value=/^(?:${motionKeys})$/][value.type='Literal'][value.raw=/${numeric}/]`,
    message: `${tokenHint} A raw number is not allowed for this property.`,
  },
  {
    selector: `Property[key.name='ease'] > ArrayExpression > Literal[raw=/${numeric}/]`,
    message: `${tokenHint} Easing curves come from the ease token.`,
  },
  {
    selector: `AssignmentExpression[left.property.name=/^(?:${motionKeys})$/][right.type='Literal'][right.raw=/${numeric}/]`,
    message: `${tokenHint} A raw number is not allowed for this property.`,
  },
];

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
    files: ['apps/client/**/*.{ts,js,svelte}', 'quality/**/*.ts', 'deploy/**/*.ts', 'design/**/*.ts'],
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
    files: ['apps/client/**/*.{ts,js,svelte}'],
    ignores: ['apps/client/src/lib/motion/tokens.ts', '**/*.test.ts'],
    rules: { 'no-restricted-syntax': ['error', ...tokenLiterals] },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: { parser: tseslint.parser, extraFileExtensions: ['.svelte'] },
    },
  },
);
