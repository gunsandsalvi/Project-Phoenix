/**
 * THE CLAUSES THIS WORLD DOES NOT MEET, as the plan's own list of work.
 *
 * `npm run plan:gaps` rewrites Part 4 of `docs/IMPLEMENTATION.md` from `docs/COVERAGE.md`.
 *
 * Every MISSING and every PARTIAL row is work somebody has to do, and until now none of them was in
 * the plan: the plan carried the items and the findings, and the unmet requirements this model
 * does not meet lived in a coverage table nobody takes work from. A measurement that is not in the
 * ordered list is a measurement that never gets done (Law 10).
 *
 * It is GENERATED, so it cannot go stale the way a hand-copied list would: re-mark a row in
 * COVERAGE.md and re-run this, in the same change (`check:existence` already refuses a stale Part 0).
 * It is grouped by the specification's declared systems. That makes it readable, while Parts 1–2
 * remain the dependency-ordered implementation plan.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { buildSpecIndex, type Requirement } from './spec-index.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

export const START = '<!-- plan:gaps -->';
export const END = '<!-- /plan:gaps -->';

export interface Gap {
  readonly system: string;
  readonly clause: string;
  readonly state: 'MISSING' | 'PARTIAL';
  readonly note: string;
  readonly specLine: number;
}

/** Every unmet clause row, sub-clauses included: what a sub-clause asks for is still work. */
export function gapsIn(coverage: string, requirements: readonly Requirement[]): Gap[] {
  const out: Gap[] = [];
  const actionable = new Map(requirements.map((r) => [r.id, r]));
  for (const line of coverage.split('\n')) {
    const row = /^\| `([^`]+)` \| (MISSING|PARTIAL) \|(.*)\|\s*$/.exec(line);
    if (row === null) continue;
    const clause = row[1];
    const state = row[2];
    if (clause === undefined || state === undefined) continue;
    const requirement = actionable.get(clause);
    if (requirement === undefined) continue;
    out.push({
      system: requirement.system,
      clause,
      state: state === 'MISSING' ? 'MISSING' : 'PARTIAL',
      note: (row[3] ?? '').trim(),
      specLine: requirement.line,
    });
  }
  return out;
}

const PLAN_ITEM_BY_SYSTEM: Readonly<Record<string, string>> = {
  Money: '1',
  Register: '1',
  Clearing: '1',
  Audit: '12',
  Seed: '13',
  Currency: '7',
  Bond: '4',
  Derivative: '10',
  'Corporate Credit': '4',
  Sovereign: '2',
  'Short-Term Debt': '4',
  Equity: '5',
  'Money Market': '6',
  'Spot FX': '7',
  'Fund Shares': '5',
  'Securities Lending': '10',
  'Prime Brokerage': '10',
  'Derivative Layer': '10',
  CDS: '10',
  IRS: '10',
  'FX Forwards': '10',
  'Commodity Futures': '10',
  'Commodities Spot': '3',
  Indices: '7',
  'Banks Lending': '4',
  'Banks Funding': '6',
  'Banks Capital': '6',
  'Dealer Desks': '5',
  Insurers: '6',
  'Hedge Funds': '10',
  'Private Equity': '10',
  Treasury: '2',
  'Central Bank': '9',
  Polity: '9',
  Firm: '3',
  'Capital Programme': '5',
  'Firm Birth': '11',
  'M&A': '8',
  'Trade Credit': '4',
  Goods: '3',
  Freight: '3',
  Labour: '3',
  Housing: '3',
  Households: '3',
  'Small-Business Pools': '4',
  'Cross-Border': '7',
  Ratings: '8',
  Reporting: '8',
  Observer: '14',
  Expectations: '3',
  Geography: '1',
};

/** The production surface that must be read before changing a system. */
const CODE_BY_SYSTEM: Readonly<Record<string, readonly string[]>> = {
  Money: ['instruments.rs', 'register.rs', 'ledger.rs', 'mechanisms/money.rs'],
  Register: ['register.rs', 'instruments.rs', 'parties.rs'],
  Clearing: ['clearing.rs', 'protocols.rs', 'session.rs', 'prices.rs'],
  Audit: ['audit.rs', 'assembly.rs'],
  Seed: ['opening.rs', 'assembly.rs', 'src/bin/world_runs.rs'],
  Currency: ['registry.rs', 'ledger.rs', 'mechanisms/currency.rs'],
  Bond: ['instruments.rs', 'stores.rs', 'mechanisms/lending.rs'],
  Derivative: ['stores.rs', 'mechanisms/derivative_layer.rs'],
  'Corporate Credit': ['mechanisms/corporate_credit.rs', 'mechanisms/lending.rs', 'mechanisms/loss.rs'],
  Sovereign: ['mechanisms/sovereign.rs', 'mechanisms/treasury.rs'],
  'Short-Term Debt': ['mechanisms/short_term_debt.rs'],
  Equity: ['mechanisms/equity.rs', 'mechanisms/redeemable.rs'],
  'Money Market': ['mechanisms/money_market.rs'],
  'Spot FX': ['mechanisms/spot_fx.rs', 'mechanisms/currency.rs'],
  'Fund Shares': ['mechanisms/funds.rs', 'mechanisms/redeemable.rs'],
  'Securities Lending': ['mechanisms/securities_lending.rs'],
  'Prime Brokerage': ['mechanisms/prime_brokerage.rs'],
  'Derivative Layer': ['mechanisms/derivative_layer.rs'],
  CDS: ['mechanisms/cds.rs'],
  IRS: ['mechanisms/irs.rs'],
  'FX Forwards': ['mechanisms/fx_forwards.rs'],
  'Commodity Futures': ['mechanisms/commodities.rs'],
  'Commodities Spot': ['mechanisms/commodities.rs'],
  Indices: ['mechanisms/benchmarks.rs'],
  'Banks Lending': ['mechanisms/lending.rs', 'mechanisms/corporate_credit.rs'],
  'Banks Funding': ['mechanisms/bank_funding.rs'],
  'Banks Capital': ['mechanisms/bank_capital.rs', 'mechanisms/loss.rs'],
  'Dealer Desks': ['mechanisms/dealing.rs'],
  Insurers: ['mechanisms/insurers.rs'],
  'Hedge Funds': ['mechanisms/hedge_funds.rs'],
  'Private Equity': ['mechanisms/private_equity.rs'],
  Treasury: ['mechanisms/treasury.rs'],
  'Central Bank': ['mechanisms/money.rs', 'mechanisms/bank_funding.rs'],
  Polity: ['mechanisms/polity.rs'],
  Firm: ['mechanisms/firms.rs'],
  'Capital Programme': ['mechanisms/capital_programme.rs', 'mechanisms/cost_of_capital.rs'],
  'Firm Birth': ['mechanisms/firms.rs', 'parties.rs'],
  'M&A': ['mechanisms/control.rs'],
  'Trade Credit': ['mechanisms/trade_credit.rs'],
  Goods: ['mechanisms/goods.rs'],
  Freight: ['mechanisms/freight.rs'],
  Labour: ['mechanisms/employment.rs'],
  Housing: ['mechanisms/housing.rs'],
  Households: ['mechanisms/households.rs'],
  'Small-Business Pools': ['mechanisms/small_business.rs', 'mechanisms/securitisation.rs'],
  'Cross-Border': ['mechanisms/cross_border.rs'],
  Ratings: ['mechanisms/ratings.rs', 'mechanisms/second_opinion.rs'],
  Reporting: ['mechanisms/reporting.rs'],
  Observer: ['mechanisms/observer.rs'],
  Expectations: ['mechanisms/expectations.rs'],
  Geography: ['geography.rs', 'registry.rs', 'places.rs', 'opening.rs', 'mechanisms/freight.rs'],
};

function todoId(gap: Gap): string {
  const owner = PLAN_ITEM_BY_SYSTEM[gap.system];
  if (owner === undefined) throw new Error(`no plan owner for ${gap.system}`);
  const system = gap.system.toUpperCase().replace(/[^A-Z0-9]+/g, '-');
  const node = gap.clause.slice(gap.system.length).trim().toUpperCase().replace(/[^A-Z0-9]+/g, '');
  return `${owner}.${system}.${node}`;
}

function codeFor(system: string): string {
  const files = CODE_BY_SYSTEM[system];
  if (files === undefined) throw new Error(`no code-review surface for ${system}`);
  return files.map((f) => `\`packages/kernel-rs/src/${f}\``).join(', ');
}

/** Generated backlog notes point at the maintained plan, not closed historical item numbers. */
function currentPlanReference(gap: Gap): string {
  const item = PLAN_ITEM_BY_SYSTEM[gap.system];
  if (item === undefined) return gap.note;
  return gap.note.replace(
    /\(item (?:0[a-z](?:\.\d+)?|2[1-5](?:[a-z]\d*|\.\d+)?)([^)]*)\)/g,
    `(item ${item}$1)`,
  );
}

/** Every unmet clause, ordered by its owning milestone and then by the specification. */
export function render(gaps: readonly Gap[], systems: readonly string[]): string {
  const bySystem = new Map<string, Gap[]>();
  for (const g of gaps) {
    const held = bySystem.get(g.system);
    if (held === undefined) bySystem.set(g.system, [g]);
    else held.push(g);
  }
  const specOrder = [
    ...systems.filter((s) => bySystem.has(s)),
    ...[...bySystem.keys()].filter((s) => !systems.includes(s)),
  ];
  const order = [...specOrder].sort((a, b) => {
    const stage = Number(PLAN_ITEM_BY_SYSTEM[a]) - Number(PLAN_ITEM_BY_SYSTEM[b]);
    return stage === 0 ? specOrder.indexOf(a) - specOrder.indexOf(b) : stage;
  });
  const missing = gaps.filter((g) => g.state === 'MISSING').length;
  const partial = gaps.length - missing;
  const lines: string[] = [
    START,
    '',
    '## Part 4 — Owned implementation backlog',
    '',
    `**${gaps.length} clauses: ${missing} MISSING, ${partial} PARTIAL.** Generated from`,
    '`docs/COVERAGE.md` by `npm run plan:gaps`, ordered by the Part 1 milestone that owns each clause.',
    'Every checkbox is one uniquely named to-do point and owns exactly one unmet requirement. Work',
    'top-to-bottom by milestone; within a milestone, satisfy prerequisites before dependent points.',
    'A MISSING clause is a mechanism nobody has written; a PARTIAL',
    'one is a mechanism that exists and does not yet do all the clause says, and its row says what is',
    'short. Neither is a finding — a finding is a defect in something that was built — and neither',
    'waits on a measurement: an absence is not measurable (Audit E1), which is exactly why it has to be',
    'in this executable list rather than in a table somebody reads later.',
    '',
    'Re-mark the row in `docs/COVERAGE.md` in the change that meets it, and re-run `npm run plan:gaps`',
    'in the same commit. Nothing here is ticked by hand. A point leaves this list only when its',
    'coverage row becomes MET; therefore no MISSING or PARTIAL clause can be unowned.',
    '',
  ];
  for (const system of order) {
    const mine = bySystem.get(system) ?? [];
    const m = mine.filter((g) => g.state === 'MISSING');
    const p = mine.filter((g) => g.state === 'PARTIAL');
    const owner = PLAN_ITEM_BY_SYSTEM[system];
    const firstLine = Math.min(...mine.map((g) => g.specLine));
    lines.push(
      `### ${owner}. ${system} — ${m.length} missing, ${p.length} partial`,
      '',
      `> **Required review before this block:** read the **${system}** section of \`docs/spec/PROJECT_PHOENIX.md\` ` +
        `(requirements begin at line ${firstLine}), then inspect ${codeFor(system)} and the registration in ` +
        '`packages/kernel-rs/src/systems.rs`. Re-read the relevant coverage row before each point; its note',
      '> identifies known dead code, missing production callers, and verification evidence. Do not implement',
      '> from this summary alone.',
      '',
    );
    for (const g of m) {
      const note = currentPlanReference(g);
      lines.push(`- [ ] **TODO ${todoId(g)}** — \`${g.clause}\` MISSING${note.length > 0 ? ` — ${note}` : ''}`);
    }
    for (const g of p)
      lines.push(
        `- [ ] **TODO ${todoId(g)}** — \`${g.clause}\` PARTIAL — ${g.note.length > 0 ? currentPlanReference(g) : 'no note in COVERAGE.md'}`,
      );
    lines.push('');
  }
  lines.push(END);
  return lines.join('\n');
}

export function run(): { written: number } {
  const idx = buildSpecIndex();
  const coverage = readFileSync(resolve(root, 'docs', 'COVERAGE.md'), 'utf8');
  const gaps = gapsIn(coverage, idx.requirements);
  const path = resolve(root, 'docs', 'IMPLEMENTATION.md');
  const plan = readFileSync(path, 'utf8');
  const section = render(gaps, idx.systems);
  const a = plan.indexOf(START);
  const b = plan.indexOf(END);
  const next =
    a >= 0 && b > a
      ? `${plan.slice(0, a)}${section}${plan.slice(b + END.length)}`
      : `${plan.trimEnd()}\n\n---\n\n${section}\n`;
  writeFileSync(path, next);
  return { written: gaps.length };
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const { written } = run();
  console.log(`${written} clauses this world does not meet are in docs/IMPLEMENTATION.md Part 4`);
}
