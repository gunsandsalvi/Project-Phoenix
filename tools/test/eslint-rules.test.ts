/**
 * The lint rules that enforce the laws, tested on fixtures — because a rule that reports nothing
 * looks exactly like a codebase with nothing to report.
 *
 * `no-cross-module-import` reported nothing in its whole life while six modules imported each other
 * including a cycle: it stripped the leading `../`s off a specifier and tested the remainder for a
 * `mechanisms/` prefix, which is true of exactly one spelling nobody writes (item 13b.1). These
 * fixtures are the spellings that are actually written.
 */
import { RuleTester } from 'eslint';
import tsParser from '@typescript-eslint/parser';
import { describe, it } from 'vitest';
import plugin from '../eslint-rules/index.js';

const crossModule = plugin.rules['no-cross-module-import'];
const bounds = plugin.rules['no-bounds'];

// The engine is TypeScript, and `import type` is half of what this rule has to tell apart.
function run(name: string, rule: unknown, valid: unknown[], invalid: unknown[]): void {
  const tester = new RuleTester({
    languageOptions: { parser: tsParser, ecmaVersion: 2022, sourceType: 'module' },
  });
  tester.run(name, rule as never, { valid, invalid } as never);
}

const inFirms = '/repo/packages/engine/src/mechanisms/firms/decide.ts';

describe('no-cross-module-import (ARCHITECTURE 4.9b, Law 15)', () => {
  it('reports the relative spelling a module actually writes', () => {
    run('no-cross-module-import', crossModule, [], [
      {
        code: "import { goodId } from '../goods/index.js';",
        filename: inFirms,
        errors: [{ messageId: 'sibling' }],
      },
      {
        code: "import type { BankDecl } from '../banks/data.js';",
        filename: '/repo/packages/engine/src/mechanisms/spot-fx/index.ts',
        errors: [{ messageId: 'sibling' }],
      },
      {
        code: "import { thing } from '../../mechanisms/goods/index.js';",
        filename: inFirms,
        errors: [{ messageId: 'sibling' }],
      },
      {
        code: "import { World } from '../../world/world.js';",
        filename: inFirms,
        errors: [{ messageId: 'world' }],
      },
    ]);
  });

  it('allows the kernel, its own module, and a type-only read of the world container', () => {
    run('no-cross-module-import', crossModule, [
      { code: "import { goodId } from '../../registry/physical.js';", filename: inFirms },
      { code: "import { firmParam } from './data.js';", filename: inFirms },
      { code: "import { add } from '../../core/num.js';", filename: inFirms },
      { code: "import type { World } from '../../world/world.js';", filename: inFirms },
      // A file outside a module is not a module and the rule says nothing about it.
      { code: "import { x } from '../mechanisms/goods/index.js';", filename: '/repo/packages/engine/src/world/world.ts' },
    ], []);
  });
});

describe('no-bounds (Law 6, Appendix B 22)', () => {
  it('reports a bound written as a comparison ternary, which is what Math.min is', () => {
    run('no-bounds', bounds, [], [
      { code: 'const x = a < b ? a : b;', errors: [{ messageId: 'ternary' }] },
      { code: 'const x = a > b ? a : b;', errors: [{ messageId: 'ternary' }] },
      // The two sides the other way round is the same operation.
      { code: 'const x = a < b ? b : a;', errors: [{ messageId: 'ternary' }] },
      // "Not less than zero" is the spelling Law 6 names.
      { code: 'const x = y > 0 ? y : 0;', errors: [{ messageId: 'ternary' }] },
      { code: 'const x = Math.min(a, b);', errors: [{ messageId: 'bound' }] },
      { code: 'const x = clamp(a, b);', errors: [{ messageId: 'bound' }] },
    ]);
  });

  it('says nothing about a ternary that decides something', () => {
    run('no-bounds', bounds, [
      // A choice between two different things is a decision, not a bound.
      { code: 'const x = a < b ? c : d;' },
      { code: 'const side = bid > offer ? "sell" : "buy";' },
      // The admissible case, named, with its reason.
      { code: "const x = atMost(a, b, 'it sells what it holds and no more');" },
      { code: "const x = atLeast(a, 0, 'there is no less of it than none');" },
    ], []);
  });
});
