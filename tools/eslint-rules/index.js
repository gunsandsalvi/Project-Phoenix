// Project-specific ESLint rules. Each rule names the law it enforces (docs/ARCHITECTURE.md §9).
// Plain ESM JavaScript so ESLint can load them without a build step.

/** Law 6 / App B 22 — no bound standing in for a decision. */
const noBounds = {
  meta: {
    type: 'problem',
    docs: { description: 'Law 6: no Math.min/Math.max/clamp outside core/num.ts' },
    schema: [],
    messages: {
      bound:
        'Law 6: "{{name}}" is a bound. Build the compensating mechanism; only arithmetic impossibility is admissible (core/num.ts).',
    },
  },
  create(context) {
    return {
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
        context.report({ node, messageId: 'magic', data: { value: String(node.value) } });
      },
    };
  },
};

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

export default {
  rules: {
    'no-bounds': noBounds,
    'no-numeric-default': noNumericDefault,
    'no-magic-numbers': noMagicNumbers,
    'no-kind-branch': noKindBranch,
    'no-clock-no-random': noClockNoRandom,
  },
};
