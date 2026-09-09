/**
 * Money is conserved (Audit B1).
 *
 * @spec Audit B1 Money A1.d Money B3.c Money C2.c Money C4 Money C4.c Money F3 Money F4
 *
 * Three reads of two independent things:
 *  - C2.c: within every settled instruction, holding deltas of money minus issued deltas net to zero
 *    per currency: a transfer is two-sided, and an issuance is counted as the thing being measured.
 *  - C4.c: the change in each money instrument's stock over the period equals the creation and
 *    destruction legs the ledger recorded for it, and nothing else.
 *  - B3.c: no money balance is negative at the close without a lender; until the corridor prices a
 *    reserve overdraft, every one is reported here by holder and size.
 */
import { carriedDust, sum, withinDust, zeroIfNone } from '../../core/num.js';
import { weightOf } from '../../parties/party.js';
import type { Family, Violation } from '../audit.js';
import type { AuditMemory } from '../memory.js';
import type { AuditView } from '../view.js';

export function moneyFamily(memory: AuditMemory): Family {
  return {
    name: 'money',
    contributor: 'kernel',
    spec: 'Audit B1',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      const moneyInstruments = new Set(
        view.instruments
          .all()
          .filter((i) => view.registry.instrumentKind(i.kind).pricing === 'money')
          .map((i) => i.id),
      );

      // C2.c per instruction.
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled') continue;
        const perCcy = new Map<string, number[]>();
        for (const d of r.deltas) {
          if (!moneyInstruments.has(d.instrument)) continue;
          const ccy = view.instruments.get(d.instrument).ccy;
          // XI-15: the weight the delta was struck at, not the weight the cell has now — it may
          // have split since, and this instruction moved what it moved.
          const signed = d.target === 'holding' ? d.qty * d.weight : -d.qty;
          const list = perCcy.get(ccy) ?? [];
          list.push(signed);
          perCcy.set(ccy, list);
        }
        for (const [ccy, terms] of perCcy) {
          const s = sum(terms);
          if (!withinDust(s.value, 0, s.dust)) {
            out.push({
              family: 'money',
              spec: 'Money C2.c',
              owner: `instruction ${r.instruction.id}`,
              size: s.value,
              unit: ccy,
              period: view.period,
              message: `money legs of instruction ${r.instruction.id} do not net to zero in ${ccy}`,
            });
          }
        }
      }

      // C4.c per money instrument, against the previous audit.
      if (memory.period !== undefined && memory.period !== view.period) {
        for (const id of moneyInstruments) {
          const before = zeroIfNone(memory.issued.get(id));
          const now = view.instruments.get(id).issued;
          const legs: number[] = [];
          for (const r of view.ledger.inPeriod(view.period)) {
            if (r.outcome !== 'settled') continue;
            for (const d of r.deltas)
              if (d.instrument === id && d.target === 'issued') legs.push(d.qty);
          }
          const s = sum(legs);
          const change = sum([now, -before]);
          // Law 7: `issued` is a running total moved once per creation and once per destruction,
          // so the comparison is a carried balance and not two readings (see carriedDust).
          if (!withinDust(change.value, s.value, carriedDust(before, now, 1, s))) {
            out.push({
              family: 'money',
              spec: 'Money C4.c',
              owner: id,
              size: change.value - s.value,
              unit: view.instruments.get(id).ccy,
              period: view.period,
              message: `stock of ${id} moved by ${change.value} but creation legs sum to ${s.value}`,
            });
          }
        }
      }

      // B3.c: a negative balance at the close is borrowing from somebody named, or it is a defect.
      for (const h of view.register.allHoldings()) {
        if (!moneyInstruments.has(h.instrument)) continue;
        const qty = sum(h.lots.map((l) => l.qty)).value;
        // Law 7: the balance is a walk, not a reading — every leg that ever moved this account is
        // entitled to its rounding, and a negative that small is that rounding rather than credit
        // anybody extended. It is the same tolerance settlement uses when it decides whether the
        // account is short enough to ask the issuer for an overdraft, because it is one fact (Law 4).
        if (qty < 0 && !withinDust(qty, 0, view.register.moneyWalk(h.holder, h.instrument).dust)) {
          out.push({
            family: 'money',
            spec: 'Money B3.c',
            owner: h.holder,
            size: qty * weightOf(view.parties.get(h.holder)),
            unit: view.instruments.get(h.instrument).ccy,
            period: view.period,
            message: `${h.holder} closes ${qty} per member on ${h.instrument} with no lender row (corridor not built)`,
          });
        }
      }
      return out;
    },
  };
}
