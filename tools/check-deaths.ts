/**
 * Every scheduled death names an item that is still open.
 *
 * @spec Law 2 Law 10 Law 16
 *
 * A PLACEHOLDER is a number that stands where a mechanism should be, and Law 2 admits one only if
 * it names the item that BUILDS that mechanism and deletes it in the same change. The same rule
 * holds for a NOUN kept in a module's bag (`registry/nouns.ts`): it names the plan item that gives
 * it a kernel home. Both guards are enforced at assembly — a placeholder without a death throws,
 * a noun without a home throws.
 *
 * NEITHER GUARD ASKS WHETHER THE ITEM IS STILL OPEN, and that is the hole this closes. An item
 * closes, the placeholder it was supposed to kill is still there, and nothing anywhere says so: the
 * stand-in has become permanent while still carrying the words that say it is temporary, which is
 * exactly the state Law 2 exists to prevent.
 *
 * It is not hypothetical and it is not rare. `docs/IMPLEMENTATION.md`'s `B-14` is one finding of
 * this shape — a finding positioned into worklist 13h, 13h closed, the work was not done, and the
 * source went on naming a future that had already passed. Four placeholders were found pointing at
 * 13f and 13h, both marked done, in the same read (item 9.2b). Six of Part II's thirteen findings
 * have the same shape read from the other end.
 *
 * A rule broken this many times should be a check rather than a reminder, and this is it.
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const WORKLIST = resolve(root, 'docs', 'WORKLIST.md');
const PLAN = resolve(root, 'docs', 'IMPLEMENTATION.md');
const SRC = resolve(root, 'packages', 'engine', 'src');

/** The worklist's own table: `| 13o | … | open |`. An id with no row is not an item at all. */
function worklistStates(): Map<string, string> {
  const out = new Map<string, string>();
  for (const line of readFileSync(WORKLIST, 'utf8').split('\n')) {
    const row = /^\|\s*([0-9][0-9a-z.]*)\s*\|.*\|\s*(done|open)\s*\|\s*$/.exec(line);
    if (row === null) continue;
    const [, id, state] = row;
    if (id !== undefined && state !== undefined) out.set(id, state);
  }
  return out;
}

/**
 * The plan's items, and whether each is open. An item is a `## <id>. <title>` heading; it is CLOSED
 * when its heading says so or when the section is gone — a closed section is deleted from the plan
 * (that file is the work that is left, not a ledger), so an unknown item is a closed one.
 */
function planItems(): Set<string> {
  const out = new Set<string>();
  const text = readFileSync(PLAN, 'utf8');
  for (const line of text.split('\n')) {
    const head = /^##\s+([0-9][0-9a-z]*)\.\s+(.*)$/.exec(line);
    if (head === null) continue;
    const [, id, title] = head;
    if (id === undefined || title === undefined) continue;
    if (title.includes('**DONE**')) continue;
    out.add(id);
  }
  // A step of an open item — `9.4`, `13.8` — is open if its own line is not ticked.
  for (const line of text.split('\n')) {
    const step = /^-\s*\[( |x)\]\s+([0-9][0-9a-z]*\.[0-9a-z]+)\s/.exec(line);
    if (step === null) continue;
    const [, tick, id] = step;
    if (id === undefined) continue;
    if (tick === ' ') out.add(id);
  }
  return out;
}

interface Death {
  readonly file: string;
  readonly line: number;
  readonly kind: 'worklistItem' | 'planItem';
  readonly names: string;
}

function* sources(dir: string): Generator<string> {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) yield* sources(path);
    else if (entry.endsWith('.ts')) yield path;
  }
}

function deathsIn(file: string): Death[] {
  const out: Death[] = [];
  const lines = readFileSync(file, 'utf8').split('\n');
  lines.forEach((text, i) => {
    const w = /worklistItem:\s*'([^']+)'/.exec(text);
    if (w?.[1] !== undefined) out.push({ file, line: i + 1, kind: 'worklistItem', names: w[1] });
    const p = /planItem:\s*'([^']+)'/.exec(text);
    if (p?.[1] !== undefined) out.push({ file, line: i + 1, kind: 'planItem', names: p[1] });
  });
  return out;
}

/** `docs/IMPLEMENTATION.md item 9.9` → `9.9`. Anything else is a citation nobody can check. */
const planId = (names: string): string | undefined =>
  /^docs\/IMPLEMENTATION\.md item ([0-9][0-9a-z]*(?:\.[0-9a-z]+)?)$/.exec(names)?.[1];

function main(): void {
  const worklist = worklistStates();
  const plan = planItems();
  const bad: string[] = [];
  let seen = 0;
  for (const file of sources(SRC)) {
    for (const d of deathsIn(file)) {
      seen += 1;
      const where = `${relative(root, d.file)}:${d.line}`;
      if (d.kind === 'worklistItem') {
        const state = worklist.get(d.names);
        if (state === undefined) {
          bad.push(`${where}: names worklist item ${d.names}, which is not in docs/WORKLIST.md`);
        } else if (state !== 'open') {
          bad.push(`${where}: names worklist item ${d.names}, which is ${state}`);
        }
        continue;
      }
      const id = planId(d.names);
      if (id === undefined) {
        bad.push(`${where}: planItem is "${d.names}"; it must read "docs/IMPLEMENTATION.md item <id>"`);
      } else if (!plan.has(id)) {
        bad.push(`${where}: names plan item ${id}, which is closed or absent from docs/IMPLEMENTATION.md`);
      }
    }
  }
  if (bad.length > 0) {
    console.log('scheduled deaths that name an item nobody will do:\n');
    for (const line of bad) console.log(`  ${line}`);
    console.log(
      `\n${bad.length} of ${seen}. A stand-in whose death has passed is a permanent one wearing a` +
        ' temporary one’s words (Law 2). Point it at the item that will actually do it, or do it.',
    );
    process.exit(1);
  }
  console.log(`all ${seen} scheduled deaths name an item that is still open`);
}

main();
