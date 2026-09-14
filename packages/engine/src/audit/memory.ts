/**
 * What the audit remembers from its last run, so period-over-period identities (Money C4.c,
 * Money D3) compare two independent records: the ledger's instructions and the register's change.
 *
 * @spec Money C4.c Money D3 Audit C2 Audit C2.a
 */
import type { Qty } from '../core/tick.js';
import { sum, zeroIfNone } from '../core/num.js';
import type { Period } from '../calendar/calendar.js';
import type { InstrumentId, PartyId } from '../core/ids.js';
import type { AuditView } from './view.js';

export interface AuditMemory {
  period: Period | undefined;
  /** issued amount per instrument at the last audit. */
  issued: Map<InstrumentId, Qty>;
  /** per-member quantity per (holder, instrument) at the last audit, and the lots it was read over. */
  holdings: Map<string, { holder: PartyId; instrument: InstrumentId; qty: Qty; lots: number }>;
  /**
   * XI-15, Part XII, A-13: HOW MANY PEOPLE THERE WERE, per party kind, at the last audit.
   *
   * A weight is a count of people and the five events are the only things that may move one, so the
   * population is a period-over-period identity like any other: what it is now, less what it was,
   * is what the events said. Nothing stores a population (Appendix B forbids a stored aggregate) —
   * this is the audit's own memory of its own last reading, which is what makes it a second record.
   */
  people: Map<string, number>;
}

export function emptyMemory(): AuditMemory {
  return { period: undefined, issued: new Map(), holdings: new Map(), people: new Map() };
}

export function holdingKey(holder: PartyId, instrument: InstrumentId): string {
  return `${holder}|${instrument}`;
}

export function remember(view: AuditView, m: AuditMemory): void {
  m.period = view.period;
  m.issued = new Map(view.instruments.all().map((i) => [i.id, i.issued]));
  m.people = new Map();
  for (const p of view.parties.alive()) {
    if (p.representation !== 'cell') continue;
    m.people.set(String(p.kind), zeroIfNone(m.people.get(String(p.kind))) + p.weight);
  }
  m.holdings = new Map();
  for (const h of view.register.allHoldings()) {
    const qty = sum(h.lots.map((l) => l.qty)).value;
    m.holdings.set(holdingKey(h.holder, h.instrument), {
      holder: h.holder,
      instrument: h.instrument,
      qty,
      lots: h.lots.length,
    });
  }
}
