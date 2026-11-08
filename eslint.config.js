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
// replaces this list. Extend tokenLiterals (or the lists it is spread into) instead; the one
// folder object that sets it (the tunnel) spreads tokenLiterals too.
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
const xtermHint =
  'Use $lib/sessions/screen: xterm.js and its add-ons are imported only in apps/client/src/lib/sessions/screen/.';
const wsHint =
  'Use a domain client in $lib/<domain>/api.ts built on $lib/hub (CQ-045): graphql-ws is imported only in apps/client/src/lib/hub/.';
const transportHint =
  'Use a domain client in $lib/<domain>/api.ts built on $lib/hub (CQ-045): fetch, WebSocket, EventSource and XMLHttpRequest are used only in apps/client/src/lib/hub/.';
// A regex, not a gitignore group: 'motion/*' would also match '$lib/motion/tokens'.
const motionImports = [{ regex: String.raw`^(?:motion|motion-dom|framer-motion)(?:/|$)`, message: motionHint }];
const xtermImports = [{ regex: String.raw`^@xterm(?:/|$)`, message: xtermHint }];
const tauriHint = 'Use $lib/desktop (CLI-003): the Tauri API is imported only in apps/client/src/lib/desktop/.';
const tauriImports = [{ regex: String.raw`^@tauri-apps(?:/|$)`, message: tauriHint }];
const wsImports = [{ regex: String.raw`^graphql-ws(?:/|$)`, message: wsHint }];
// The vendored Apache Guacamole client (R-045), and the npm repackaging it replaces.
const npmGuacamoleImports = [{ regex: String.raw`^guacamole-common-js(?:/|$)`, message: guacamoleHint }];
// Any path that names the vendor folder or the vendored file, however it is reached.
const vendorPath = String.raw`(?:^|/)tunnel/vendor(?:/|$)|vendor/guacamole`;
const guacamoleImports = [...npmGuacamoleImports, { regex: vendorPath, message: guacamoleHint }];
// `import()` is not covered by no-restricted-imports; the tunnel imports the client statically, so a dynamic
// import of it is refused everywhere (added to the no-restricted-syntax lists below). A template is checked as
// written (`raw`) and as evaluated (`cooked`, where `\x2f` is a `/`).
// esquery ends a regex at an unescaped `/`, so the selector escapes them.
const guacamoleSelectorPath = `${vendorPath}|^guacamole-common-js(?:/|$)`.replaceAll('/', String.raw`\/`);
const guacamoleDynamicImports = [
  `ImportExpression > Literal[value=/${guacamoleSelectorPath}/]`,
  `ImportExpression > TemplateLiteral > TemplateElement[value.raw=/${guacamoleSelectorPath}/]`,
  `ImportExpression > TemplateLiteral > TemplateElement[value.cooked=/${guacamoleSelectorPath}/]`,
].map((selector) => ({ selector, message: guacamoleHint }));
// A computed `import()` (a variable, a concatenation, a template with expressions) cannot be checked against
// the boundary, so client code outside the tunnel refuses it; the client has none (D-2).
const computedImports = [
  {
    selector: "ImportExpression:not([source.type='Literal']):not([source.type='TemplateLiteral'][source.expressions.length=0])",
    message: 'Import a module by a literal path (D-2): a computed import() cannot be checked against the import boundaries.',
  },
];
const transportGlobal = (name) => ({ name, message: transportHint });
const transportGlobals = ['fetch', 'WebSocket', 'EventSource', 'XMLHttpRequest'].map(transportGlobal);
const fetchProperties = ['window', 'globalThis', 'self'].map((object) => ({
  object,
  property: 'fetch',
  message: transportHint,
}));
const clientFiles = ['apps/client/**/*.{ts,mts,cts,js,mjs,cjs,svelte}'];
const clientTests = ['**/*.test.ts'];
const hubFolder = ['apps/client/src/lib/hub/**'];
const motionFolder = ['apps/client/src/lib/motion/**'];
const tunnelFolder = ['apps/client/src/lib/sessions/tunnel/**'];
const desktopFolder = ['apps/client/src/lib/desktop/**'];
const screenFolder = ['apps/client/src/lib/sessions/screen/**'];
const importsRule = (patterns) => ({
  'no-restricted-imports': ['error', { patterns }],
});
const boundaryRules = [
  {
    files: clientFiles,
    ignores: [...clientTests, ...screenFolder],
    rules: {
      ...importsRule([...motionImports, ...wsImports, ...guacamoleImports, ...tauriImports, ...xtermImports]),
      'no-restricted-globals': ['error', ...transportGlobals],
      'no-restricted-properties': ['error', ...fetchProperties],
    },
  },
  {
    // The one folder that owns xterm.js (RA-011); every other ban of the client still applies.
    files: screenFolder,
    ignores: clientTests,
    rules: {
      ...importsRule([...motionImports, ...wsImports, ...guacamoleImports, ...tauriImports]),
      'no-restricted-globals': ['error', ...transportGlobals],
      'no-restricted-properties': ['error', ...fetchProperties],
    },
  },
  {
    files: motionFolder,
    ignores: clientTests,
    rules: importsRule([...wsImports, ...guacamoleImports, ...tauriImports, ...xtermImports]),
  },
  {
    files: desktopFolder,
    ignores: clientTests,
    rules: importsRule([...motionImports, ...wsImports, ...guacamoleImports, ...xtermImports]),
  },
  {
    files: hubFolder,
    ignores: clientTests,
    rules: {
      ...importsRule([...motionImports, ...guacamoleImports, ...tauriImports, ...xtermImports]),
      'no-restricted-globals': 'off',
      'no-restricted-properties': 'off',
    },
  },
  {
    // The Guacamole tunnel opens its own WebSocket (K4 §2); `socket.ts` observes its close code.
    files: tunnelFolder,
    ignores: clientTests,
    rules: {
      ...importsRule([...motionImports, ...wsImports, ...npmGuacamoleImports, ...tauriImports, ...xtermImports]),
      'no-restricted-globals': ['error', ...['fetch', 'EventSource', 'XMLHttpRequest'].map(transportGlobal)],
    },
  },
];

// The hub face (Q2-12, CQ1-17, WP-42 D-5): outside lib/hub, client code reaches the hub through `$lib/hub/hub` (its
// TypeScript API), `$lib/hub/context` (Svelte context), `$lib/hub/notice/LiveNotice.svelte` (a component, which a .ts
// barrel cannot re-export) and the codegen output `generated` and `generated/graphql`. Everything else under lib/hub
// is internal; `socket/fake` is a test fixture, refused outside tests and fixtures.
// The updater (UPD-001) is reached only by the lazy port in $lib/updates/desktop.svelte.ts: a static value import or
// re-export puts it in the browser build. This is the typescript-eslint rule, so it does not replace the core
// `no-restricted-imports` lists above; one object per file set, since a second object for the same files replaces the options.
const hubFaces = String.raw`(?:hub|context|generated(?:/graphql)?|notice/LiveNotice\.svelte)(?:\.ts|\.js)?`;
const hubInternalRegex = String.raw`(?:^\$lib/|^\.{1,2}/(?:.*/)?)hub/(?!(?:${hubFaces})$)(?!socket/fake(?:\.ts)?$)`;
const hubFakeRegex = String.raw`(?:^\$lib/|^\.{1,2}/(?:.*/)?)hub/socket/fake(?:\.ts)?$`;
const hubFaceHint =
  'Import from the hub face (CQ1-17): $lib/hub/hub, $lib/hub/context, $lib/hub/notice/LiveNotice.svelte or the generated documents. Everything else in apps/client/src/lib/hub/ is internal.';
const hubInternalImports = [{ regex: hubInternalRegex, message: hubFaceHint }];
const hubFakeImports = [
  { regex: hubFakeRegex, message: 'The hub socket fake is a test fixture (CQ1-17): import it only in tests and *.fixture files.' },
];
const updaterImports = [
  {
    regex: String.raw`(?:^|/)desktop/updater(?:\.ts|\.js)?$|^\./updater(?:\.ts|\.js)?$`,
    allowTypeImports: true,
    message:
      'Reach the updater through the lazy port in $lib/updates/desktop.svelte.ts (UPD-001): a static import puts it in the browser build.',
  },
];
const tsImportsRule = (patterns) => ({ '@typescript-eslint/no-restricted-imports': ['error', { patterns }] });
const fixtureFiles = ['**/*.fixture.ts', '**/*.fixture.svelte.ts'];
const faceRules = [
  {
    files: clientFiles,
    // `codegen.ts` is the entry of the hub's own type generator (API-008), a build tool outside the app code.
    ignores: [...clientTests, ...fixtureFiles, ...hubFolder, 'apps/client/codegen.ts'],
    rules: tsImportsRule([...updaterImports, ...hubInternalImports, ...hubFakeImports]),
  },
  {
    files: fixtureFiles.map((glob) => `apps/client/${glob}`),
    ignores: hubFolder,
    rules: tsImportsRule([...updaterImports, ...hubInternalImports]),
  },
  { files: hubFolder, ignores: clientTests, rules: tsImportsRule(updaterImports) },
  { files: ['apps/client/**/*.test.ts'], ignores: hubFolder, rules: tsImportsRule(hubInternalImports) },
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
    files: [...clientFiles, 'quality/**/*.ts', 'deploy/**/*.ts', 'design/**/*.ts'],
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
    files: clientFiles,
    ignores: ['apps/client/src/lib/motion/tokens.ts', '**/*.test.ts'],
    rules: { 'no-restricted-syntax': ['error', ...tokenLiterals, ...guacamoleDynamicImports, ...computedImports] },
  },
  {
    // The tunnel keeps every syntax ban but the one on computed imports (this object replaces the list above).
    files: tunnelFolder,
    ignores: ['**/*.test.ts'],
    rules: { 'no-restricted-syntax': ['error', ...tokenLiterals, ...guacamoleDynamicImports] },
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
  ...faceRules,
);
