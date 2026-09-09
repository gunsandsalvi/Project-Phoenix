/**
 * What the audit may see: reads only. The type has no mutating method, so the audit cannot repair
 * (Audit C4) whatever it finds.
 *
 * @spec Audit A1 Audit C4
 */
import type { Calendar, Period } from '../calendar/calendar.js';
import type { Journal } from '../journal/journal.js';
import type { Ledger } from '../ledger/ledger.js';
import type { Parties } from '../parties/party.js';
import type { PriceStore } from '../prices/price-store.js';
import type { Valuation } from '../prices/value.js';
import type { Instruments } from '../register/instruments.js';
import type { Register } from '../register/register.js';
import type { ParamRegister } from '../registry/params.js';
import type { Registry } from '../registry/registry.js';
import type { MarketDecl } from '../clearing/market.js';

export interface AuditView {
  readonly period: Period;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<ParamRegister, 'report' | 'all' | 'get'>;
  readonly parties: Pick<Parties, 'get' | 'has' | 'all' | 'alive' | 'ofKind' | 'resolve'>;
  readonly instruments: Pick<Instruments, 'get' | 'has' | 'all'>;
  readonly register: Pick<
    Register,
    | 'holding'
    | 'quantity'
    | 'totalQuantity'
    | 'encumbered'
    | 'free'
    | 'holdingsOf'
    | 'holdersOf'
    | 'heldTotal'
    | 'allHoldings'
    | 'equity'
    | 'hasEquityAccount'
  >;
  readonly prices: Pick<PriceStore, 'read' | 'latest' | 'history' | 'instruments'>;
  readonly valuation: Pick<
    Valuation,
    'markPerUnit' | 'carryingPerUnit' | 'valueAtMark' | 'valueOfLots'
  >;
  readonly ledger: Pick<Ledger, 'all' | 'inPeriod' | 'length'>;
  readonly journal: Pick<Journal, 'all' | 'inPeriod' | 'ofKind' | 'tail'>;
  readonly markets: readonly MarketDecl[];
}
