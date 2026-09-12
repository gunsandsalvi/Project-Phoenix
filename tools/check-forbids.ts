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
    spec: 'Reporting F2.a',
    why: 'the only path from a surprise to an order is a party’s own outlook (§46 C3); a module that read the SURPRISE and posted differently because of it would have written a price path, and the print would look exactly the same as one that had not. The report and the estimate are public information and G1 requires that something read them — it is the surprise that must reach a price only through somebody changing their mind',
    // Again the observer is the exception, and the same one: it shows the surprise and decides
    // nothing with it (§45 B2.a, Observer D3 — no display-only number that changes the model).
    applies: (p) =>
      !p.includes(join('mechanisms', 'research')) && !p.includes(join('src', 'observer')),
    pattern: /'research\.surprise'/,
  },
  {
    spec: 'Commodities Spot B3, Goods B4, Freight B4, Insurers B4, Law 4',
    why: 'ONE PHYSICAL EVENT HAS ONE REPRESENTATION. A storm is a producer’s lost crop, a blocked passage and every policy in the region at once; a module that drew its own hazard, frequency or severity would be a second draw for one real thing, and the two would disagree about an event they are both supposed to be about. The physical state is `mechanisms/environment`’s and crosses as a public event (13c). This is the same refusal 13h makes of `catastrophe.probability` as a primitive, seen from the other side — and it breaks silently, because a world with two hazards runs, balances and looks exactly like one with none',
    applies: (p) => !p.includes(join('mechanisms', 'environment')),
    pattern: /\b(hazard|catastrophe|disaster)(Rate|Probability|Frequency|Severity|PerPeriod)?\s*[:=(]|['"`][a-zA-Z.]*\.(hazard|catastrophe|disaster)[a-zA-Z.]*['"`]/,
  },
  {
    spec: 'Reporting E2, E3',
    why: 'there is no variable in this world called the market’s expectation (§46 A2.b); a party may observe the consensus as one more published statistic, but nothing may read it AS its outlook',
    // The OBSERVER is the one exception and it is the one §45 B2.a names: a surface decides
    // nothing, and looking at it changes nothing. E2 forbids a consensus a DECISION consults.
    applies: (p) =>
      !p.includes(join('mechanisms', 'research')) && !p.includes(join('src', 'observer')),
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
