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
    return {
      ImportDeclaration(node) {
        const source = String(node.source.value);
        const resolved = source.replace(/^(\.\.\/)+/, '').replace(/^\.\//, '');
        const sib = /^(mechanisms|seeds)\/([^/]+)/.exec(resolved);
        if (sib !== null && `${sib[1]}/${sib[2]}` !== own) {
          context.report({ node, messageId: 'sibling', data: { target: `${sib[1]}/${sib[2]}` } });
        }
        if (
          /world\/world\.js$/.test(resolved) &&
          !/^import type/.test(context.sourceCode.getText(node))
        ) {
          context.report({ node, messageId: 'world' });
        }
      },
    };
  },
};

pluginRules['no-cross-module-import'] = noCrossModuleImport;
