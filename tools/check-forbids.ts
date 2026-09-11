/**
 * The FORBIDs that break silently, checked over the source itself.
 *
 * Some prohibitions in this specification cannot be observed in any output. A world that broke them
 * would run, publish, print and balance, and look exactly like one that did not — which is what
 * makes them the dangerous ones (§48 C6, F2.a, E2; Appendix B). A test cannot see them either,
 * because there is nothing to assert about: the numbers would simply be somebody else's.
 *
 * So they are checked where the failure IS visible — in the code — and this file is where. It runs
 * with `npm run check`, beside the citation check, for the same reason: "a rule that can be a check
 * should be one" (CLAUDE.md), and a rule stated only in a comment is a reminder rather than a guard.
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const SRC = join(root, 'packages', 'engine', 'src');

/** One rule: what it forbids, where it applies, and the clause it enforces. */
interface Forbid {
  readonly spec: string;
  readonly why: string;
  /** Files this applies to. */
  readonly applies: (path: string) => boolean;
  readonly pattern: RegExp;
}

const FORBIDS: readonly Forbid[] = [
  {
    spec: 'Reporting C6',
    why: 'an estimate that reads the share price is a restatement of the market: it cannot disagree with it, and it makes the surprise a tautology (§44 A2.a is the same defect in ratings)',
    applies: (p) => p.includes(join('mechanisms', 'research')),
    pattern: /\bview\.print\(|\bview\.mark\(|\bprices\./,
  },
  {
    spec: 'Reporting F2.a, G1',
    why: 'the only path from a surprise to an order is a party’s own outlook; a module that read a report or an estimate directly and posted differently because of it would have written a price path, and the print would look exactly the same as one that had not',
    applies: (p) =>
      !p.includes(join('mechanisms', 'reporting')) && !p.includes(join('mechanisms', 'research')),
    pattern: /'reporting\.|'research\./,
  },
  {
    spec: 'Reporting E2, E3',
    why: 'there is no variable in this world called the market’s expectation (§46 A2.b); a party may observe the consensus as one more published statistic, but nothing may read it AS its outlook',
    applies: (p) => !p.includes(join('mechanisms', 'research')),
    pattern: /consensusOf\(/,
  },
];

function sources(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) sources(path, out);
    else if (name.endsWith('.ts')) out.push(path);
  }
  return out;
}

/** Comments say what a thing must not do; the check is about what the code does. */
function code(text: string): string {
  return text.replace(/\/\*[\s\S]*?\*\/|\/\/.*$/gm, '');
}

const files = sources(SRC);
const broken: string[] = [];
for (const rule of FORBIDS) {
  for (const path of files) {
    if (!rule.applies(path)) continue;
    if (!rule.pattern.test(code(readFileSync(path, 'utf8')))) continue;
    broken.push(`${path.slice(root.length + 1)}: [${rule.spec}] ${rule.why}`);
  }
}

if (broken.length > 0) {
  for (const line of broken) process.stderr.write(`${line}\n`);
  process.exit(1);
}
process.stdout.write(`all ${FORBIDS.length} silent FORBIDs hold over ${files.length} files\n`);
