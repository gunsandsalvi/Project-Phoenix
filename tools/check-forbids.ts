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
    spec: 'Private Equity E3, C5.a, Law 3',
    why:
      'NO EXIT AT A PRICE NOBODY PAID. An unlisted holding has a MARK and not a price (C5, C5.a: *“an unlisted mark is not a cleared price, and it must never be treated as one by the holder’s own accounts”*), and the one place the difference could be laundered is the exit: a tender or a flotation struck at what the holder carries it at would turn a belief into a realisation, book a gain nobody funded, and print exactly like a deal real money cleared. So the module that runs a change of control may not read a MARK at all. What it may read is a PRINT — what a share of this actually traded at (a buyer bids only above it, B1) — and a holder’s own LOTS, which is what it PAID. Both are money somebody moved; a mark is not.',
    applies: (p) => p.includes(join('mechanisms', 'control')),
    pattern: /\.mark\(|markPerUnit\(/,
  },
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
    spec: 'Private Equity A2.b',
    why:
      'A CALL BOUNDED BY THE INVESTOR’S SPARE CASH IS NOT AN OBLIGATION. Every other payment in this world is a budget and correctly so — a household spends what it has, a fund redeems what its cash reaches, a bank lends what its room allows — and a capital call is the one that is not. It must go to the wire for the WHOLE amount, so that an investor which cannot pay gets a REFUSED instruction (Money E1), which is the default §29 A2.b names and the only way anybody in this world ever fails one. It breaks in perfect silence: a call trimmed to what the payer happens to hold settles, balances, prints and looks exactly like one that was not — the only difference is that nobody ever defaults, and the entire clause evaporates without a single number moving. `commitment.ts` is therefore the whole call path, handed the subscription it needs rather than importing it, so that this rule has one file to be true of',
    applies: (p) => p.endsWith(join('mechanisms', 'funds', 'commitment.ts')),
    pattern: /\batMost\(|\batLeast\(|Math\.(min|max)\(/,
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
  {
    spec: 'Hedge Funds C1, Fund Shares A3',
    why:
      'A SPECULATIVE POSITION SIZED BY THE EQUITY ACCOUNT IS A POSITION NO POOL CAN EVER TAKE. A fund\u2019s equity is ZERO by construction \u2014 the holders own the assets, so assets minus liabilities is nothing (A3), and a fund with equity has mislaid somebody\u2019s money \u2014 so `view.equity()` answers zero for the one party \u00a728 C1 calls *\u201cthe natural home of the speculative side of every derivative book\u201d*. Every one of the nine contract classes read it, and every one of them therefore let a hedge fund into the book and gave it nothing to say. It breaks in perfect silence: the pool is asked, it is permitted by its mandate, it computes a conviction of zero and returns no order, and the session prints exactly as it would have. What a party has behind a position is `view.standsBehind()`, which is the equity account for anybody whose module has not said otherwise and a pool\u2019s NAV for a pool (item 13.2b)',
    applies: (p) =>
      CONTRACT_CLASSES.some((dir) => p.includes(join('mechanisms', dir))),
    pattern: /\bview\.equity\(\)/,
  },
];

/**
 * The modules that own a class of derivative, which is where a position is SIZED (item 13.2b). It
 * is written out because it is a fact about this world's modules rather than a pattern in a path,
 * and a tenth class added without a line here is a tenth class this rule does not cover.
 */
const CONTRACT_CLASSES: readonly string[] = [
  'cds',
  'commodity-futures',
  'bond-futures',
  'index-futures',
  'options',
  'irs',
  'fx-derivatives',
  'derivative-layer',
];

/**
 * Law 6, Law 12: ARITHMETIC THAT ROUNDS, OUTSIDE THE ONE PLACE THAT MAY (`core/`).
 *
 * `Math.min`, `Math.max` and `clamp` are already refused by lint. The rest of `Math` is where a
 * bound hides in plainer clothes: a `floor` is a floor, a `ceil` is a ceiling, and an `abs` is a
 * sign somebody decided not to carry. `core/` may, because that is where a tick, a piece and a day
 * count are defined and rounding to a grid is what those ARE.
 *
 * It is a RATCHET and not a ban, because 125 of them exist today across 53 files and
 * deleting them is item 21, one file at a time. The baseline is what each file has now; the check
 * fails when a file has MORE than it did, or when a file that had none acquires one. A number here
 * only ever goes down, and a file that reaches zero is deleted from the list.
 */
const ROUNDING = /\bMath\.(floor|round|ceil|abs|exp|pow|sqrt|min|max)\b/g;

const ROUNDING_BASELINE: Readonly<Record<string, number>> = {
  'audit/audit.ts': 2,
  'audit/families/currency.ts': 2,
  'audit/families/ownership.ts': 1,
  'calendar/calendar.ts': 2,
  'calendar/civil.ts': 15,
  'clearing/solver.ts': 1,
  'mechanisms/banks/data.ts': 1,
  'mechanisms/banks/dealing.ts': 1,
  'mechanisms/banks/index.ts': 4,
  'mechanisms/banks/staff.ts': 2,
  'mechanisms/bond-futures/index.ts': 1,
  'mechanisms/capital-programme/index.ts': 5,
  'mechanisms/central-bank-omo/index.ts': 2,
  'mechanisms/commodity-futures/index.ts': 1,
  'mechanisms/derivative-layer/house.ts': 1,
  'mechanisms/derivative-layer/index.ts': 2,
  'mechanisms/environment/data.ts': 2,
  'mechanisms/environment/state.ts': 1,
  'mechanisms/equity/index.ts': 1,
  'mechanisms/expectations/index.ts': 2,
  'mechanisms/external/index.ts': 1,
  'mechanisms/firms/data.ts': 2,
  'mechanisms/firms/index.ts': 4,
  'registry/capital.ts': 1,
  'mechanisms/firms/produce.ts': 1,
  'mechanisms/freight/index.ts': 3,
  'mechanisms/funds/data.ts': 2,
  'mechanisms/funds/index.ts': 2,
  'mechanisms/fx-derivatives/participants.ts': 2,
  'mechanisms/households/lifecycle.ts': 2,
  'mechanisms/index-futures/index.ts': 1,
  'mechanisms/irs/index.ts': 3,
  'mechanisms/labour/index.ts': 2,
  'mechanisms/labour/matching.ts': 2,
  'mechanisms/money-market/index.ts': 2,
  'mechanisms/money-market/resolution.ts': 1,
  'mechanisms/options/index.ts': 3,
  // 19.2: the largest-remainder count in `allot` — whole SEATS, because a seat is a person and half
  // a person does not sit. It is arithmetic about what the thing IS, like a whole piece of a good,
  // and the remainders it leaves are given out rather than dropped: the house is always full.
  'mechanisms/polity/data.ts': 1,
  'mechanisms/reporting/guidance.ts': 3,
  'mechanisms/research/index.ts': 2,
  'mechanisms/small-business/data.ts': 2,
  'mechanisms/spot-fx/arbitrage.ts': 1,
  'mechanisms/treasury/index.ts': 3,
  'prices/curve.ts': 2,
  'register/register.ts': 1,
  'register/voyages.ts': 1,
  'registry/geography.ts': 5,
  'rng/prng.ts': 1,
  'rng/spread.ts': 2,
  'seeds/foundation.ts': 1,
  'seeds/map.ts': 15,
  'world/actions.ts': 1,
  'world/world.ts': 1,
};


/**
 * Law 6: A FLOOR AT ZERO, wearing the name of the door that was built to avoid one.
 *
 * `atLeast(x, 0)` is "not less than zero" — the exact phrase Law 6 forbids — and it is harder to
 * see than a `Math.max` because `core/num.ts` is where the honest uses of both live. The seven that
 * exist say what they are in their own `why`: "nobody misses fewer payments than none", "there is
 * no period before the world began". Two of those are arithmetic impossibility and belong; the rest
 * are a number that should not have gone negative and a mechanism that is missing (item 21).
 *
 * A ratchet, like the rounding one, and for the same reason: they go one at a time.
 */
const ZERO_FLOOR = /at(?:Least|Most)\([^,]+,\s*(?:0|NO_QTY)\s*[,)]/g;

const ZERO_FLOOR_BASELINE: Readonly<Record<string, number>> = {
  'ledger/settlement.ts': 1,
  'mechanisms/options/index.ts': 1,
  'mechanisms/ratings/assess.ts': 3,
  'mechanisms/treasury/index.ts': 1,
};

/**
 * Observer A4, Corporate Credit A4 (17.0a): A MODULE READS ANOTHER PARTY'S OWN VIEW.
 *
 * `ctx.participant(x)` and `ctx.blind(x)` hand a phase the private view of ANY party — its
 * holdings, its equity, what it took in, what it owes — and a module that asks for a counterparty's
 * is reading state that party never showed it: a lender pricing a borrower's books over its
 * shoulder, an assessor grading a ledger it was never handed. What a party may know about another
 * is what was PUBLISHED or what was SHOWN to it (`disclose`, `disclosedToMe`), and the statement a
 * company prepares every quarter is what there is to show. This ratchets the count of such reads
 * per file: a site is either the module's own party deciding (which is what the door is for) or a
 * read that should be a disclosure, and the count may only fall. The table of sites is finding
 * 21.57; the baseline is the day the rule was written.
 */
const PEEK = /ctx\.(?:participant|blind)\(/g;

const PEEK_BASELINE: Readonly<Record<string, number>> = {
  'mechanisms/banks/capital.ts': 2,
  'mechanisms/banks/dealing.ts': 2,
  'mechanisms/banks/index.ts': 11,
  'mechanisms/banks/prime.ts': 2,
  'mechanisms/banks/treasury.ts': 1,
  'mechanisms/capital-programme/index.ts': 1,
  'mechanisms/commodities/index.ts': 1,
  'mechanisms/control/index.ts': 3,
  // 17.1: the ISSUER's own outlook of the rate it would float over, for the ISSUER's own decision
  // between a fixed coupon and a margin (Corporate Credit A2.c). It reads no counterparty's state,
  // which is what this ratchet is about; finding 21.57 carries the whole list.
  'mechanisms/corporate-bond/index.ts': 1,
  // 17.5: the SELLER's own published funding gap, for the SELLER's own decision whether to ship on
  // terms (Trade Credit B2, B5). It is asked through the view because the decision is taken inside
  // the kernel's own session, which declares no module's reads; it sees no counterparty's state.
  'mechanisms/trade-credit/index.ts': 1,
  'mechanisms/derivative-layer/house.ts': 2,
  'mechanisms/derivative-layer/index.ts': 2,
  'mechanisms/derivative-layer/margin.ts': 2,
  'mechanisms/equity/index.ts': 1,
  'mechanisms/estate/index.ts': 1,
  'mechanisms/firms/born.ts': 1,
  'mechanisms/firms/index.ts': 2,
  'mechanisms/firms/produce.ts': 2,
  'mechanisms/freight/index.ts': 4,
  'mechanisms/funds/commitment.ts': 1,
  'mechanisms/funds/index.ts': 2,
  'mechanisms/fx-derivatives/index.ts': 1,
  'mechanisms/households/index.ts': 1,
  'mechanisms/households/lifecycle.ts': 3,
  'mechanisms/housing/index.ts': 5,
  'mechanisms/insurers/allocate.ts': 1,
  'mechanisms/insurers/index.ts': 1,
  'mechanisms/insurers/pensions.ts': 1,
  'mechanisms/money-market/index.ts': 11,
  // 18a.1: THE CENTRAL BANK'S OWN view, for the central bank's own decision — its outlook of the
  // basket it has a mandate about. It reads no counterparty's state: the basket is public and the
  // view of it is the deciding party's, which is what §46 A2 requires of a decision at all.
  'mechanisms/money-market/policy.ts': 1,
  'mechanisms/money-market/resolution.ts': 3,
  'mechanisms/money-market/session.ts': 2,
  'mechanisms/property/index.ts': 1,
  // 17f: EACH SIDE'S OWN view, for that side's own decision — whether to break its own contract
  // (Law 2) and whether to go on with it when the term runs out (17f.3). The buyer reads what the
  // buyer expects to pay and the seller what the seller expects to get; neither is shown the
  // other's number, and the two of them reaching opposite answers out of their own outlooks is the
  // mechanism (§46 A3). What a BILATERAL negotiation has no door for yet is the offer itself — the
  // seller's price reaching the buyer is a disclosure this world cannot express, so the phase holds
  // both views at once. It is on 21.57's list and it is the honest count.
  'mechanisms/supply/index.ts': 3,
  'mechanisms/ratings/index.ts': 2,
  'mechanisms/reporting/guidance.ts': 1,
  'mechanisms/reporting/statement.ts': 1,
  'mechanisms/research/index.ts': 1,
  'mechanisms/securities-lending/index.ts': 2,
  'mechanisms/securitisation/index.ts': 2,
  'mechanisms/short-term-debt/index.ts': 2,
  'mechanisms/small-business/found.ts': 1,
  'mechanisms/small-business/index.ts': 2,
  'mechanisms/small-business/profile.ts': 3,
  'mechanisms/spot-fx/arbitrage.ts': 1,
  'mechanisms/treasury/index.ts': 2,
};

function ratchet(
  files: readonly string[],
  pattern: RegExp,
  baseline: Readonly<Record<string, number>>,
  spec: string,
  why: string,
): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const path of files) {
    const rel = path.slice(SRC.length + 1);
    // `core/` may round: a tick, a piece and a day count are defined there and rounding to a grid
    // is what they ARE. A `.d.ts` is generated output and not source at all.
    if (rel.startsWith('core/') || rel.endsWith('.d.ts')) continue;
    const n = (readFileSync(path, 'utf8').match(pattern) ?? []).length;
    const was = baseline[rel] ?? 0;
    seen.add(rel);
    if (n > was) {
      out.push(`${rel}: [${spec}] ${String(n)} where it had ${String(was)}. ${why}`);
    }
  }
  for (const rel of Object.keys(baseline)) {
    if (!seen.has(rel)) out.push(`${rel}: [${spec}] is in the baseline and no longer exists; delete its row`);
  }
  return out;
}

/**
 * Law 15, ARCHITECTURE 4.9b: A MODULE READS ANOTHER MODULE'S EVENT BY NAME.
 *
 * "A module never imports another module" is checked by lint, and this is the hole it leaves: the
 * journal is public, so `banks` reads `firms.funding` and `housing.funding` BY NAME and a third
 * borrower has to be added to that list by hand — which is why the small-business sector got no
 * bank credit at all. The coupling is the same coupling an import would be, with nothing to see it.
 *
 * A cross-module fact belongs in one of two places: a QUESTION the kernel asks the owning module
 * (`registry/questions.ts`), or a registry read both modules make (`registry/wages.ts`,
 * `registry/environment.ts`, `registry/physical.ts`). An event is a LOG — something happened, and
 * anybody may watch — and reading one to decide is reading somebody else's variable.
 *
 * IT WAS A RATCHET and the list is now EMPTY (item 0e′.3): 52 pairs across 31 event kinds, closed
 * in four passes — `registry/wages.ts`, `registry/banking.ts`, `registry/funding.ts`,
 * `registry/notices.ts`. The pair is `<reader module>:<event kind>` and the check fails on any pair
 * not listed, so with nothing listed it is a plain FORBID: a mechanism that names another module's
 * event kind fails the build. Nothing is added back — the registry read is the fix.
 */
const CROSS_MODULE_EVENT_READS: ReadonlySet<string> = new Set([
]);

/** `ctx.record('x.y')` in a module: the kinds that module WRITES. */
const WRITES = /\.record\(\s*'([\w.]+)'/g;
/** Reading a kind by name, through the journal or through a participant's view. */
const READS =
  /(?:journal|events)\.(?:ofKind|ofKindIn|lastOf|forSubject)\(\s*'([\w.]+)'|(?:lastPublic|lastOwn|lastPublicAbout|lastOwnSince)\(\s*'([\w.]+)'/g;

function moduleOf(path: string): string | undefined {
  const rel = path.slice(SRC.length + 1);
  const m = /^mechanisms\/([^/]+)\//.exec(rel);
  return m?.[1];
}

function crossModuleReads(files: readonly string[]): string[] {
  const writers = new Map<string, Set<string>>();
  const readers = new Map<string, Set<string>>();
  const into = (at: Map<string, Set<string>>, kind: string, mod: string): void => {
    const held = at.get(kind);
    if (held === undefined) at.set(kind, new Set([mod]));
    else held.add(mod);
  };
  for (const path of files) {
    const mod = moduleOf(path);
    if (mod === undefined) continue;
    const text = readFileSync(path, 'utf8');
    for (const m of text.matchAll(WRITES)) into(writers, m[1] ?? '', mod);
    for (const m of text.matchAll(READS)) into(readers, m[1] ?? m[2] ?? '', mod);
  }
  const out: string[] = [];
  const seen = new Set<string>();
  for (const [kind, mods] of [...readers].sort()) {
    const ws = writers.get(kind);
    if (ws === undefined) continue;
    for (const mod of [...mods].sort()) {
      if (ws.has(mod)) continue;
      const pair = `${mod}:${kind}`;
      seen.add(pair);
      if (CROSS_MODULE_EVENT_READS.has(pair)) continue;
      out.push(
        `mechanisms/${mod}: [Law 15] reads "${kind}", which ${[...ws].sort().join(' and ')} writes. ` +
          `A cross-module fact is a question or a registry read, never an event name (0e′.3: the baseline is empty)`,
      );
    }
  }
  for (const pair of CROSS_MODULE_EVENT_READS) {
    if (!seen.has(pair)) out.push(`${pair}: [Law 15] is in the cross-module baseline and is gone; delete its row`);
  }
  return out;
}

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
const broken: string[] = [
  ...crossModuleReads(files),
  ...ratchet(
    files,
    ROUNDING,
    ROUNDING_BASELINE,
    'Law 6',
    'A floor is a floor and a ceiling is a ceiling; build the mechanism (item 21)',
  ),
  ...ratchet(
    files,
    ZERO_FLOOR,
    ZERO_FLOOR_BASELINE,
    'Law 6',
    'atLeast(x, 0) is "not less than zero"; only arithmetic impossibility is admissible (item 21)',
  ),
  ...ratchet(
    files,
    PEEK,
    PEEK_BASELINE,
    'Observer A4',
    'a module reading another party’s own view is reading state that party never showed it; what it may know is what was published or disclosed (17.0a, finding 21.57)',
  ),
];
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
process.stdout.write(
  `all ${FORBIDS.length} silent FORBIDs hold over ${files.length} files; ` +
    `${String(Object.keys(ROUNDING_BASELINE).length)} files round outside core/ and ` +
    `${String(Object.keys(ZERO_FLOOR_BASELINE).length)} floor at zero (item 21), ` +
    `${String(CROSS_MODULE_EVENT_READS.size)} read another module's event by name (0e′.3)\n`,
);
