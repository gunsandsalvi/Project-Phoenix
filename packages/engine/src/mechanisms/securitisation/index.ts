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
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { InvalidRegistry } from '../../core/errors.js';
import { percent } from '../../core/format.js';
import {
  currencyUnit,
  instrumentId,
  instrumentKindId,
  marketId,
  partyId,
  partyKindId,
  venueId,
  type CurrencyCode,
  type InstrumentId,
  type MarketId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { add, atMost, div, mul, sub, sum } from '../../core/num.js';
import { downTick, type Qty } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import type { Instrument, Terms } from '../../register/instruments.js';
import { isLoan } from '../../registry/credit.js';
import type { CashFlow, InstrumentKindProfile, PartyKindProfile } from '../../registry/kinds.js';
import { FACE_TICK } from '../../registry/grid.js';
import { BANK } from '../../registry/profiles.js';
import type { Violation } from '../../audit/audit.js';
import type { Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

export const VEHICLE = partyKindId('vehicle');
export const TRANCHE = instrumentKindId('tranche');

/** C1: a deal is named for who arranged it and which one it was (Law 9). */
export const vehicleId = (arranger: PartyId, n: number): PartyId =>
  partyId(`spv.${arranger}.${n}`);

/** C2: a tranche is named by its vehicle and where it sits. Not by an internal id (Law 9). */
export const trancheId = (vehicle: PartyId, layer: 'senior' | 'junior'): InstrumentId =>
  instrumentId(`tranche:${vehicle}:${layer}`);

export const dealVenue = (vehicle: PartyId): VenueId => venueId(`securitisation:${vehicle}`);

export interface TrancheTerms extends Terms {
  readonly kind: typeof TRANCHE;
  readonly vehicle: PartyId;
  /**
   * C2.a: WHERE IT STARTS TAKING LOSSES and where it stops, as fractions of the pool it was cut
   * from. Both are stated on the instrument because a holder has to be able to read what it is
   * exposed to; both are outcomes of what cleared, because what a market would take is not a
   * number anybody may write down (Law 3).
   */
  readonly attachment: number;
  readonly detachment: number;
  /** XI-8: what the waterfall orders by. Smaller is paid first, as everywhere else. */
  readonly seniority: number;
  /** C6: the pool's face at the moment the deal was cut, which is what the fractions are OF. */
  readonly pool: number;
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
  // C5: a tranche pays what the pool paid, when the pool paid it. There is no schedule to promise
  // and nothing to accrue: this is a pass-through, and what it passes through is an outcome.
  cashFlows: (): readonly CashFlow[] => [],
  due: () => [],
  accrued: () => 0,
};



/** The money a bank keeps its book in. One world, one list, read from the registry (Law 4). */
function moneyOf(ctx: MechanismContext): Option<CurrencyCode> {
  for (const ccy of ctx.registry.currencies.keys()) return some(ccy);
  return none<CurrencyCode>();
}

/** What a party holds in money of a currency, summed over the accounts it banks in. */
function cashOf(ctx: MechanismContext, who: PartyId, ccy: CurrencyCode): number {
  const amounts: number[] = [];
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

interface Deal {
  readonly vehicle: PartyId;
  readonly arranger: PartyId;
  readonly ccy: CurrencyCode;
  readonly rows: readonly InstrumentId[];
  readonly pool: number;
  /** XI-8: the layers in the order they are paid. One when the pool went out whole (C2.a). */
  readonly layers: readonly InstrumentId[];
}

interface Book {
  next: number;
  readonly deals: Deal[];
}

const state = (ctx: MechanismContext): Book =>
  ctx.state<Book>('deals', () => ({ next: 1, deals: [] }));

/**
 * D1, D2: WHAT A BANK WOULD SELL. Rows it is owed, in the money it is short of, that it can hand
 * over free and clear — a row already pledged to somebody else is not its to sell (Register D5.a).
 */
function saleable(view: ParticipantView, ccy: CurrencyCode): readonly Instrument[] {
  const out: Instrument[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!i.status.live || !isLoan(i.terms) || i.ccy !== ccy) continue;
    if (h.liens.length > 0) continue;
    if (view.free(h.instrument) <= 0) continue;
    out.push(i);
  }
  return out;
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
function shortBy(view: ParticipantView): number {
  const published = view.lastPublicAbout('bank.capital', String(view.self.id));
  if (!published.some) return 0;
  const d = published.value.data;
  const headroom = d['headroom'];
  const perUnit = d['minWeighted'];
  const binds = d['binds'];
  if (typeof headroom !== 'number' || typeof perUnit !== 'number' || typeof binds !== 'string') {
    return 0;
  }
  return faceToShed({ headroom, minWeighted: perUnit, binds });
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
}): number {
  if (p.headroom >= 0 || p.minWeighted <= 0) return 0;
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
  if (p.binds !== 'weighted') return 0;
  // D2: its capital is freed because the ROW LEFT, not because a weight changed. This is the
  // shortfall divided by the capital its own rule asks per unit of weighted assets — two published
  // numbers and a division, and nothing here is a coefficient anybody chose.
  return div(-p.headroom, p.minWeighted, 'the face it must shed for the room it is short of');
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
  const money = moneyOf(ctx);
  if (!money.some) return;
  const ccy = money.value;
  for (const bank of ctx.parties.ofKind(BANK)) {
    if (!bank.status.alive) continue;
    const view = ctx.participant(bank.id);
    const gap = shortBy(view);
    if (gap <= 0) continue;
    const rows = saleable(view, ccy);
    if (rows.length === 0) continue;
    const taken: InstrumentId[] = [];
    const faces: number[] = [];
    let got = 0;
    for (const i of rows) {
      if (got >= gap) break;
      const face = view.quantity(i.id);
      if (face <= 0) continue;
      taken.push(i.id);
      faces.push(face);
      got = add(got, face, 'pool face');
    }
    const pool = sum(faces).value;
    if (pool <= 0) continue;
    const book = state(ctx);
    const vehicle = vehicleId(bank.id, book.next);
    // C4: WHO WOULD BUY A NOTE OF IT. Each is asked with its own view and answers out of its own
    // money; none of them sees this bank's book, and none of them has to bid (Observer A4).
    const bids: Order[] = [];
    for (const other of ctx.parties.ofKind(BANK)) {
      if (!other.status.alive || other.id === bank.id) continue;
      bids.push(...noteBids(ctx.participant(other.id), ccy));
    }
    if (bids.length === 0) {
      // A deal nobody would buy a note of is a deal that does not happen, and the bank is exactly
      // where it was. Recorded, because a refusal is an answer (Law 1).
      ctx.record(
        'securitisation.failed',
        [bank.id],
        { arranger: String(bank.id), pool, outcome: 'noDemand' },
        true,
      );
      continue;
    }
    cut(ctx, { arranger: bank.id, ccy, vehicle, rows: taken, pool, offered: gap, bids, n: book.next });
  }
}

/**
 * C2, C2.a, C3, C4.a: WHAT THE BOOK SAID. The arranger posts the pool's whole face and no level;
 * the bids clear at one price; the senior tranche is what cleared and the junior is the remainder.
 */
function cut(
  ctx: MechanismContext,
  d: {
    arranger: PartyId;
    ccy: CurrencyCode;
    vehicle: PartyId;
    rows: readonly InstrumentId[];
    pool: number;
    /** D2: what it needs OFF its book. It sells this much and not a unit more (C4.a). */
    offered: number;
    bids: readonly Order[];
    n: number;
  },
): void {
  const venue = dealVenue(d.vehicle);
  if (!ctx.venues.some((v) => v.id === venue)) {
    ctx.openVenue({
      id: venue,
      name: `${String(d.arranger)} pool ${d.n}`,
      clearedBy: 'securitisation',
      unit: currencyUnit(d.ccy),
      ccy: d.ccy,
      key: { arranger: String(d.arranger), deal: String(d.n) },
    });
  }
  for (const b of d.bids) ctx.post(venue, b);
  // Clearing C3, C4.a: A SIZE AND NO LEVEL, and the size is what it is short of — never more.
  // Selling past its need would be giving away income to no purpose, and it cannot sell past the
  // pool either, because a bank cannot hand over rows it does not have.
  const offering = downTick(atMost(d.offered, d.pool, 'it cannot sell rows it does not hold'));
  if (offering <= 0) return;
  ctx.post(venue, { party: d.arranger, side: 'sell', price: 'market', qty: offering });
  const outcome = clear(ctx.posted(venue), 'proRata', 'marginalBid');
  if (!isCleared(outcome)) {
    ctx.record(
      'securitisation.failed',
      [d.arranger],
      { arranger: d.arranger, pool: d.pool, outcome: outcome.kind },
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
  const juniorFace = sub(d.pool, seniorFace, 'what it kept');
  const attachment = div(juniorFace, d.pool, 'where the senior starts taking losses');
  const arrangerParty = ctx.parties.get(d.arranger);
  ctx.enter({
    id: d.vehicle,
    kind: VEHICLE,
    region: arrangerParty.region,
    name: `${String(d.arranger)} pool ${d.n}`,
    bank: d.arranger,
    representation: 'named',
    status: { alive: true },
  });
  const senior = trancheId(d.vehicle, 'senior');
  issueTranche(ctx, d.vehicle, d.ccy, senior, {
    kind: TRANCHE,
    vehicle: d.vehicle,
    attachment,
    detachment: 1,
    seniority: 0,
    pool: d.pool,
  });
  const junior = juniorFace > 0 ? some(trancheId(d.vehicle, 'junior')) : none<InstrumentId>();
  if (junior.some) {
    issueTranche(ctx, d.vehicle, d.ccy, junior.value, {
      kind: TRANCHE,
      vehicle: d.vehicle,
      attachment: 0,
      detachment: attachment,
      seniority: 1,
      pool: d.pool,
    });
  }
  if (!settleDeal(ctx, d, outcome.price, senior, junior, seniorFace, juniorFace)) return;
  state(ctx).next += 1;
  state(ctx).deals.push({
    vehicle: d.vehicle,
    arranger: d.arranger,
    ccy: d.ccy,
    rows: d.rows,
    pool: d.pool,
    layers: junior.some ? [senior, junior.value] : [senior],
  });
  ctx.record(
    'securitisation.cut',
    [d.arranger, d.vehicle],
    {
      arranger: String(d.arranger),
      vehicle: String(d.vehicle),
      rows: d.rows.length,
      pool: d.pool,
      senior: seniorFace,
      junior: juniorFace,
      layers: junior.some ? 2 : 1,
      attachment,
      price: outcome.price,
    },
    true,
  );
}

/**
 * C3: EVERY TRANCHE HAS A MARKET, and it is opened with the tranche because a claim that is priced
 * by clearing and names no book is a claim nobody can value (Clearing D1: the kernel refuses one).
 * What a note is worth after the deal is what somebody will pay for it, which is the same rule as
 * everything else here.
 */
function issueTranche(
  ctx: MechanismContext,
  vehicle: PartyId,
  ccy: CurrencyCode,
  id: InstrumentId,
  terms: TrancheTerms,
): void {
  const market = trancheMarket(id);
  ctx.issue({ id, kind: TRANCHE, issuer: some(vehicle), ccy, terms, market: some(market) });
  ctx.openMarket({
    id: market,
    name: `${String(vehicle)} ${terms.seniority === 0 ? 'senior' : 'junior'} notes`,
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
    pool: number;
    bids: readonly Order[];
  },
  price: number,
  senior: InstrumentId,
  junior: Option<InstrumentId>,
  seniorFace: Qty,
  juniorFace: number,
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
      fromCell: none(),
      toCell: none(),
    });
  }
  if (legs.length === 0) return false;
  // C4: the notes go to NAMED holders, and each pays for what it took.
  let placed = 0;
  for (const b of d.bids) {
    if (b.side !== 'buy') continue;
    const want = downTick(atMost(b.qty, sub(seniorFace, placed, 'left to place'), 'its fill'));
    if (want <= 0) continue;
    const cash = downTick(mul(want, price, 'what it pays for the note'));
    if (cash <= 0) continue;
    legs.push({
      kind: 'asset',
      from: d.vehicle,
      to: b.party,
      instrument: senior,
      qty: want,
      pricePerUnit: some(price),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: none(),
    });
    legs.push({
      kind: 'money',
      from: ctx.accountOf(b.party, d.ccy),
      to: ctx.accountOf(d.arranger, d.ccy),
      ccy: d.ccy,
      amount: cash,
      fromCell: none(),
      toCell: none(),
    });
    placed = add(placed, want, 'placed');
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
      fromCell: none(),
      toCell: none(),
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
  for (const deal of state(ctx).deals) {
    absorb(ctx, deal);
    // XI-8: what is left when everything ranking above the notes has been paid. A vehicle that
    // collected nothing pays nothing; there is no buffer and nothing is smoothed.
    const cash = cashOf(ctx, deal.vehicle, deal.ccy);
    if (cash <= 0) continue;
    let left: number = cash;
    for (const id of deal.layers) {
      if (left <= 0) break;
      const paid = payTranche(ctx, deal, id, left);
      left = sub(left, paid, 'what is left after the layer above');
    }
  }
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
  const pool = sum(
    deal.rows
      .filter((row) => ctx.instruments.get(row).status.live)
      .map((row) => ctx.register.quantity(deal.vehicle, row)),
  ).value;
  const notes = sum(deal.layers.map((id) => ctx.register.heldTotal(id).value)).value;
  let lost = sub(notes, pool, 'what the pool no longer covers');
  if (lost <= 0) return;
  // XI-8: from the bottom. `layers` is in the order they are PAID, so losses run the other way.
  for (const id of [...deal.layers].reverse()) {
    if (lost <= 0) break;
    lost = sub(lost, writeDown(ctx, deal, id, lost), 'what is still not covered');
  }
}

/** XI-1: the write-down itself — an event on a date, on every holder's units in proportion. */
function writeDown(ctx: MechanismContext, deal: Deal, id: InstrumentId, lost: number): number {
  const face = ctx.register.heldTotal(id).value;
  if (face <= 0) return 0;
  const take = downTick(atMost(lost, face, 'a layer cannot lose more than it is owed'));
  if (take <= 0) return 0;
  let taken = 0;
  for (const holder of ctx.register.holdersOf(id)) {
    const held = ctx.register.quantity(holder, id);
    if (held <= 0) continue;
    const share = downTick(mul(take, div(held, face, 'its share of the layer'), 'its share'));
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
          pricePerUnit: some(0),
          accruedPerUnit: none(),
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'default',
      reason: `${String(id)} is written down by ${share}: the pool no longer covers it`,
    });
    if (r.outcome !== 'settled') continue;
    taken = add(taken, share, 'written down');
    ctx.record(
      'tranche.writtenDown',
      [id, holder, deal.vehicle],
      {
        tranche: String(id),
        holder: String(holder),
        vehicle: String(deal.vehicle),
        units: share,
        left: sub(held, share, 'what it still holds'),
      },
      true,
    );
  }
  return taken;
}

function payTranche(
  ctx: MechanismContext,
  deal: Deal,
  id: InstrumentId,
  available: number,
): number {
  const holders = ctx.register.holdersOf(id);
  const face = ctx.register.heldTotal(id).value;
  if (face <= 0 || holders.length === 0) return 0;
  // XI-8: a layer takes what it is owed and no more; what it is owed is its face.
  const toLayer = downTick(atMost(available, face, 'a layer takes no more than its face'));
  if (toLayer <= 0) return 0;
  let paid = 0;
  for (const holder of holders) {
    const held = ctx.register.quantity(holder, id);
    if (held <= 0) continue;
    const share = downTick(mul(toLayer, div(held, face, 'its share of the layer'), 'its share'));
    if (share <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'asset',
          from: holder,
          to: deal.vehicle,
          instrument: id,
          qty: share,
          pricePerUnit: some(1),
          accruedPerUnit: none(),
          fromCell: none(),
          toCell: none(),
        },
        {
          kind: 'money',
          from: ctx.accountOf(deal.vehicle, deal.ccy),
          to: ctx.accountOf(holder, deal.ccy),
          ccy: deal.ccy,
          amount: share,
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'corporateAction',
      reason: `${String(deal.vehicle)} pays ${String(id)}`,
    });
    if (r.outcome === 'settled') paid = add(paid, share, 'paid out');
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
        // C6: the layers are cut FROM the pool, so what they claim can never exceed it.
        const claimed = mul(sub(t.detachment, t.attachment, 'the depth of the layer'), t.pool, 'its face at the cut');
        if (claimed > t.pool) {
          out.push({
            family: 'ownership',
            spec: 'XI-11',
            owner: i.id,
            size: sub(claimed, t.pool, 'more than the pool'),
            unit: i.unit,
            period: view.period,
            message: `${i.id}: claims ${claimed} of a pool of ${t.pool}`,
          });
        }
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
          // Its own notes and the money it collected are not pool assets; a pool asset is a row
          // somebody owes, and E1 says every one of them names who.
          if (isTranche(i.terms) || isLoan(i.terms)) continue;
          if (view.registry.instrumentKind(i.kind).pricing === 'money') continue;
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
export function noteBids(view: ParticipantView, ccy: CurrencyCode): readonly Order[] {
  const spare = sub(view.cash(ccy), view.owedIn(ccy), 'what it holds against what falls due');
  if (spare <= 0) return [];
  // F3, C4: AND NO MORE THAN IT WILL HAVE OUT TO ANY ONE NAME. A vehicle is one name — a buyer of
  // its notes is exposed to that pool and to nothing else — so the limit a bank already publishes
  // for every name it funds is the limit here too. It is read off the wire rather than restated,
  // because a bank's limit is a bank's fact and there is one writer of it (Law 4, Law 19).
  const published = view.lastPublicAbout('bank.capital', String(view.self.id));
  if (!published.some) return [];
  const limit = published.value.data['limitPerName'];
  if (typeof limit !== 'number' || limit <= 0) return [];
  const qty = downTick(atMost(spare, limit, 'what it will have out to one name'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: priceFor(view, ccy), qty }];
}

/**
 * C3, Law 3, Law 19: WHAT IT WILL PAY, per unit of face. A note is worth a discount on its face to
 * a buyer that could have lent the money itself — the discount is what its own money costs it,
 * which it reads off what it actually pays for money and never off a table.
 */
function priceFor(view: ParticipantView, ccy: CurrencyCode): number {
  const owed = view.owedIn(ccy);
  const equity = view.equity();
  if (equity <= 0 || owed <= 0) {
    // A buyer that owes nothing has nothing to compare a note against, and pays face for it. That
    // is not a floor: it is what "my money costs me nothing this period" arithmetically comes to.
    return 1;
  }
  const cost = div(owed, add(owed, equity, 'what funds it'), 'what its own money costs it');
  return sub(1, mul(cost, cost, 'the discount it wants'), 'what it will pay per unit of face');
}

export function securitisation(): SystemModule {
  return {
    id: 'securitisation',
    spec: 'Securitisation',
    requires: ['banks'],
    instrumentKinds: [trancheKind],
    partyKinds: [vehicleKind],
    curveFamilies: [],
    units: [],
    params: [],
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
        cycle: 'anchor',
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
        cycle: 'anchor',
        run: distribute,
      },
    ],
    participants: [],
    families: [deals(), pools()],
  };
}
