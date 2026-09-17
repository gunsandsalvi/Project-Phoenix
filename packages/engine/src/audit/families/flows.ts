/**
 * Flows are complete (Audit B7, Money D3): for every (holder, instrument), the ledger's deltas over
 * the period equal the change in the holding; for every instrument, issued deltas equal the change
 * in issued. A book that changed with nothing behind it is Money D4's defect.
 *
 * @spec Audit B7 Money D1.a Money D1.b Money D3 Money D4 Register E4 Register E5 Register F3 Equity D4 Law 7
 *
 * The two records are not two readings of one number, and Law 7's dust is what the arithmetic did.
 * The balance was carried from one end of the period to the other by applying these legs ONE AT A
 * TIME — every application rounding at the magnitude the balance passed through, not at the size of
 * the leg — and each end of it was itself read as a sum over the lots it is held in. That walk is
 * the tolerance. Derived any smaller (as if two readings of one balance), a busy account reports a
 * violation every time it is paid more than a handful of times in a week.
 */
import type { Qty } from '../../core/tick.js';
import { asRatio, minus, negated, scale } from '../../core/measure.js';
import { carriedDust, moveDust, mul, sum, withinDust, zeroIfNone } from '../../core/num.js';
import type { Family, Violation } from '../audit.js';
import { type AuditMemory, holdingKey } from '../memory.js';
import { movedByWeightEvents } from '../weights.js';
import type { AuditView } from '../view.js';

export function flowsFamily(memory: AuditMemory): Family {
  return {
    name: 'flows',
    contributor: 'kernel',
    spec: 'Audit B7',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      if (memory.period === undefined || memory.period === view.period) return out;
      // 0g.2: the period's deltas, indexed by the ledger as each record was appended (Law 19: the
      // record itself, arranged; never a copy this family keeps). Copied into working maps here
      // because the weight events below add to the holding side.
      const indexed = view.ledger.deltasIn(view.period);
      const holdingDeltas = new Map<string, Qty[]>();
      for (const [key, list] of indexed.holding) holdingDeltas.set(key, [...list]);
      const issuedDeltas = indexed.issued;
      // Register E4, E5, Equity D4: a SPLIT restates the unit a line is counted in. No units
      // changed hands and no money moved — the event says so — so the identity is not "what the
      // legs did" but "what the legs did to a balance that was restated first". The ratio is on
      // the event, publicly, which is what makes this a read of the record rather than an
      // exemption: a holding that moved for any other reason still has to be explained by a leg.
      const restated = new Map<string, number>();
      const ratioOf = (instrument: string): number => {
        // A line nobody restated is counted in the unit it was counted in: one of it is one of it.
        // eslint-disable-next-line phoenix/no-numeric-default -- absence here is "not restated"
        return restated.get(instrument) ?? 1;
      };
      for (const e of view.journal.inPeriod(view.period)) {
        if (e.kind !== 'instrument.split') continue;
        const instrument = e.data['instrument'];
        const ratio = e.data['ratio'];
        if (typeof instrument !== 'string' || typeof ratio !== 'number') continue;
        // Two restatements in one period compose, like two multiplications.
        restated.set(
          instrument,
          mul(ratioOf(instrument), ratio, 'the restatement this period'),
        );
      }
      /**
       * A-11: AND A COPIED BOOK IS COMPARED AGAINST THE BOOK IT WAS COPIED FROM.
       *
       * This used to put the new cell in a `copied` set and skip EVERY holding of it for the WHOLE
       * period — so Money D3 was switched off for a cell across every line it held, and everything
       * that cell then did was unexplained by construction. What has no leg behind it is the copy
       * of the parent's book AT THE INSTANT OF THE SPLIT, and nothing after that.
       *
       * A split is exact: the new cell's opening per-member position IS the parent's, because that
       * is what splitting a cell means. So the parent's remembered per-member state is the `before`
       * this cell is measured from, and every difference from it is a leg like anybody else's.
       *
       * A MERGE is the one that really vanishes: the absorbed cell's book is forgotten with no
       * instruction behind it, so that holder — and only that holder — is still exempt.
       */
      // XI-15, 0f.6: A WEIGHT EVENT MOVES HOLDINGS WITH NO LEG — a share of every lot goes with the
      // members that moved, and a merge brings a cell's whole book into the one it joins. The event
      // says what moved, per instrument, and that is read here as the explanation on both sides:
      // the mover lost it and the destination gained it. A cell that merged away has no book to
      // measure and is skipped; everything else is measured like any holding (Law 19: the record,
      // never an inferred copy).
      // 21.102: the one reading of what a weight event moved (`audit/weights.ts`), shared with the
      // capital programme's own units family — two copies of "what came with the members" is the
      // second one going stale the day a sixth event is added (Law 4).
      const weights = movedByWeightEvents(view, (holder: string, instrument: string) =>
        holdingKey(holder as never, instrument as never),
      );
      const vanished = weights.vanished;
      for (const [k, list] of weights.byHolding) {
        const held = holdingDeltas.get(k) ?? [];
        holdingDeltas.set(k, [...held, ...list]);
      }

      const keys = new Set<string>([...memory.holdings.keys(), ...holdingDeltas.keys()]);
      for (const h of view.register.allHoldings()) keys.add(holdingKey(h.holder, h.instrument));
      for (const key of keys) {
        const parts = key.split('|');
        const holder = parts[0];
        const instrument = parts[1];
        if (holder === undefined || instrument === undefined) continue;
        if (vanished.has(holder)) continue;
        if (!view.parties.has(holder as never) || !view.instruments.has(instrument as never))
          continue;
        // A cell opened by a split starts from its PARENT's remembered book, per member, because
        // that is exactly what it was given. Its own remembered entry does not exist — it did not
        // exist last period — and using it would measure the whole copy as unexplained.
        const remembered = memory.holdings.get(key);
        const before = scale(
          zeroIfNone<Qty>(remembered?.qty),
          asRatio(ratioOf(instrument), 'restated by the split'),
          'restated by the split',
        );
        const held = view.register.holding(holder as never, instrument as never);
        const now = view.register.quantity(holder as never, instrument as never);
        const legs = sum(holdingDeltas.get(key) ?? []);
        const change = sum([now, negated(before, 'the other way')]);
        const lots = zeroIfNone(remembered?.lots) + (held.some ? held.value.lots.length : 0);
        // Law 7: restating is one more rounding, at the magnitude the balance now is.
        const dust = carriedDust(before, now, lots, legs) + moveDust(before, 0);
        if (!withinDust(change.value, legs.value, dust)) {
          out.push({
            family: 'flows',
            spec: 'Money D3',
            owner: holder,
            size: minus(change.value, legs.value, 'what moved with no leg behind it'),
            unit: view.instruments.get(instrument as never).unit,
            period: view.period,
            message: `${holder}/${instrument} moved ${change.value} but instructions and weight events sum to ${legs.value}`,
          });
        }
      }
      for (const i of view.instruments.all()) {
        const before = scale(
          zeroIfNone<Qty>(memory.issued.get(i.id)),
          asRatio(ratioOf(i.id), 'restated by the split'),
          'restated by the split',
        );
        const legs = sum(issuedDeltas.get(i.id) ?? []);
        const change = sum([i.issued, negated(before, 'the other way')]);
        // Issued is one balance, not lots, but it is carried by the same leg-at-a-time walk.
        const dust = carriedDust(before, i.issued, 1, legs) + moveDust(before, 0);
        if (!withinDust(change.value, legs.value, dust)) {
          out.push({
            family: 'flows',
            spec: 'Register B1',
            owner: i.id,
            size: minus(change.value, legs.value, 'what was issued with no leg behind it'),
            unit: i.unit,
            period: view.period,
            message: `issued of ${i.id} moved ${change.value} but issuance legs sum to ${legs.value}`,
          });
        }
      }
      return out;
    },
  };
}

