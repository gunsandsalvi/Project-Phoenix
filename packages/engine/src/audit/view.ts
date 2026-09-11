/**
 * What the audit may see: reads only. The type has no mutating method, so the audit cannot repair
 * (Audit C4) whatever it finds.
 *
 * @spec Audit A1 Audit C4
 */
import type { Calendar, Period } from '../calendar/calendar.js';
import type { InstrumentId, PartyId } from '../core/ids.js';
import { withinDust } from '../core/num.js';
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
import type { IndexDecl, IndexRead } from '../prices/index-read.js';
import type { Option } from '../core/option.js';

export interface AuditView {
  readonly period: Period;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<ParamRegister, 'report' | 'all' | 'get'>;
  readonly parties: Pick<Parties, 'get' | 'has' | 'all' | 'alive' | 'ofKind' | 'resolve' | 'cell'>;
  readonly instruments: Pick<Instruments, 'get' | 'has' | 'all' | 'issuedBy'>;
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
    | 'equityWalk'
    | 'hasEquityAccount'
    | 'revaluation'
    | 'revaluationWalk'
    | 'moneyWalk'
  >;
  readonly prices: Pick<PriceStore, 'read' | 'latest' | 'history' | 'instruments'>;
  readonly valuation: Pick<
    Valuation,
    'markPerUnit' | 'carryingPerUnit' | 'valueAtMark' | 'valueOfLots' | 'equityDust' | 'inMoney' | 'rateInForce'
  >;
  readonly ledger: Pick<Ledger, 'all' | 'inPeriod' | 'length'>;
  readonly journal: Pick<Journal, 'all' | 'inPeriod' | 'ofKind' | 'tail'>;
  readonly markets: readonly MarketDecl[];
  /**
   * Indices E3, D5: the index rules this world declares, and what each of them reads as. Both, so a
   * check can put the level against the constituents that made it — which is the one thing an index
   * can be wrong about once nothing stores it (a cache, a weight restated, a basket read at the
   * wrong period).
   */
  readonly indexList: readonly IndexDecl[];
  index(id: string): Option<IndexRead>;
}

/**
 * Law 7: whether a party is really holding something, or whether the balance is the rounding of the
 * moves that produced it. Money is ONE balance with a walk behind it, so what it may call nothing is
 * that walk — the same number settlement uses when it decides whether an account is short enough to
 * ask its issuer for an overdraft, because it is one fact and it may not have two tolerances (Law 4).
 * Anything else is lots drawn at the quantities the legs named, where nothing but zero is nothing.
 */
export function holdsSomething(
  view: AuditView,
  holder: PartyId,
  instrument: InstrumentId,
): boolean {
  const qty = view.register.quantity(holder, instrument);
  if (qty === 0) return false;
  const kind = view.registry.instrumentKind(view.instruments.get(instrument).kind);
  if (kind.pricing !== 'money') return true;
  return !withinDust(qty, 0, view.register.moneyWalk(holder, instrument).dust);
}
