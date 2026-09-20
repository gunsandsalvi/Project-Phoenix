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
}

/** Every unmet requirement row; marks on explanatory NOTES belong in coverage, not the work queue. */
export function gapsIn(coverage: string, requirements: readonly Requirement[]): Gap[] {
  const out: Gap[] = [];
  const actionable = new Map(requirements.filter((r) => r.form !== 'NOTE').map((r) => [r.id, r]));
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
};

/** Generated backlog notes point at the maintained plan, not closed historical item numbers. */
function currentPlanReference(gap: Gap): string {
  const item = PLAN_ITEM_BY_SYSTEM[gap.system];
  if (item === undefined) return gap.note;
  return gap.note.replace(
    /\(item (?:0[a-z](?:\.\d+)?|2[1-5](?:[a-z]\d*|\.\d+)?)([^)]*)\)/g,
    `(item ${item}$1)`,
  );
}

/** The coverage backlog, grouped by the specification's declared systems (not execution order). */
export function render(gaps: readonly Gap[], systems: readonly string[]): string {
  const bySystem = new Map<string, Gap[]>();
  for (const g of gaps) {
    const held = bySystem.get(g.system);
    if (held === undefined) bySystem.set(g.system, [g]);
    else held.push(g);
  }
  const order = [
    ...systems.filter((s) => bySystem.has(s)),
    ...[...bySystem.keys()].filter((s) => !systems.includes(s)),
  ];
  const missing = gaps.filter((g) => g.state === 'MISSING').length;
  const partial = gaps.length - missing;
  const lines: string[] = [
    START,
    '',
    '## Part 4 — What this world does not meet',
    '',
    `**${gaps.length} clauses: ${missing} MISSING, ${partial} PARTIAL.** Generated from`,
    "`docs/COVERAGE.md` by `npm run plan:gaps`, grouped in the specification's declared system order.",
    'This is a coverage backlog, **not** the execution order: take implementation order and prerequisites',
    'from Parts 1–2. A MISSING clause is a mechanism nobody has written; a PARTIAL',
    'one is a mechanism that exists and does not yet do all the clause says, and its row says what is',
    'short. Neither is a finding — a finding is a defect in something that was built — and neither',
    'waits on a measurement: an absence is not measurable (Audit E1), which is exactly why it has to be',
    'in the list rather than in a table somebody reads later.',
    '',
    'Re-mark the row in `docs/COVERAGE.md` in the change that meets it, and re-run `npm run plan:gaps`',
    'in the same commit. Nothing here is ticked by hand.',
    '',
  ];
  for (const system of order) {
    const mine = bySystem.get(system) ?? [];
    const m = mine.filter((g) => g.state === 'MISSING');
    const p = mine.filter((g) => g.state === 'PARTIAL');
    lines.push(`### ${system} — ${m.length} missing, ${p.length} partial`, '');
    for (const g of m) {
      const note = currentPlanReference(g);
      lines.push(`- [ ] \`${g.clause}\` MISSING${note.length > 0 ? ` — ${note}` : ''}`);
    }
    for (const g of p)
      lines.push(
        `- [ ] \`${g.clause}\` PARTIAL — ${g.note.length > 0 ? currentPlanReference(g) : 'no note in COVERAGE.md'}`,
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
