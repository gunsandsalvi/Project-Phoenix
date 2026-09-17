/**
 * SUPPLY CONTRACTS: a named buyer and a named seller, a quantity every period, at a price, until a
 * day — and a cost, agreed at the start, for walking away from it.
 *
 * @spec Goods C3 Goods C4 Clearing A2 Clearing B2 Law 1 Law 3 Law 5 Law 15 XI-5
 *
 * THE ABSENCE. Every input in this world was bought in a session, every period, at whatever the
 * session cleared. Nobody could lock anything in: a mill facing a grain market that doubles had no
 * answer except to pay it or stop, and a grower facing one that halves had no answer except to
 * take it. That is not how anything is actually supplied, and it is the owner's ask.
 *
 * IT IS NOT A DERIVATIVE AND IT IS NOT A FORWARD PRICE. Nothing here is priced off a curve, a
 * parity formula or a carry: the price is what a buyer and a seller CROSSED at in a book of their
 * own (Law 3), and what makes the contract worth having is precisely that the spot session will
 * move away from it. Two parties who expected the same thing would never sign one — the
 * disagreement is the mechanism (§46 A3).
 *
 * WHAT IT IS NOT, AND WHY THAT MATTERS. A contract does not conjure goods: the seller delivers what
 * it actually has, and a delivery it cannot make is a delivery that did not happen, recorded as
 * such — the row stands, because what ends a contract is somebody ENDING it (`supply.broke`), not
 * a period going wrong. There is no penalty schedule, no damages formula and no arbiter: there is
 * one number both of them agreed to, and either may pay it and walk.
 */
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { addDays, compareCivil, dayNumber, formatCivil, type Civil } from '../../calendar/civil.js';
import { asAmount, scale, valueAt, asRatio, type PerPiece } from '../../core/measure.js';
import { addQty, asQty, downTick, subQty, type Qty } from '../../core/tick.js';
import { atMost, div, mul, sub } from '../../core/num.js';
import {
  agreementKindId,
  paramId,
  venueId,
  type AgreementKindId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
  type RegionId,
  type VenueId,
} from '../../core/ids.js';
import type { Agreement, AgreementKindDecl, AgreementTerms } from '../../register/agreements.js';
import type { Leg } from '../../ledger/instruction.js';
import type { ParamDecl } from '../../registry/params.js';
import { goodId, isGoodTerms } from '../../registry/physical.js';
import { expectedPriceOf } from '../../registry/expectation.js';
import { FIRM } from '../../registry/profiles.js';
import { about, type MechanismContext, type ParticipantView, type SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import type { Family, Violation } from '../../audit/audit.js';

/** The kind: a standing promise to deliver, which is a RELATIONSHIP and not a debt (XI-8). */
export const SUPPLY: AgreementKindId = agreementKindId('supply.contract');

export const SUPPLY_PARAMS = {
  /** How long one runs. A convention of the trade, and what the two of them are agreeing to. */
  term: paramId('supply.termPeriods'),
  /** What walking away costs, as periods of the deliveries it promised. Agreed at the start. */
  breakPeriods: paramId('supply.breakPeriods'),
};

export const supplyVenue = (subUnit: string, region: RegionId): VenueId =>
  venueId(`venue.supply.${subUnit}.${region}`);

export const SUPPLY_PRINT = 'supply.print';
export const SUPPLY_STRUCK = 'supply.struck';
export const SUPPLY_DELIVERED = 'supply.delivered';
export const SUPPLY_SHORT = 'supply.short';
export const SUPPLY_BROKE = 'supply.broke';
export const SUPPLY_ENDED = 'supply.ended';
export const SUPPLY_EXTENDED = 'supply.extended';

/**
 * Law 15: what a supply contract is, and everything about it the kernel does not understand. The
 * quantity is per PERIOD and the price is per PIECE, both named here so no reader has to guess
 * which (Law 8), and the break cost is a number of pieces of the region's money struck when the
 * contract was — never a number this world holds and applies.
 */
export interface SupplyTerms extends AgreementTerms {
  readonly region: RegionId;
  readonly instrument: InstrumentId;
  readonly qtyPerPeriod: Qty;
  readonly pricePerPiece: PerPiece;
  readonly until: Civil;
  readonly breakCost: number;
}

/** Law 15: a contract is told by the shape of its terms, never by its kind id. */
export const isSupplyTerms = (t: AgreementTerms): t is SupplyTerms =>
  'qtyPerPeriod' in t && 'breakCost' in t && 'pricePerPiece' in t;

const supplyKind: AgreementKindDecl = {
  id: SUPPLY,
  what: 'a named seller delivering a named buyer so much of a good every period, at a price, until a day',
  // XI-8: it is a promise to keep DOING something. An acquirer of a going concern keeps it; an
  // estate keeps none, because an estate realises what is there rather than running it.
  binds: 'aGoingConcern',
};

/* ------------------------------------------------------------------------------------------------
 * READS
 * ---------------------------------------------------------------------------------------------- */

/** Every live contract, from the kernel's own book of commitments (Law 19). */
function contractsOf(ctx: MechanismContext): readonly (Agreement & { terms: SupplyTerms })[] {
  const out: (Agreement & { terms: SupplyTerms })[] = [];
  for (const a of ctx.agreements.ofKind(SUPPLY)) {
    if (a.state !== 'performing') continue;
    if (!isSupplyTerms(a.terms)) continue;
    out.push(a as Agreement & { terms: SupplyTerms });
  }
  return out;
}

/**
 * Law 4, Corporate Credit C9's lesson (17f.3): THE LIVE CONTRACT BETWEEN THESE TWO FOR THIS THING,
 * if there is one. There is at most one, and everything that would open a second restates it
 * instead — two rows for one relationship are two answers to what was agreed.
 */
function liveBetween(
  ctx: MechanismContext,
  buyer: PartyId,
  seller: PartyId,
  instrument: InstrumentId,
): (Agreement & { terms: SupplyTerms }) | undefined {
  return contractsOf(ctx).find(
    (a) =>
      String(a.creditor) === String(buyer) &&
      String(a.debtor) === String(seller) &&
      String(a.terms.instrument) === String(instrument),
  );
}

/** What this party has already locked in of one good, either way round. It never signs it twice. */
function lockedIn(view: ParticipantView, instrument: InstrumentId): Qty {
  let out = 0;
  for (const a of view.commitments()) {
    if (a.state !== 'performing' || !isSupplyTerms(a.terms)) continue;
    if (String(a.terms.instrument) !== String(instrument)) continue;
    out += a.terms.qtyPerPeriod;
  }
  return asQty(out, 'what it has already contracted for a period');
}

/**
 * Clearing A2, §46 A3: WHAT A PARTY POSTS IN THE BOOK FOR A CONTRACT, and both sides are one read.
 *
 * A firm that MAKES the good asks; one that buys it bids. Each posts the quantity its own outlook
 * says it trades in a period — what it has been buying, or what it has been selling, formed from
 * its own fills and from nothing else (§46 A2) — less whatever it has locked in already, and at
 * what it expects that good to cost. It is a reservation and not a valuation: nothing in this
 * world prices CERTAINTY yet, and inventing a premium for it would be a number nobody agreed.
 */
function contractOrders(view: ParticipantView, venue: VenueDecl): readonly Order[] {
  if (venue.clearedBy !== 'supply') return [];
  const subUnit = venue.key['subUnit'];
  const region = venue.key['region'] as RegionId | undefined;
  if (subUnit === undefined || region === undefined || view.self.region !== region) return [];
  const good = goodId(subUnit, region);
  if (!view.instruments.has(good)) return [];
  const level = expectedPriceOf(view, good);
  if (!level.some || level.value <= 0) return [];
  const makes = view.made(good) > 0;
  const seen = view.outlook(about({ on: makes ? 'sold' : 'bought', instrument: good }));
  if (!seen.some || seen.value.expected <= 0) return [];
  // Law 8: what it promises is whole pieces, and what a party CAN commit to rounds DOWN — a
  // contract for the piece it has not got is a contract it breaks in its first period.
  const want = subQty(
    downTick(asAmount<'piece'>(seen.value.expected, 'what it trades in a period')),
    lockedIn(view, good),
    'what it has not locked in',
  );
  if (want <= 0) return [];
  return [{ party: view.self.id, side: makes ? 'sell' : 'buy', price: level.value, qty: want }];
}

/* ------------------------------------------------------------------------------------------------
 * STRIKING ONE
 * ---------------------------------------------------------------------------------------------- */

/**
 * Sign one. It is exported because a SCALE MODEL of this mechanism has to be able to put a contract
 * into a world without a session having crossed one (`test/supply.test.ts`): what the book does is
 * one question and what living with a contract does is another (Law 11).
 */
export function signContract(
  ctx: MechanismContext,
  d: {
    buyer: PartyId;
    seller: PartyId;
    region: RegionId;
    instrument: InstrumentId;
    qtyPerPeriod: Qty;
    pricePerPiece: PerPiece;
    until: Civil;
    ccy: CurrencyCode;
  },
): void {
  // Law 2: the cost of walking away is the deliveries it promised, for as many periods as the two
  // of them agreed to count — struck ONCE, on this contract, out of numbers they both just saw.
  const breakCost = costOfLeaving(ctx, d.qtyPerPeriod, d.pricePerPiece, d.ccy);
  // 17f.3: THE SAME TWO, FOR THE SAME THING, ARE ONE CONTRACT. A pair that meets in the book again
  // is not a second relationship: the row they have is restated at what they have just agreed, for
  // what they have now promised between them, and what it costs to leave is struck again on that.
  // Nothing multiplies, which is the owner's ask and Law 4's rule (`reagree` does it for a line).
  const standing = liveBetween(ctx, d.buyer, d.seller, d.instrument);
  const qtyPerPeriod =
    standing === undefined
      ? d.qtyPerPeriod
      : addQty(standing.terms.qtyPerPeriod, d.qtyPerPeriod, 'what the two of them now promise');
  const terms: SupplyTerms = {
    kind: SUPPLY,
    region: d.region,
    instrument: d.instrument,
    qtyPerPeriod,
    pricePerPiece: d.pricePerPiece,
    until: d.until,
    breakCost: standing === undefined ? breakCost : costOfLeaving(ctx, qtyPerPeriod, d.pricePerPiece, d.ccy),
  };
  if (standing === undefined) {
    ctx.owes({
      debtor: d.seller,
      creditor: d.buyer,
      ccy: d.ccy,
      owed: 0,
      terms,
      why: `${d.seller} supplies ${d.buyer} ${qtyPerPeriod} of ${d.instrument} a period`,
    });
  } else {
    ctx.restate(standing.id, terms);
  }
  ctx.record(
    standing === undefined ? SUPPLY_STRUCK : SUPPLY_EXTENDED,
    [d.buyer, d.seller, String(d.instrument)],
    {
      contract: standing === undefined ? '' : standing.id,
      buyer: d.buyer,
      seller: d.seller,
      instrument: String(d.instrument),
      qtyPerPeriod,
      pricePerPiece: d.pricePerPiece,
      breakCost: terms.breakCost,
      until: formatCivil(d.until),
      why: standing === undefined ? 'struck' : 'the two of them met again and restated it',
    },
    true,
  );
}

/** Law 2: what leaving it costs — the deliveries it promises, for the periods the two of them count. */
function costOfLeaving(
  ctx: MechanismContext,
  qtyPerPeriod: Qty,
  pricePerPiece: PerPiece,
  ccy: CurrencyCode,
): number {
  return ctx.registry.payable(
    scale(
      valueAt(pricePerPiece, qtyPerPeriod, ccy, 'what a period of it comes to'),
      asRatio(ctx.params.periods(SUPPLY_PARAMS.breakPeriods), 'the periods of it a break costs'),
      'what walking away from it costs',
    ),
  );
}

/**
 * Clearing A2, B2, Law 3: THE BOOK FOR A CONTRACT, and it is a book like any other. Buyers with the
 * most to pay are supplied first, sellers with the least to ask supply first, and the price is the
 * one the solver struck — read off its outcome and never re-derived from the fills (Law 19).
 */
function strike(ctx: MechanismContext, venue: VenueDecl): void {
  const subUnit = venue.key['subUnit'];
  const region = venue.key['region'] as RegionId | undefined;
  if (subUnit === undefined || region === undefined) return;
  const orders = ctx.posted(venue.id).filter((o) => o.price !== 'market');
  const outcome = clear(orders, 'proRata', 'marginalBid');
  // Law 1, XI-6: a book that did not cross SAYS SO, with both sides counted. A world where nobody
  // ever locks anything in is a finding about the world, and this is where it is visible.
  ctx.record(
    SUPPLY_PRINT,
    [venue.id],
    {
      venue: venue.id,
      outcome: outcome.kind,
      bids: orders.filter((o) => o.side === 'buy').length,
      asks: orders.filter((o) => o.side === 'sell').length,
      ...(isCleared(outcome) ? { pricePerPiece: outcome.price, volume: outcome.volume } : {}),
    },
    true,
  );
  if (!isCleared(outcome)) return;
  const good = goodId(subUnit, region);
  const ccy = ctx.registry.currencyOf(region);
  const until = addDays(
    ctx.calendar.startOf(ctx.period),
    ctx.params.periods(SUPPLY_PARAMS.term) * ctx.calendar.periodDays,
  );
  const sellers = outcome.fills.filter((f) => f.side === 'sell' && f.qty > 0).sort((a, b) => a.at - b.at);
  const buyers = outcome.fills.filter((f) => f.side === 'buy' && f.qty > 0).sort((a, b) => b.at - a.at);
  let at = 0;
  let left: Qty = sellers[0]?.qty ?? asQty(0);
  for (const b of buyers) {
    let want = b.qty;
    while (want > 0 && at < sellers.length) {
      const s = sellers[at];
      if (s === undefined) break;
      if (left <= 0) {
        at += 1;
        left = sellers[at]?.qty ?? asQty(0);
        continue;
      }
      if (String(s.party) === String(b.party)) {
        // Law 1: nobody contracts with itself. It takes the next seller, or none.
        at += 1;
        left = sellers[at]?.qty ?? asQty(0);
        continue;
      }
      const qty = asQty(atMost(want, left, 'there is no more of it promised than was offered'));
      signContract(ctx, {
        buyer: b.party,
        seller: s.party,
        region,
        instrument: good,
        qtyPerPeriod: qty,
        pricePerPiece: outcome.price,
        until,
        ccy,
      });
      want = subQty(want, qty, 'what it is still short of');
      left = subQty(left, qty, 'what this seller has left to promise');
    }
  }
}

/* ------------------------------------------------------------------------------------------------
 * LIVING WITH ONE
 * ---------------------------------------------------------------------------------------------- */

/**
 * Law 5, XI-5: THE DELIVERY, both legs in one instruction. The seller hands over what it has of what
 * it promised and the buyer pays for what arrived, atomically — so nobody delivers into a buyer that
 * cannot pay and nobody pays for goods that did not come.
 *
 * A seller short of stock delivers what it has. That is a real failure with a real record and the
 * contract STANDS: the buyer's answer to a supplier that cannot supply is to break the contract
 * (below), which costs the supplier what the two of them agreed it would.
 */
function deliver(ctx: MechanismContext, row: Agreement & { terms: SupplyTerms }): void {
  const t = row.terms;
  const has = ctx.register.free(row.debtor, t.instrument);
  const qty = asQty(atMost(has, t.qtyPerPeriod, 'it cannot deliver more than it holds'));
  if (qty <= 0) {
    ctx.record(
      SUPPLY_SHORT,
      [row.creditor, row.debtor, String(t.instrument)],
      { contract: row.id, seller: row.debtor, buyer: row.creditor, promised: t.qtyPerPeriod, delivered: 0 },
      true,
    );
    return;
  }
  const due = ctx.registry.payable(valueAt(t.pricePerPiece, qty, row.ccy, 'what the delivery comes to'));
  if (due <= 0) {
    // Law 5, Law 8: what this period's delivery comes to is less than one piece of the money, and
    // goods for no money is a one-sided flow. So nothing moves, and the record SAYS nothing moved
    // rather than a delivery quietly not happening — a contract this small is one the grid cannot
    // carry, which is a fact about the two of them and not a rounding anybody may do.
    ctx.record(
      SUPPLY_SHORT,
      [row.creditor, row.debtor, String(t.instrument)],
      {
        contract: row.id,
        seller: row.debtor,
        buyer: row.creditor,
        promised: t.qtyPerPeriod,
        delivered: 0,
        why: 'a period of it comes to less than a piece of the money',
      },
      true,
    );
    return;
  }
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: row.debtor,
      to: row.creditor,
      instrument: t.instrument,
      qty,
      // C2.a: what it changed hands at is the CONTRACT's price, which is the whole point of one.
      pricePerUnit: { some: true, value: t.pricePerPiece },
      accruedPerUnit: { some: false },
    },
    {
      kind: 'money',
      from: ctx.accountOf(row.creditor, row.ccy),
      to: ctx.accountOf(row.debtor, row.ccy),
      receipt: { of: 'disposal' },
      ccy: row.ccy,
      amount: asQty(due, 'what the buyer pays for it'),
    },
  ];
  const record = ctx.settle({ legs, cause: 'trade', reason: `delivery on ${row.id}` });
  ctx.record(
    record.outcome === 'settled' ? SUPPLY_DELIVERED : SUPPLY_SHORT,
    [row.creditor, row.debtor, String(t.instrument)],
    {
      contract: row.id,
      seller: row.debtor,
      buyer: row.creditor,
      promised: t.qtyPerPeriod,
      delivered: record.outcome === 'settled' ? qty : 0,
      pricePerPiece: t.pricePerPiece,
      paid: record.outcome === 'settled' ? due : 0,
      outcome: record.outcome,
    },
    true,
  );
}

/**
 * Law 2, Law 5 (17f.2): BREAKING ONE COSTS WHAT WAS AGREED, and either side may do it.
 *
 * Both of them are looking at the same two numbers and reaching opposite answers, which is what a
 * contract IS: the buyer keeps one whose price is below what it expects to pay and walks from one
 * whose price is above, by more than the break cost over the periods it has left; the seller the
 * other way round. Nothing is compared against a threshold — what is compared is what the contract
 * saves against what leaving it costs, both in the same money, both of them the party's own reads.
 *
 * The cost is one number struck when the contract was, paid in one instruction with both legs, and
 * the row ends. It is the same mechanism whichever side walks: there is no branch on who.
 */
function breaks(ctx: MechanismContext, row: Agreement & { terms: SupplyTerms }): boolean {
  const t = row.terms;
  const left = periodsLeft(ctx, t);
  if (left <= 0) return false;
  for (const side of ['buyer', 'seller'] as const) {
    const who = side === 'buyer' ? row.creditor : row.debtor;
    const view = ctx.participant(who);
    const spot = expectedPriceOf(view, t.instrument);
    if (!spot.some) continue;
    // What this side loses by staying, a piece: a buyer loses what it overpays, a seller loses what
    // it undersells for. One subtraction, read the other way round for the other side.
    const perPiece =
      side === 'buyer'
        ? sub(t.pricePerPiece, spot.value, 'what it overpays a piece')
        : sub(spot.value, t.pricePerPiece, 'what it undersells for a piece');
    if (perPiece <= 0) continue;
    const overTheTerm = mul(
      mul(perPiece, t.qtyPerPeriod, 'what a period of it costs it'),
      left,
      'what the rest of it costs it',
    );
    if (overTheTerm <= t.breakCost) continue;
    const paid = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(who, row.ccy),
          to: ctx.accountOf(side === 'buyer' ? row.debtor : row.creditor, row.ccy),
          receipt: { of: 'transfer' },
          ccy: row.ccy,
          amount: asQty(t.breakCost, 'what walking away from it costs'),
        },
      ],
      cause: 'transfer',
      reason: `${who} breaks ${row.id}`,
    });
    // Money E1: a party that cannot pay what breaking costs has not broken anything. It stays in.
    if (paid.outcome !== 'settled') continue;
    ctx.endAgreement(row.id, `${who} broke it and paid what that was agreed to cost`);
    ctx.record(
      SUPPLY_BROKE,
      [row.creditor, row.debtor, String(t.instrument)],
      {
        contract: row.id,
        broke: who,
        paid: t.breakCost,
        periodsLeft: left,
        wouldHaveCost: overTheTerm,
        pricePerPiece: t.pricePerPiece,
      },
      true,
    );
    return true;
  }
  return false;
}

/** Law 8: how many periods of it are left, from the two dates and the one calendar. */
/**
 * Law 4 (17f.3): A CONTRACT THAT RAN OUT IS EXTENDED, and it is the SAME ROW.
 *
 * The owner's ask was that contracts not multiply. A relationship that both of them still want does
 * not become a second row beside the first: its terms are restated — a new day, and whatever the two
 * of them now agree the thing is worth — and its identity does not move (Register F1's rule for an
 * instrument, and the same reason).
 *
 * WHAT THEY NOW AGREE is one offer and one answer, which is what a bilateral price IS when there is
 * no book in front of them: the seller would go on at what it expects to get, and the buyer goes on
 * if that is at or under what it expects to pay. Two parties whose expectations have crossed the
 * other way let it run out, and either of them may meet somebody else in the session (Law 3: the
 * price is still what two parties agreed, and no rule renewed it at the old one).
 */
function extended(ctx: MechanismContext, row: Agreement & { terms: SupplyTerms }): boolean {
  const t = row.terms;
  const asks = expectedPriceOf(ctx.participant(row.debtor), t.instrument);
  const pays = expectedPriceOf(ctx.participant(row.creditor), t.instrument);
  if (!asks.some || !pays.some || asks.value <= 0) return false;
  if (pays.value < asks.value) return false;
  const until = addDays(
    ctx.calendar.startOf(ctx.period),
    ctx.params.periods(SUPPLY_PARAMS.term) * ctx.calendar.periodDays,
  );
  const terms: SupplyTerms = {
    ...t,
    pricePerPiece: asks.value,
    until,
    breakCost: costOfLeaving(ctx, t.qtyPerPeriod, asks.value, row.ccy),
  };
  ctx.restate(row.id, terms);
  ctx.record(
    SUPPLY_EXTENDED,
    [row.creditor, row.debtor, String(t.instrument)],
    {
      contract: row.id,
      buyer: row.creditor,
      seller: row.debtor,
      instrument: String(t.instrument),
      qtyPerPeriod: t.qtyPerPeriod,
      was: t.pricePerPiece,
      pricePerPiece: asks.value,
      breakCost: terms.breakCost,
      until: formatCivil(until),
      why: 'its term ran out and both of them wanted another',
    },
    true,
  );
  return true;
}

function periodsLeft(ctx: MechanismContext, t: SupplyTerms): number {
  const today = ctx.calendar.startOf(ctx.period);
  if (compareCivil(t.until, today) <= 0) return 0;
  return div(
    sub(dayNumber(t.until), dayNumber(today), 'days of it left'),
    ctx.calendar.periodDays,
    'periods of it left',
  );
}

export function supply(): SystemModule {
  const mine = (v: VenueDecl): boolean => v.clearedBy === 'supply';
  return {
    id: 'supply',
    spec: 'Goods C3 Clearing A2 Law 3 Law 5',
    requires: ['goods'],
    instrumentKinds: [],
    agreementKinds: [supplyKind],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: paramsOf(),
    phases: [
      {
        name: 'supply.strike',
        spec: 'Clearing A2 Clearing B2 Law 3',
        // Before the spot session: what a firm has locked in is part of what it takes to market.
        anchor: { before: 'markets' },
        reads: [],
        writes: [
          { kind: 'event', name: SUPPLY_PRINT },
          { kind: 'event', name: SUPPLY_STRUCK },
          { kind: 'event', name: SUPPLY_EXTENDED },
        ],
        run: (ctx: MechanismContext): void => {
          for (const v of ctx.venues.filter(mine)) {
            ctx.gather(v.id);
            strike(ctx, v);
          }
        },
      },
      {
        name: 'supply.run',
        spec: 'Law 5 Law 2 XI-5',
        // After the session, so a seller that bought what it was short of can still deliver.
        anchor: { after: 'markets' },
        reads: [],
        writes: [
          { kind: 'event', name: SUPPLY_EXTENDED },
          { kind: 'event', name: SUPPLY_DELIVERED },
          { kind: 'event', name: SUPPLY_SHORT },
          { kind: 'event', name: SUPPLY_BROKE },
          { kind: 'event', name: SUPPLY_ENDED },
        ],
        run: (ctx: MechanismContext): void => {
          for (const row of contractsOf(ctx)) {
            const buyer = ctx.parties.get(row.creditor);
            const seller = ctx.parties.get(row.debtor);
            if (!buyer.status.alive || !seller.status.alive) {
              ctx.endAgreement(row.id, 'a side of it has ceased');
              continue;
            }
            if (periodsLeft(ctx, row.terms) <= 0) {
              if (!extended(ctx, row)) {
                ctx.endAgreement(row.id, 'its term ran out and neither of them wanted another');
                ctx.record(
                  SUPPLY_ENDED,
                  [row.creditor, row.debtor],
                  { contract: row.id, why: 'its term ran out and neither of them wanted another' },
                  true,
                );
              }
              continue;
            }
            if (breaks(ctx, row)) continue;
            deliver(ctx, row);
          }
        },
      },
    ],
    participants: [],
    venueParticipants: [
      {
        partyKind: FIRM,
        /**
         * Law 18 (0g): THE SUPPLY VENUES OF ITS OWN REGION. `contractOrders` returns nothing for
         * any other — a contract is struck where the good is made and bought — so this names what
         * that function already decides, out of the same read (`view.self.region`, `venue.key`).
         *
         * It named none, so every firm was asked about every venue in the world: 221,130 questions
         * a period in the scale model, of which all but a region's worth answered nothing.
         */
        venues: (view: ParticipantView): readonly VenueId[] => {
          const mine: VenueId[] = [];
          for (const v of view.venues) {
            if (v.clearedBy !== 'supply') continue;
            if (v.key['region'] !== String(view.self.region)) continue;
            mine.push(v.id);
          }
          return mine;
        },
        orders: (view, venue) => contractOrders(view, venue),
      },
    ],
    families: [oneEach()],
    /**
     * A BOOK FOR THE THINGS ANYBODY BUYS TO MAKE SOMETHING ELSE, and for nothing else.
     *
     * A contract locks in SUPPLY, so the goods that have one are the goods that are an input to some
     * other line — a mill contracts for grain, nobody contracts for a loaf on a shelf — and they
     * have to be storable: a service is made where it is bought and cannot be promised out of
     * stock. Both of those are read off the goods' own terms (Law 19), so this list is not a table
     * anybody keeps and a world with different recipes gets different books.
     */
    seed(ctx: SeedContext): void {
      const wanted = new Set<string>();
      for (const i of ctx.instruments.all()) {
        if (!isGoodTerms(i.terms)) continue;
        for (const input of i.terms.recipe.inputs) wanted.add(input.subUnit);
      }
      for (const region of ctx.registry.regions.values()) {
        for (const subUnit of wanted) {
          const id = goodId(subUnit, region.id);
          if (!ctx.instruments.has(id)) continue;
          const good = ctx.instruments.get(id);
          if (!isGoodTerms(good.terms)) continue;
          // A thing that is all gone by the end of the period it was made in cannot be promised for
          // delivery in a later one: what perishes entirely is made where it is bought (Goods E4).
          if (ctx.params.ratio(good.terms.spoilage) >= 1) continue;
          ctx.openVenue({
            id: supplyVenue(subUnit, region.id),
            name: `Supply contracts, ${subUnit}, ${region.name}`,
            clearedBy: 'supply',
            unit: good.unit,
            ccy: ctx.registry.currencyOf(region.id),
            key: { region: region.id, subUnit },
          });
        }
      }
    },
  };
}

function paramsOf(): ParamDecl[] {
  return [
    {
      id: SUPPLY_PARAMS.term,
      value: 26,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Half a year of deliveries. It is what the two of them are agreeing to when they sign, and it is long enough that the session price will have moved away from the struck one before it runs out — which is what makes a contract worth having and worth breaking.',
    },
    {
      id: SUPPLY_PARAMS.breakPeriods,
      value: 4,
      unit: 'periods of deliveries',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'What walking away costs, as periods of what was promised — the convention the two of them strike it by, turned into a number of pieces of money ON THIS CONTRACT when it is signed. It is never applied to a contract afterwards: what a party owes for leaving is what it agreed to, and this is what both of them meant by that.',
    },
  ];
}

/**
 * Law 4, Audit A1 (17f.3): THE COUNT SAYS SO IF THEY EVER MULTIPLY.
 *
 * A FORBID is as valuable as a mechanism and it breaks silently, so it is guarded: at most one live
 * contract per (buyer, seller, thing), which is what "extended, never multiplied" means measured
 * rather than asserted. A world that starts making two rows out of one relationship says so here,
 * with both names and the count, and nothing repairs it (Audit D3).
 */
export function oneEach(): Family {
  return {
    name: 'names',
    contributor: 'supply',
    spec: 'Law 4 Goods C3',
    built: true,
    check: (view): Violation[] => {
      const out: Violation[] = [];
      const seen = new Map<string, number>();
      for (const a of view.agreements.ofKind(SUPPLY)) {
        if (a.state !== 'performing' || !isSupplyTerms(a.terms)) continue;
        const key = `${String(a.creditor)}|${String(a.debtor)}|${String(a.terms.instrument)}`;
        const had = seen.get(key);
        seen.set(key, had === undefined ? 1 : had + 1);
      }
      for (const [key, n] of seen) {
        if (n <= 1) continue;
        const [buyer, seller, instrument] = key.split('|');
        out.push({
          family: 'names',
          spec: 'Law 4',
          owner: String(buyer),
          size: n,
          unit: 'contracts',
          period: view.period,
          message: `${String(buyer)} and ${String(seller)} have ${n} live contracts for ${String(instrument)}: one relationship with two answers to what was agreed (17f.3)`,
        });
      }
      return out;
    },
  };
}
