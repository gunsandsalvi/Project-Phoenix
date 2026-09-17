/**
 * Units (Part XII): a weight is a positive count; every holding is a whole number of the smallest
 * member; every represented party sits in exactly one cell. Physical-unit identities (goods,
 * dwellings, plant) join this family with their systems.
 *
 * @spec Part XII Commodities Spot D5 Commodities Spot F1 Goods E4 Appendix A Small-Business Pools E5 XI-15 Law 6
 */
import { combineDust, sum, withinDust, zeroIfNone } from '../../core/num.js';
import { negQty, onTick } from '../../core/tick.js';
import type { Family, Violation } from '../audit.js';
import type { AuditMemory } from '../memory.js';
import type { AuditView } from '../view.js';
import { says } from '../../registry/facts.js';
import { WEIGHT } from '../../world/facts.js';

export function unitsFamily(memory: AuditMemory): Family {
  return {
    name: 'units',
    contributor: 'kernel',
    spec: 'Part XII',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      for (const p of view.parties.alive()) {
        if (p.representation !== 'cell') continue;
        if (!Number.isInteger(p.weight) || p.weight <= 0) {
          out.push({
            family: 'units',
            spec: 'Appendix A',
            owner: p.id,
            size: p.weight,
            unit: 'count',
            period: view.period,
            message: `weight of ${p.id} is not a count`,
          });
        }
      }
      // Part XII, Commodities Spot D5: what exists now is what existed, plus what was made, less
      // what was used up. The two records are independent — the instrument's issued total and the
      // legs that said why units appeared or left — so a unit conjured or lost shows here.
      if (memory.period !== undefined && memory.period !== view.period) {
        // 0g.2: what was made and used up, indexed by the ledger as each record was appended.
        const made = view.ledger.deltasIn(view.period).made;
        for (const i of view.instruments.all()) {
          if (view.registry.instrumentKind(i.kind).physical !== true) continue;
          const before = zeroIfNone(memory.issued.get(i.id));
          const legs = sum(made.get(i.id) ?? []);
          const change = sum([i.issued, negQty(before, 'the other way')]);
          if (!withinDust(change.value, legs.value, combineDust(legs, change) + i.issuedDust)) {
            out.push({
              family: 'units',
              spec: 'Commodities Spot D5',
              owner: i.id,
              size: change.value - legs.value,
              unit: i.unit,
              period: view.period,
              message: `${i.id}: the stock moved by ${change.value} and ${legs.value} was made or used up`,
            });
          }
        }
      }
      /**
       * A-13, XI-15, Part XII: A POPULATION IS AN IDENTITY, FOR EVERY CELL AND NOT ONLY EMPLOYED ONES.
       *
       * Part XII names it — "the sum of cell weights equals the population it stands for, and every
       * represented party sits in exactly one cell" — and neither half was anywhere in the kernel.
       * The only contributor that counted people was `labour`, through employment, so a cell that
       * nobody employs could lose or invent members and no family looked: the firms' small-business
       * pools and XI-15 cells generally were unwatched.
       *
       * The two records: how many people there are now, per kind, read off the parties store; and
       * how many there were at the last audit plus what the five WEIGHT EVENTS said happened in
       * between. A weight may move for no other reason, so the difference is exactly the events.
       */
      if (memory.period !== undefined && memory.period !== view.period) {
        const now = new Map<string, number>();
        for (const p of view.parties.alive()) {
          if (p.representation !== 'cell') continue;
          now.set(String(p.kind), zeroIfNone(now.get(String(p.kind))) + p.weight);
        }
        const moved = new Map<string, number>();
        for (const e of view.journal.inPeriod(view.period)) {
          if (e.kind !== WEIGHT.kind) continue;
          const said = says(e, WEIGHT);
          const who = e.subjects[0];
          if (who === undefined) continue;
          /**
           * 0i, Law 19: WHAT THE EVENT SAYS, not which keys it happens to carry.
           *
           * This counted `after - before` for the events that wrote both and skipped the ones that
           * wrote neither — so the meaning was in an ABSENCE, and the two writers of a promotion
           * disagreed about it: `promoteCell` carried them and `reKeyCell` did not. It was right by
           * accident. What it is actually asking is whether the members left the CELL POPULATION
           * this sums (`now` counts cells and no named party), and the event says where they went.
           *
           * A split or a merge moves them between two cells and the population is unchanged; a
           * promotion into a named party takes them out of it (a member outgrows the tier and
           * becomes a firm); an entry or a death has nowhere they came from or went to, and moves
           * the population by what it moved.
           */
          const stillACell =
            said.to !== null && view.parties.get(said.to as never).representation === 'cell';
          if (stillACell) continue;
          const kind = String(view.parties.get(who as never).kind);
          moved.set(kind, zeroIfNone(moved.get(kind)) + (said.after - said.before));
        }
        for (const kind of new Set([...now.keys(), ...memory.people.keys()])) {
          const was = zeroIfNone(memory.people.get(kind));
          const change = zeroIfNone(now.get(kind)) - was;
          const events = zeroIfNone(moved.get(kind));
          if (change === events) continue;
          out.push({
            family: 'units',
            spec: 'XI-15',
            owner: kind,
            size: change - events,
            unit: 'people',
            period: view.period,
            message: `${kind} cells stand for ${change - events} people more than the weight events account for`,
          });
        }
      }
      /**
       * Part XII: AND EVERY REPRESENTED PARTY SITS IN EXACTLY ONE CELL. Two live cells of one kind
       * carrying the same key are one population wearing two states — the thing A4.c forbids one
       * cell over, a whole cell wide.
       */
      const seats = new Map<string, string>();
      for (const p of view.parties.alive()) {
        if (p.representation !== 'cell') continue;
        // The key is every dimension the registry declares for this kind, in one string, read off
        // the cell itself — never a dimension named here, which would be a second list of them.
        const dims = Object.entries(p.key)
          .sort(([a], [b]) => (a < b ? -1 : 1))
          .map(([d, value]) => `${d}=${value}`)
          .join(',');
        const seat = `${String(p.kind)}|${String(p.region)}|${String(p.bank)}|${dims}`;
        const already = seats.get(seat);
        if (already !== undefined) {
          out.push({
            family: 'units',
            spec: 'XI-15',
            owner: String(p.id),
            size: p.weight,
            unit: 'people',
            period: view.period,
            message: `${p.id} and ${already} are two cells standing for the same ${seat}`,
          });
        } else seats.set(seat, String(p.id));
      }
      for (const h of view.register.allHoldings()) {
        const inst = view.instruments.get(h.instrument);
        for (const lot of h.lots) {
          // Law 8: a lot holds a whole number of the smallest piece of its unit. Anything else is a
          // quantity of something that does not exist, and it can only have got there by arithmetic
          // rather than by a leg — which is the thing this family is for.
          if (!onTick(lot.qty)) {
            out.push({
              family: 'units',
              spec: 'Law 8',
              owner: h.holder,
              size: lot.qty,
              unit: inst.unit,
              period: view.period,
              message: `${h.holder} holds ${lot.qty} of ${inst.id}, which is not a whole number of pieces`,
            });
          }
        }
      }
      return out;
    },
  };
}
