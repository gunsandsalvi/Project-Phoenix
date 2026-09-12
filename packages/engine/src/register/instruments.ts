/**
 * Instruments: what can be held. An instrument has a stable identity for its whole life (Register F1),
 * an issuer, a currency, a unit, an issued amount, and kind-specific terms validated by its kind's
 * profile at registration.
 *
 * @spec Register A1.b Register A4 Register B1 Register B4 Register E4 Register F1 Register F1.a Fund Shares E1 Fund Shares E2 Bond N1 Bond N2 Bond N3 Bond N14 Equity A2.a Equity D4 Money D2 Law 15
 */
import type { Period } from '../calendar/calendar.js';
import { forbid, impossible } from '../core/assert.js';
import { Forbidden, Missing } from '../core/errors.js';
import type {
  CurrencyCode,
  InstrumentId,
  InstrumentKindId,
  MarketId,
  PartyId,
  UnitId,
} from '../core/ids.js';
import { finite, moveDust } from '../core/num.js';
import { NO_QTY, asQty, onTick, type Qty } from '../core/tick.js';
import { some, type Option } from '../core/option.js';
import type { Registry } from '../registry/registry.js';

/**
 * Terms are fixed at issuance and are the structure of the instrument, not an opening condition
 * (Seed C4.b). Each kind's profile knows its own shape and validates it.
 */
export interface Terms {
  readonly kind: InstrumentKindId;
}

/**
 * Banks Lending E2, Bond N12: a status no path ever writes is not a status. A live instrument is
 * performing until a payment it promised failed, and then it is not — written once, by the kernel,
 * off the instrument's own definition of default, and read by everybody.
 */
export type InstrumentStatus =
  | { readonly live: true; readonly performing: boolean }
  | { readonly live: false; readonly ceasedIn: Period };

/** What a module supplies to register an instrument; the kernel derives unit and status. */
export interface InstrumentDecl {
  readonly id: InstrumentId;
  readonly kind: InstrumentKindId;
  /**
   * Who promised it. A claim has an issuer whose liability it is (Register B3); a physical thing
   * has none — nobody issued a tonne of wheat — and says so rather than naming a party that would
   * then have to be carried through every check as a fiction (Goods A1).
   */
  readonly issuer: Option<PartyId>;
  readonly ccy: CurrencyCode;
  readonly terms: Terms;
  readonly market: Option<MarketId>;
}

export interface Instrument extends InstrumentDecl {
  /** The unit its quantity is counted in (Register A1.c): par, shares, ccy:XXX ... */
  readonly unit: UnitId;
  /** B1: set at issuance, changed only by issuance, re-opening, buyback, amortisation, maturity. */
  /** Law 8: how many pieces of it exist — a count, restated only by a split (Register E4). */
  readonly issued: Qty;
  /**
   * Law 7: the arithmetic dust `issued` has accumulated. It is a running total over every issuance
   * and redemption this line has seen, so the tolerance any comparison against it may use grows
   * with that history — it is not the dust of one addition. Carried with the number because it IS
   * the number's own error bar, not a second representation of it.
   */
  readonly issuedDust: number;
  readonly status: InstrumentStatus;
}

/**
 * The party whose liability a claim is (Register B3). A physical thing has no issuer, and asking
 * for one is a defect in the caller, not a number to invent (Appendix A).
 */
export function issuerOf(i: Instrument): PartyId {
  if (!i.issuer.some) {
    throw new Missing('Register B3', `${i.id} is a physical thing; nobody issued it`, { id: i.id });
  }
  return i.issuer.value;
}

/** Whether this instrument is a claim on the named party. */
export function issuedBy(i: Instrument, party: PartyId): boolean {
  return i.issuer.some && i.issuer.value === party;
}

export class Instruments {
  private readonly map = new Map<InstrumentId, Instrument>();
  /**
   * Law 18: the same instruments under two more arrangements. A bank asks what it has out every
   * time somebody shops it, and walking every instrument in the world for one issuer's own is a
   * scan that grows with the world rather than with the answer — a region with a share line per
   * listed firm has hundreds of them. Ids, not records: a record is replaced when something is
   * issued or redeemed, and an index of stale copies is a second register (Law 4).
   */
  private readonly byIssuer = new Map<PartyId, InstrumentId[]>();
  private everything: readonly Instrument[] | undefined;

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
    } else if (profile.pricing !== 'derived') {
      // XI-6: a thing whose value is what it cost, or one of itself, has nothing to clear.
      forbid(!decl.market.some, 'XI-6', `${decl.id} is not priced by clearing but names a market`);
    }
    // Fund Shares E1, E2: a DERIVED line may also trade, and then it has TWO VALUES and they are
    // different numbers. What its holders and its issuer carry it at is the derived one — a claim
    // on a book is worth what the book comes to over how many claims there are, whatever anybody
    // paid for one this morning — and what the market printed is what a third party paid. Neither
    // is the other's approximation and neither is invented: the gap between them is a read (E4),
    // and it is the reason an exchange-traded fund has an arbitrageur at all (E3).
    const status: InstrumentStatus = { live: true, performing: true };
    const i: Instrument = Object.freeze({ ...decl, unit, issued: NO_QTY, issuedDust: 0, status });
    this.map.set(i.id, i);
    this.everything = undefined;
    if (i.issuer.some) {
      const list = this.byIssuer.get(i.issuer.value);
      if (list === undefined) this.byIssuer.set(i.issuer.value, [i.id]);
      else list.push(i.id);
    }
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
    this.everything ??= [...this.map.values()];
    return this.everything;
  }

  /** Register B3: what one party has promised — the instruments whose issuer it is, by name. */
  issuedBy(party: PartyId): readonly Instrument[] {
    const ids = this.byIssuer.get(party);
    if (ids === undefined) return [];
    return ids.map((id) => this.get(id));
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
        issued: onTheGrid(finite(i.issued + delta, `issued of ${id}`), `what is issued of ${id}`),
        issuedDust: finite(
          i.issuedDust + moveDust(i.issued, delta),
          `issued dust of ${id}`,
        ),
      }),
    );
    this.everything = undefined;
  }

  /**
   * Banks Lending E1, E2: the one path that writes the status. It is called by the kernel when a
   * payment this instrument promised failed and its own profile called that a default (Bond N12);
   * nothing else may, and nothing restores it — a cure is a state of its own and needs a workout
   * to reach it (E3, worklist 6).
   */
  markDefaulted(id: InstrumentId): void {
    const i = this.get(id);
    forbid(i.status.live, 'Banks Lending E2', `instrument ${id} has ceased; it cannot default`);
    // Already written: a second missed payment on the same line is a second EVENT, and the journal
    // has both, but the status only ever moves the one way and only once (E2).
    if (!i.status.performing) return;
    const status: InstrumentStatus = { live: true, performing: false };
    this.map.set(id, Object.freeze({ ...i, status }));
    this.everything = undefined;
  }

  /**
   * XI-8, Firm Birth D5, Banks Capital D6: an issuer died and its paper did not vanish with it. The
   * estate that succeeded it becomes the issuer of record, so every holder still holds a claim on
   * somebody who exists — which is what "every reference to the party must resolve" means when the
   * reference is inside an instrument rather than beside it. Only the module that opens estates
   * calls it, and it never changes anything else about the line: same terms, same holders, same
   * amount outstanding, a different name owing it.
   */
  reseat(id: InstrumentId, issuer: PartyId): void {
    const i = this.get(id);
    forbid(i.status.live, 'Register B4', `instrument ${id} has ceased; nobody owes it`);
    forbid(i.issuer.some, 'Register B3', `${id} was promised by nobody, so nobody can succeed to it`);
    this.map.set(id, Object.freeze({ ...i, issuer: some(issuer) }));
    this.everything = undefined;
    // B3 guarantees an issuer above, so the old one is there to move the line off.
    const was = this.byIssuer.get(i.issuer.value);
    if (was !== undefined) this.byIssuer.set(i.issuer.value, was.filter((x) => x !== id));
    const now = this.byIssuer.get(issuer);
    if (now === undefined) this.byIssuer.set(issuer, [id]);
    else now.push(id);
  }

  /**
   * Register E4, Equity D4: the count of a line changes and nothing else does. Only the split door
   * calls it, with the register restated in the same operation; what each holder holds and what it
   * cost them is unchanged in value, so no equity moves and no money does either — which is the
   * "explicitly says why not" E5 asks of an event that moves a register without moving money.
   */
  restate(id: InstrumentId, ratio: number): void {
    const i = this.get(id);
    forbid(i.status.live, 'Register B4', `instrument ${id} has ceased; its count cannot change`);
    const issued = onTheGrid(
      finite(i.issued * ratio, `issued of ${id}`),
      `what is issued of ${id} after a ${ratio}-for-one split`,
    );
    this.map.set(
      id,
      Object.freeze({
        ...i,
        issued,
        // Law 7: restating every unit is one more rounding, at the magnitude the count now is.
        issuedDust: finite(i.issuedDust * ratio + moveDust(issued, 0), `issued dust of ${id}`),
      }),
    );
    // Law 4, Law 19: AND THE MEMO IS NOT A SECOND REGISTER. Every other mutator in this file ends
    // here; this one did not, so after a split `all()` handed back frozen records carrying the
    // PRE-split count for the rest of the run — to the ownership family, to the observer and to
    // every derived value (item 13b.1). A memo that can disagree with the map is the index this
    // file's own comment says an index must never be.
    this.everything = undefined;
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
    const ceased: InstrumentStatus = { live: false, ceasedIn: period };
    this.map.set(id, Object.freeze({ ...i, status: ceased }));
    this.everything = undefined;
  }
}

/** The read-only face of the instruments store, for mechanisms and participants. */
export type InstrumentsReads = Pick<Instruments, 'has' | 'get' | 'all' | 'issuedBy'>;

/**
 * Law 8, Register A1.c: WHAT IS OUTSTANDING IS A WHOLE NUMBER OF PIECES, like every holding of it.
 *
 * Holdings sum to issued (the ownership audit family), so a fractional issued count is a fractional
 * holding somewhere or an identity that cannot close. Both doors that move it are guarded here
 * rather than at their callers, so a new one cannot forget (Law 4).
 */
function onTheGrid(qty: number, what: string): Qty {
  impossible(onTick(qty), 'Law 8', `${what} is ${qty}, which is not a whole number of pieces`);
  return asQty(qty, what);
}
