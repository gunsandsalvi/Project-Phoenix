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
    for (const g of m)
      lines.push(`- [ ] \`${g.clause}\` MISSING${g.note.length > 0 ? ` — ${g.note}` : ''}`);
    for (const g of p)
      lines.push(
        `- [ ] \`${g.clause}\` PARTIAL — ${g.note.length > 0 ? g.note : 'no note in COVERAGE.md'}`,
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
