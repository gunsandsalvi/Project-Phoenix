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
import { carriedDust, moveDust, mul, sum, withinDust, zeroIfNone } from '../../core/num.js';
import type { Family, Violation } from '../audit.js';
import { type AuditMemory, holdingKey } from '../memory.js';
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
      const holdingDeltas = new Map<string, number[]>();
      const issuedDeltas = new Map<string, number[]>();
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled') continue;
        for (const d of r.deltas) {
          const map = d.target === 'holding' ? holdingDeltas : issuedDeltas;
          const key = d.target === 'holding' ? holdingKey(d.party, d.instrument) : d.instrument;
          const list = map.get(key) ?? [];
          list.push(d.qty);
          map.set(key, list);
        }
      }
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
        restated.set(instrument, mul(ratioOf(instrument), ratio, 'the restatement this period'));
      }

      /**
       * XI-15, Audit C3: THE ONE CELL WHOSE BOOK REALLY DID APPEAR OR VANISH, and no other.
       *
       * Every quantity compared here is PER MEMBER, so a weight event that only changes how many
       * members there are changes nothing this family is looking at: an entry, a death and a
       * promotion are checked exactly like any other period. Two of the five copy a book without
       * an instruction — a split gives the NEW cell its parent's per-member state, and a merge
       * forgets the absorbed one — and it is those two cells, named on the event itself, that have
       * a holding with no leg behind it.
       *
       * It used to exempt every SUBJECT of every weight event, which is both cells of a split or a
       * merge and the cell itself for the other three — so a household cell that gained a member
       * had Money D3 switched off across every line it held, for that period, and households are
       * cells whose weights move constantly (item 13b.1). C3 says every family runs the same checks
       * every period.
       */
      const copied = new Set<string>();
      for (const e of view.journal.inPeriod(view.period)) {
        if (e.kind !== 'weight') continue;
        const kind = e.data['kind'];
        const who = kind === 'split' ? e.data['to'] : kind === 'merge' ? e.data['from'] : undefined;
        if (typeof who === 'string') copied.add(who);
      }

      const keys = new Set<string>([...memory.holdings.keys(), ...holdingDeltas.keys()]);
      for (const h of view.register.allHoldings()) keys.add(holdingKey(h.holder, h.instrument));
      for (const key of keys) {
        const parts = key.split('|');
        const holder = parts[0];
        const instrument = parts[1];
        if (holder === undefined || instrument === undefined) continue;
        if (copied.has(holder)) continue;
        if (!view.parties.has(holder as never) || !view.instruments.has(instrument as never))
          continue;
        const remembered = memory.holdings.get(key);
        const before = mul(zeroIfNone(remembered?.qty), ratioOf(instrument), 'restated by the split');
        const held = view.register.holding(holder as never, instrument as never);
        const now = view.register.quantity(holder as never, instrument as never);
        const legs = sum(holdingDeltas.get(key) ?? []);
        const change = sum([now, -before]);
        const lots = zeroIfNone(remembered?.lots) + (held.some ? held.value.lots.length : 0);
        // Law 7: restating is one more rounding, at the magnitude the balance now is.
        const dust = carriedDust(before, now, lots, legs) + moveDust(before, 0);
        if (!withinDust(change.value, legs.value, dust)) {
          out.push({
            family: 'flows',
            spec: 'Money D3',
            owner: holder,
            size: change.value - legs.value,
            unit: view.instruments.get(instrument as never).unit,
            period: view.period,
            message: `${holder}/${instrument} moved ${change.value} per member but instructions sum to ${legs.value}`,
          });
        }
      }
      for (const i of view.instruments.all()) {
        const before = mul(zeroIfNone(memory.issued.get(i.id)), ratioOf(i.id), 'restated by the split');
        const legs = sum(issuedDeltas.get(i.id) ?? []);
        const change = sum([i.issued, -before]);
        // Issued is one balance, not lots, but it is carried by the same leg-at-a-time walk.
        const dust = carriedDust(before, i.issued, 1, legs) + moveDust(before, 0);
        if (!withinDust(change.value, legs.value, dust)) {
          out.push({
            family: 'flows',
            spec: 'Register B1',
            owner: i.id,
            size: change.value - legs.value,
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

