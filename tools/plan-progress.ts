/**
 * Recount the plan's completion (Appendix C: recount rather than adjust).
 *
 * THE PLAN IS `docs/IMPLEMENTATION.md` and this counts ITS sections, in the order they are written.
 * An item is `## <id>. <title>` and its steps are the `- [ ]` / `- [x]` lines under it.
 *
 * It used to walk the rows of `docs/WORKLIST.md` instead, and items 0a to 24 have no row there —
 * they live in the plan alone — so every step of every one of them was invisible: ticking all five
 * of item 0a moved the figure by nothing, which is how the defect was found (item 0a, positioned
 * at 0c). The worklist is still read, for one thing it is the one writer of: which of the items it
 * worked are `done`, so a closed item counts every step it had rather than the none its deleted
 * section shows. Coverage comes from docs/COVERAGE.md. The result is written between the markers in
 * docs/PLAN.md. Run `npm run plan:progress`; `npm run check` runs it with `--check`, which fails if
 * the block is stale.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { readCoverage } from './spec-coverage.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

/** An item as the worklist states it: its id and the state in its last cell. */
export interface WorklistItem {
  readonly id: string;
  readonly state: string;
}

export interface ItemProgress extends WorklistItem {
  readonly title: string;
  readonly steps: number;
  readonly done: number;
  /** Whether `docs/IMPLEMENTATION.md` has a section for it. */
  readonly present: boolean;
  /** The worklist's state column, read (Law 19), never inferred from whether the section is there. */
  readonly closed: boolean;
}

/**
 * The item ids on the worklist, in the order it works them (Law 10). A row is `| <id> | ...`, and
 * the state is the LAST cell: one item's prose has pipes in it.
 */
export function worklistItems(): WorklistItem[] {
  const text = readFileSync(resolve(root, 'docs', 'WORKLIST.md'), 'utf8');
  const out: WorklistItem[] = [];
  for (const line of text.split('\n')) {
    const m = /^\|\s*([A-Za-z0-9][A-Za-z0-9.]*)\s*\|/.exec(line);
    if (m?.[1] === undefined || m[1] === 'item') continue;
    const cells = line
      .trim()
      .replace(/^\||\|$/g, '')
      .split('|');
    const last = cells[cells.length - 1];
    out.push({ id: m[1], state: (last ?? '').trim() });
  }
  return out;
}

export function worklistIds(): string[] {
  return worklistItems().map((w) => w.id);
}

export interface PlanSection {
  readonly title: string;
  readonly checked: number;
  readonly unchecked: number;
}

/**
 * Law 4, Law 16: WHO SAYS HOW MANY STEPS AN OPEN ITEM HAS ALREADY CLOSED.
 *
 * CLAUDE.md's loop is *tick the steps, delete the section when the item closes* — so a step that
 * closes is ticked and a whole item that closes is deleted, and until now those were the only two
 * states this could count. An item that closes EIGHT steps and stays open has no way to say so once
 * the ticked lines are deleted: 0g had closed eight of fifteen and the generated table read "7
 * steps, 0 done, open", which is a figure that lies.
 *
 * So the plan declares it, once, in the item's own section, and this READS it (Law 19) rather than
 * inferring it from an absence. The outcomes themselves are in `docs/RECORD.md`, which is the
 * ledger; this is only the count.
 */
const CLOSED_LINE = /^\*\*Steps closed and deleted:\s*([0-9]+)\*\*/;

/** The plan's sections by item id: `## <id>. <title>` down to the next `## ` heading. */
export function planSections(text: string): Map<string, PlanSection> {
  const out = new Map<string, PlanSection>();
  let current: { id: string; title: string; checked: number; unchecked: number } | undefined;
  const close = (): void => {
    if (current !== undefined) {
      out.set(current.id, {
        title: current.title,
        checked: current.checked,
        unchecked: current.unchecked,
      });
    }
  };
  for (const line of text.split('\n')) {
    const head = /^##\s+(.*)$/.exec(line);
    if (head !== null) {
      close();
      const item = /^([0-9][0-9a-z′]*(?:\.[0-9]+)?)\.\s+(.*)$/.exec(head[1] ?? '');
      current =
        item?.[1] === undefined
          ? undefined
          : { id: item[1], title: item[2] ?? '', checked: 0, unchecked: 0 };
      continue;
    }
    if (current === undefined) continue;
    const said = CLOSED_LINE.exec(line);
    if (said?.[1] !== undefined) {
      current.checked += Number(said[1]);
      continue;
    }
    if (line.toLowerCase().startsWith('- [x]')) current.checked += 1;
    else if (line.startsWith('- [ ]')) current.unchecked += 1;
  }
  close();
  return out;
}

export function itemProgress(): ItemProgress[] {
  const sections = planSections(readFileSync(resolve(root, 'docs', 'IMPLEMENTATION.md'), 'utf8'));
  const worklist = new Map(worklistItems().map((w) => [w.id, w.state]));
  const out: ItemProgress[] = [];
  // The PLAN's own order first: every item that has a section, whether or not the worklist knows it.
  for (const [id, s] of sections) {
    const steps = s.checked + s.unchecked;
    /**
     * Law 4, Law 19: WHO SAYS AN ITEM IS CLOSED. The worklist speaks for the items it WORKED, and
     * only through `done` — that row is what tells this to count a closed item's steps, whose
     * section was deleted when it closed. It speaks for nothing else, because the two files used
     * one id for two items: the worklist's `14` was the polity and the plan's is the insurers.
     *
     * Otherwise the item's own steps say it: every one ticked IS closed, because that is what
     * closing an item means (CLAUDE.md: tick the steps, delete the section). Nothing infers it from
     * a section's absence, which is the other direction and would make an unwritten item look done.
     *
     * 12d.1: and an item WITH a section is judged by its section alone. The worklist's `12d` (the
     * test migration, closed by the owner) shares its id with the plan's `12d` (observation), and
     * the worklist's `done` was reporting four open steps as a closed item.
     */
    const closed = steps > 0 && s.unchecked === 0;
    const state = closed ? 'done' : 'open';
    out.push({ id, state, title: s.title, steps, done: closed ? steps : s.checked, present: true, closed });
  }
  // Then the worklist rows the plan has no section for: the items it worked and closed, whose
  // sections were deleted when they closed, and whose steps are in docs/RECORD.md.
  for (const [id, state] of worklist) {
    if (sections.has(id)) continue;
    out.push({ id, state, title: '', steps: 0, done: 0, present: false, closed: state === 'done' });
  }
  return out;
}

/** A finding whose stated position names something that cannot take it. */
export interface BadPosition {
  readonly finding: string;
  readonly target: string;
  readonly why: string;
}

/**
 * **A FINDING LEAVES THIS FILE ONLY BY BEING PLACED** (`CLAUDE.md`), and a placement into an item
 * that then closes without it is the silent drop that rule exists to stop.
 *
 * 21.137 found seventeen of them at once — positions pointing at 18, 18.0, 18.4, 18.5, 18a, 18a.1,
 * 16.5, 17.0, 19 and 19.9, every one closed — and the sweep that found them MISSED TWO, because it
 * read only the line a finding begins on and a position written on a continuation line was invisible
 * to it. Then the next commit broke the rule again by closing 21.113 with 21.117 pointing at it.
 * Three times, two of them by the hand doing the sweeping: a rule that can be a check should be one.
 *
 * **The operative position is the LAST one stated**, because a re-read appends: a finding keeps what
 * it said and adds what it now says, so the newest sentence is the live one and the earlier ones are
 * history. That is a convention this check makes load-bearing, which is the point of writing it down
 * here rather than in a comment beside one finding.
 */
/** Where one finding says it is placed: its id, and the operative target, if it states one. */
export interface StatedPosition {
  readonly finding: string;
  readonly target: string;
}

/**
 * The parse, apart from the resolving, so the RULE can be asserted against a fixture rather than
 * against today's plan (21.119: that is what a test of this should do, and what the deleted one
 * did not). The two things it gets right are the two that were got wrong:
 *
 * - a finding is its opening line **and the continuation lines under it**, because a position
 *   written on the fourth line is the finding's and reading only the first line lost two of them;
 * - the operative position is the **last** one stated, because a re-read appends.
 */
export function positionsIn(text: string): StatedPosition[] {
  const findings: { id: string; text: string }[] = [];
  for (const line of text.split('\n')) {
    const opens = /^- \[[ x]\] ([0-9][0-9a-zA-Z.]*)/.exec(line);
    if (opens?.[1] !== undefined) {
      findings.push({ id: opens[1], text: line });
      continue;
    }
    const last = findings[findings.length - 1];
    if (last !== undefined && /^\s+\S/.test(line)) last.text += `\n${line}`;
  }
  const out: StatedPosition[] = [];
  for (const f of findings) {
    const stated = [...f.text.matchAll(/[Pp]ositioned (?:at|with) ([0-9][0-9a-z]*(?:\.[0-9]+)?)/g)];
    const target = stated[stated.length - 1]?.[1];
    if (target !== undefined) out.push({ finding: f.id, target });
  }
  return out;
}

export function checkPositions(): BadPosition[] {
  const text = readFileSync(resolve(root, 'docs', 'IMPLEMENTATION.md'), 'utf8');
  const items = new Map(itemProgress().map((i) => [i.id, i]));
  const open = new Set(
    text
      .split('\n')
      .map((l) => /^- \[ \] ([0-9][0-9a-zA-Z.]*)/.exec(l)?.[1])
      .filter((id): id is string => id !== undefined),
  );

  const out: BadPosition[] = [];
  for (const f of positionsIn(text)) {
    const target = f.target;
    // A position naming a finding rather than an item: it must still be an open step. This is the
    // one that caught 21.117 pointing at 21.113 one commit after 21.113 closed.
    if (/^21\.[0-9]+$/.test(target) && target !== f.finding) {
      if (!open.has(target)) {
        out.push({ finding: f.finding, target, why: 'names a finding that is closed or was never written' });
      }
      continue;
    }
    // Otherwise it names an item, or a step of one: `23.3` is item 23, `18a.1` is item 18a.
    const item = target.replace(/\.[0-9]+$/, '');
    const known = items.get(item);
    if (known === undefined) {
      out.push({ finding: f.finding, target, why: `names ${item}, which neither the plan nor the worklist knows` });
    } else if (known.closed) {
      out.push({ finding: f.finding, target, why: `names ${item}, which is closed` });
    }
  }
  return out;
}

export function render(items: readonly ItemProgress[]): string {
  const total = items.reduce((s, i) => s + i.steps, 0);
  const done = items.reduce((s, i) => s + i.done, 0);
  const rows = readCoverage(resolve(root, 'docs', 'COVERAGE.md'));
  const met = rows.filter((r) => r.status === 'MET').length;
  const partial = rows.filter((r) => r.status === 'PARTIAL').length;
  const scope = rows.filter((r) => r.status === 'OUT OF SCOPE').length;
  const pct = (a: number, b: number): string => (b === 0 ? '0.0' : ((100 * a) / b).toFixed(1));
  const lines: string[] = [];
  const planned = items.filter((i) => i.present);
  const closedItems = planned.filter((i) => i.steps > 0 && i.done === i.steps).length;
  const missing = rows.filter((r) => r.status === 'MISSING').length;
  /**
   * THE WORK IS THE CLAUSES, NOT THE STEPS. A progress line counted against the plan's own sections
   * measures how much of what somebody has already written down is done, which is a number that
   * only ever flatters: three hundred clauses this world does not meet sat outside the plan
   * entirely while it read "nine items". What is left is every MISSING and every PARTIAL clause
   * (Part 4 of `docs/IMPLEMENTATION.md`, generated from the coverage table), and the steps of the
   * items somebody has broken out so far — the second is a subset of the work the first names.
   */
  const notMet = missing + partial;
  lines.push(
    `**What is left: ${notMet} clauses this world does not meet** — ${missing} MISSING, ${partial} PARTIAL ` +
      `(\`docs/IMPLEMENTATION.md\` Part 4, one line each). Of ${rows.length} clauses, ${pct(met + scope, rows.length)}% are met or out of scope.`,
  );
  lines.push(
    `**The items broken out of that so far: ${closedItems} of ${planned.length} closed** ` +
      `(${done} of ${total} steps). What has closed is in \`docs/RECORD.md\`, not here.`,
  );
  lines.push('');
  lines.push('| item | steps | done | state |');
  lines.push('|---|---|---|---|');
  for (const i of items) {
    // A worklist row that was superseded says where it went; the plan item it named is the row
    // that counts. Only a row that is neither closed nor moved and has no section is a gap.
    const moved = i.state.startsWith('moved to plan ');
    const state = i.closed
      ? 'closed'
      : moved
        ? i.state
        : !i.present
          ? 'open (no plan section)'
          : i.done === 0
            ? 'open'
            : i.done === i.steps
              ? 'closing'
              : 'in progress';
    const name = i.title === '' ? i.id : `${i.id} — ${i.title}`;
    const steps = i.present ? String(i.steps) : '—';
    const doneCol = i.present ? String(i.done) : '—';
    lines.push(`| ${name} | ${steps} | ${doneCol} | ${state} |`);
  }
  return lines.join('\n');
}

const START = '<!-- progress:start -->';
const END = '<!-- progress:end -->';

function markers(text: string): { a: number; b: number } {
  const a = text.indexOf(START);
  const b = text.indexOf(END);
  if (a < 0 || b < 0 || b < a) throw new Error('docs/PLAN.md has no progress markers');
  return { a, b };
}

export function updatePlan(): string {
  const path = resolve(root, 'docs', 'PLAN.md');
  const text = readFileSync(path, 'utf8');
  const { a, b } = markers(text);
  const block = render(itemProgress());
  const next = `${text.slice(0, a + START.length)}\n${block}\n${text.slice(b)}`;
  if (next !== text) writeFileSync(path, next);
  return block;
}

/** `--check`: fail when the block in docs/PLAN.md is stale instead of rewriting it (used by `npm run check`). */
export function checkPlan(): string {
  const dropped = checkPositions();
  if (dropped.length > 0) {
    const said = dropped.map((d) => `  ${d.finding} -> ${d.target}: ${d.why}`).join('\n');
    throw new Error(
      `a finding leaves docs/IMPLEMENTATION.md only by being PLACED, and ${dropped.length} ` +
        `${dropped.length === 1 ? 'position names' : 'positions name'} somewhere that cannot take ` +
        `it (21.137):\n${said}`,
    );
  }
  const path = resolve(root, 'docs', 'PLAN.md');
  const text = readFileSync(path, 'utf8');
  const { a, b } = markers(text);
  const block = render(itemProgress());
  const norm = (t: string): string =>
    t
      .split('\n')
      .map((l) =>
        l
          .replace(/-+/g, '-')
          .replace(/[ \t]+/g, '')
          .trim(),
      )
      .filter((l) => l.length > 0)
      .join('\n');
  if (norm(text.slice(a + START.length, b)) !== norm(block)) {
    throw new Error('docs/PLAN.md progress block is stale: run `npm run plan:progress` and commit');
  }
  return block;
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  console.log(process.argv.includes('--check') ? checkPlan() : updatePlan());
}
