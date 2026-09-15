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
import { asPerPiece } from '../../core/measure.js';
import { addDays, compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import {
  instrumentId,
  instrumentKindId,
  paramId,
  type InstrumentId,
  type ParamId,
  type PartyId,
} from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import { none, some, type Option } from '../../core/option.js';
import { currencyUnit } from '../../core/ids.js';
import type { CashFlow, InstrumentKindProfile } from '../../registry/kinds.js';
import type { Terms, Instrument } from '../../register/instruments.js';
import { FIRM, SMALL_FIRM } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule, TermsSale } from '../../world/module.js';

export const INVOICE = instrumentKindId('invoice');

export const TRADE_CREDIT_PARAMS = {
  days: paramId('tradeCredit.days'),
} as const satisfies Record<string, ParamId>;

/** A1: one row per (seller, buyer, sale). Named so a reader can see whose it is (Law 9). */
export const invoiceId = (seller: PartyId, buyer: PartyId, n: number): InstrumentId =>
  instrumentId(`invoice:${seller}:${buyer}:${n}`);

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
  for (let n = 1; ; n += 1) {
    const id = invoiceId(seller, buyer, n);
    if (!ctx.instruments.has(id)) return id;
  }
}

export interface InvoiceTerms extends Terms {
  readonly kind: typeof INVOICE;
  readonly seller: PartyId;
  readonly buyer: PartyId;
  /** A2: the day the whole of it falls due. There is no coupon and no schedule; there is a date. */
  readonly due: Civil;
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
  },
  displayName: (i) =>
    isInvoice(i.terms)
      ? `${String(i.terms.seller)} on ${String(i.terms.buyer)}, due ${formatCivil(i.terms.due)}`
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
    // Money G3.a: a date falls in the period the calendar places it in, and the calendar says.
    const from = calendar.startOf(period);
    const to = calendar.endOf(period);
    if (compareCivil(due, from) < 0 || compareCivil(due, to) > 0) return [];
    // The whole of it, once. What does not settle is the missed payment the kernel records (XI-1).
    return [{ kind: 'maturity', date: due }];
  },
  accrued: () => 0,
};

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
  const days = ctx.params.days(TRADE_CREDIT_PARAMS.days);
  const on = ctx.calendar.startOf(ctx.period);
  const id = freeRow(ctx, seller, buyer);
  const invoice: InvoiceTerms = {
    kind: INVOICE,
    seller,
    buyer,
    due: addDays(on, days),
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
export function overdue(ctx: MechanismContext, seller: PartyId): readonly Written[] {
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
    params: [
      {
        id: TRADE_CREDIT_PARAMS.days,
        value: 30,
        unit: 'days',
        dimension: 'days',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Trade Credit A3: how long a buyer has to pay. It is the convention of the trade — thirty days is what most of the world writes on an invoice — and it is what makes the credit a real one: a seller that ships today and is paid in a month has lent the money for a month, whether or not either of them calls it that. What a seller does about a buyer that has not paid is a DECISION and is not this number.',
      },
    ],
    phases: [],
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
