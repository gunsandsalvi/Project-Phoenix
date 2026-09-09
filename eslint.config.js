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
    // Where arithmetic impossibility and declared data legitimately live.
    files: ['packages/engine/src/core/num.ts'],
    rules: { 'phoenix/no-bounds': 'off', 'phoenix/no-numeric-default': 'off' },
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
    files: ['packages/engine/src/mechanisms/**/*.ts'],
    rules: { 'phoenix/no-kind-branch': 'error' },
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
