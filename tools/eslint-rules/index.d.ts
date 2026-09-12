/**
 * The shape of the rules plugin, so the fixtures that test it can be typechecked like everything
 * else. The rules themselves stay plain ESM JavaScript, which is what lets ESLint load them with no
 * build step (`index.js`'s own first line); this file is the declaration that says so.
 */
import type { Rule } from 'eslint';

declare const plugin: { readonly rules: Readonly<Record<string, Rule.RuleModule>> };
export default plugin;
