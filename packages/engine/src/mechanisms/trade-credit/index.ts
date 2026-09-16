/**
 * Trade credit: the receivable and the payable that are one row read from two sides.
 *
 * @spec Trade Credit A1 Trade Credit A2 Trade Credit A3 Trade Credit A4 Trade Credit B1 Trade Credit B2 Trade Credit B3 Trade Credit B4 Trade Credit B5 Trade Credit C1 Trade Credit C2 Trade Credit C4 Trade Credit D1 Trade Credit D2 Trade Credit D4 Trade Credit E1 Trade Credit E3 Firm Birth D2.b XI-1 XI-11 Law 4 Law 5 Law 15 Law 19
 *
 * MOST OF THE CREDIT IN AN ECONOMY IS NOT A BANK'S. A firm that buys from another firm pays in
 * thirty days, and until it does the seller has lent it the money without either of them calling it
 * that. This world had none of it: every trade settled money-for-goods in the same instant, so a
 * supply chain could not transmit anything except a price, and a firm could not fail because
 * somebody else failed to pay it.
 *
 * AN INVOICE IS ONE ROW (A1). It is the seller's receivable and the buyer's payable — the same
 * instrument, held by one and issued by the other — so there is exactly one writer of the fact and
 * the two sides cannot disagree (Law 4). It is written in the SAME numbered instruction the goods
 * move in: the buyer pays with a promise instead of with money, both legs are there, and there is
 * never an instant where one side has parted with something and the other has given nothing.
 *
 * IT IS A ZERO-COUPON BILL AND NOTHING MORE (A2). One payment, of the whole amount, on a date —
 * so the kernel's own corporate-action phase collects it, the borrower's failure to pay is the
 * recorded state that phase already writes (XI-1), and the claim ranks in the estate because it is
 * an ordinary liability of an ordinary party (Firm Birth D2.b). Nothing here re-implements any of
 * that; what this module adds is the DECISION.
 *
 * AND THE DECISION IS THE SELLER'S JUDGEMENT OF THE BUYER (A3, B5). It ships on terms to a buyer it
 * expects to be paid by, and for cash otherwise — read from its own record of that buyer, which is
 * the same shape a bank's is and is held by the seller rather than by anybody else (Corporate Credit
 * A4: an opinion is somebody's). A buyer that has failed to pay it before gets no terms, which is
 * B3's "stop shipment" and D4's tightening in one read: what a seller knows about a customer is
 * what that customer did to it.
 */
import { asPerPiece, asRatio, minus, over, scale, type Ratio } from '../../core/measure.js';
import { addDays, compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import { percent } from '../../core/format.js';
import { between, betweenWhole } from '../../rng/spread.js';
import { assertTermsAreOrdered, sellerParam, TERMS_SPREAD } from './data.js';
import { depositRateFor } from '../../registry/banking.js';
import { downTick } from '../../core/tick.js';
import {
  instrumentId,
  instrumentKindId,
  type InstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import { none, some, type Option } from '../../core/option.js';
import { currencyUnit } from '../../core/ids.js';
import type { CashFlow, InstrumentKindProfile } from '../../registry/kinds.js';
import type { Terms, Instrument } from '../../register/instruments.js';
import { FIRM, SMALL_FIRM } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';
import type { Period } from '../../calendar/calendar.js';
import { isShort, ownFundingThisPeriod } from '../../registry/funding.js';
import type { SystemModule, TermsSale } from '../../world/module.js';

export const INVOICE = instrumentKindId('invoice');

/** A1: one row per (seller, buyer, sale). Named so a reader can see whose it is (Law 9). */
export const invoiceId = (
  seller: PartyId,
  buyer: PartyId,
  at: Period,
  n: number,
): InstrumentId => instrumentId(`invoice:${seller}:${buyer}:${String(at)}:${String(n)}`);

/**
 * A1, Law 4, Law 19: THE NEXT FREE ROW BETWEEN THIS PAIR, asked of the register rather than of a
 * counter this module keeps.
 *
 * One pair can trade twice in a period — two goods, two books — so the period alone does not name a
 * row, and a second sale would collide with the first at `ctx.issue`. The register is the one
 * writer of what exists, so it is the one that can say which name is free: no second copy of the
 * sequence, and nothing to keep true.
 */
function freeRow(ctx: MechanismContext, seller: PartyId, buyer: PartyId): InstrumentId {
  // Law 9, Law 18 (17.5): the PERIOD is part of the name. One pair can trade twice in a period, so
  // the period alone does not name a row — but a pair that has traded every week for a year had a
  // scan of a year's rows to do before writing this week's, and the name said nothing about when.
  // With the period in it the scan is over this period's rows alone, and a reader can see the week.
  for (let n = 1; ; n += 1) {
    const id = invoiceId(seller, buyer, ctx.period, n);
    if (!ctx.instruments.has(id)) return id;
  }
}

export interface InvoiceTerms extends Terms {
  readonly kind: typeof INVOICE;
  readonly seller: PartyId;
  readonly buyer: PartyId;
  /** A2: the day the whole of it falls due. There is no coupon and no schedule; there is a date. */
  readonly due: Civil;
  /**
   * A3 (17.7a): what this seller takes off for being paid by `discountBy`, and the day that window
   * shuts. They are on the ROW because they are terms of the sale the two of them struck, like the
   * date — a buyer reads what it was offered, not what its supplier is offering this week.
   *
   * WITH THE TWO DATES THEY ARE AN INTEREST RATE (`impliedRate`), which is the whole of why A3 is a
   * clause: it is the rate at which the seller buys its own money back early, and the rate a factor
   * has to beat to buy the receivable instead.
   */
  readonly discount: Ratio;
  readonly discountBy: Civil;
}

export const isInvoice = (t: Terms): t is InvoiceTerms =>
  'seller' in t && 'buyer' in t && 'due' in t;

export function invoiceTerms(i: Instrument): InvoiceTerms {
  if (!isInvoice(i.terms)) {
    throw new InvalidRegistry('Trade Credit A1', `${i.id} is not an invoice`);
  }
  return i.terms;
}

export const invoiceKind: InstrumentKindProfile = {
  id: INVOICE,
  // A2, D1: it promises one sum on one day and nobody makes a market in it, so it is carried at what
  // it cost. What a LATE one is worth is its holder's own question (D1) and is a write-down of the
  // lot, the same door inventory and a loan use (Goods E2, Banks Lending D2).
  pricing: 'carriedAtCost',
  carry: 'cost',
  liabilityOfIssuer: true,
  // Register B3: a buyer owes the invoice. A supplier's doubt about it is the SUPPLIER's write-down.
  owes: 'face',
  unit: (ccy) => currencyUnit(ccy),
  validateTerms: (t) => {
    if (!isInvoice(t)) throw new InvalidRegistry('Trade Credit A1', 'not invoice terms');
    if (t.seller === t.buyer) {
      throw new InvalidRegistry('Trade Credit A1', 'an invoice needs two parties');
    }
    // A3: the window is inside the term, or the discount is an offer that expires after the money
    // was due — which is not an early-payment discount, it is nothing.
    if (compareCivil(t.discountBy, t.due) >= 0) {
      throw new InvalidRegistry('Trade Credit A3', 'the early-payment window outlasts the invoice');
    }
  },
  // Law 9: as the trade names it — who on whom, what is off for paying early, and when it is due.
  displayName: (i) =>
    isInvoice(i.terms)
      ? `${String(i.terms.seller)} on ${String(i.terms.buyer)}, ${percent(i.terms.discount)} by ` +
        `${formatCivil(i.terms.discountBy)}, due ${formatCivil(i.terms.due)}`
      : String(i.id),
  ranking: () => ({
    seniority: 0,
    secured: [],
    // Firm Birth D2.b: a trade creditor ranks with the other unsecured ones, and it is stated here
    // rather than in the estate, because what a claim ranks as is a fact about the claim.
    claim: 'unsecured: a supplier that shipped and has not been paid',
  }),
  /** A2: one payment, of everything, on the day. The kernel's own phase collects it (XI-1). */
  cashFlows: (i: Instrument, on: Civil): readonly CashFlow[] => {
    if (!isInvoice(i.terms)) return [];
    const due = i.terms.due;
    // A2: one payment, of everything, on the day. Nothing before it and nothing after it.
    return compareCivil(due, on) < 0
      ? []
      : [{ date: due, perUnit: asPerPiece(1, 'an invoice pays its face') }];
  },
  due: (i, period, calendar) => {
    if (!isInvoice(i.terms)) return [];
    const due = i.terms.due;
    // Trade Credit A3, Money E1 (12a.3): FROM THE DUE DATE UNTIL IT IS PAID. It was presented in the
    // period the date fell in and never again, so a buyer that missed it owed a row nobody asked
    // for; what is not paid stands and is asked for every period, and the miss is the default.
    const to = calendar.endOf(period);
    if (compareCivil(due, to) > 0) return [];
    return [{ kind: 'maturity', date: compareCivil(due, calendar.startOf(period)) < 0 ? calendar.startOf(period) : due }];
  },
  accrued: () => 0,
};

/**
 * A3, B5 (17.7a, finding 21.61): THIS SELLER'S OWN TERMS, drawn once and declared under its own name.
 *
 * B5 says the seller decides its terms, and one number for the whole world said the opposite: a mill
 * and a corner shop wrote the same invoice. What stopped this being fixed where the other drawn
 * preferences are is that a seller here is a firm, a small-business cell or a merchant, and the
 * firms module draws numbers for firms only — so the preference had no home every seller has. THE
 * PARAMETER REGISTER IS THAT HOME (XI-14): it is where a firm born after the seed declares its
 * hurdle and its horizon, it is keyed by the party's own name, and it holds a number for the life of
 * the world. The draw happens the first time a seller ships on terms, from the module's own stated
 * spreads, and every invoice it writes afterwards reads what it declared.
 *
 * The stream is derived from the SELLER's name so the same world gives the same seller the same
 * terms whenever it first ships (the period the parent stream carries never reaches the draw).
 */
export interface SellerTerms {
  readonly days: number;
  readonly discountDays: number;
  readonly discount: Ratio;
}

function termsOf(ctx: MechanismContext, seller: PartyId): SellerTerms {
  const days = sellerParam(String(seller), 'days');
  const windowOf = sellerParam(String(seller), 'discountDays');
  const off = sellerParam(String(seller), 'discount');
  if (!ctx.params.has(days)) {
    assertTermsAreOrdered(TERMS_SPREAD);
    const rng = ctx.rng.derive(`terms/${String(seller)}`);
    const drawn = {
      days: betweenWhole(rng, TERMS_SPREAD.days),
      discountDays: betweenWhole(rng, TERMS_SPREAD.discountDays),
      discount: between(rng, TERMS_SPREAD.discount),
    };
    ctx.declare({
      id: days,
      value: drawn.days,
      unit: 'days a buyer has to pay',
      dimension: 'days',
      kind: 'preference',
      owner: 'model',
      why: `Trade Credit A3, B5: how long ${String(seller)} gives a buyer to pay, drawn from the stated width the first time it shipped on terms. ${TERMS_SPREAD.days.why}`,
    });
    ctx.declare({
      id: windowOf,
      value: drawn.discountDays,
      unit: 'days the early-payment window is open',
      dimension: 'days',
      kind: 'preference',
      owner: 'model',
      why: `Trade Credit A3: how long ${String(seller)} leaves its discount open. ${TERMS_SPREAD.discountDays.why}`,
    });
    ctx.declare({
      id: off,
      value: drawn.discount,
      unit: 'share of the face taken off for paying inside the window',
      dimension: 'ratio',
      kind: 'preference',
      owner: 'model',
      why: `Trade Credit A3: what ${String(seller)} takes off to be paid early. ${TERMS_SPREAD.discount.why}`,
    });
  }
  return {
    days: ctx.params.days(days),
    discountDays: ctx.params.days(windowOf),
    discount: ctx.params.ratio(off),
  };
}

/**
 * A3: THE DISCOUNT IS AN INTEREST RATE, and this is the one place it is read as one.
 *
 * Paying `1 - d` on the window's last day instead of `1` on the due day buys the buyer the days
 * between them, and what it pays for them is `d` of what it still owed — so the rate per annum is
 * `d / (1 - d)` over the fraction of a year those days are. It is a DERIVED read of terms the two
 * of them struck, never a number anybody declared, and it is what a factor has to beat to buy the
 * receivable instead (17.7b) and what a buyer weighs its own money against.
 *
 * Nothing where the window has already shut: an offer that has expired has no rate.
 */
export function impliedRate(t: InvoiceTerms, on: Civil): Option<Ratio> {
  if (compareCivil(on, t.discountBy) > 0) return none<Ratio>();
  const span = yearFraction('ACT/365F', t.discountBy, t.due);
  if (span <= 0) return none<Ratio>();
  const paid = minus(asRatio(1, 'the face'), t.discount, 'what it pays if it pays early');
  return some(
    over(
      over(t.discount, paid, 'what the days cost, per unit it still owed'),
      asRatio(span, 'the fraction of a year they are'),
      'per annum',
    ),
  );
}

/**
 * A3, B5, D4: WHETHER THIS SELLER SHIPS THIS BUYER ON TERMS. It is a read of its own record of that
 * buyer and of nothing else: a buyer that has failed to pay it is a buyer it sells to for cash, and
 * one it has never been let down by gets the terms this trade is done on.
 *
 * B3 and D4 are the same read from two sides — stopping shipment and tightening terms are one
 * decision a seller takes about one customer — and D4.a's "a solvent firm starved of terms can die
 * of a rumour" is what falls out of it when several sellers have seen the same miss.
 */
function shipsOnTerms(ctx: MechanismContext, sale: TermsSale): Option<InstrumentId> {
  const { seller, buyer, ccy, cash, sold } = sale;
  if (cash <= 0 || seller === buyer) return none<InstrumentId>();
  // A1: TERMS ARE FOR GOODS, AND A GOOD IS A THING NOBODY ISSUED. A tonne is what its holder owns;
  // a share or a bond is somebody's promise, bought and paid for on delivery (XI-5), and a firm
  // selling its own paper is raising money rather than shipping. The register already knows which
  // is which — it holds the issuer or it holds none — so nothing here branches on a kind (Law 15).
  if (ctx.instruments.get(sold).issuer.some) return none<InstrumentId>();
  const both = ctx.parties.get(seller).status.alive && ctx.parties.get(buyer).status.alive;
  if (!both) return none<InstrumentId>();
  // A3, Households C1.d, Law 15 (11.0b): terms are for buyers of a kind that takes them, which the
  // registry says; a household pays for a loaf with money it has, whatever its seller thinks of it.
  if (!ctx.registry.partyKind(ctx.parties.get(buyer).kind).buysOnTerms) return none<InstrumentId>();
  // B5, Corporate Credit A4: the seller's own record of this buyer. A missed payment it was a side
  // of is something it saw; what other sellers saw is theirs.
  if (letDown(ctx, seller, buyer)) return none<InstrumentId>();
  /**
   * B2, B5, C1.a (17.5): A SELLER SHORT OF CASH SHIPS FOR CASH.
   *
   * Offering terms is lending: the seller hands over the goods now and is paid in a month, so it
   * funds the buyer for a month out of its own account. A seller that has published that it is
   * SHORT of money this period is a seller with nothing to fund anybody with — and the thing it
   * needs from this sale is the cash, not a receivable it will have to finance or factor.
   *
   * It is a read of what the seller itself published (E4, Law 19), so a firm whose gap closed says
   * so the next period and its terms come back with it. B5 is explicit that terms are the seller's
   * decision per buyer on that buyer's condition; this is the other half — on its OWN condition —
   * and it is what makes a cash squeeze travel along the supply chain rather than through a bank
   * (D3: a contagion path that runs firm to firm).
   */
  if (shortOfCash(ctx, seller)) return none<InstrumentId>();
  // B5, 21.61: the terms are THIS seller's, drawn under its own name and read from the register.
  const mine = termsOf(ctx, seller);
  const on = ctx.calendar.startOf(ctx.period);
  const id = freeRow(ctx, seller, buyer);
  const invoice: InvoiceTerms = {
    kind: INVOICE,
    seller,
    buyer,
    due: addDays(on, mine.days),
    discount: mine.discount,
    discountBy: addDays(on, mine.discountDays),
  };
  ctx.issue({
    id,
    kind: INVOICE,
    issuer: some(buyer),
    ccy,
    terms: invoice,
    market: none(),
  });
  return some(id);
}

/**
 * B2, C1.a (17.5): HAS THIS SELLER SAID IT IS SHORT OF MONEY THIS PERIOD? A read of its own funding
 * publication — the one door every borrower in this world publishes a gap through — and nothing
 * where it published none, which is a seller with no gap rather than a seller with no view.
 */
function shortOfCash(ctx: MechanismContext, seller: PartyId): boolean {
  // Observer A4: THE SELLER'S OWN VIEW, for the seller's own decision — this is what the door is
  // for, and the decision reads no counterparty's state (17.0a, 21.57). It is asked through the
  // view rather than the wire because the shipping decision is taken inside the kernel's own
  // session (`markets`), which declares no module's reads.
  const said = ownFundingThisPeriod(ctx.participant(seller), ctx.period);
  if (!said.some) return false;
  // E4: what it is short of NOW is the half a receivable in a month cannot answer.
  return isShort(said.value.shortNow, said.value.ccy, 'what it is short of now').some;
}

/**
 * D1, D4: IS THIS BUYER IN ARREARS TO THIS SELLER? Read off the seller's own book, which is where
 * the answer already is: an invoice it still holds, issued by this buyer, whose day has gone by.
 *
 * There is no memory here and no number for one. A supplier does not consult a record of old
 * grievances — it looks at what is on its books and overdue, and the moment the buyer pays, the row
 * is redeemed and the seller no longer holds it, so terms resume. The ageing IS the tightening (D4),
 * lateness IS a state rather than an event to remember (D1), and D4.a's "a solvent firm starved of
 * terms can die of a rumour" is what happens when several suppliers are each looking at their own
 * overdue row at once.
 */
function letDown(ctx: MechanismContext, seller: PartyId, buyer: PartyId): boolean {
  return overdue(ctx, seller).some((row) => row.buyer === buyer);
}

/**
 * D1, Law 19: THE RECEIVABLES AGEING, which is one read and not a stored number. What this seller
 * shipped, has not been paid for, and should have been paid for by now.
 *
 * IT IS A READ OF THE REGISTER, and item 9.1 is why. An invoice is an INSTRUMENT — it has an
 * issuer, a holder, an issued amount and terms, and the register is its kernel home — so the
 * `invoices` book beside it was a second copy of facts the world already held: the id, the seller,
 * the buyer and the due date were every one of them on the instrument. A mirror is the defect Law
 * 19 names, and the docstring's reason for keeping it (*"so nothing asks an instrument what kind it
 * is"*) was a misreading of Law 15: filtering a list by kind is not branching a MECHANISM on one,
 * and `isInvoice` is this module's own predicate, declared for exactly this.
 *
 * What the seller holds is the register's answer; what each row promises is the instrument's; and
 * it is overdue when the day on its own terms has gone by.
 */
interface Ageing {
  at: number | undefined;
  bySeller: Map<string, readonly Written[]>;
}

/**
 * D1, D4, Law 18 (17.5): WHAT THIS SELLER IS OWED PAST ITS DAY — walked once per seller per period.
 *
 * Every sale asks it, and a seller with a thousand rows on its book was walking all of them for
 * every tonne it shipped. A settled period's invoices cannot change their due dates and a row
 * redeemed mid-period leaves the seller's book, so the walk is taken when the period's first sale
 * asks and read by the rest. It is a memo of a walk and never a second copy of the register: the
 * rows are the source, and the next period walks them again.
 */
export function overdue(ctx: MechanismContext, seller: PartyId): readonly Written[] {
  const held = ctx.state<Ageing>('tradeCredit.overdue', () => ({
    at: undefined,
    bySeller: new Map(),
  }));
  if (held.at !== ctx.period) {
    held.at = ctx.period;
    held.bySeller.clear();
  }
  const had = held.bySeller.get(String(seller));
  if (had !== undefined) return had;
  const walked = walkOverdue(ctx, seller);
  held.bySeller.set(String(seller), walked);
  return walked;
}

function walkOverdue(ctx: MechanismContext, seller: PartyId): readonly Written[] {
  const today = ctx.calendar.endOf(ctx.period);
  const out: Written[] = [];
  for (const h of ctx.register.holdingsOf(seller)) {
    if (ctx.register.quantity(seller, h.instrument) <= 0) continue;
    const i = ctx.instruments.get(h.instrument);
    // Law 15: the structural predicate, not a comparison of kind ids — what makes these terms an
    // invoice is that they name a seller, a buyer and the day the whole of it falls due.
    if (!isInvoice(i.terms)) continue;
    // A2: one sum on one day. A day past its day, with the row still on the seller's book, is a
    // buyer that has not paid.
    if (compareCivil(i.terms.due, today) > 0) continue;
    out.push({ id: i.id, seller: i.terms.seller, buyer: i.terms.buyer, due: i.terms.due });
  }
  return out;
}

/**
 * A3, B1, B4 (17.7a): WHAT A BUYER'S MONEY EARNS WHERE IT SITS, which is what it is choosing
 * between when it decides whether to pay early.
 *
 * Its money is a deposit at its own bank, and what that pays is the board the bank published for the
 * CLASS this buyer's kind belongs to — a public number, read where everybody can reach it
 * (`registry/banking.ts`), never re-derived and never a table here (Law 4, Law 19). Which class it
 * is, is the registry's to say and never this module's (Law 15).
 *
 * Nothing where the kind keeps no deposit or the bank has posted no board: a buyer that does not
 * know what its money earns has nothing to compare the discount with, and that is a stated answer
 * rather than a zero anybody chose (Appendix A).
 */
function whatMoneyEarns(ctx: MechanismContext, buyer: PartyId): Option<Ratio> {
  const who = ctx.parties.get(buyer);
  const cls = ctx.registry.partyKind(who.kind).depositClass;
  if (cls === null) return none<Ratio>();
  return depositRateFor(ctx.journal, String(who.bank), cls);
}

/**
 * A3, B1 (17.7a): THE BUYER PAYS EARLY WHEN THE DISCOUNT IS DEARER THAN ITS OWN MONEY IS.
 *
 * A3 makes the discount an interest rate, and a rate is a thing somebody chooses against something.
 * What the buyer chooses against is what its money earns where it is (B1's other side: the terms
 * bridge a gap, and a buyer with no gap is holding cash that is earning the deposit board). If the
 * seller is offering more for its money than its bank is, it pays; otherwise it keeps the money and
 * pays on the day. Neither branch is a preference — both sides of it are numbers somebody published.
 *
 * Paying early is a redemption BELOW PAR, and that is the seller's cost of it: the holder gives up
 * the row at what it agreed to take, realises the difference against what it was carrying, and the
 * buyer keeps what it did not pay. Nothing is written off and nothing is created; it is one
 * instruction with two legs, like every other (Law 5).
 *
 * It settles whoever HOLDS the row, not whoever wrote it: a receivable that has been sold is owed to
 * its buyer, and an early payment pays the party that is owed (Law 19).
 */
function paysEarly(ctx: MechanismContext): void {
  const today = ctx.calendar.startOf(ctx.period);
  for (const i of ctx.instruments.ofKind(INVOICE)) {
    if (!i.status.live || !isInvoice(i.terms)) continue;
    const t = i.terms;
    const rate = impliedRate(t, today);
    if (!rate.some) continue;
    const buyer = ctx.parties.resolve(t.buyer).id;
    if (!ctx.parties.get(buyer).status.alive) continue;
    const earns = whatMoneyEarns(ctx, buyer);
    if (!earns.some || rate.value <= earns.value) continue;
    for (const holder of ctx.register.holdersOf(i.id)) {
      const units = ctx.register.quantity(holder, i.id);
      if (units <= 0) continue;
      const per = minus(asRatio(1, 'the face'), t.discount, 'what it pays for a unit');
      // Law 8: money moves in whole pieces of itself, so what it pays is the discounted face down
      // to money that exists. What the rounding drops is not paid, because it is not money.
      const pay = downTick(scale(units, per, 'what it pays for the lot'));
      if (pay <= 0) continue;
      const r = ctx.settle({
        legs: [
          {
            kind: 'asset',
            from: holder,
            to: buyer,
            instrument: i.id,
            qty: units,
            pricePerUnit: some(asPerPiece(per, 'the face less what was taken off')),
            accruedPerUnit: none(),
          },
          {
            kind: 'money',
            from: ctx.accountOf(buyer, i.ccy),
            to: ctx.accountOf(holder, i.ccy),
            receipt: { of: 'returnOfCapital' },
            ccy: i.ccy,
            amount: pay,
          },
        ],
        cause: 'maturity',
        reason: `${String(buyer)} pays ${String(i.id)} early and takes ${percent(t.discount)} off`,
      });
      if (r.outcome !== 'settled') continue;
      ctx.record(
        'tradeCredit.paidEarly',
        [String(i.id), String(holder), String(buyer)],
        {
          invoice: String(i.id),
          holder: String(holder),
          buyer: String(buyer),
          paid: pay,
          face: units,
          impliedRate: rate.value,
          earns: earns.value,
        },
        false,
      );
    }
  }
}

/** One invoice, as this module reads it: who shipped, who owes, which row and when it falls due. */
export interface Written {
  readonly id: InstrumentId;
  readonly seller: PartyId;
  readonly buyer: PartyId;
  readonly due: Civil;
}

export function tradeCredit(): SystemModule {
  return {
    id: 'trade-credit',
    nouns: [
      {
        name: 'tradeCredit.overdue',
        kind: 'working',
        holds: 'which period this ageing covers and what each seller was owed past its day in it',
        why: 'a within-period memo of a walk over the register (Law 18): every sale asks whether this seller has been let down, and a seller with a thousand rows was walking all of them for every tonne it shipped. The rows are the source and the next period walks them again; nothing here outlives the period it was read in (17.5).',
      },
    ],
    // XI-8, item 9.1: NO NOUNS. The `invoices` book was declared a placeholder for `Agreement`,
    // and it was not one: an invoice is an INSTRUMENT and the register is its kernel home already.
    // What the book held — the id, the seller, the buyer, the due date — was on the instrument, so
    // it was a mirror of the world (Law 19), not a noun with nowhere to live. It is deleted and the
    // ageing reads the register.
    spec: 'Trade Credit',
    requires: ['firms', 'goods'],
    instrumentKinds: [invoiceKind],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    /**
     * A3, B5 (17.7a, finding 21.61): NO NUMBER FOR THE WHOLE WORLD. `tradeCredit.days` was one —
     * thirty days, a technology, the same for a mill and a corner shop — and B5 says the seller
     * decides. Each seller's days, its window and its discount are drawn from `TERMS_SPREAD` and
     * declared under its own name the first time it ships on terms (`termsOf`), which is where every
     * other drawn preference in this world lives (XI-14).
     */
    params: [],
    phases: [
      {
        name: 'tradeCredit.payEarly',
        spec: 'Trade Credit A3 Trade Credit B1',
        // After the period's shipping (`firms.invoice`, cycle 2) so a buyer can take a discount on
        // the invoice it was handed this week, and before the marks, so what the seller realised on
        // a row it gave up is in the books the revaluation reads (Clearing F1).
        anchor: { before: 'revaluation' },
        reads: [{ kind: 'event', name: 'bank.depositRate', of: 'anyPeriod' }],
        writes: [{ kind: 'event', name: 'tradeCredit.paidEarly' }],
        run: paysEarly,
      },
    ],
    participants: [],
    // A3, B5: exactly one module answers what a firm ships on, and it is the one that owns firms'
    // judgements of each other. The kernel writes the leg; this decides what goes in it.
    // §36 A4, 11.0b: AND A SMALL FIRM SHIPS ON TERMS TOO — the same judgement of the same record,
    // because a corner shop's supplier and a corner shop are both firms (§42 A1), and the row a
    // cell writes to a cell is the trade credit the tier lives on.
    termsOffered: [
      { partyKind: FIRM, decide: shipsOnTerms },
      { partyKind: SMALL_FIRM, decide: shipsOnTerms },
    ],
    families: [],
  };
}
