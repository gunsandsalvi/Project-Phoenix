/**
 * Instruments: what can be held. An instrument has a stable identity for its whole life (Register F1),
 * an issuer, a currency, a unit, an issued amount, and kind-specific terms.
 *
 * @spec Register A1.b Register A4 Register B1 Register B4 Register F1 Register F1.a Bond N1 Bond N2 Bond N3 Bond N4 Bond N5 Bond N6 Bond N10 Bond N14 Sovereign B1 Money D2
 */
import type { Period } from '../calendar/calendar.js';
import type { Civil } from '../calendar/civil.js';
import type { DayCount } from '../calendar/daycount.js';
import { forbid } from '../core/assert.js';
import { Forbidden, Missing } from '../core/errors.js';
import type { CurrencyCode, InstrumentId, MarketId, PartyId, UnitId } from '../core/ids.js';
import { finite } from '../core/num.js';
import type { Option } from '../core/option.js';
import type { Periodicity, Rate } from '../core/rate.js';
import type { InstrumentKind } from '../registry/kinds.js';

/** Terms are fixed at issuance and are the structure of the instrument, not an opening condition (Seed C4.b). */
export type InstrumentTerms =
  | { readonly kind: 'money' }
  | {
      readonly kind: 'sovereign.bond';
      /** N5.a: fixed, locked at issuance, quoted per annum. */
      readonly coupon: Rate;
      /** N6: how often it pays. */
      readonly couponPeriodicity: Periodicity;
      /** N6: how interest accrues between payments. */
      readonly dayCount: DayCount;
      readonly issueDate: Civil;
      /** N4: the date the principal is due. */
      readonly maturity: Civil;
    }
  | {
      readonly kind: 'sovereign.bill';
      readonly issueDate: Civil;
      readonly maturity: Civil;
    };

export type InstrumentStatus =
  { readonly live: true } | { readonly live: false; readonly ceasedIn: Period };

export interface Instrument {
  readonly id: InstrumentId;
  readonly kind: InstrumentKind;
  /** N1: who owes. */
  readonly issuer: PartyId;
  /** N3: every figure about it is in this money. */
  readonly ccy: CurrencyCode;
  /** The unit its quantity is counted in (Register A1.c): par, shares, ccy:XXX ... */
  readonly unit: UnitId;
  readonly terms: InstrumentTerms;
  /** B1: set at issuance, changed only by issuance, re-opening, buyback, amortisation, maturity. */
  readonly issued: number;
  readonly status: InstrumentStatus;
  /** The market that prints its price, if its pricing is cleared. */
  readonly market: Option<MarketId>;
}

export class Instruments {
  private readonly map = new Map<InstrumentId, Instrument>();

  add(i: Instrument): void {
    forbid(!this.map.has(i.id), 'Register F1', `instrument ${i.id} already exists`, { id: i.id });
    forbid(
      i.terms.kind === i.kind,
      'Law 4',
      `instrument ${i.id} is ${i.kind} with ${i.terms.kind} terms`,
    );
    finite(i.issued, `issued of ${i.id}`);
    this.map.set(i.id, Object.freeze({ ...i }));
  }

  has(id: InstrumentId): boolean {
    return this.map.has(id);
  }

  get(id: InstrumentId): Instrument {
    const i = this.map.get(id);
    if (i === undefined)
      throw new Missing('Register A4', `instrument ${id} does not exist`, { id });
    return i;
  }

  all(): readonly Instrument[] {
    return [...this.map.values()];
  }

  /** Only settlement calls this, when an issuance or redemption leg applies (B1). */
  adjustIssued(id: InstrumentId, delta: number): void {
    const i = this.get(id);
    forbid(
      i.status.live,
      'Register B4',
      `instrument ${id} has ceased; nothing can be issued or redeemed`,
    );
    this.map.set(id, Object.freeze({ ...i, issued: finite(i.issued + delta, `issued of ${id}`) }));
  }

  /** B4: an instrument ceases, and every holding in it has already resolved to something else, named. */
  cease(id: InstrumentId, period: Period): void {
    const i = this.get(id);
    forbid(i.status.live, 'Register B4', `instrument ${id} has already ceased`);
    if (i.issued !== 0) {
      throw new Forbidden(
        'Register B4',
        `instrument ${id} still has ${i.issued} issued; it cannot cease`,
        {
          id,
          issued: i.issued,
        },
      );
    }
    this.map.set(id, Object.freeze({ ...i, status: { live: false, ceasedIn: period } }));
  }
}
