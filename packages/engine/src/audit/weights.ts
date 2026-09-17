/**
 * What a weight event moved, per holder and per line — the one reading of it (Law 4).
 *
 * @spec XI-15 Money D3 Capital Programme A6.b Audit A1.a Law 4 Law 19
 *
 * A book moves with no instruction behind it exactly five times, and they are XI-15's five events:
 * members enter, die, are promoted, split away or merge in, and a share of every lot goes with them.
 * The event says what moved, per instrument, which is what makes those moves READABLE rather than
 * an exemption — and two audit families need to read it: the kernel's `flows`, which checks that
 * every holding's change is accounted for, and the capital programme's `units`, which checks the
 * same of plant and goods.
 *
 * The second one did not, so every merge of a cell holding plant reported as plant that appeared
 * from nowhere: three landlord cells merging in period 3 of the scale model produced 131 violations
 * (`-1800`, `-1800`, `+3600` on one vintage, and the same on every other line they held). The
 * reading is here, once, rather than in each family, because two copies of "what came with the
 * members" is the second one going stale the day a sixth event is added.
 */
import type { InstrumentId, PartyId } from '../core/ids.js';
import { asQty, negQty, type Qty } from '../core/tick.js';
import type { AuditView } from './view.js';

export interface WeightMoves {
  /** `holder|instrument` → the quantities that moved with members, both sides of every event. */
  readonly byHolding: ReadonlyMap<string, readonly Qty[]>;
  /**
   * The cells that merged AWAY this period. Their book is the absorbing cell's now and there is
   * nothing of theirs left to measure — the one holder a family skips rather than explains.
   */
  readonly vanished: ReadonlySet<string>;
}

export function movedByWeightEvents(
  view: AuditView,
  key: (holder: string, instrument: string) => string,
): WeightMoves {
  const byHolding = new Map<string, Qty[]>();
  const vanished = new Set<string>();
  const add = (holder: string, instrument: string, qty: Qty): void => {
    const k = key(holder, instrument);
    const held = byHolding.get(k);
    if (held === undefined) byHolding.set(k, [qty]);
    else held.push(qty);
  };
  for (const e of view.journal.inPeriod(view.period)) {
    if (e.kind !== 'weight') continue;
    const kind = e.data['kind'];
    const to = e.data['to'];
    const from = e.data['from'];
    const moved = e.data['moved'];
    if (kind === 'merge' && typeof from === 'string') vanished.add(from);
    if (typeof moved !== 'object' || moved === null) continue;
    for (const [instrument, qty] of Object.entries(moved as Record<string, unknown>)) {
      if (typeof qty !== 'number') continue;
      if (typeof to === 'string') add(to, instrument, asQty(qty, 'what arrived with the members'));
      // A cell that merged away keeps no book to take it off: what it held is the other one's now.
      if (typeof from === 'string' && kind !== 'merge') {
        add(from, instrument, negQty(asQty(qty, 'what left with the members'), 'what left'));
      }
    }
  }
  return { byHolding, vanished };
}

/** The same, keyed the way the register keys a holding — for a family that has no key of its own. */
export const holdingKeyOf = (holder: PartyId | string, instrument: InstrumentId | string): string =>
  `${String(holder)}|${String(instrument)}`;
