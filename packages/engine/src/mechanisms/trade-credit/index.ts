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
import { FIRM } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule, TermsSale } from '../../world/module.js';

export const INVOICE = instrumentKindId('invoice');

export const TRADE_CREDIT_PARAMS = {
  days: paramId('tradeCredit.days'),
} as const satisfies Record<string, ParamId>;

/** A1: one row per (seller, buyer, sale). Named so a reader can see whose it is (Law 9). */
export const invoiceId = (seller: PartyId, buyer: PartyId, n: number): InstrumentId =>
  instrumentId(`invoice:${seller}:${buyer}:${n}`);

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
    return compareCivil(due, on) < 0 ? [] : [{ date: due, perUnit: 1 }];
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
  // B5, Corporate Credit A4: the seller's own record of this buyer. A missed payment it was a side
  // of is something it saw; what other sellers saw is theirs.
  if (letDown(ctx, seller, buyer)) return none<InstrumentId>();
  const days = ctx.params.days(TRADE_CREDIT_PARAMS.days);
  const on = ctx.calendar.startOf(ctx.period);
  const book = state(ctx);
  const id = invoiceId(seller, buyer, book.next);
  book.next += 1;
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
  book.written.push({ id, seller, buyer, due: invoice.due });
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
 * D1: THE RECEIVABLES AGEING, which is one read and not a stored number. What this seller shipped,
 * has not been paid for, and should have been paid for by now.
 *
 * The rows come from this module's own book — every one of them is an invoice because this module
 * wrote it — so nothing asks an instrument what kind it is (Law 15), and whether it is still owed
 * comes from the register, which is the one writer of who holds what (Law 4, Law 19).
 */
export function overdue(ctx: MechanismContext, seller: PartyId): readonly Written[] {
  const today = ctx.calendar.endOf(ctx.period);
  return state(ctx).written.filter(
    (row) =>
      row.seller === seller &&
      // A2: one sum on one day. A day past its day, with the row still on the seller's book, is a
      // buyer that has not paid.
      compareCivil(row.due, today) <= 0 &&
      ctx.register.quantity(seller, row.id) > 0,
  );
}

/** One invoice this module wrote: who shipped, who owes, which row and when it falls due. */
export interface Written {
  readonly id: InstrumentId;
  readonly seller: PartyId;
  readonly buyer: PartyId;
  readonly due: Civil;
}

interface Book {
  next: number;
  /** A1: every row this module wrote, so its own reads never have to ask what kind a thing is. */
  readonly written: Written[];
}

const state = (ctx: MechanismContext): Book =>
  ctx.state<Book>('invoices', () => ({ next: 1, written: [] }));

export function tradeCredit(): SystemModule {
  return {
    id: 'trade-credit',
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
    termsOffered: [{ partyKind: FIRM, decide: shipsOnTerms }],
    families: [],
  };
}
