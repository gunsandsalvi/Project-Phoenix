/**
 * Securitisation: a loan leaves a bank's book, and a real party takes it.
 *
 * @spec XI-11 XI-8 XI-1 Banks Lending D4 Banks Capital B1.a Banks Capital B1.b Banks Capital B3 Housing C6 Clearing C3 Clearing D1 Law 3 Law 4 Law 6 Law 9 Law 15 Law 19
 *
 * XI-11 IS THE WHOLE CLAUSE: THERE IS NO RISK TRANSFER WITHOUT A TRANSFEREE. A world where a bank
 * "moves risk off its balance sheet" and nobody is named as having taken it has not moved anything
 * — it has written a smaller number down. So a sale here is a SALE: the rows move in the register
 * to a party that exists, has a balance sheet, collects the payments, and bears the losses.
 *
 * WHAT A VEHICLE IS (C1): a named party that holds loan rows and has no decisions except its
 * waterfall. It has no employees, no strategy and no view; it takes in what the borrowers pay and
 * passes it out by seniority. Everything that could be a decision was taken before it existed.
 *
 * WHERE THE ATTACHMENT COMES FROM (C2.a, Law 3): IT IS CLEARED, NOT STATED, AND NOBODY CHOSE IT.
 * The arranger sells no more than it must: its relief is what LEFT its book, so a bank short of a
 * given amount of room offers exactly that much senior paper — a size and no level (Clearing C3) —
 * and keeps the rest, because every unit it sells beyond its need is income it gave away for
 * nothing. That is C4.a, and it is not a rule about retention: it is what a bank does when selling
 * has a cost. The pool is whole rows and a row does not divide, so the overshoot lands in the
 * junior too. So the boundaries are stated on the instrument, as C2.a requires, and the number they
 * state is an outcome of a shortfall and a lumpy book. A world that declared "the junior is 8%"
 * would have imported the answer (Law 2).
 *
 * WHY A BANK DOES IT (D1, D2): it owes money sooner than its book pays it. A loan is a promise to be
 * paid over years and a deposit is a promise to pay today, and a bank caught between the two has one
 * honest answer that is not the central bank: sell the promise. Its capital falls because THE ROW
 * LEFT — not because a weight changed — which is D2 as an outcome rather than a rule.
 *
 * WHAT A LOSS DOES (C2, C6): a write-off inside the pool is taken by the bottom tranche first, up to
 * its detachment, and then by the one above it. Σ tranche face equals the pool's face after every
 * event, exactly, and the audit says so rather than the code assuming it. D4's senior losses are
 * what happens when the junior is not deep enough, and nothing anywhere stops that.
 */
import { dayNumber, type Civil } from '../../calendar/civil.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { levelsUpTo, rungsUpTo } from '../../clearing/schedule.js';
import { priceAt } from '../../prices/curve.js';
import { securedOn, securedOnSaid } from '../../registry/secured.js';
import { InvalidRegistry } from '../../core/errors.js';
import { percent } from '../../core/format.js';
import {
  agreementKindId,
  type AgreementId,
  currencyUnit,
  instrumentId,
  instrumentKindId,
  marketId,
  paramId,
  partyId,
  partyKindId,
  venueId,
  type CurrencyCode,
  type InstrumentId,
  type MarketId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import type { Agreement, AgreementTerms } from '../../register/agreements.js';
import { atMost, div, sum, zeroIfNone, addTo } from '../../core/num.js';
import { addQty, downTick, NO_QTY, type Qty, subQty } from '../../core/tick.js';
import {
  asRatio,
  heldAsMoney,
  minus,
  plus,
  asPerPiece,
  pricedAt,
  type Cash,
  type PerPiece,
  type Ratio,
  ratioOf,
  scale,
  sumCash,
  valueAt,
  asAmount,
} from '../../core/measure.js';
import { none, some, type Option } from '../../core/option.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import type { Instrument, Terms } from '../../register/instruments.js';
import type { CashFlow, InstrumentKindProfile, PartyKindProfile } from '../../registry/kinds.js';
import { FACE_TICK } from '../../registry/grid.js';
import { BANK } from '../../registry/profiles.js';
import type { Violation } from '../../audit/audit.js';
import type { Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import {
  capitalPublished,
  costOfFundsIn as fundingCostIn,
  limitPerName,
} from '../../registry/banking.js';

export const VEHICLE = partyKindId('vehicle');
export const TRANCHE = instrumentKindId('tranche');

/** C1: a deal is named for who arranged it and which one it was (Law 9). */
export const vehicleId = (arranger: PartyId, n: number): PartyId => partyId(`spv.${arranger}.${n}`);

/** C2: a tranche is named by its vehicle and where it sits. Not by an internal id (Law 9). */
export const trancheId = (vehicle: PartyId, layer: 'senior' | 'junior'): InstrumentId =>
  instrumentId(`tranche:${vehicle}:${layer}`);

export const dealVenue = (vehicle: PartyId): VenueId => venueId(`securitisation:${vehicle}`);

/** Clearing A2: how many levels of its own curve a bidder posts. A RESOLUTION (Law 2). */
export const DEAL_STEPS = paramId('securitisation.demand.steps');

export interface TrancheTerms extends Terms {
  readonly kind: typeof TRANCHE;
  readonly vehicle: PartyId;
  /**
   * C2.a: WHERE IT STARTS TAKING LOSSES and where it stops, as fractions of the pool it was cut
   * from. Both are stated on the instrument because a holder has to be able to read what it is
   * exposed to; both are outcomes of what cleared, because what a market would take is not a
   * number anybody may write down (Law 3).
   */
  readonly attachment: Ratio;
  readonly detachment: Ratio;
  /** XI-8: what the waterfall orders by. Smaller is paid first, as everywhere else. */
  readonly seniority: number;
  /** C6: the pool's face at the moment the deal was cut, which is what the fractions are OF. */
  readonly pool: Qty;
}

export const isTranche = (t: Terms): t is TrancheTerms =>
  'vehicle' in t && 'attachment' in t && 'detachment' in t;

export function trancheTerms(i: Instrument): TrancheTerms {
  if (!isTranche(i.terms)) {
    throw new InvalidRegistry('XI-11', `${i.id} is not a tranche`);
  }
  return i.terms;
}

export const vehicleKind: PartyKindProfile = {
  /** item 15: what somebody else set it up to do, and it does not get to change it. */
  objective: 'itsMandate',
  id: VEHICLE,
  representation: 'named',
  moneyIssuer: null,
  /**
   * XI-3, Banks Capital B1.a: IT CAN FAIL, and saying otherwise would have been the whole trick.
   *
   * A vehicle owes its notes their face and pays them out of a pool of loans that can go wrong, so
   * a holder of its paper can be left short — which is a cash failure and, when the pool is worth
   * less than the notes, a solvency one. The temptation was to call it bankruptcy-remote and write
   * nothing here, because it has no other business to be brought down by; but `fails` is not about
   * whether a party has other business, it is about whether a claim on it can go wrong.
   *
   * Left empty, every capital rule in this world would have weighed a tranche at NOTHING — and a
   * bank could then sell its book to a vehicle, buy the notes back, and watch its requirement
   * vanish with the risk still on its own balance sheet. That is regulatory arbitrage by
   * construction and it is exactly what XI-11 exists to forbid.
   */
  fails: ['cash', 'solvency'],
  borrows: false,
  buysOnTerms: false,
  // 17.8: XI-11, §42 C1: it exists to HOLD the rows — a named party the loans are transferred into, funded by notes its holders bought. It is how credit risk reaches an investor without the row itself leaving the banking system.
  banking: true,
  // Banks Funding E1: it does not shop for a bank. It banks where its arranger banks, because that
  // is who set it up, and it is over when the pool is.
  depositClass: null,
};

export const trancheKind: InstrumentKindProfile = {
  id: TRANCHE,
  // C3: EVERY TRANCHE HAS A MARKET and its price is what somebody paid. The yield a reader wants is
  // derived from that price (Law 3); there is no spread table and no rating-implied level anywhere.
  pricing: 'cleared',
  // Law 8: A NOTE IS QUOTED PER UNIT OF FACE, like every other claim to a sum of money, so the
  // smallest step in its price is the smallest step in that — a bond's grid, not a share's.
  priceTick: FACE_TICK,
  /**
   * C2, Law 4: AND ITS HOLDER CARRIES IT AT WHAT IT PAID. A note's loss is the pool writing down
   * beneath it — an event on a date, allocated by attachment (C2, C6, XI-1) — and marking it to a
   * thin book every period as well would be two systems for one fact. It is a credit instrument and
   * it is carried the way every other credit instrument here is carried; what a market says it is
   * worth is a print, and a print is what C3 wanted.
   */
  carry: 'cost',
  liabilityOfIssuer: true,
  // Register B3: a note is owed its face and loses it only when the POOL loses it — which is the
  // write-down below, an event on a date, and never a re-mark of what somebody would pay today.
  owes: 'face',
  unit: (ccy) => currencyUnit(ccy),
  validateTerms: (t) => {
    if (!isTranche(t)) throw new InvalidRegistry('XI-11', 'not tranche terms');
    if (t.attachment >= t.detachment) {
      throw new InvalidRegistry(
        'XI-11',
        'a tranche that detaches where it attaches is not a layer',
      );
    }
  },
  displayName: (i) =>
    isTranche(i.terms)
      ? `${String(i.terms.vehicle)} ${i.terms.seniority === 0 ? 'senior' : 'junior'} ` +
        `${percent(i.terms.attachment)}-${percent(i.terms.detachment)}`
      : String(i.id),
  ranking: (i) => ({
    seniority: isTranche(i.terms) ? i.terms.seniority : 0,
    secured: [],
    // C2: it is secured on the pool in the sense that the pool is all there is; there is no other
    // asset to reach, so naming one would be naming something that does not exist.
    claim: 'a layer of a pool, paid in its turn out of what the borrowers paid',
  }),
  /**
   * C5, Law 17, item 8.2: A TRANCHE PAYS WHAT THE POOL PAID, WHEN THE POOL PAID IT, and there is no
   * schedule to promise. This is a pass-through and what it passes through is an OUTCOME.
   *
   * The plan asked for real `cashFlows` and `due` here so a noteholder's yield could be derived from
   * them. It cannot be, and building them would be the defect rather than the fix: what a
   * pass-through will pay depends on what borrowers who have not paid yet do, so a schedule would be
   * a FORECAST with no falsification test (Law 17) — and `due` would make the KERNEL pay a coupon
   * this module's own waterfall is already paying, which is one fact with two writers (Law 4).
   *
   * What the plan was actually after was that a noteholder should EARN something, and it does now:
   * `distribute` separates interest from principal and pays the interest out by seniority (`B-8`).
   * Its yield is what those payments came to against the price it paid, which is a measurement of
   * what happened (item 23) and never an input to a price (C3, Law 3).
   */
  cashFlows: (): readonly CashFlow[] => [],
  due: () => [],
  accrued: () => 0,
};

/** What a party holds in money of a currency, summed over the accounts it banks in. */
function cashOf(ctx: MechanismContext, who: PartyId, ccy: CurrencyCode): Qty {
  const amounts: Qty[] = [];
  for (const h of ctx.register.holdingsOf(who)) {
    const i = ctx.instruments.get(h.instrument);
    if (ctx.registry.instrumentKind(i.kind).pricing !== 'money' || i.ccy !== ccy) continue;
    amounts.push(ctx.register.quantity(who, i.id));
  }
  return sum(amounts).value;
}

/* --------------------------------------------------------------------------------------------
 * THE DEAL
 * ------------------------------------------------------------------------------------------ */

/**
 * XI-11, Law 4, Law 19: THE NEXT FREE VEHICLE NAME FOR THIS ARRANGER, asked of the world rather
 * than of a counter this module keeps. The parties store is the one writer of who exists, so it is
 * the one that can say which name is taken (item 9.1).
 */
function freeVehicle(ctx: MechanismContext, bank: PartyId): PartyId {
  for (let n = 1; ; n += 1) {
    const id = vehicleId(bank, n);
    if (!ctx.parties.has(id)) return id;
  }
}

/**
 * XI-11, C4.a, item 9.1: A DEAL IS AN AGREEMENT BETWEEN THE VEHICLE AND ITS ARRANGER.
 *
 * The vehicle owes the arranger what is left when the notes are paid (C4.a: the arranger keeps the
 * bottom, which is what makes E3's *no risk transfer without a transferee* true rather than
 * vacuous), so the vehicle is the debtor and the arranger the creditor.
 *
 * It carries ONE fact, because one fact is all a deal has that the world does not already hold:
 * WHICH ROWS LEFT THE ARRANGER'S BOOK. The vehicle, the layers, the money and the pool's face are
 * every one of them reads — the party, what it issued, what those are denominated in, and the
 * `pool` on each tranche's own terms — and keeping a second copy of them beside the deal was the
 * mirror Law 19 is about. What is sold in cannot be read back once a row has run off the vehicle's
 * book, and XI-11's traceability is exactly the question of which loans left whose book.
 */
export const DEAL = agreementKindId('securitisation.deal');

export interface DealTerms extends AgreementTerms {
  readonly kind: typeof DEAL;
  /** XI-11, E3: the rows the arranger sold in. The only fact here nothing else records. */
  readonly sold: readonly InstrumentId[];
}

/** Law 15: structural — what makes these terms a deal is that they name what was sold into it. */
export const isDeal = (t: AgreementTerms): t is DealTerms => 'sold' in t;

/** One deal as this module reads it: the two named parties, and everything else derived. */
interface Deal {
  readonly id: AgreementId;
  readonly vehicle: PartyId;
  readonly arranger: PartyId;
  readonly sold: readonly InstrumentId[];
}

function dealOf(a: Agreement): Deal {
  if (!isDeal(a.terms)) throw new InvalidRegistry('XI-11', `${a.id} is not a deal`);
  return { id: a.id, vehicle: a.debtor, arranger: a.creditor, sold: a.terms.sold };
}

const dealsOpen = (ctx: MechanismContext): readonly Deal[] =>
  ctx.agreements
    .ofKind(DEAL)
    .filter((a) => a.state === 'performing')
    .map(dealOf);

/**
 * XI-8, Law 19: THE LAYERS, IN THE ORDER THEY ARE PAID — what this vehicle issued, ordered by the
 * seniority on each one's own terms. It used to be the insertion order of a stored list, which
 * says the same thing only while nothing is ever issued out of order.
 */
function layersOf(ctx: MechanismContext, vehicle: PartyId): readonly Instrument[] {
  return ctx.instruments
    .issuedBy(vehicle)
    .filter((i) => isTranche(i.terms))
    .sort((a, b) => trancheTerms(a).seniority - trancheTerms(b).seniority);
}

/** Law 8: the money a deal is in, which is what it issued its layers in. */
function ccyOfDeal(ctx: MechanismContext, deal: Deal): CurrencyCode | undefined {
  return layersOf(ctx, deal.vehicle)[0]?.ccy;
}

/**
 * D1, D2: WHAT A BANK WOULD SELL — anything it is OWED that it cannot simply sell, in the money it
 * is short of, and that it can hand over free and clear (a row already pledged to somebody else is
 * not its to sell, Register D5.a).
 *
 * THIS GATED ON `isLoan`, WHICH WAS A KIND BRANCH IN A MECHANISM (Law 15) and also a narrower
 * world than the one it models: a bank securitises whatever it holds that nobody will make a market
 * in, and item 11 is about to fill this world with small firms whose invoices are exactly that.
 * What replaces it is not a longer list but TWO FACTS EVERY KIND ALREADY DECLARES:
 *
 *  - `pricing === 'carriedAtCost'` — NO MARKET EXISTS FOR IT. That is the whole of what the tag
 *    means and every kind carrying it says so in its own words. If a thing had a market the bank
 *    would sell it and need no vehicle at all.
 *  - `liabilityOfIssuer` — A NAMED PARTY OWES IT. Inventory says `false` (*"a tonne is nobody's
 *    promise"*) and drops out on its own: there is no stream of payments to tranche.
 *
 * The two together are the definition of securitisable, and that is not a coincidence — it is what
 * a securitisation is FOR. Neither is a property this module invents or maintains; both are read
 * off the kind's own profile, so a kind that gains a market (item 10d does this to a bank's
 * subordinated debt) stops being securitisable the same day, with nothing here to edit.
 */
export function saleable(view: ParticipantView, ccy: CurrencyCode): readonly Instrument[] {
  const out: Instrument[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!i.status.live || i.ccy !== ccy) continue;
    const profile = view.registry.instrumentKind(i.kind);
    if (profile.pricing !== 'carriedAtCost' || !profile.liabilityOfIssuer) continue;
    if (h.liens.length > 0) continue;
    if (view.free(h.instrument) <= 0) continue;
    out.push(i);
  }
  return out;
}

/**
 * D1, D2: THE TWO REASONS A BANK BRINGS A DEAL, AND THEY ARE NOT THE SAME REASON.
 *
 * NEED is the one this module was built for: the bank is below its capital line, it must shrink,
 * and it takes what the book gives it — at a loss if a loss is what is there, because a bank that
 * refused a bad price would not be shrinking. It posts a size and NO LEVEL (Clearing C3).
 *
 * DEMAND is the one it was missing, and without it a bank that could sell a pool for more than it
 * carries it at simply did not — so no deal in this world ever happened because somebody WANTED the
 * paper, only because somebody had to shed it. Here the bank has no need, so a worse price is
 * simply a deal it does not do: it posts its CARRYING VALUE as a level and sells what clears above
 * it. That level is not a floor on an outcome (Law 6) — it is the alternative it already has, which
 * is to keep the rows and be paid on them, the same construction an issuer's walk-away is.
 *
 * One mechanism, two entry conditions, and the record says which brought each deal.
 */
interface Reason {
  /** Which of D1's two it is, published on the deal so a reader can tell them apart. */
  readonly why: 'need' | 'demand';
  /** How much face it brings. What it must shed, or everything it could sell. */
  readonly wants: Qty;
  /** The least it will take per unit, where it has a choice. Absent is C3's size-and-no-level. */
  readonly at: Option<PerPiece>;
}

function reasonToSell(
  view: ParticipantView,
  rows: readonly Instrument[],
  pool: Qty,
): Option<Reason> {
  const gap = shortBy(view);
  if (gap > 0) return some({ why: 'need' as const, wants: gap, at: none<PerPiece>() });
  // D1: it is not short, so it sells only above what it is carrying the rows at — and it has to be
  // able to say what that is. A bank that cannot value its own book does not bring a deal.
  const carried = carryingOf(view, rows);
  if (!carried.some || pool <= 0) return none<Reason>();
  return some({
    why: 'demand' as const,
    wants: pool,
    at: some(pricedAt(carried.value, pool, 'what it is carrying the pool at, per unit of face')),
  });
}

/**
 * Law 19: WHAT THE BANK IS CARRYING THESE ROWS AT, read off its own marks and never recomputed.
 * A row it cannot value is a row it cannot say it would profit by selling, so the answer is
 * Missing rather than a zero that would make every deal look like a gain.
 */
function carryingOf(view: ParticipantView, rows: readonly Instrument[]): Option<Cash> {
  const first = rows[0];
  if (first === undefined) return none<Cash>();
  const out: Cash[] = [];
  for (const i of rows) {
    const at = view.mark(i.id);
    if (!at.some) return none<Cash>();
    out.push(valueAt(at.value, view.quantity(i.id), i.ccy, 'what it carries this row at'));
  }
  return some(sumCash(first.ccy, out, 'what it carries these rows at').value);
}

/**
 * D1, D2: THE REASON, and it is a real one: THE BANK HAS RUN OUT OF ROOM.
 *
 * A bank's own capital position is a thing it publishes every period — what it holds, what its own
 * rules require of it, and the headroom between them (Banks Capital B3). This reads that number off
 * the wire, because a bank's room is a bank's fact and there is exactly one writer of it (Law 4,
 * Law 19). Nothing here recomputes a requirement or reaches into another module for one.
 *
 * A bank below its line has three answers and this world already has two: raise capital (Banks
 * Capital C2), which somebody has to be willing to provide, and lend less. The third is to SHRINK —
 * and shrinking by selling rows is exactly the thing XI-11 is about. What it needs to shed is the
 * shortfall divided by the capital its own rule asks per unit of assets: arithmetic from two
 * published numbers, not a coefficient anybody chose.
 *
 * A bank with room sells nothing, because the shortfall is not positive. That is arithmetic, not a
 * threshold (Law 6).
 */
function shortBy(view: ParticipantView): Qty {
  const published = capitalPublished(view, String(view.self.id));
  return published.some ? faceToShed(published.value) : NO_QTY;
}

/**
 * D1, D2, B1.b: HOW MUCH OF ITS BOOK HAS TO LEAVE, from the three numbers a bank publishes about
 * itself every period. Separated from the read so it can be asked directly: what it decides is
 * arithmetic on published facts, and arithmetic is a thing a test can put a case to.
 */
export function faceToShed(p: {
  readonly headroom: number;
  readonly minWeighted: number;
  readonly binds: string;
}): Qty {
  if (p.headroom >= 0 || p.minWeighted <= 0) return NO_QTY;
  /**
   * AND ONLY IF THE RULE THIS WOULD RELIEVE IS THE ONE THAT BINDS.
   *
   * A sale at a price is a swap: the rows go out and money of the same value comes in, so the
   * bank's TOTAL assets do not move and its leverage ratio does not move with them. What falls is
   * the WEIGHTED total, because a loan weighs one and the money it was sold for weighs nothing —
   * which is why securitisation relieves a risk-weighted requirement and relieves a leverage one by
   * exactly zero. A bank bound by leverage that sold its book anyway would have shed its entire
   * business for no relief at all, which is not a trade any bank takes.
   *
   * This is a READ of the bank's own published position (B3), not a rule stated here: which of its
   * two requirements binds is its own fact and it says so every period (Law 4, Law 19).
   */
  if (p.binds !== 'weighted') return NO_QTY;
  // D2: its capital is freed because the ROW LEFT, not because a weight changed. This is the
  // shortfall divided by the capital its own rule asks per unit of weighted assets — two published
  // numbers and a division, and nothing here is a coefficient anybody chose.
  return asAmount<'piece'>(
    div(-p.headroom, p.minWeighted, 'the face it must shed for the room it is short of'),
    'the face it must shed',
  );
}

/**
 * C1, C2, D1, XI-11: THE ARRANGEMENT. Every bank that owes money sooner than its book pays it, in
 * the order the world holds them, offers rows it can hand over free and clear.
 *
 * It offers the pool as senior paper and takes what the book gives it, which can be nothing: there
 * is no forced buyer anywhere in this (Appendix B). What clears is the senior tranche; the junior is
 * the rest and the arranger keeps it (C4.a). Then the rows move, the cash moves and the junior is
 * issued — one instruction, every leg in it, so there is no instant where the pool belongs to
 * nobody (XI-5).
 */
export function arrange(ctx: MechanismContext): void {
  for (const bank of ctx.parties.ofKind(BANK)) {
    if (!bank.status.alive) continue;
    // Money A1, XI-12 (11.4): THE BANK'S OWN MONEY — the currency of the region it issues in. It
    // read the registry's first currency for every bank in the world, so in a world with four a
    // bank abroad sold rows in a money it does not issue and its notes were bid for by nobody.
    const ccy = ctx.registry.currencyOf(bank.region);
    const view = ctx.participant(bank.id);
    const rows = saleable(view, ccy);
    if (rows.length === 0) continue;
    const whole = sum(rows.map((i) => view.quantity(i.id))).value;
    const reason = reasonToSell(view, rows, whole);
    if (!reason.some) continue;
    const { why, wants, at } = reason.value;
    const taken: InstrumentId[] = [];
    const faces: Qty[] = [];
    let got = NO_QTY;
    for (const i of rows) {
      if (got >= wants) break;
      const face = view.quantity(i.id);
      if (face <= 0) continue;
      taken.push(i.id);
      faces.push(face);
      got = addQty(got, face, 'pool face');
    }
    const pool = sum(faces).value;
    if (pool <= 0) continue;
    const vehicle = freeVehicle(ctx, bank.id);
    // C4: WHO WOULD BUY A NOTE OF IT. Each is asked with its own view and answers out of its own
    // money; none of them sees this bank's book, and none of them has to bid (Observer A4).
    const schedule = poolSchedule(view, taken, faces, pool);
    const bids: Order[] = [];
    for (const other of ctx.parties.ofKind(BANK)) {
      if (!other.status.alive || other.id === bank.id) continue;
      bids.push(...noteBids(ctx.participant(other.id), ccy, schedule));
    }
    if (bids.length === 0) {
      // A deal nobody would buy a note of is a deal that does not happen, and the bank is exactly
      // where it was. Recorded, because a refusal is an answer (Law 1).
      ctx.record(
        'securitisation.failed',
        [bank.id],
        { arranger: String(bank.id), pool, outcome: 'noDemand', why },
        true,
      );
      continue;
    }
    cut(ctx, { arranger: bank.id, ccy, vehicle, rows: taken, pool, offered: wants, at, why, bids });
  }
}

/**
 * C2, C2.a, C3, C4.a: WHAT THE BOOK SAID. The arranger posts the pool's whole face and no level;
 * the bids clear at one price; the senior tranche is what cleared and the junior is the remainder.
 */
/**
 * §42 C6, Law 9, Law 19 (17g.2): WHAT THIS DEAL IS MADE OF, read off the rows that went into it.
 *
 * Not a label the arranger types and not a field anybody stores: every row says what is pledged
 * behind it and every pledged thing has a kind, so what a pool is secured on is arithmetic over the
 * register. A note on houses and a note on shops are different things to whoever holds one, and
 * until this the world could not tell them apart.
 */
function behind(ctx: MechanismContext, rows: readonly InstrumentId[]): readonly { readonly on: string; readonly rows: number }[] {
  return securedOn(
    rows.map((id) => ctx.instruments.get(id)),
    (id) => String(ctx.instruments.get(id).kind),
    (i) => ctx.registry.instrumentKind(i.kind).ranking(i),
  );
}

function cut(
  ctx: MechanismContext,
  d: {
    arranger: PartyId;
    ccy: CurrencyCode;
    vehicle: PartyId;
    rows: readonly InstrumentId[];
    pool: Qty;
    /** D2: what it needs OFF its book. It sells this much and not a unit more (C4.a). */
    offered: Qty;
    /** D1: the least it will take, where it HAS a choice. Absent is C3's size and no level. */
    at: Option<PerPiece>;
    why: 'need' | 'demand';
    bids: readonly Order[];
  },
): void {
  const venue = dealVenue(d.vehicle);
  if (!ctx.venues.some((v) => v.id === venue)) {
    ctx.openVenue({
      id: venue,
      // Law 9: a deal is named for the vehicle that IS it, which is the name the world already
      // carries. It used to be named for a counter this module kept beside its own book.
      name: `${String(d.vehicle)} pool`,
      clearedBy: 'securitisation',
      unit: currencyUnit(d.ccy),
      ccy: d.ccy,
      key: { arranger: String(d.arranger), deal: String(d.vehicle) },
    });
  }
  for (const b of d.bids) ctx.post(venue, b);
  // Clearing C3, C4.a: A SIZE AND NO LEVEL, and the size is what it is short of — never more.
  // Selling past its need would be giving away income to no purpose, and it cannot sell past the
  // pool either, because a bank cannot hand over rows it does not have.
  const offering = downTick(atMost(d.offered, d.pool, 'it cannot sell rows it does not hold'));
  if (offering <= 0) return;
  // D1: a bank that MUST shrink posts no level and wears whatever the book gives it; one that need
  // not posts what it is carrying the rows at, because keeping them and being paid on them is the
  // alternative it already has. The two reasons differ in exactly this one thing.
  ctx.post(venue, {
    party: d.arranger,
    side: 'sell',
    price: d.at.some ? d.at.value : 'market',
    qty: offering,
  });
  const outcome = clear(ctx.posted(venue), 'proRata', 'marginalBid');
  if (!isCleared(outcome)) {
    ctx.record(
      'securitisation.failed',
      [d.arranger],
      { arranger: d.arranger, pool: d.pool, outcome: outcome.kind, why: d.why },
      true,
    );
    return;
  }
  const seniorFace = downTick(
    sum(outcome.fills.filter((f) => f.side === 'buy').map((f) => f.qty)).value,
  );
  if (seniorFace <= 0) return;
  /**
   * C2.a, C4.a: WHAT IT KEPT — the room it did not need to sell, plus the part of the last row that
   * would not divide. HOW MANY LAYERS A DEAL HAS IS AN OUTCOME TOO: a bank that had to sell
   * everything keeps nothing, and what the buyers hold is then a pass-through over the whole pool
   * with nobody underneath them, which is a real instrument and an honest answer. A junior is not
   * invented so that there can be one.
   */
  const juniorFace = subQty(d.pool, seniorFace, 'what it kept');
  const attachment = ratioOf(juniorFace, d.pool, 'where the senior starts taking losses');
  const arrangerParty = ctx.parties.get(d.arranger);
  ctx.enter({
    id: d.vehicle,
    kind: VEHICLE,
    region: arrangerParty.region,
    name: `${String(d.vehicle)} pool`,
    bank: d.arranger,
    representation: 'named',
    status: { alive: true, standing: 'good' },
  });
  const senior = trancheId(d.vehicle, 'senior');
  const on = securedOnSaid(behind(ctx, d.rows));
  issueTranche(
    ctx,
    d.vehicle,
    d.ccy,
    senior,
    {
      kind: TRANCHE,
      vehicle: d.vehicle,
      attachment,
      detachment: asRatio(1, 'up to the whole of the pool'),
      seniority: 0,
      pool: d.pool,
    },
    on,
  );
  const junior = juniorFace > 0 ? some(trancheId(d.vehicle, 'junior')) : none<InstrumentId>();
  if (junior.some) {
    issueTranche(
      ctx,
      d.vehicle,
      d.ccy,
      junior.value,
      {
        kind: TRANCHE,
        vehicle: d.vehicle,
        attachment: asRatio(0, 'from the first loss'),
        detachment: attachment,
        seniority: 1,
        pool: d.pool,
      },
      on,
    );
  }
  if (!settleDeal(ctx, d, outcome.price, senior, junior, seniorFace, juniorFace)) return;
  // XI-11, C4.a: the deal is the relation between the vehicle and its arranger, and the vehicle
  // owes it what is left when the notes are paid. It owes nothing yet — the residual is what a
  // wind-up finds — which is why the store admits a zero (item 9.1a).
  const terms: DealTerms = { kind: DEAL, sold: d.rows };
  ctx.owes({
    debtor: d.vehicle,
    creditor: d.arranger,
    ccy: d.ccy,
    owed: 0,
    terms,
    why: `${d.arranger} sold ${d.rows.length} rows into ${d.vehicle} and kept the bottom`,
  });
  ctx.record(
    'securitisation.cut',
    [d.arranger, d.vehicle],
    {
      arranger: String(d.arranger),
      vehicle: String(d.vehicle),
      rows: d.rows.length,
      // 17g.2: AND WHAT THEY ARE SECURED ON, counted off the rows themselves — houses, shops, or
      // nothing. "A pool of loans" is not a description anybody can price (§42 C6, Law 9).
      securedOn: on,
      pool: d.pool,
      senior: seniorFace,
      junior: juniorFace,
      layers: junior.some ? 2 : 1,
      attachment,
      price: outcome.price,
      // D1 (item 10c): which of the two reasons brought this deal — a bank that had to shrink, or
      // one that was offered more than it was carrying the rows at. They are different events and
      // a reader that could not tell them apart would read a healthy market as a wave of distress.
      why: d.why,
    },
    true,
  );
}

/**
 * C3: EVERY TRANCHE HAS A MARKET, and it is opened with the tranche because a note nobody can
 * trade is a note nobody can price, and what a tranche is worth after the deal is what somebody
 * will pay for it — the same rule as everything else here. (The kernel no longer refuses a cleared
 * line with no market: a private company's shares are one, and they are carried at cost, §29 C5.
 * This is a decision of the deal's and not a rule the register enforces.)
 */
function issueTranche(
  ctx: MechanismContext,
  vehicle: PartyId,
  ccy: CurrencyCode,
  id: InstrumentId,
  terms: TrancheTerms,
  /** 17g.2, Law 9: what the pool under it is secured on, in the words a market would use. */
  on: string,
): void {
  const market = trancheMarket(id);
  ctx.issue({ id, kind: TRANCHE, issuer: some(vehicle), ccy, terms, market: some(market) });
  ctx.openMarket({
    id: market,
    name: `${String(vehicle)} ${terms.seniority === 0 ? 'senior' : 'junior'} notes, ${on}`,
    instrument: id,
    ccy,
    rationing: 'proRata',
  });
}

/** Law 9: a market is named for the note it trades, and a note for the pool it is cut from. */
export const trancheMarket = (note: InstrumentId): MarketId => marketId(`market:${note}`);

/**
 * XI-5, XI-11, Law 5: THE SALE, and every leg of it in one numbered instruction. The rows go to the
 * vehicle; the buyers' money goes to the arranger; the senior notes go to the buyers; the junior
 * goes to the arranger, which is the rest of what it was paid. Nothing is left over and there is no
 * instant where the pool belongs to nobody.
 */
function settleDeal(
  ctx: MechanismContext,
  d: {
    arranger: PartyId;
    ccy: CurrencyCode;
    vehicle: PartyId;
    rows: readonly InstrumentId[];
    pool: Qty;
    bids: readonly Order[];
  },
  price: PerPiece,
  senior: InstrumentId,
  junior: Option<InstrumentId>,
  seniorFace: Qty,
  juniorFace: Qty,
): boolean {
  const legs: Leg[] = [];
  for (const row of d.rows) {
    const qty = ctx.register.quantity(d.arranger, row);
    if (qty <= 0) continue;
    legs.push({
      kind: 'asset',
      from: d.arranger,
      to: d.vehicle,
      instrument: row,
      qty,
      pricePerUnit: some(price),
      accruedPerUnit: none(),
    });
  }
  if (legs.length === 0) return false;
  // C4: the notes go to NAMED holders, and each pays for what it took.
  let placed = NO_QTY;
  for (const b of d.bids) {
    if (b.side !== 'buy') continue;
    const want = downTick(atMost(b.qty, subQty(seniorFace, placed, 'left to place'), 'its fill'));
    if (want <= 0) continue;
    const cash = downTick(
      valueAt(price, want, ctx.instruments.get(senior).ccy, 'what it pays for the note').pieces,
    );
    if (cash <= 0) continue;
    legs.push({
      kind: 'asset',
      from: d.vehicle,
      to: b.party,
      instrument: senior,
      qty: want,
      pricePerUnit: some(price),
      accruedPerUnit: none(),
    });
    legs.push({
      kind: 'money',
      // 0i.5: C4: what the investor pays the arranger for the notes.
      receipt: { of: 'disposal' },
      from: ctx.accountOf(b.party, d.ccy),
      to: ctx.accountOf(d.arranger, d.ccy),
      ccy: d.ccy,
      amount: cash,
    });
    placed = addQty(placed, want, 'placed');
  }
  // C4.a: and the arranger keeps the bottom, which is the rest of the price of its own pool. A bank
  // that sold the whole pool kept nothing and there is no leg for it.
  if (junior.some && juniorFace > 0) {
    legs.push({
      kind: 'asset',
      from: d.vehicle,
      to: d.arranger,
      instrument: junior.value,
      qty: downTick(juniorFace),
      pricePerUnit: some(price),
      accruedPerUnit: none(),
    });
  }
  const r = ctx.settle({
    legs,
    cause: 'transfer',
    reason: `${String(d.arranger)} sells ${d.rows.length} rows into ${String(d.vehicle)}`,
  });
  return r.outcome === 'settled';
}

/* --------------------------------------------------------------------------------------------
 * THE WATERFALL
 * ------------------------------------------------------------------------------------------ */

/**
 * C5, XI-8: WHAT THE VEHICLE COLLECTED GOES OUT BY SENIORITY. The kernel's own corporate-action
 * phase already paid the vehicle whatever the borrowers paid — it is the holder of record of every
 * row (Law 19) — so this reads its cash and passes it on, senior first and junior with what is
 * left. A vehicle that collected nothing pays nothing; there is no buffer and nothing is smoothed.
 */
export function distribute(ctx: MechanismContext): void {
  for (const deal of dealsOpen(ctx)) {
    /**
     * XI-8, Money E4: A VEHICLE THAT HAS CEASED IS ITS ESTATE'S BUSINESS. It can fail like anything
     * else here — it owes its notes and pays them out of loans that can go wrong — and when it does,
     * an estate opens, assumes its paper and winds the pool up under the ONE waterfall this world
     * has. There is nothing left for this module to instruct, and the kernel says so plainly: a
     * ceased party's legs are settled by whoever succeeded it, and addressing the dead party
     * directly is a defect in the module that did it.
     *
     * So the deal leaves the book, once, with the fact recorded. It is not a deal that never
     * happened — the rows moved and the notes are held — it is a deal whose vehicle is now an
     * estate, and the holders are creditors of that estate like any other.
     */
    if (!ctx.parties.get(deal.vehicle).status.alive) {
      ctx.endAgreement(deal.id, 'its vehicle has ceased and its estate owes the notes now');
      ctx.record(
        'securitisation.wound',
        [deal.arranger, deal.vehicle],
        { vehicle: String(deal.vehicle), arranger: String(deal.arranger), sold: deal.sold.length },
        true,
      );
      continue;
    }
    const ccy = ccyOfDeal(ctx, deal);
    // Law 8: a vehicle with no layer left has no money of its own to speak in. It has nothing to
    // pay and nothing to pay it with, and `windUp` below is what ends it.
    if (ccy === undefined) {
      windUp(ctx, deal, ccy);
      continue;
    }
    const layers = layersOf(ctx, deal.vehicle).map((i) => i.id);
    absorb(ctx, deal);
    // XI-8: what is left when everything ranking above the notes has been paid. A vehicle that
    // collected nothing pays nothing; there is no buffer and nothing is smoothed.
    const cash = cashOf(ctx, deal.vehicle, ccy);
    if (cash > 0) {
      /**
       * C5, B-8: INTEREST IS NOT PRINCIPAL, and paying it out as principal was the defect that
       * disabled this module's own subject.
       *
       * The vehicle collects PRINCIPAL AND INTEREST — `LOAN.due` emits a coupon every period and a
       * maturity at the end, both paid to the holder of record, which is the vehicle. What went out
       * was principal ONLY: `payTranche` redeems face at par, so `Σ out ≤ Σ face = the pool's
       * opening principal` and the interest — the whole economic return of the deal — never left.
       *
       * Three things followed. A noteholder earned nothing but its discount. The interest piled up
       * in a vehicle nobody owns, which is a residual with no holder (Appendix B). And `absorb`'s
       * `notes − pool` went the wrong way round for ever, because notes fell faster than the pool
       * when interest redeemed face — so the junior/senior waterfall, the attachment points, the
       * write-down leg and D4's senior losses were all downstream of a subtraction that could not
       * be positive, and NO LOSS WAS EVER ALLOCATED TO ANY TRANCHE whatever the borrowers did.
       *
       * What it collected as interest is READ off the wire (Law 19): the money legs into its own
       * account this period whose receipt says what the money IS to the party getting it. Nothing is
       * inferred by subtraction.
       */
      const earned = interestCollected(ctx, deal, ccy);
      let forInterest = downTick(atMost(earned, cash, 'no more than it actually holds'));
      const faces = new Map<InstrumentId, Qty>(
        layers.map((id) => [id, ctx.register.heldTotal(id).value]),
      );
      const outstanding = sum([...faces.values()]).value;
      for (const id of layers) {
        if (forInterest <= 0) break;
        const face = zeroIfNone(faces.get(id));
        if (face <= 0 || outstanding <= 0) continue;
        // C5: each layer's share of what the pool earned is its share of what is outstanding, and
        // SENIOR FIRST when there is not enough of it — which is what a waterfall is.
        const due = downTick(
          scale(
            earned,
            ratioOf(face, outstanding, 'its share of what is outstanding'),
            'its interest',
          ),
        );
        const paid = payInterest(
          ctx,
          deal,
          ccy,
          id,
          atMost(due, forInterest, 'and no more than is here'),
        );
        forInterest = subQty(forInterest, paid, 'what is left for the layer below');
      }
      // XI-8: and what is left is PRINCIPAL, which redeems face, senior first.
      let left: Qty = cashOf(ctx, deal.vehicle, ccy);
      for (const id of layers) {
        if (left <= 0) break;
        const paid = payTranche(ctx, deal, ccy, id, left);
        left = subQty(left, paid, 'what is left after the layer above');
      }
    }
    windUp(ctx, deal, ccy);
  }
}

/**
 * C5, Law 19, B-8: WHAT THE POOL EARNED THIS PERIOD, read off the wire and never by subtraction.
 *
 * A coupon carries `receipt: { of: 'interest' }` — what the money IS to the party getting it, said
 * by the payer (`world/actions.ts`) — and a redemption does not. So the split between what the
 * vehicle collected as interest and what it collected as principal is a READ of this period's
 * settled money legs into its own account, which is the one place the fact exists.
 */
function interestCollected(ctx: MechanismContext, deal: Deal, ccy: CurrencyCode): Qty {
  const account = ctx.accountOf(deal.vehicle, ccy);
  const terms: Qty[] = [];
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled') continue;
    for (const leg of r.instruction.legs) {
      if (!isMoneyLeg(leg) || leg.ccy !== ccy) continue;
      if (leg.to.holder !== account.holder || leg.to.issuer !== account.issuer) continue;
      if (leg.receipt.of !== 'interest') continue;
      terms.push(leg.amount);
    }
  }
  return sum(terms).value;
}

/**
 * C5: a layer's share of what the pool EARNED, paid as interest — no face is redeemed, because the
 * note still owes what it owes. This is the return a noteholder actually gets, and its yield is what
 * that return comes to against the price it paid (C3: derived from price, never into it).
 */
function payInterest(
  ctx: MechanismContext,
  deal: Deal,
  ccy: CurrencyCode,
  id: InstrumentId,
  available: Qty,
): Qty {
  const holders = ctx.register.holdersOf(id);
  const face = ctx.register.heldTotal(id).value;
  if (available <= 0 || face <= 0 || holders.length === 0) return NO_QTY;
  let paid = NO_QTY;
  for (const holder of holders) {
    const held = ctx.register.quantity(holder, id);
    if (held <= 0) continue;
    const share = downTick(
      scale(available, ratioOf(held, face, 'its share of the layer'), 'its share'),
    );
    if (share <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(deal.vehicle, ccy),
          to: ctx.accountOf(holder, ccy),
          // Treasury C1: what this money IS to the party getting it. A note pays interest.
          receipt: { of: 'interest' },
          ccy: ccy,
          amount: share,
        },
      ],
      cause: 'corporateAction',
      reason: `${String(deal.vehicle)} pays interest on ${String(id)}`,
    });
    if (r.outcome === 'settled') paid = addQty(paid, share, 'paid out');
  }
  return paid;
}

/**
 * A-57, C4.a, XI-3, XI-8: A VEHICLE WHOSE POOL HAS RUN OFF WINDS UP, AND ITS RESIDUAL HAS A HOLDER.
 *
 * Nothing ceased a vehicle: `fails: ['cash','solvency']` will not fire on a party with positive
 * equity that has paid every coupon (since 12a.1 a senior coupon it could not fund is an arrear it
 * issued, and THAT is what `cash` fires on), the kind has no owner and no distribution, and
 * `distribute` removed a deal only when the vehicle had ALREADY ceased. So a run-off deal sat on the book for ever holding whatever was left,
 * which is Appendix B's residual with no holder wearing a party's name.
 *
 * The arranger keeps the bottom (C4.a) — it holds the junior, which is the equity of the deal — so
 * the arranger is who the residual belongs to, and that is what makes XI-11 true rather than vacuous:
 * the risk did not leave, and neither did the last of the return. When every layer is redeemed and
 * no row of the pool is still live, what is left goes to the arranger and the vehicle ceases to it.
 */
function windUp(ctx: MechanismContext, deal: Deal, ccy: CurrencyCode | undefined): void {
  if (!ctx.parties.get(deal.vehicle).status.alive) return;
  const owed = sum(
    layersOf(ctx, deal.vehicle).map((i) => ctx.register.heldTotal(i.id).value),
  ).value;
  if (owed > 0) return;
  // XI-11: which of the rows it was sold are still running. The list of what was sold in is the
  // deal's own fact; whether each is still live and still held is the world's (Law 19).
  const running = deal.sold.filter(
    (row) => ctx.instruments.get(row).status.live && ctx.register.quantity(deal.vehicle, row) > 0,
  );
  if (running.length > 0) return;
  const left = ccy === undefined ? NO_QTY : cashOf(ctx, deal.vehicle, ccy);
  if (left > 0 && ccy !== undefined) {
    ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(deal.vehicle, ccy),
          to: ctx.accountOf(deal.arranger, ccy),
          // C4.a: the residual of a deal belongs to whoever held the bottom of it.
          receipt: { of: 'transfer' },
          ccy: ccy,
          amount: left,
        },
      ],
      cause: 'corporateAction',
      reason: `${String(deal.vehicle)} winds up and pays its residual to ${String(deal.arranger)}`,
    });
  }
  // XI-3, Money E4: it can only go when it holds nothing, and every reference to it resolves to
  // the party that took what was left (Register F2).
  if (ctx.register.holdingsOf(deal.vehicle).length > 0) return;
  ctx.cease(deal.vehicle, deal.arranger, 'the deal paid down and the vehicle has nothing left to hold');
  ctx.endAgreement(deal.id, 'the pool ran off, every layer was redeemed and the vehicle has gone');
  ctx.record(
    'securitisation.wound',
    [deal.arranger, deal.vehicle],
    {
      vehicle: String(deal.vehicle),
      arranger: String(deal.arranger),
      sold: deal.sold.length,
      residual: left,
      why: 'the pool ran off and every layer was redeemed',
    },
    true,
  );
}

/**
 * C2, C2.a, C6, D4, XI-1: WHAT A LOSS IN THE POOL DOES TO THE LAYERS ABOVE IT.
 *
 * The pool is worth what the vehicle still holds of it. When a borrower fails, the row is written
 * off and the pool falls — and the notes are still claims for their face, so the difference has to
 * land on somebody. It lands FROM THE BOTTOM: the layer that attaches at zero takes it until there
 * is none of it left, and then the one above, which is what an attachment point IS.
 *
 * D4, D4.a: NOTHING HERE STOPS A SENIOR LOSS. A junior that is not deep enough for what the pool
 * actually did is a junior that runs out, and the layer above it takes the rest — that is the whole
 * of what "senior losses when correlation exceeds what the attachment assumed" means, and it is an
 * outcome of the arithmetic rather than a case anybody wrote.
 *
 * C6 is then true by construction AND measured anyway (the ownership family): Σ layer face equals
 * the pool's face after every event. A mechanism that keeps an identity and an audit that checks it
 * are not the same thing, and the one that breaks silently is the one worth checking.
 */
function absorb(ctx: MechanismContext, deal: Deal): void {
  const pool: Qty = sum(
    deal.sold
      .filter((row) => ctx.instruments.get(row).status.live)
      .map((row) => ctx.register.quantity(deal.vehicle, row)),
  ).value;
  const layers = layersOf(ctx, deal.vehicle).map((i) => i.id);
  const notes = sum(layers.map((id) => ctx.register.heldTotal(id).value)).value;
  const incurred = subQty(notes, pool, 'what the pool no longer covers');
  if (incurred <= 0) return;
  /**
   * C6, B-8: WHAT THE POOL LOST, said out loud, before any of it is allocated. The audit's C6 line
   * reads this against the write-downs the loop below journals, which is a second record of the
   * same fact reached from the other end (Audit A1.a) — and neither number existed at all while
   * interest was paid out as principal, because `notes − pool` could not be positive.
   */
  ctx.record(
    'securitisation.absorbed',
    [deal.arranger, deal.vehicle],
    { vehicle: String(deal.vehicle), pool, notes, lost: incurred },
    true,
  );
  let lost: Qty = incurred;
  // XI-8: from the bottom. `layers` is in the order they are PAID, so losses run the other way.
  for (const id of [...layers].reverse()) {
    if (lost <= 0) break;
    lost = subQty(lost, writeDown(ctx, deal, id, lost), 'what is still not covered');
  }
}

/** XI-1: the write-down itself — an event on a date, on every holder's units in proportion. */
function writeDown(ctx: MechanismContext, deal: Deal, id: InstrumentId, lost: Qty): Qty {
  const face = ctx.register.heldTotal(id).value;
  if (face <= 0) return NO_QTY;
  const take = downTick(atMost(lost, face, 'a layer cannot lose more than it is owed'));
  if (take <= 0) return NO_QTY;
  let taken = NO_QTY;
  for (const holder of ctx.register.holdersOf(id)) {
    const held = ctx.register.quantity(holder, id);
    if (held <= 0) continue;
    const share = downTick(scale(take, ratioOf(held, face, 'its share of the layer'), 'its share'));
    if (share <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          // The units go back to the issuer at nothing, which is what a claim that will not be paid
          // is worth: the same door an estate writes a claim off through (XI-8).
          kind: 'asset',
          from: holder,
          to: deal.vehicle,
          instrument: id,
          qty: share,
          pricePerUnit: some(asPerPiece(0, 'at what it promised')),
          accruedPerUnit: none(),
        },
      ],
      cause: 'default',
      reason: `${String(id)} is written down by ${share}: the pool no longer covers it`,
    });
    if (r.outcome !== 'settled') continue;
    taken = addQty(taken, share, 'written down');
    ctx.record(
      'tranche.writtenDown',
      [id, holder, deal.vehicle],
      {
        tranche: String(id),
        holder: String(holder),
        vehicle: String(deal.vehicle),
        units: share,
        left: subQty(held, share, 'what it still holds'),
      },
      true,
    );
  }
  return taken;
}

function payTranche(
  ctx: MechanismContext,
  deal: Deal,
  ccy: CurrencyCode,
  id: InstrumentId,
  available: Qty,
): Qty {
  const holders = ctx.register.holdersOf(id);
  const face = ctx.register.heldTotal(id).value;
  if (face <= 0 || holders.length === 0) return NO_QTY;
  // XI-8: a layer takes what it is owed and no more; what it is owed is its face.
  const toLayer = downTick(atMost(available, face, 'a layer takes no more than its face'));
  if (toLayer <= 0) return NO_QTY;
  let paid = NO_QTY;
  for (const holder of holders) {
    const held = ctx.register.quantity(holder, id);
    if (held <= 0) continue;
    const share = downTick(
      scale(toLayer, ratioOf(held, face, 'its share of the layer'), 'its share'),
    );
    if (share <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'asset',
          from: holder,
          to: deal.vehicle,
          instrument: id,
          qty: share,
          pricePerUnit: some(asPerPiece(1, 'at what it promised')),
          accruedPerUnit: none(),
        },
        {
          kind: 'money',
          // 0i.5: the waterfall pays the note it is due on.
          receipt: { of: 'interest' },
          from: ctx.accountOf(deal.vehicle, ccy),
          to: ctx.accountOf(holder, ccy),
          ccy: ccy,
          amount: share,
        },
      ],
      cause: 'corporateAction',
      reason: `${String(deal.vehicle)} pays ${String(id)}`,
    });
    if (r.outcome === 'settled') paid = addQty(paid, share, 'paid out');
  }
  return paid;
}

/* --------------------------------------------------------------------------------------------
 * THE AUDIT
 * ------------------------------------------------------------------------------------------ */

/**
 * C6, E1, E2: the three things that must be true of every deal, measured and never enforced.
 *
 * A family that PASSES here is as valuable as a mechanism that works (Part II): each of these
 * breaks silently. A pool whose tranches no longer sum to it has lost a residual with no holder
 * (Appendix B); a tranche nobody holds is a liability with no beneficiary; a vehicle whose rows
 * name no borrower is the risk transfer with no transferee XI-11 is about.
 */
function deals(): Family {
  return {
    name: 'ownership',
    contributor: 'securitisation',
    spec: 'XI-11',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live || !isTranche(i.terms)) continue;
        const t = i.terms;
        const holders = view.register.holdersOf(i.id);
        if (holders.length === 0 && view.register.heldTotal(i.id).value > 0) {
          out.push({
            family: 'ownership',
            spec: 'XI-11',
            owner: i.id,
            size: view.register.heldTotal(i.id).value,
            unit: i.unit,
            period: view.period,
            message: `${i.id}: a layer with a face and nobody holding it`,
          });
        }
        /**
         * C6, B-8: WHAT WAS WRITTEN DOWN, AGAINST WHAT THE POOL ACTUALLY LOST — no tranching
         * creates or destroys loss.
         *
         * Two records that are not two readings of one number: the write-down events this module
         * journals per layer (`tranche.writtenDown`, what it took off a holder's book), against the
         * pool's own arithmetic as `absorb` measured it (`securitisation.absorbed`, the notes over
         * what the vehicle still holds of the rows). They agree only if every unit of loss landed on
         * exactly one layer — and for a long time neither number existed at all, because interest
         * paid out as principal made `notes − pool` negative for ever and no loss was ever allocated
         * to any tranche whatever the borrowers did.
         */
        // C6: the layers are cut FROM the pool, so what they claim can never exceed it.
        const claimed = scale(
          t.pool,
          minus(t.detachment, t.attachment, 'the depth of the layer'),
          'its face at the cut',
        );
        if (claimed > t.pool) {
          out.push({
            family: 'ownership',
            spec: 'XI-11',
            owner: i.id,
            size: minus(claimed, t.pool, 'more than the pool'),
            unit: i.unit,
            period: view.period,
            message: `${i.id}: claims ${claimed} of a pool of ${t.pool}`,
          });
        }
      }
      /**
       * C6: LOSSES ALLOCATED SUM TO LOSSES INCURRED, EXACTLY. Per vehicle, this period: what the
       * write-downs took off the layers against what `absorb` measured the pool to have lost.
       */
      const takenBy = new Map<string, number>();
      for (const e of view.journal.ofKindIn('tranche.writtenDown', view.period)) {
        const vehicle = e.data['vehicle'];
        const units = e.data['units'];
        if (typeof vehicle !== 'string' || typeof units !== 'number') continue;
        addTo(takenBy, vehicle, units);
      }
      for (const e of view.journal.ofKindIn('securitisation.absorbed', view.period)) {
        const vehicle = e.data['vehicle'];
        const lost = e.data['lost'];
        if (typeof vehicle !== 'string' || typeof lost !== 'number' || lost <= 0) continue;
        const taken = zeroIfNone(takenBy.get(vehicle));
        takenBy.delete(vehicle);
        if (taken === lost) continue;
        out.push({
          family: 'ownership',
          spec: 'XI-11',
          owner: vehicle,
          size: taken - lost,
          unit: 'units of face',
          period: view.period,
          message: `${vehicle}: the pool lost ${lost} and ${taken} was written off the layers`,
        });
      }
      // A layer written down against a pool that lost nothing is the same defect the other way.
      for (const [vehicle, taken] of takenBy) {
        if (taken <= 0) continue;
        out.push({
          family: 'ownership',
          spec: 'XI-11',
          owner: vehicle,
          size: taken,
          unit: 'units of face',
          period: view.period,
          message: `${vehicle}: ${taken} was written off the layers and the pool lost nothing`,
        });
      }
      return out;
    },
  };
}

/**
 * E1, XI-11: every row a vehicle holds names a borrower who owes it. This is the clause that says
 * the transfer was real: a pool of anonymous exposure is exactly what XI-11 forbids.
 */
function pools(): Family {
  return {
    name: 'names',
    contributor: 'securitisation',
    spec: 'XI-11',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const p of view.parties.ofKind(VEHICLE)) {
        if (!p.status.alive) continue;
        for (const h of view.register.holdingsOf(p.id)) {
          const i = view.instruments.get(h.instrument);
          if (!i.status.live) continue;
          // Its own notes and the money it collected are not pool assets; a pool asset is a claim
          // somebody owes, and E1 says every one of them names who.
          if (isTranche(i.terms)) continue;
          const profile = view.registry.instrumentKind(i.kind);
          if (profile.pricing === 'money') continue;
          // Item 10c: this asked `isLoan`, which is the same kind branch `saleable` carried and
          // fails the same way — the day a bank pools INVOICES, every one of them would be reported
          // as naming no borrower. What a pool asset has is a NAMED OBLIGOR, which is two facts the
          // instrument itself already states: its kind says somebody owes it, and it says who.
          if (profile.liabilityOfIssuer && i.issuer.some) continue;
          out.push({
            family: 'names',
            spec: 'XI-11',
            owner: h.instrument,
            size: view.register.quantity(p.id, h.instrument),
            unit: i.unit,
            period: view.period,
            message: `${String(p.id)} holds ${String(h.instrument)}, which names no borrower`,
          });
        }
      }
      return out;
    },
  };
}

/* --------------------------------------------------------------------------------------------
 * WHO BUYS A NOTE
 * ------------------------------------------------------------------------------------------ */

/**
 * C4, C3: A BANK BIDS FOR SENIOR NOTES out of the money it has, at a price it works out itself from
 * what the pool pays and what money costs it. Nobody has to buy and nothing forces a bid — a bank
 * with no spare money posts nothing, and a deal nobody answers does not happen.
 *
 * Law 3: it bids a PRICE, which is what it will pay per unit of face, and the deal clears at what
 * the book produced. There is no spread over anything and no rating-implied level: an unrated world
 * would price these the same way, because the price is what somebody paid.
 */
export function noteBids(
  view: ParticipantView,
  ccy: CurrencyCode,
  /** C4: the pool it is being offered, per unit of face. A buyer asked to bid is told what it pays. */
  schedule: readonly CashFlow[],
): readonly Order[] {
  const spare = minus(view.cash(ccy), view.owedIn(ccy), 'what it holds against what falls due');
  if (spare <= 0) return [];
  // F3, C4: AND NO MORE THAN IT WILL HAVE OUT TO ANY ONE NAME. A vehicle is one name — a buyer of
  // its notes is exposed to that pool and to nothing else — so the limit a bank already publishes
  // for every name it funds is the limit here too. It is read off the wire rather than restated,
  // because a bank's limit is a bank's fact and there is one writer of it (Law 4, Law 19).
  const published = limitPerName(view, String(view.self.id));
  if (!published.some) return [];
  const limit = published.value;
  const price = priceFor(view, ccy, schedule);
  if (!price.some) return [];
  /**
   * Clearing A2, A2.a, A-58: A CURVE, NOT A POINT. This posted one order for the whole of its spare
   * cash at its own reservation, which says nothing about what it would do at any other level — and
   * the cheaper a note is the MORE face the same money buys, up to the limit it publishes for every
   * name it funds. So it posts what its money reaches at each level, stopped by that limit, which is
   * the same ladder a household posts a loaf's demand over (`clearing/schedule.ts`).
   */
  const steps = view.params.count(DEAL_STEPS);
  // Item 16: a balance is a COUNT of the money's own pieces; what it will buy is that as money.
  const budget = heldAsMoney(spare, ccy, 'what it holds against what falls due');
  return rungsUpTo(levelsUpTo(price.value, steps), budget, limit).map((r) => ({
    party: view.self.id,
    side: 'buy' as const,
    price: r.price,
    qty: r.qty,
  }));
}

/** Law 8: one day count for discounting what a pool pays, stated once, in the file that does it. */
const NOTE_DAY_COUNT = 'ACT/ACT';

/**
 * C3, C4, Law 19: WHAT THE POOL PAYS AND WHEN, per unit of its face.
 *
 * A senior note over a pool is a pass-through: what it pays is what the borrowers pay, on the days
 * they pay it. So the schedule a buyer discounts is the pool's own — every row's cash flows, each
 * scaled by how much of that row is in the pool, added up by date and divided by the pool's face.
 * Nothing here is a forecast (Law 17) and nothing is a curve somebody posted: it is the dated
 * promises the rows already carry, read off the instruments (Law 19).
 *
 * Observer A4: this is the OFFER, and an offer is described to whoever is asked to bid on it. It is
 * not a look inside the arranger's book — a buyer sees the rows it is being sold and nothing else.
 */
function poolSchedule(
  view: ParticipantView,
  rows: readonly InstrumentId[],
  faces: readonly Qty[],
  pool: Qty,
): readonly CashFlow[] {
  if (pool <= 0) return [];
  const on = view.calendar.startOf(view.period);
  const byDate = new Map<number, { date: Civil; paid: Cash }>();
  rows.forEach((id, k) => {
    const face = faces[k];
    if (face === undefined || face <= 0) return;
    const i = view.instruments.get(id);
    for (const f of view.registry
      .instrumentKind(i.kind)
      .cashFlows(i, on, view.calendar, view.registry)) {
      const paid = valueAt(f.perUnit, face, i.ccy, 'what this row pays on the day');
      const at = byDate.get(dayNumber(f.date));
      byDate.set(
        dayNumber(f.date),
        at === undefined
          ? { date: f.date, paid }
          : { date: at.date, paid: plus(at.paid, paid, 'what the pool pays that day') },
      );
    }
  });
  return [...byDate.values()].map((v) => ({
    date: v.date,
    perUnit: pricedAt(v.paid, pool, 'what a unit of the pool pays'),
  }));
}

/**
 * B2, Law 19: WHAT THIS BANK ITSELF PUBLISHED THAT MONEY COSTS IT, per annum, in the deal's money.
 *
 * `banks/index.ts:publishCostOfFunds` writes one event per bank per period carrying the blend for
 * its home money and a named row for every other money it might lend in. This is a read of that —
 * never a re-derivation of it (Law 19), and never a table.
 */
function costOfFundsIn(view: ParticipantView, ccy: CurrencyCode): Option<Ratio> {
  return fundingCostIn(view, String(view.self.id), ccy);
}

/**
 * C3, Law 3, Law 8, Law 19: WHAT IT WILL PAY, per unit of face — the pool's own dated payments
 * discounted at what this bank's money costs it. A note is worth what its payments are worth to a
 * buyer that could have lent the money itself, and that is the whole of the comparison.
 *
 * A-58: this used to be `1 − (owed/(owed+equity))²` — the DEBT SHARE OF THE BANK'S FUNDING, squared
 * — with the variable named `cost` and the docstring already describing the number built here. It
 * had no periodicity (the note's tenor appeared nowhere), no relation to any rate this world
 * produces (a bank funded 90% by deposits bid 0.19 per unit of face, an 81% discount on a senior
 * tranche), and no dependence on the pool at all, so two vehicles with completely different loan
 * books got the same bid from the same bank. `owedIn` was also the wrong quantity even as leverage:
 * it is what falls due in `ccy` this period less what the bank holds of it, a funding gap.
 *
 * `Missing` where the bank has published no funding cost yet or the pool promises nothing: it has
 * no basis to bid and it does not bid. That is a refusal (Law 1), not a zero.
 */
function priceFor(
  view: ParticipantView,
  ccy: CurrencyCode,
  schedule: readonly CashFlow[],
): Option<PerPiece> {
  if (schedule.length === 0) return none<PerPiece>();
  const required = costOfFundsIn(view, ccy);
  if (!required.some) return none<PerPiece>();
  return some(
    priceAt(
      schedule,
      required.value,
      view.calendar.startOf(view.period),
      NOTE_DAY_COUNT,
      'what it will pay per unit of face',
    ),
  );
}

export function securitisation(): SystemModule {
  return {
    id: 'securitisation',
    agreementKinds: [
      {
        id: DEAL,
        // XI-8: the deal is a relationship between a live arranger and a live vehicle. An acquirer
        // that took the arranger's book took its deals; an estate services nothing.
        binds: 'aGoingConcern',
        what: 'a vehicle and the arranger that cut it, and which rows left the arranger\u2019s book',
      },
    ],
    // XI-8, item 9.1: no nouns. The deal is an agreement between the vehicle and its arranger, and
    // every other field the book carried — the layers, the money, the pool's face — is a read.
    // 21.92: XI-11 is where this system lives in the spec. It cited a §Securitisation, and there
    // is none: the citation a reader of a finding is handed was to a system that does not exist.
    spec: 'XI-11',
    requires: ['banks'],
    instrumentKinds: [trancheKind],
    partyKinds: [vehicleKind],
    curveFamilies: [],
    units: [],
    params: [
      {
        id: DEAL_STEPS,
        value: 5,
        unit: 'count',
        dimension: 'count',
        kind: 'resolution',
        owner: 'model',
        why: 'Clearing A2, Securitisation C4: how finely a bank posts its own demand for a note into the deal book. Its shape is the bank own — what its spare money takes at a price, up to what it will have out to one name — and this is only how many levels of it the book sees; change it and the answer must not move.',
      },
    ],
    phases: [
      {
        name: 'securitisation.arrange',
        spec: 'XI-11',
        /**
         * After the markets, and BEFORE the banks turn what is left of the period into rows.
         *
         * What a bank is short of — and so what it would sell — is what this period's trading and
         * funding left it short of, so a pool sold on last period's position is a pool sold on a
         * stale number (D1). But a deal moves money, and a buyer that pays for a note out of a
         * balance it turns out not to have is overdrawn — which in this world is not a state, it is
         * a LOAN the bank writes at its own rate (`lending.book`, Money B3.a). Running after that
         * phase would have left a raw negative balance with no row behind it, which is the thing
         * the accounts family is built to catch, and it caught it.
         */
        anchor: { before: 'lending.book' },
        reads: [{ kind: 'event', name: 'credit.default', of: 'anyPeriod' }],
        writes: [{ kind: 'event', name: 'securitisation.cut' }],
        run: arrange,
      },
      {
        name: 'vehicle.distribute',
        spec: 'XI-11 XI-8',
        /**
         * C5, XI-8: LAST, and that is the whole of what a waterfall is. What goes out is what came
         * in (Law 19) — so it has to be after the borrowers have paid — and what goes out to the
         * NOTES is what is left after everything the vehicle itself owes has been taken, which in
         * this world is the tax on the interest it received (Treasury C1).
         *
         * It used to run straight after the corporate actions, and the vehicle then handed every
         * penny to its note holders before the treasury came for the tax — so it was refused an
         * overdraft it is not allowed to have (`borrows: false`), failed on cash, and its estate
         * opened. A pass-through that pays its investors before its own obligations is not a
         * waterfall, it is a hole; the residual is the bottom of the ladder and this is where the
         * bottom of the ladder runs.
         *
         * And before the banks turn what is left of the period into rows (`lending.book`), because
         * a vehicle banks where its arranger banks: paying note holders at other banks moves that
         * bank's reserves, and a bank left short of them is a bank that has borrowed — which is a
         * loan somebody writes, never a raw negative balance (Money B3.a).
         */
        anchor: { before: 'lending.book' },
        // Law 10, Clearing F1.a: this phase has never RUN — no period of either world has reached
        // it — so what it reads is read off its module's source and not off a measurement, and
        // it is the module's whole read set rather than this phase's. It narrows the first time
        // the phase runs and the check can say which of these it actually wanted.
        reads: [
          { kind: 'event', name: 'bank.capital', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.costOfFunds', of: 'anyPeriod' },
          { kind: 'event', name: 'securitisation.absorbed', of: 'anyPeriod' },
          { kind: 'event', name: 'tranche.writtenDown', of: 'anyPeriod' },
        ],
        writes: [],
        run: distribute,
      },
    ],
    participants: [],
    families: [deals(), pools()],
  };
}
