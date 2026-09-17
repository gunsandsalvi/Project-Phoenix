/**
 * Liveness (Part XII): something good eventually happens.
 *
 * NO CLAUSE ASKS FOR THIS FAMILY. §Audit's nine are B1–B8 and every one of them is a SAFETY
 * property; the tenth is this project's own, positioned at 0h.3, and each of its checks rests on a
 * clause that does exist — Audit E2 for a declaration nothing came of, Money A3 and XI-1 for a
 * party that is a side of nothing, XI-8 for a commitment nobody works out. Saying so here is the
 * difference between MISSING and OUT OF SCOPE (Part II): the spec does not forbid a liveness
 * property, it simply never names one, and a family citing an invented clause would hide that.
 *
 * @spec Audit A1 Audit A2 Audit A3 Audit C3 Audit C4 Audit E2 Part XII Law 10
 *
 * THE OTHER NINE FAMILIES ARE SAFETY PROPERTIES — nothing bad ever happens — and a world in which
 * NOTHING happens satisfies every one of them. That is how 428 declared capabilities that never
 * produced anything, thirteen modules that never ran, 700,000 labour hours offered against none
 * wanted and a household paid once in thirty periods all passed every gate this project has.
 *
 * A liveness property is not refutable by a finite run, so it could not be a check. A BOUNDED one
 * is: "within `horizon` periods" is a safety property again, refutable by the prefix the audit has
 * in front of it, which is why every check here carries a horizon and names it in what it reports.
 * The horizon is a RESOLUTION and is tested as one: lengthening it may only remove findings and can
 * never add one, and nothing in the world reads it, because the audit never repairs (C4).
 *
 * Audit E1 says the audit cannot find an absence — "no invariant fires because credit has no price"
 * — and that is still true and is why this is not a family about what was NEVER BUILT. What it is
 * about is a thing this world DECLARED it could do, or a party it says is alive, that has then done
 * nothing: the declaration is the standard, and it is the world's own.
 */
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';
import { sub } from '../../core/num.js';

export function livenessFamily(horizon: () => number): Family {
  return {
    name: 'liveness',
    contributor: 'kernel',
    spec: 'Part XII',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      const within = horizon();
      const v = (spec: string, owner: string, size: number, unit: string, message: string): void => {
        out.push({ family: 'liveness', spec, owner, size, unit, period: view.period, message });
      };
      /**
       * Audit E2: EVERY CAPABILITY THIS WORLD DECLARED HAS PRODUCED SOMETHING. `reach` already
       * carries the declaration, the module that made it and what has come of it; what was missing
       * was anything that FAILED when nothing had. A market is in here too, and for a market
       * "produced" means it printed a price real supply met real demand at — so "every declared
       * book has cleared" is this check and needs nothing of its own.
       */
      for (const c of view.reach) {
        if (c.produced > 0) continue;
        const silent = view.period;
        if (silent < within) continue;
        v(
          'Audit E2',
          c.owner,
          silent,
          'periods declared and never reached',
          `${c.kind} ${c.id}, declared by ${c.owner}, has produced nothing in ${silent} periods`,
        );
      }
      /**
       * Money A3, XI-1: A LIVING PARTY IS A SIDE OF SOMETHING. Everything a party does in this world
       * that reaches anybody else is a settled instruction — being paid, paying, buying, selling,
       * lending, being lent to — so a party that has been a side of none of them since it was born
       * is a party this world is carrying and not running. It is the one check that says "no cell
       * has been paid", "no firm has sold" and "no bank has lent" at once, and it says it without
       * knowing what a cell, a firm or a bank is (Law 15: nothing here branches on a kind).
       *
       * The clock starts at the party's own birth, not at the world's: a party that arrived last
       * period has not been silent for thirty.
       */
      for (const p of view.parties.alive()) {
        const born = view.parties.bornAt(p.id);
        if (!born.some) continue;
        const since = view.ledger.lastSettledFor(String(p.id));
        const from = since.some ? since.value : born.value;
        const silent = sub(view.period, from, 'periods since it was last a side of anything');
        if (silent < within) continue;
        v(
          since.some ? 'XI-1' : 'Money A3',
          String(p.id),
          silent,
          'periods',
          since.some
            ? `${p.id} has been a side of nothing that settled since period ${from}`
            : `${p.id} was born in period ${from} and has never been a side of anything that settled`,
        );
      }
      /**
       * XI-8, Money E1.b: AND A COMMITMENT IN ARREARS IS EITHER PAID, CURED OR ENDED. A row that has
       * stood breached for longer than the horizon is one nobody is working out and nobody is
       * closing — the state 21.62 named when it found that `breached` reaches only `discharged` and
       * `terminated`, with no way back. The size is how long it has stood there.
       */
      for (const row of view.agreements.all()) {
        if (row.state !== 'breached') continue;
        const silent = sub(view.period, row.since, 'periods it has stood breached');
        if (silent < within) continue;
        v(
          'XI-8',
          String(row.debtor),
          silent,
          'periods',
          `${row.terms.kind} ${row.id} has stood breached since period ${row.since} and is neither paid nor ended`,
        );
      }
      return out;
    },
  };
}
