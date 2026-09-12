// Project-specific ESLint rules. Each rule names the law it enforces (docs/ARCHITECTURE.md §9).
// Plain ESM JavaScript so ESLint can load them without a build step.

/**
 * Law 6 / App B 22 — no bound standing in for a decision.
 *
 * It used to forbid a SPELLING — `Math.min`, `Math.max`, an identifier called `clamp` — and the
 * engine wrote the same operation thirty-one times as `a < b ? a : b`, which is not a spelling this
 * rule had heard of. So a reviewer grepping for bounds found nothing (item 13b.1). The ternary is
 * forbidden here too, and the admissible case has a name: `atMost`/`atLeast` in `core/num.ts`,
 * whose third argument is the reason — the thing that is not there. A bound that cannot be written
 * as "there is no more of it" is a decision or a missing mechanism, and Law 6 says which.
 */
const noBounds = {
  meta: {
    type: 'problem',
    docs: { description: 'Law 6: no Math.min/Math.max/clamp and no comparison-ternary bound outside core/num.ts' },
    schema: [],
    messages: {
      bound:
        'Law 6: "{{name}}" is a bound. Build the compensating mechanism; only arithmetic impossibility is admissible (core/num.ts).',
      ternary:
        'Law 6: `{{code}}` is a bound written as a ternary — the same operation Math.min is. If it is arithmetic impossibility, say so: atMost/atLeast in core/num.ts take the reason as their third argument. If it is not, build the compensating mechanism.',
    },
  },
  create(context) {
    /** Both branches are the two sides of the test, either way round: that IS min or max. */
    const isBound = (node) => {
      const t = node.test;
      if (t.type !== 'BinaryExpression' || !['<', '<=', '>', '>='].includes(t.operator)) return false;
      const text = (n) => context.sourceCode.getText(n);
      const [l, r] = [text(t.left), text(t.right)];
      const [a, b] = [text(node.consequent), text(node.alternate)];
      return (a === l && b === r) || (a === r && b === l);
    };
    return {
      ConditionalExpression(node) {
        if (!isBound(node)) return;
        context.report({
          node,
          messageId: 'ternary',
          data: { code: context.sourceCode.getText(node).slice(0, 60) },
        });
      },
      CallExpression(node) {
        const callee = node.callee;
        let name = null;
        if (
          callee.type === 'MemberExpression' &&
          callee.object.type === 'Identifier' &&
          callee.object.name === 'Math' &&
          callee.property.type === 'Identifier' &&
          (callee.property.name === 'min' || callee.property.name === 'max')
        ) {
          name = `Math.${callee.property.name}`;
        }
        if (callee.type === 'Identifier' && /^clamp|^saturate|^bound(ed)?$/i.test(callee.name)) {
          name = callee.name;
        }
        if (name !== null) context.report({ node, messageId: 'bound', data: { name } });
      },
    };
  },
};

/** App A "Missing values" — a missing number is missing, never a default. */
const noNumericDefault = {
  meta: {
    type: 'problem',
    docs: { description: 'No `?? <number>`, `|| <number>` or numeric default parameters' },
    schema: [],
    messages: {
      dflt: 'Missing is missing, never {{value}}. Read it, or use the explicit try-variant and handle none.',
    },
  },
  create(context) {
    const isNumeric = (n) =>
      (n.type === 'Literal' && typeof n.value === 'number') ||
      (n.type === 'UnaryExpression' && n.operator === '-' && isNumeric(n.argument));
    const text = (n) => context.sourceCode.getText(n);
    return {
      LogicalExpression(node) {
        if ((node.operator === '??' || node.operator === '||') && isNumeric(node.right)) {
          context.report({ node, messageId: 'dflt', data: { value: text(node.right) } });
        }
      },
      AssignmentPattern(node) {
        if (isNumeric(node.right)) {
          context.report({ node, messageId: 'dflt', data: { value: text(node.right) } });
        }
      },
    };
  },
};

/** Law 2 / XI-14 — every behaviour-shaping number is declared in the parameter register. */
const noMagicNumbers = {
  meta: {
    type: 'problem',
    docs: { description: 'Numeric literals other than 0, 1, -1, 2 are registry data' },
    schema: [],
    messages: {
      magic:
        'Law 2: the literal {{value}} shapes behaviour. Declare it in the parameter register (kind, unit, owner) and read it via params.',
    },
  },
  create(context) {
    const allowed = new Set([0, 1, 2]);
    return {
      Literal(node) {
        if (typeof node.value !== 'number') return;
        if (allowed.has(node.value)) return;
        // Allow indexes into arrays/tuples (a[3]) and TS enum-like const members? No: report.
        const parent = node.parent;
        if (
          parent &&
          parent.type === 'MemberExpression' &&
          parent.property === node &&
          parent.computed
        ) {
          return;
        }
        // A parameter's own declared value IS the register (Law 2, XI-14): a ParamDecl states the
        // number with its kind, unit, owner and reason, which is exactly what this rule asks for.
        if (isParamDeclValue(parent)) return;
        context.report({ node, messageId: 'magic', data: { value: String(node.value) } });
      },
    };
  },
};

/** True when this literal is the `value` of an object literal that declares a parameter. */
function isParamDeclValue(parent) {
  if (!parent || parent.type !== 'Property' || parent.key === undefined) return false;
  const name = parent.key.name ?? parent.key.value;
  if (name !== 'value') return false;
  const object = parent.parent;
  if (!object || object.type !== 'ObjectExpression') return false;
  const keys = new Set(
    object.properties
      .filter((p) => p.type === 'Property' && p.key !== undefined)
      .map((p) => p.key.name ?? p.key.value),
  );
  return keys.has('kind') && keys.has('unit') && keys.has('owner') && keys.has('why');
}

/** Law 15 / App B 48 — mechanics never branch on a kind. */
const noKindBranch = {
  meta: {
    type: 'problem',
    docs: { description: 'No `x.kind === ...` style branches inside mechanisms' },
    schema: [],
    messages: {
      branch:
        'Law 15: a branch on "{{prop}}" is a bug report about the registry. Put the behaviour in a profile behind a dispatch table.',
    },
  },
  create(context) {
    const props = new Set([
      'kind',
      'sector',
      'industry',
      'type',
      'entityType',
      'product',
      'productId',
    ]);
    const check = (node, side) => {
      if (
        side.type === 'MemberExpression' &&
        !side.computed &&
        side.property.type === 'Identifier' &&
        props.has(side.property.name)
      ) {
        context.report({ node, messageId: 'branch', data: { prop: side.property.name } });
      }
    };
    return {
      BinaryExpression(node) {
        if (
          node.operator === '===' ||
          node.operator === '!==' ||
          node.operator === '==' ||
          node.operator === '!='
        ) {
          check(node, node.left);
          check(node, node.right);
        }
      },
      SwitchStatement(node) {
        check(node, node.discriminant);
      },
    };
  },
};

/** Seed A5 / Audit D3 — the engine has no clock and no randomness of its own. */
const noClockNoRandom = {
  meta: {
    type: 'problem',
    docs: { description: 'No Date, Math.random, performance.now in the engine' },
    schema: [],
    messages: {
      clock:
        'Reproducibility: "{{name}}" is not allowed in the engine. Use the calendar and the injected PRNG.',
    },
  },
  create(context) {
    return {
      NewExpression(node) {
        if (node.callee.type === 'Identifier' && node.callee.name === 'Date') {
          context.report({ node, messageId: 'clock', data: { name: 'new Date' } });
        }
      },
      CallExpression(node) {
        const c = node.callee;
        if (
          c.type === 'MemberExpression' &&
          c.object.type === 'Identifier' &&
          c.property.type === 'Identifier'
        ) {
          const full = `${c.object.name}.${c.property.name}`;
          if (full === 'Math.random' || full === 'performance.now' || full === 'Date.now') {
            context.report({ node, messageId: 'clock', data: { name: full } });
          }
        }
      },
    };
  },
};

const pluginRules = {
  'no-bounds': noBounds,
  'no-numeric-default': noNumericDefault,
  'no-magic-numbers': noMagicNumbers,
  'no-kind-branch': noKindBranch,
  'no-clock-no-random': noClockNoRandom,
};

export default { rules: pluginRules };

/**
 * Law 15 / docs/ARCHITECTURE.md: a module never imports another module, and never imports the
 * kernel's world container. Everything crosses through the kernel's stores and contexts.
 */
const noCrossModuleImport = {
  meta: {
    type: 'problem',
    docs: {
      description: 'A mechanism or seed module imports only the kernel, never a sibling module',
    },
    schema: [],
    messages: {
      sibling:
        'A module never imports another module ({{target}}); it reaches other systems only through the kernel state and contexts.',
      world:
        'A module never imports the world container; it works through MechanismContext, ParticipantView and SeedContext.',
    },
  },
  create(context) {
    const file = context.filename.replace(/\\/g, '/');
    const m = /\/src\/(mechanisms|seeds)\/([^/]+)\//.exec(file);
    if (m === null) return {};
    const own = `${m[1]}/${m[2]}`;
    // The importing file's own directory, so a relative specifier can be walked against it. The
    // rule used to strip the leading `../`s and test the remainder for a `mechanisms/` prefix —
    // which is true of exactly one spelling nobody writes. `'../goods/index.js'` became
    // `'goods/index.js'`, matched nothing, and the rule reported nothing in its whole life while
    // six modules imported each other, including a cycle (item 13b.1).
    const here = file.slice(0, file.lastIndexOf('/'));
    return {
      ImportDeclaration(node) {
        const source = String(node.source.value);
        if (!source.startsWith('.')) return;
        const parts = `${here}/${source}`.split('/');
        const walked = [];
        for (const part of parts) {
          if (part === '.' || part === '') continue;
          if (part === '..') walked.pop();
          else walked.push(part);
        }
        const resolved = walked.join('/');
        const sib = /\/src\/(mechanisms|seeds)\/([^/]+)/.exec(`/${resolved}`);
        if (sib !== null && `${sib[1]}/${sib[2]}` !== own) {
          context.report({ node, messageId: 'sibling', data: { target: `${sib[1]}/${sib[2]}` } });
        }
        if (
          /\/world\/world\.js$/.test(`/${resolved}`) &&
          !/^import type/.test(context.sourceCode.getText(node))
        ) {
          context.report({ node, messageId: 'world' });
        }
      },
    };
  },
};

pluginRules['no-cross-module-import'] = noCrossModuleImport;

/**
 * Goods A2.b — a recipe is a physical quantity per unit of output, never a share of cost or of
 * revenue. The defect reads as an ordinary units calculation (money needed / the input's price),
 * so it is refused by the name it has to be given: anything that calls itself a cost or value
 * share, ratio or per-revenue coefficient is that defect wherever it appears.
 */
const noValueRecipe = {
  meta: {
    type: 'problem',
    docs: { description: 'Goods A2.b: no recipe expressed as a share of cost or revenue' },
    schema: [],
    messages: {
      share:
        'Goods A2.b: "{{name}}" is a recipe in money. A price that doubles would halve the physical draw, which is the strongest substitution assumption there is, sitting where the model chose none. State the quantity in the input\'s own units.',
    },
  },
  create(context) {
    const pattern =
      /^(input|recipe|bom|material|unit)?(cost|value|spend|price)(share|ratio|perrevenue|persale|persales|percurrency)$/i;
    const report = (node, name) => {
      if (pattern.test(name)) context.report({ node, messageId: 'share', data: { name } });
    };
    return {
      Identifier(node) {
        report(node, node.name);
      },
      Property(node) {
        if (node.key.type === 'Literal' && typeof node.key.value === 'string') {
          report(node, node.key.value);
        }
      },
    };
  },
};

pluginRules['no-value-recipe'] = noValueRecipe;
