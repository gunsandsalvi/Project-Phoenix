/**
 * Instruments: what can be held. An instrument has a stable identity for its whole life (Register F1),
 * an issuer, a currency, a unit, an issued amount, and kind-specific terms validated by its kind's
 * profile at registration.
 *
 * @spec Register A1.b Register A4 Register B1 Register B4 Register F1 Register F1.a Bond N1 Bond N2 Bond N3 Bond N14 Money D2 Law 15
 */
import type { Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import { Forbidden, Missing } from '../core/errors.js';
import type {
  CurrencyCode,
  InstrumentId,
  InstrumentKindId,
  MarketId,
  PartyId,
  UnitId,
} from '../core/ids.js';
import { dustOf, finite } from '../core/num.js';
import type { Option } from '../core/option.js';
import type { Registry } from '../registry/registry.js';

/**
 * Terms are fixed at issuance and are the structure of the instrument, not an opening condition
 * (Seed C4.b). Each kind's profile knows its own shape and validates it.
 */
export interface Terms {
  readonly kind: InstrumentKindId;
}

export type InstrumentStatus =
  { readonly live: true } | { readonly live: false; readonly ceasedIn: Period };

/** What a module supplies to register an instrument; the kernel derives unit and status. */
export interface InstrumentDecl {
  readonly id: InstrumentId;
  readonly kind: InstrumentKindId;
  readonly issuer: PartyId;
  readonly ccy: CurrencyCode;
  readonly terms: Terms;
  readonly market: Option<MarketId>;
}

export interface Instrument extends InstrumentDecl {
  /** The unit its quantity is counted in (Register A1.c): par, shares, ccy:XXX ... */
  readonly unit: UnitId;
  /** B1: set at issuance, changed only by issuance, re-opening, buyback, amortisation, maturity. */
  readonly issued: number;
  /**
   * Law 7: the arithmetic dust `issued` has accumulated. It is a running total over every issuance
   * and redemption this line has seen, so the tolerance any comparison against it may use grows
   * with that history — it is not the dust of one addition. Carried with the number because it IS
   * the number's own error bar, not a second representation of it.
   */
  readonly issuedDust: number;
  readonly status: InstrumentStatus;
}

export class Instruments {
  private readonly map = new Map<InstrumentId, Instrument>();

  constructor(private readonly registry: Registry) {}

  /** Register a new instrument with nothing issued; issuance is a settlement leg (B1). */
  add(decl: InstrumentDecl): Instrument {
    forbid(!this.map.has(decl.id), 'Register F1', `instrument ${decl.id} already exists`, {
      id: decl.id,
    });
    forbid(
      decl.terms.kind === decl.kind,
      'Law 4',
      `instrument ${decl.id} is ${decl.kind} with ${decl.terms.kind} terms`,
    );
    const profile = this.registry.instrumentKind(decl.kind);
    profile.validateTerms(decl.terms);
    this.registry.currency(decl.ccy);
    const unit = profile.unit(decl.ccy);
    this.registry.unit(unit);
    if (profile.pricing === 'cleared') {
      forbid(
        decl.market.some,
        'Clearing D1',
        `${decl.id} is priced by clearing but names no market`,
      );
    } else {
      forbid(!decl.market.some, 'XI-6', `${decl.id} is not priced by clearing but names a market`);
    }
    const status: InstrumentStatus = { live: true };
    const i: Instrument = Object.freeze({ ...decl, unit, issued: 0, issuedDust: 0, status });
    this.map.set(i.id, i);
    return i;
  }

  has(id: InstrumentId): boolean {
    return this.map.has(id);
  }

  get(id: InstrumentId): Instrument {
    const i = this.map.get(id);
    if (i === undefined) {
      throw new Missing('Register A4', `instrument ${id} does not exist`, { id });
    }
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
    this.map.set(
      id,
      Object.freeze({
        ...i,
        issued: finite(i.issued + delta, `issued of ${id}`),
        issuedDust: finite(
          i.issuedDust + dustOf(1, Math.abs(i.issued) + Math.abs(delta)),
          `issued dust of ${id}`,
        ),
      }),
    );
  }

  /** B4: an instrument ceases, and every holding in it has already resolved to something else, named. */
  cease(id: InstrumentId, period: Period): void {
    const i = this.get(id);
    forbid(i.status.live, 'Register B4', `instrument ${id} has already ceased`);
    if (i.issued !== 0) {
      throw new Forbidden(
        'Register B4',
        `instrument ${id} still has ${i.issued} issued; it cannot cease`,
        { id, issued: i.issued },
      );
    }
    this.map.set(id, Object.freeze({ ...i, status: { live: false, ceasedIn: period } }));
  }
}

/** The read-only face of the instruments store, for mechanisms and participants. */
export type InstrumentsReads = Pick<Instruments, 'has' | 'get' | 'all'>;
