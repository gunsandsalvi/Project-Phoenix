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
import type { Rate } from '../core/rate.js';
import { NO_QTY, asQty, onTick, type Qty } from '../core/tick.js';
import { none, some, type Option } from '../core/option.js';
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
  /** Law 18 (15.5): the same lines under their kind, written where a line is added. A reader asking for every loan walked every line in the world. */
  private readonly byKind = new Map<InstrumentKindId, InstrumentId[]>();
  private everything: readonly Instrument[] | undefined;
  /**
   * Law 18: HOW MANY TIMES THIS REGISTER HAS CHANGED. It is not a fact about the world and nothing
   * decides anything by it — it is how a reader whose answer is a function of these records can tell
   * that its answer still holds. An index rule walks every line in the world to say what is in it
   * and what each counts for; a real period asks one rule ten thousand times, and between most of
   * those asks nothing was issued, redeemed, split, reseated or ceased.
   *
   * It is bumped wherever `everything` is dropped, because those are the same moments: a record
   * replaced, a line added, a status changed.
   */
  private changes = 0;

  get version(): number {
    return this.changes;
  }

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
    profile.validateTerms(decl.terms, decl.issuer);
    this.registry.currency(decl.ccy);
    const unit = profile.unit(decl.ccy);
    this.registry.unit(unit);
    /**
     * XI-6: a thing whose value is what it cost, or one of itself, has nothing to clear.
     *
     * The other half of this used to be here and is GONE (item 10f.1): a cleared kind with no
     * market was refused as *"priced by clearing but names no market"*, and that is a real state
     * rather than a defect. A share of a private company is the same instrument as a share of a
     * public one and NOBODY TRADES IT, so it names no market, nothing ever clears a price of it,
     * and its holders carry it at what it cost — §29 C5, and C5.a's *"an unlisted mark is not a
     * cleared price"* is kept by there being no price to mistake for one. What answers the
     * question this forbid was asking is `Valuation.atCost`, which reads the LINE and not only its
     * kind, and every valuation in this world goes through it (Law 4).
     */
    if (profile.pricing !== 'cleared' && profile.pricing !== 'derived') {
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
    this.changes += 1;
    if (i.issuer.some) {
      const list = this.byIssuer.get(i.issuer.value);
      if (list === undefined) this.byIssuer.set(i.issuer.value, [i.id]);
      else list.push(i.id);
    }
    const ofKind = this.byKind.get(i.kind);
    if (ofKind === undefined) this.byKind.set(i.kind, [i.id]);
    else ofKind.push(i.id);
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

  /**
   * Law 18 (0g.11): AND THE RESOLVED LIST STANDS WHILE THE REGISTER HAS NOT CHANGED.
   *
   * These hold IDS and not records, for the reason the fields' own note gives: a record is replaced
   * when something is issued, redeemed, split, reseated or ceased, and an index of stale copies is
   * a second register (Law 4). So every call resolved every id through `get` and built a fresh
   * array — and once 0g.6 pointed fourteen callers at these doors, that became **92,408 of the
   * period's 388,560 instrument lookups**, the largest single caller in the engine, in the two
   * functions that were supposed to have removed the walking.
   *
   * What is kept is the RESOLVED list, dropped the instant `changes` moves — which is exactly the
   * moment a record could have been replaced, because that counter is bumped wherever `everything`
   * is dropped. It is the same construction `all()` above already is: a memo of a derivation with
   * one writer and no way to be stale, not a copy of the register.
   */
  private readonly issuedResolved = new Map<PartyId, readonly Instrument[]>();
  private readonly kindResolved = new Map<InstrumentKindId, readonly Instrument[]>();
  private resolvedAt = -1;

  private freshen(): void {
    if (this.resolvedAt === this.changes) return;
    this.issuedResolved.clear();
    this.kindResolved.clear();
    this.resolvedAt = this.changes;
  }

  /** Register B3: what one party has promised — the instruments whose issuer it is, by name. */
  issuedBy(party: PartyId): readonly Instrument[] {
    this.freshen();
    const held = this.issuedResolved.get(party);
    if (held !== undefined) return held;
    const ids = this.byIssuer.get(party);
    const made = ids === undefined ? [] : ids.map((id) => this.get(id));
    this.issuedResolved.set(party, made);
    return made;
  }

  /** Law 15, Law 18 (15.5): every line of one declared kind, live or ceased — the reader says which it wants. */
  ofKind(kind: InstrumentKindId): readonly Instrument[] {
    this.freshen();
    const held = this.kindResolved.get(kind);
    if (held !== undefined) return held;
    const ids = this.byKind.get(kind);
    const made = ids === undefined ? [] : ids.map((id) => this.get(id));
    this.kindResolved.set(kind, made);
    return made;
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
    this.changes += 1;
  }

  /**
   * Banks Lending E1, E2: the one path that writes the status. It is called by the kernel when a
   * payment this instrument promised failed and its own profile called that a default (Bond N12);
   * nothing else may. One path restores it and only one: `reterm`, where the two parties agree new
   * terms and the claim performs on those (E3, 17.7). Nothing else does, and nothing forgets it.
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
    this.changes += 1;
  }

  /**
   * Banks Lending E3, 21.59 (17.7): THE TERMS WERE RE-AGREED, and the claim is the same claim.
   *
   * A bilateral claim has two parties and they can agree new terms: a relationship that is still
   * performing is ROLLED at maturity rather than repaid and rewritten, and one that stopped
   * performing is RESTRUCTURED — more time, a different rate — rather than enforced. Both are the
   * same fact about the register: the row keeps its identity (Register F1), its issuer, its kind
   * and its holders, and what it promises from here is what the two of them just agreed.
   *
   * THE LINE PERFORMS ON THE TERMS THAT STAND. That is one sentence and not a branch: performing
   * means no promise of this claim has been broken, and the promise that was broken is not a
   * promise of this claim any more. It is the only path that restores what `markDefaulted` took
   * away, and the journal still has the miss and the re-agreement both, so nothing is forgotten —
   * what is forgiven is a redemption at what it fetched (E5) and is not this.
   *
   * The kernel is the one caller and it asks the KIND first (`profile.reagree`): what a kind refuses
   * to have changed about itself is the kind's to say, and a kind that says nothing at all cannot
   * be re-agreed. Here there are two guards only, and both are facts about the register rather than
   * about credit: a line that has ceased has nothing left to agree about, and a claim that changed
   * kind would be a different claim under one id.
   */
  reterm(id: InstrumentId, terms: Terms): void {
    const i = this.get(id);
    forbid(i.status.live, 'Banks Lending E3', `instrument ${id} has ceased; there is nothing to agree`);
    forbid(
      terms.kind === i.terms.kind,
      'Banks Lending E3',
      `${id} is a ${i.terms.kind} and would be re-agreed as a ${terms.kind}`,
    );
    const status: InstrumentStatus = { live: true, performing: true };
    this.map.set(id, Object.freeze({ ...i, terms: Object.freeze(terms), status }));
    this.everything = undefined;
    this.changes += 1;
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
    this.changes += 1;
    // B3 guarantees an issuer above, so the old one is there to move the line off.
    const was = this.byIssuer.get(i.issuer.value);
    if (was !== undefined) this.byIssuer.set(i.issuer.value, was.filter((x) => x !== id));
    const now = this.byIssuer.get(issuer);
    if (now === undefined) this.byIssuer.set(issuer, [id]);
    else now.push(id);
  }

  /**
   * Equity E3, D1, §29 D2, item 10f.2: A LINE THAT DID NOT TRADE NOW DOES.
   *
   * It is the one thing a flotation changes about the INSTRUMENT: same terms, same issuer, same
   * holders, same count — a place to sell it. Everything else a listing means is elsewhere, and
   * that is the point: the shares a private company's owners hold are the shares its public
   * shareholders hold, which is why an IPO is a sale and not a conversion.
   *
   * `delist` is the same door the other way and a take-private is what walks through it (10f.3):
   * the register is bought out, the market closes, and the firm does not die — it is owned
   * differently. Neither may be called on a line whose kind is not priced by clearing at all: a
   * loan row has nothing to list.
   */
  list(id: InstrumentId, market: MarketId): void {
    const i = this.get(id);
    forbid(i.status.live, 'Register B4', `instrument ${id} has ceased; it cannot be listed`);
    forbid(!i.market.some, 'Clearing D1', `${id} already trades in ${i.market.some ? i.market.value : ''}`);
    forbid(
      this.registry.instrumentKind(i.kind).pricing === 'cleared',
      'XI-6',
      `${id} is not priced by clearing, so a market in it would print nothing`,
    );
    this.map.set(id, Object.freeze({ ...i, market: some(market) }));
    this.everything = undefined;
    this.changes += 1;
  }

  /** Equity E3: the line stops trading and the holders keep their shares (10f.3's take-private). */
  delist(id: InstrumentId): void {
    const i = this.get(id);
    forbid(i.status.live, 'Register B4', `instrument ${id} has ceased; it does not trade`);
    forbid(i.market.some, 'Clearing D1', `${id} has no market to close`);
    this.map.set(id, Object.freeze({ ...i, market: none<MarketId>() }));
    this.everything = undefined;
    this.changes += 1;
  }

  /**
   * Register E4, Equity D4: the count of a line changes and nothing else does. Only the split door
   * calls it, with the register restated in the same operation; what each holder holds and what it
   * cost them is unchanged in value, so no equity moves and no money does either — which is the
   * "explicitly says why not" E5 asks of an event that moves a register without moving money.
   */
  /**
   * Bond N5.b (17.1): THE COUPON A FLOATING LINE CARRIES FOR THE ACCRUAL PERIOD IT HAS JUST ENTERED.
   * A floating note promises a margin over a named reference, and what it owes is settled when that
   * reference FIXES — so the terms carry the rate in force and every reader of the schedule (what
   * falls due, what has accrued, what it is worth) reads one number rather than each re-deriving a
   * fixing (Law 4, Law 19). What it paid before is settled and in the ledger; nothing re-derives it.
   */
  fixCoupon(id: InstrumentId, coupon: Rate): void {
    const i = this.get(id);
    forbid(i.status.live, 'Register B4', `instrument ${id} has ceased; its coupon cannot fix`);
    const terms = i.terms;
    forbid(
      'coupon' in terms,
      'Bond N5.b',
      `instrument ${id} promises no coupon, so there is nothing to fix`,
    );
    this.map.set(id, Object.freeze({ ...i, terms: Object.freeze({ ...terms, coupon }) }));
    this.changes += 1;
  }

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
    this.changes += 1;
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
    this.changes += 1;
  }
}

/** The read-only face of the instruments store, for mechanisms and participants. */
export type InstrumentsReads = Pick<Instruments, 'has' | 'get' | 'all' | 'issuedBy' | 'ofKind' | 'version'>;

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
