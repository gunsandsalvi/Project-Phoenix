/**
 * Where a phase runs, and the check that it is not too late.
 *
 * @spec Law 10 Law 4 Clearing F1 Clearing F1.a Money G2
 *
 * A module says where it sits against the three kernel acts (`anchor`) and what it needs of the
 * period it is in (`reads`). The first is a fact only the module has — see `PhaseDecl.anchor` for
 * why it is not derivable — and the second is what makes the first CHECKABLE.
 *
 * TWO STOPS IN ITEM 0 WERE THIS, and neither of them threw where it was caused. `funds.manager`
 * anchored to `funds.strike` and was declared above it, so the anchor resolved against a phase that
 * did not exist yet. `paper.backstop` ran after the maturity it was supposed to fund, and what said
 * so was a run that ended in a default nobody had asked for. Both are a phase in front of something
 * it needs, and both are what this refuses at assembly, by name, with both positions in the message.
 *
 * WHAT IT DOES NOT DO is choose the order. It was going to (item 0a's first design), and the
 * measurement said no: of eighty-four phases, NINE read anything of the period they are in, and the
 * order this world already had satisfied all nine. Almost everything a phase needs is holdings and
 * a calendar, which every other phase also writes — so a dataflow order would have been a cycle
 * where it was not simply empty. The order is the anchor's; this is the guard on it.
 */
import { forbid } from '../core/assert.js';
import { Missing } from '../core/errors.js';
import type { Dependency, PhaseDecl, Produces } from './module.js';

/** A phase as the world runs it: named, placed, and owned by the module that declared it. */
export interface Placed {
  readonly name: string;
  readonly owner: string;
  readonly reads: readonly Dependency[];
  readonly writes: readonly Produces[];
}

/** The kernel phase every print in this world comes from (`runOne` is called there and nowhere else). */
export const PRINTS = 'markets';

/**
 * Clearing F1.a, Law 10: every `thisPeriod` read has a writer, and the writer runs first.
 *
 * A read of a kind nothing writes is a phase waiting for something this world does not produce —
 * which is not an ordering problem and cannot be fixed by moving it, so it is named as what it is.
 */
export function refuseLateReads(order: readonly Placed[]): void {
  const at = new Map<string, number>();
  order.forEach((p, i) => at.set(p.name, i));
  const writers = new Map<string, string[]>();
  for (const p of order) {
    for (const w of p.writes) {
      const held = writers.get(w.name);
      if (held === undefined) writers.set(w.name, [p.name]);
      else held.push(p.name);
    }
  }
  const where = (name: string): number => {
    const i = at.get(name);
    if (i === undefined) throw new Missing('Law 10', `phase ${name} is not in the order`, { name });
    return i;
  };
  const prints = where(PRINTS);
  for (const p of order) {
    const mine = where(p.name);
    for (const r of p.reads) {
      if (r.of !== 'thisPeriod') continue;
      if (r.kind === 'print') {
        forbid(
          prints < mine,
          'Clearing F1.a',
          `phase ${p.name} reads this period's price at ${mine}, before ${PRINTS} at ${prints}`,
        );
        continue;
      }
      const ws = writers.get(r.name);
      forbid(
        ws !== undefined,
        'Law 10',
        `phase ${p.name} needs ${r.name} of this period and no phase writes it`,
      );
      for (const w of ws) {
        if (w === p.name) continue;
        forbid(
          where(w) < mine,
          'Law 10',
          `phase ${p.name} (at ${mine}) needs ${r.name} of this period from ${w} (at ${where(w)})`,
        );
      }
    }
  }
}

/** Law 10: the phase a module anchored to, and the refusal when it is not there yet. */
export function anchorOf(decl: PhaseDecl): string {
  return 'before' in decl.anchor ? decl.anchor.before : decl.anchor.after;
}
