/**
 * What the audit remembers from its last run, so period-over-period identities (Money C4.c,
 * Money D3) compare two independent records: the ledger's instructions and the register's change.
 *
 * @spec Money C4.c Money D3 Audit C2 Audit C2.a
 */
import type { Period } from '../calendar/calendar.js';
import type { InstrumentId, PartyId } from '../core/ids.js';
import type { AuditView } from './view.js';

export interface AuditMemory {
  period: Period | undefined;
  /** issued amount per instrument at the last audit. */
  issued: Map<InstrumentId, number>;
  /** per-member quantity per (holder, instrument) at the last audit, and the lots it was read over. */
  holdings: Map<string, { holder: PartyId; instrument: InstrumentId; qty: number; lots: number }>;
}

export function emptyMemory(): AuditMemory {
  return { period: undefined, issued: new Map(), holdings: new Map() };
}

export function holdingKey(holder: PartyId, instrument: InstrumentId): string {
  return `${holder}|${instrument}`;
}

export function remember(view: AuditView, m: AuditMemory): void {
  m.period = view.period;
  m.issued = new Map(view.instruments.all().map((i) => [i.id, i.issued]));
  m.holdings = new Map();
  for (const h of view.register.allHoldings()) {
    const qty = h.lots.reduce((s, l) => s + l.qty, 0);
    m.holdings.set(holdingKey(h.holder, h.instrument), {
      holder: h.holder,
      instrument: h.instrument,
      qty,
      lots: h.lots.length,
    });
  }
}
