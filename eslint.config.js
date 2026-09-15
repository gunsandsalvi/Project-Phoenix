import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import phoenix from './tools/eslint-rules/index.js';

export default tseslint.config(
  {
    ignores: [
      '**/dist/**',
      '**/dist-types/**',
      '**/coverage/**',
      '**/node_modules/**',
      'packages/app/android/**',
      'playwright-report/**',
      'test-results/**',
      '**/vitest.config.ts',
      'vitest.config.ts',
      'playwright.config.ts',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: { phoenix },
    rules: {
      '@typescript-eslint/switch-exhaustiveness-check': [
        'error',
        { considerDefaultExhaustiveForUnions: false, requireDefaultForNonUnion: true },
      ],
      '@typescript-eslint/no-non-null-assertion': 'error',
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/strict-boolean-expressions': [
        'error',
        { allowString: false, allowNumber: false, allowNullableObject: false },
      ],
      '@typescript-eslint/explicit-module-boundary-types': 'error',
      '@typescript-eslint/consistent-type-imports': 'error',
      '@typescript-eslint/no-unnecessary-condition': 'error',
      '@typescript-eslint/restrict-template-expressions': ['error', { allowNumber: true }],
      'no-empty': ['error', { allowEmptyCatch: false }],
      eqeqeq: ['error', 'always'],
      'no-fallthrough': 'error',
    },
  },
  {
    // The engine: pure, deterministic, bound-free, literal-free.
    files: ['packages/engine/src/**/*.ts'],
    rules: {
      'phoenix/no-bounds': 'error',
      'phoenix/no-numeric-default': 'error',
      'phoenix/no-magic-numbers': 'error',
      'phoenix/no-clock-no-random': 'error',
      'phoenix/no-value-recipe': 'error',
      'no-console': 'error',
      'no-restricted-syntax': [
        'error',
        {
          selector: 'TryStatement',
          message:
            'The engine does not catch: a contract violation stops the run at its site (docs/ARCHITECTURE.md §5).',
        },
      ],
    },
  },
  {
    /**
     * Where arithmetic impossibility legitimately lives — `atMost` and `atLeast` ARE the admissible
     * bound, so the bound rule cannot apply to the file that defines it.
     *
     * `no-numeric-default` used to be off here too and had no reason to be: the comment justified
     * the exemption for BOUNDS and carried the other rule along with it, so the one file that owns
     * "missing is missing" was the one file where a numeric default could not be seen. It hid an
     * `acc.get(key) ?? 0` in `addTo`, the accumulator the tax and half the engine sum through
     * (`docs/AUDIT.md` item 2). A check switched off is the defect it was meant to catch.
     */
    files: ['packages/engine/src/core/num.ts'],
    rules: { 'phoenix/no-bounds': 'off' },
  },
  {
    files: [
      'packages/engine/src/core/**/*.ts',
      'packages/engine/src/registry/**/*.ts',
      'packages/engine/src/rng/**/*.ts',
      'packages/engine/src/calendar/**/*.ts',
      'packages/engine/src/world/seed*.ts',
    ],
    rules: { 'phoenix/no-magic-numbers': 'off' },
  },
  {
    files: ['packages/engine/src/mechanisms/**/*.ts', 'packages/engine/src/seeds/**/*.ts'],
    rules: { 'phoenix/no-kind-branch': 'error', 'phoenix/no-cross-module-import': 'error' },
  },
  {
    // OB5: the observer may import no module (0e′.5).
    files: ['packages/engine/src/observer/**/*.ts'],
    rules: { 'phoenix/no-cross-module-import': 'error' },
  },
  {
    // A seed states endowments: its numbers are the seed's, declared in its param list.
    files: ['packages/engine/src/seeds/**/*.ts'],
    rules: { 'phoenix/no-magic-numbers': 'off' },
  },
  {
    // Law 15: all DATA in a registry. A module's own data.ts IS its registry — declared tables and
    // nothing else — so the numbers in it are data, not behaviour smuggled into code.
    files: ['packages/engine/src/mechanisms/*/data.ts'],
    rules: { 'phoenix/no-magic-numbers': 'off' },
  },
  {
    files: ['**/*.test.ts', 'packages/app/e2e/**/*.ts', 'tools/**/*.ts', 'packages/app/**/*.ts'],
    rules: {
      'phoenix/no-magic-numbers': 'off',
      'phoenix/no-numeric-default': 'off',
      '@typescript-eslint/no-non-null-assertion': 'off',
      'no-console': 'off',
    },
  },
  {
    files: ['**/*.js'],
    extends: [tseslint.configs.disableTypeChecked],
    rules: { '@typescript-eslint/explicit-module-boundary-types': 'off' },
  },
);
