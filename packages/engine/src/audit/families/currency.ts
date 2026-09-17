/**
 * Currency D4: what the rate move did to everybody, against what it did to the positions.
 *
 * @spec Currency D2 Currency D2.a Currency D4 Money A2.b Audit B1 Audit B4 Law 5 Law 7 Law 19
 *
 * D4 is a closing identity and it is the only one a currency layer has: a rate is not a thing
 * anybody holds, so it can neither be created nor destroyed by moving — what it can do is change
 * what a position is worth to its holder, and every unit of that has to land on somebody's account.
 *
 * So the check is what every revaluation actually BOOKED this period against what the period's rate
 * move comes to on the positions AS THE REGISTER HOLDS THEM.
 *
 * A-10: it used to reach both halves out of the same event. `revalueForeign` computes
 * `delta = carried x (now - was)` and journals `delta`, `carried`, `was` and `now` together; this
 * read `delta` for one side and recomputed `carried x (now - was)` from the other three for the
 * other, so it was one path twice and passed in every world — a green family that had never been
 * capable of red, standing as evidence behind thirteen `done` rows (Audit A1.a: "never a read of
 * one thing against itself, which always passes").
 *
 * The positions come from the REGISTER now: the lots this holder actually has, at the carrying the
 * valuation derives from the period BEFORE (so re-reading it after the marks have run gives the
 * same number the FX step used). Only the two RATES come from the event, because the rate a period
 * OPENED at is a fact nothing else stores — `rateInForce` is answering with this period's by the
 * time the audit runs. A holding whose carrying has drifted from the copy the event took, a
 * revaluation booked twice, one booked to the wrong account, and — new here — a foreign position the
 * FX step SKIPPED ALTOGETHER now all show up.
 *
 * Law 5's other half is already the wire's: the gain and the loss here are not two sides of a flow,
 * because nothing flowed. They are one holder's position being worth more in a money nobody paid.
 */
import { sumCash, valueAt } from '../../core/measure.js';
import type { InstrumentId, PartyId } from '../../core/ids.js';
import { addTo, sub, sum, withinDust } from '../../core/num.js';
import { says } from '../../registry/facts.js';
import { REVALUATION_FX } from '../../world/facts.js';
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

export function revaluationAddsUp(): Family {
  return {
    name: 'money',
    contributor: 'currency',
    spec: 'Currency D4',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      // Law 7: the three are magnitudes in one money each (the key), kept as pieces.
      const booked = new Map<string, number>();
      const implied = new Map<string, number>();
      const magnitude = new Map<string, number>();
      /** The two rates the period moved between, which only the event that used them remembers. */
      const rates = new Map<string, { was: number; now: number; ccy: string }>();
      const key = (holder: PartyId, instrument: InstrumentId): string => `${holder}|${instrument}`;
      for (const e of view.journal.ofKindIn('revaluation.fx', view.period)) {
        const { ccy, delta, was, now } = says(e, REVALUATION_FX);
        const holder = e.subjects[0];
        const instrument = e.subjects[1];
        if (holder === undefined || instrument === undefined) continue;
        // WHAT ITS BOOK BOOKED. The other half is not read here, on purpose: it comes off the
        // register below, which is the second independent thing this family needs (Audit A1.a).
        addTo(booked, ccy, delta);
        addTo(magnitude, ccy, Math.abs(delta));
        rates.set(key(holder as PartyId, instrument as InstrumentId), { was, now, ccy });
      }
      for (const h of view.register.allHoldings()) {
        const inst = view.instruments.get(h.instrument);
        const home = view.registry.currencyOf(view.parties.get(h.holder).region);
        if (inst.ccy === home) continue;
        // D2: THE POSITION AS THE REGISTER HOLDS IT, at what the book has recognised on it. The
        // carrying is derived from the period BEFORE this one, so reading it after the marks have
        // run gives the number the rate step used — and reading it from the LOTS rather than from
        // the event's copy of it is what makes this a second path.
        const carried = sumCash(
          inst.ccy,
          h.lots.map((lot) =>
            valueAt(
              view.valuation.carryingPerUnit(inst.id, lot, view.period),
              lot.qty,
              inst.ccy,
              'carried',
            ),
          ),
          'carried',
        ).value.pieces;
        if (carried === 0) continue;
        const moved = rates.get(key(h.holder, h.instrument));
        if (moved === undefined) {
          // Currency D2, D4: a foreign position the rate step never looked at. It is not a
          // disagreement of sizes — there is no second number at all — so it is named as itself.
          const rate = view.valuation.rateInForce(inst.ccy, home, view.period);
          out.push({
            family: 'money',
            spec: 'Currency D2',
            owner: String(h.holder),
            size: carried * rate,
            unit: String(home),
            period: view.period,
            message: `${h.holder} carries ${inst.id} in ${inst.ccy} and no revaluation looked at it`,
          });
          continue;
        }
        addTo(implied, moved.ccy, carried * sub(moved.now, moved.was, 'what the rate moved by'));
        addTo(magnitude, moved.ccy, Math.abs(carried * moved.now));
      }
      for (const [ccy, total] of booked) {
        // Every currency in `booked` was put there with its pair in the other two maps, in the same
        // pass over the same events: a missing one is impossible rather than absent (no `?? 0`).
        const should = implied.get(ccy);
        const passed = magnitude.get(ccy);
        if (should === undefined || passed === undefined) continue;
        // Law 7: the dust of the two sums that made these, never a band.
        const dust = sum([total, should, passed]).dust;
        if (withinDust(total, should, dust)) continue;
        out.push({
          family: 'money',
          spec: 'Currency D4',
          owner: ccy,
          size: total - should,
          unit: ccy,
          period: view.period,
          message:
            `revaluation in ${ccy} booked ${total} where the period's rate move on the positions ` +
            `revalued comes to ${should}`,
        });
      }
      return out;
    },
  };
}
