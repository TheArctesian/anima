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

// Import and call boundaries for client code (MV-008, MOT-020, CQ-045). One config object per
// rule for the same files: a second object that sets the same rule replaces the first, so the
// folders that own a library get their own object listing only the other bans.
const motionHint =
  'Use $lib/motion (MV-008, MOT-020): the motion library is imported only in apps/client/src/lib/motion/.';
const guacamoleHint =
  'Use $lib/sessions/tunnel (D-2, R-045): the vendored Guacamole client is imported only in apps/client/src/lib/sessions/tunnel/.';
const wsHint =
  'Use a domain client in $lib/<domain>/api.ts built on $lib/hub (CQ-045): graphql-ws is imported only in apps/client/src/lib/hub/.';
const transportHint =
  'Use a domain client in $lib/<domain>/api.ts built on $lib/hub (CQ-045): fetch, WebSocket, EventSource and XMLHttpRequest are used only in apps/client/src/lib/hub/.';
// A regex, not a gitignore group: 'motion/*' would also match '$lib/motion/tokens'.
const motionImports = [{ regex: String.raw`^(?:motion|motion-dom|framer-motion)(?:/|$)`, message: motionHint }];
const wsImports = [{ regex: String.raw`^graphql-ws(?:/|$)`, message: wsHint }];
// The vendored Apache Guacamole client (R-045), and the npm repackaging it replaces.
const npmGuacamoleImports = [{ regex: String.raw`^guacamole-common-js(?:/|$)`, message: guacamoleHint }];
const guacamoleImports = [...npmGuacamoleImports, { regex: String.raw`(?:^|/)tunnel/vendor(?:/|$)`, message: guacamoleHint }];
const transportGlobal = (name) => ({ name, message: transportHint });
const transportGlobals = ['fetch', 'WebSocket', 'EventSource', 'XMLHttpRequest'].map(transportGlobal);
const fetchProperties = ['window', 'globalThis', 'self'].map((object) => ({
  object,
  property: 'fetch',
  message: transportHint,
}));
const clientFiles = ['apps/client/**/*.{ts,js,svelte}'];
const clientTests = ['**/*.test.ts'];
const hubFolder = ['apps/client/src/lib/hub/**'];
const motionFolder = ['apps/client/src/lib/motion/**'];
const tunnelFolder = ['apps/client/src/lib/sessions/tunnel/**'];
const importsRule = (patterns) => ({
  'no-restricted-imports': ['error', { patterns }],
});
const boundaryRules = [
  {
    files: clientFiles,
    ignores: clientTests,
    rules: {
      ...importsRule([...motionImports, ...wsImports, ...guacamoleImports]),
      'no-restricted-globals': ['error', ...transportGlobals],
      'no-restricted-properties': ['error', ...fetchProperties],
    },
  },
  {
    files: motionFolder,
    ignores: clientTests,
    rules: importsRule([...wsImports, ...guacamoleImports]),
  },
  {
    files: hubFolder,
    ignores: clientTests,
    rules: {
      ...importsRule([...motionImports, ...guacamoleImports]),
      'no-restricted-globals': 'off',
      'no-restricted-properties': 'off',
    },
  },
  {
    // The Guacamole tunnel opens its own WebSocket (K4 §2); `socket.ts` observes its close code.
    files: tunnelFolder,
    ignores: clientTests,
    rules: {
      ...importsRule([...motionImports, ...wsImports, ...npmGuacamoleImports]),
      'no-restricted-globals': ['error', ...['fetch', 'EventSource', 'XMLHttpRequest'].map(transportGlobal)],
    },
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
      'apps/client/src/lib/hub/generated/**',
      'apps/client/src/lib/sessions/tunnel/vendor/guacamole.js',
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
  {
    files: ['**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: { parser: tseslint.parser },
    },
  },
  ...boundaryRules,
);
