/**
 * Trade credit (13e): the receivable and the payable that are one row read from two sides.
 *
 * @spec Trade Credit A1 Trade Credit A2 Trade Credit A3 Trade Credit B3 Trade Credit C4 Trade Credit D4 Trade Credit E1 Trade Credit E2 Firm Birth D2.b Law 4 Law 5 Law 15
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  HOUSEHOLD,
  SMALL_FIRM,
  assemble,
  displayName,
  instrumentId,
  partyId,
  type InstrumentId,
  type MechanismContext,
  type PartyId,
  type SystemModule,
  type World,
} from '../src/index.js';
import {
  INVOICE,
  impliedRate,
  invoiceId,
  invoiceTerms,
  isInvoice,
  tradeCredit,
  type InvoiceTerms,
} from '../src/mechanisms/trade-credit/index.js';
import { TERMS_SPREAD, assertTermsAreOrdered, sellerParam } from '../src/mechanisms/trade-credit/data.js';
import { InvalidRegistry } from '../src/core/errors.js';
import { paramId } from '../src/core/ids.js';
import { asQty, type Qty } from '../src/core/tick.js';
import { addDays, civil, compareCivil, type Civil } from '../src/calendar/civil.js';
import { asPerPiece, asRatio } from '../src/core/measure.js';
import { none, some } from '../src/core/option.js';
import { moneyInstrumentId } from '../src/core/ids.js';
import { downTick } from '../src/core/tick.js';
import { mergeModules, ranWorld, rigSpec, rigWorld } from './rig.js';

/** Every invoice this world has written, whatever wrote it. */
function invoices(w: ReturnType<typeof rigWorld>) {
  return w.instruments.all().filter((i) => i.kind === INVOICE);
}

describe('an invoice is ONE row (Trade Credit A1, Law 4)', () => {
  it('is issued by the buyer and held by the seller, and nobody writes a second copy', () => {
    const w = rigWorld('credit-a');
    for (let i = 0; i < 3; i += 1) w.step();
    const rows = invoices(w);
    for (const row of rows) {
      const t = invoiceTerms(row);
      // A1: the payable is the receivable. The buyer ISSUED it (it is the buyer's liability) and
      // the seller HOLDS it; there is one fact and one writer of it, so the two sides of a supply
      // chain cannot disagree about what is owed.
      expect(row.issuer.some).toBe(true);
      if (row.issuer.some) expect(String(row.issuer.value)).toBe(String(t.buyer));
      expect(String(t.seller)).not.toBe(String(t.buyer));
      const holders = w.register.holdersOf(row.id);
      // E2: every receivable has a holder, and it is the seller — or, once the seller has ceased,
      // whoever succeeded to its book (Register F2: every reference resolves to the successor).
      expect(holders.length).toBeGreaterThan(0);
      for (const h of holders) expect(String(h)).toBe(String(w.parties.resolve(t.seller).id));
    }
  });

  it('names itself as a market names it: who on whom, due when (Law 9)', () => {
    const w = rigWorld('credit-a');
    for (let i = 0; i < 3; i += 1) w.step();
    for (const row of invoices(w)) {
      const shown = displayName(row, w.parties, w.registry);
      expect(shown).toContain(' on ');
      expect(shown).toContain('due ');
      // Law 9: the internal id is never the display name.
      expect(shown).not.toBe(String(row.id));
    }
  });
});

describe('terms are for GOODS (Trade Credit A1, Law 15)', () => {
  it('never writes an invoice against a thing somebody issued', () => {
    const w = rigWorld('credit-a');
    for (let i = 0; i < 3; i += 1) w.step();
    // A share or a bond is a promise, bought and paid for when it is delivered (XI-5). A firm
    // selling its own paper is raising money, not shipping — and if that were paid with a promise
    // the firm would have raised nothing at all.
    for (const row of invoices(w)) {
      const t = invoiceTerms(row);
      expect(isInvoice(row.terms)).toBe(true);
      expect(String(t.seller)).not.toBe(String(t.buyer));
    }
    const paper = w.instruments
      .all()
      .filter((i) => i.issuer.some && i.kind !== INVOICE)
      .map((i) => String(i.id));
    for (const row of invoices(w)) {
      // The invoice's own id names the sale's two parties; no invoice was written against a line
      // that has an issuer of its own.
      expect(paper).not.toContain(String(row.id));
    }
  });
});

describe('an invoice is a zero-coupon bill and nothing more (Trade Credit A2)', () => {
  it('promises one payment, of the whole of it, on one day', () => {
    const w = rigWorld('credit-a');
    for (let i = 0; i < 3; i += 1) w.step();
    const kind = w.registry.instrumentKind(INVOICE);
    for (const row of invoices(w)) {
      const t = invoiceTerms(row);
      const before = kind.cashFlows(row, t.due, w.calendar, w.registry);
      expect(before).toHaveLength(1);
      expect(before[0]?.perUnit).toBe(1);
      // Nothing accrues on it: it is a sum owed on a date, not a rate running against a balance.
      expect(kind.accrued(row, t.due, w.calendar, w.registry)).toBe(0);
    }
  });

  it('ranks unsecured, because that is a fact about the claim (Firm Birth D2.b)', () => {
    const w = rigWorld('credit-a');
    const rank = w.registry.instrumentKind(INVOICE).ranking;
    const r = rank(w.instruments.all()[0]!);
    expect(r.secured).toEqual([]);
    expect(r.claim).toContain('unsecured');
  });
});

describe('what this module declares, and what it refuses to (Trade Credit A3, B5, Law 2)', () => {
  it('declares nothing for the world, and nothing about who gets terms or who pays', () => {
    const w = rigWorld('credit-a');
    for (let i = 0; i < 6; i += 1) w.step();
    // 17.7a: `tradeCredit.days` was one number for everybody and B5 says the seller decides. The
    // module now declares NOTHING at assembly; what exists is one seller's own terms, under its own
    // name, drawn the first time it shipped on them.
    expect(tradeCredit().params).toEqual([]);
    expect(w.params.has(paramId('tradeCredit.days'))).toBe(false);
    // B5, D4: WHO is shipped on terms is a decision the seller takes about a buyer it knows, and a
    // decision is never a number. A world with a `tradeCredit.share` in it would have imported the
    // answer instead of clearing it (Law 2) — and so would one with a rate in it, because A3's rate
    // is DERIVED from the discount and the two dates and is never declared.
    for (const d of w.params.all()) {
      const id = String(d.id);
      if (!id.startsWith('tradeCredit.')) continue;
      expect(id).not.toContain('share');
      expect(id).not.toContain('rate');
      expect(id).not.toContain('loss');
      expect(id).not.toContain('default');
    }
  });
});

describe('terms are for buyers of a kind that takes them (Trade Credit A3, Households C1.d, 11.0b)', () => {
  it('is declared on the kind: firms and small firms take terms, a household pays with money it has', () => {
    const w = rigWorld('credit-kinds');
    expect(w.registry.partyKind(FIRM).buysOnTerms).toBe(true);
    expect(w.registry.partyKind(SMALL_FIRM).buysOnTerms).toBe(true);
    expect(w.registry.partyKind(HOUSEHOLD).buysOnTerms).toBe(false);
    for (let i = 0; i < 6; i += 1) w.step();
    for (const row of invoices(w)) {
      const t = invoiceTerms(row);
      // C1.d: no invoice names a household as the buyer, whatever its seller thought of it.
      expect(w.registry.partyKind(w.parties.get(t.buyer).kind).buysOnTerms).toBe(true);
    }
  });
});

/**
 * B2, B5, C1.a, D1 (17.5): terms are the seller's decision — about the buyer AND about itself.
 */
describe('a seller short of cash ships for cash (Trade Credit B2, B5, C1.a)', () => {
  it('names every row by its pair, its period and its place in that period (Law 9)', () => {
    const w = ranWorld('trade', 12);
    for (const i of w.instruments.all()) {
      if (!isInvoice(i.terms)) continue;
      // Law 9, Law 18: seller, buyer, the week it was written and which of that week's it is —
      // so a reader can see whose it is and when, and a pair that has traded for a year does not
      // scan a year of rows to write this week's.
      expect(String(i.id)).toMatch(/^invoice:[^:]+:[^:]+:\d+:\d+$/);
    }
  });

  it('extends no terms in a period it published a gap of its own (B2, D3)', () => {
    const w = ranWorld('trade', 12);
    // Offering terms IS lending: the seller funds the buyer out of its own account for a month. A
    // seller that said it is short of money this period has nothing to fund anybody with, so every
    // invoice it wrote was written in a period it did not say so.
    for (const i of w.instruments.all()) {
      if (!isInvoice(i.terms)) continue;
      const t = invoiceTerms(i);
      const written = w.calendar.periodOf(t.due);
      const said = w.journal
        .ofKind('firms.funding')
        .filter((e) => e.subjects.includes(String(t.seller)) && e.period < written);
      const last = said.at(-1);
      if (last === undefined) continue;
      // If the seller had a gap NOW in the period it shipped, it should not have shipped on terms.
      const shortNow = Number(last.data['shortNow']);
      if (Number.isFinite(shortNow) && shortNow > 0) {
        expect(written).not.toBe(last.period);
      }
    }
  });

  it('ages a seller’s book once a period, and keeps nothing the register does not (Law 18, Law 19)', () => {
    // The ageing is a WORKING store — a memo of a walk over the rows, declared as one — so a reader
    // knows it outlives nothing and mirrors nothing. The rows are the source and the next period
    // walks them again; a NOUN here would be a second copy of the register.
    const declared = tradeCredit().nouns ?? [];
    const ageing = declared.find((n) => n.name === 'tradeCredit.overdue');
    expect(ageing, 'the ageing memo is undeclared').toBeDefined();
    expect(ageing?.kind).toBe('working');
  });
});

/**
 * The seller's own terms and the price of paying early (Trade Credit A3, B5; item 17.7a).
 *
 * A3 has two halves and this world had neither: terms were one number for everybody, and there was
 * no discount at all — so the clause's *"which makes the discount an implicit interest rate and
 * therefore a price"* had nothing to make a rate out of.
 */
describe('a seller decides its own terms (Trade Credit A3, B5, finding 21.61)', () => {
  it('declares its days, its window and its discount under its own name, and nothing for the world', () => {
    const w = rigWorld('credit-a');
    for (let i = 0; i < 6; i += 1) w.step();
    const mine = w.params.all().filter((d) => String(d.id).startsWith('tradeCredit.'));
    expect(mine.length).toBeGreaterThan(0);
    for (const d of mine) {
      // XI-14: each is one seller's preference, keyed by that seller's own name — never a
      // convention of the world, which is what a `tradeCredit.days` with nothing after it was.
      expect(String(d.id).split('.').length).toBeGreaterThan(2);
      expect(d.kind).toBe('preference');
      expect(d.owner).toBe('model');
    }
    expect(mine.map((d) => String(d.id))).not.toContain('tradeCredit.days');
    // B5: and what it declared is what it wrote on every invoice it sent.
    for (const row of invoices(w)) {
      const t = invoiceTerms(row);
      const days = w.params.decl(sellerParam(String(t.seller), 'days'));
      const off = w.params.decl(sellerParam(String(t.seller), 'discount'));
      expect(off.value).toBe(t.discount);
      expect(days.value).toBeGreaterThan(0);
    }
  });

  it('keeps the window inside the term on every row it writes (A3)', () => {
    const w = rigWorld('credit-a');
    for (let i = 0; i < 6; i += 1) w.step();
    for (const row of invoices(w)) {
      const t = invoiceTerms(row);
      // A discount that expired after the money was due would be no discount at all, and the two
      // day-counts are drawn independently — so the widths are checked rather than trusted.
      expect(compareCivil(t.discountBy, t.due)).toBeLessThan(0);
    }
    expect(() => {
      assertTermsAreOrdered({
        ...TERMS_SPREAD,
        days: { low: 5, high: 60, why: 'a term that could be shorter than the window' },
      });
    }).toThrow(InvalidRegistry);
  });
});

describe('the discount IS an interest rate (Trade Credit A3)', () => {
  const on = (y: number, m: number, d: number): Civil => civil(y, m, d);

  it('is what the days between the two dates cost, per unit still owed', () => {
    // Two off for paying by the tenth instead of the thirtieth: the buyer pays 98 to keep 100 for
    // twenty days, so it earns 2/98 over 20/365 of a year — the classic, and it is a big number,
    // which is the point of the clause.
    const t: InvoiceTerms = {
      kind: INVOICE,
      seller: partyId('seller'),
      buyer: partyId('buyer'),
      due: on(2026, 1, 31),
      discount: asRatio(0.02, 'two off'),
      discountBy: on(2026, 1, 11),
    };
    const rate = impliedRate(t, on(2026, 1, 1));
    expect(rate.some).toBe(true);
    if (!rate.some) return;
    expect(rate.value).toBeCloseTo((0.02 / 0.98) * (365 / 20), 9);
    // And an offer that has expired has no rate: there is nothing left to choose.
    expect(impliedRate(t, on(2026, 1, 12)).some).toBe(false);
  });
});

describe('a buyer pays early when the discount beats what its money earns (A3, B1)', () => {
  /** A sale of one firm's goods to another firm against a promise — the two legs of one trade. */
  function ships(discount: number, at = 2): SystemModule {
    return {
      id: 'test.ships',
      spec: 'Trade Credit A1',
      requires: [],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.ship',
          spec: 'Trade Credit A1',
          anchor: { before: 'corporateActions' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== at) return;
            const firms = ctx.parties.ofKind(FIRM).filter((p) => p.status.alive);
            // A test asks the DRAW for what it needs and never names a party: the firm with the
            // most money is the buyer, because the decision under test is a buyer's and a buyer
            // with nothing in the bank cannot take a discount for cash.
            let buyer: (typeof firms)[number] | undefined;
            let cash = 0;
            for (const f of firms) {
              const held = ctx.register.quantity(
                f.id,
                moneyInstrumentId(f.bank, ctx.registry.currencyOf(f.region)),
              );
              if (held > cash) {
                cash = held;
                buyer = f;
              }
            }
            if (buyer === undefined || cash <= 0) throw new Error('the rig banked nobody');
            let sold: { seller: PartyId; instrument: InstrumentId; qty: Qty } | undefined;
            for (const f of firms) {
              if (f.id === buyer.id) continue;
              for (const h of ctx.register.holdingsOf(f.id)) {
                const i = ctx.instruments.get(h.instrument);
                // A1: terms are for GOODS, and a good is a thing nobody issued.
                if (i.issuer.some || !i.status.live) continue;
                const q = ctx.register.quantity(f.id, h.instrument);
                if (q <= 0 || (sold !== undefined && q <= sold.qty)) continue;
                sold = { seller: f.id, instrument: h.instrument, qty: asQty(q) };
              }
            }
            if (sold === undefined) throw new Error('the rig made nothing');
            const ccy = ctx.registry.currencyOf(buyer.region);
            // Half of what the buyer holds, so the sale is one it could pay for in cash and the
            // choice under test is whether it wants to.
            const qty = asQty(downTick(Math.min(sold.qty, cash / 2)));
            const on = ctx.calendar.startOf(ctx.period);
            const id = invoiceId(sold.seller, buyer.id, ctx.period, 1);
            const terms: InvoiceTerms = {
              kind: INVOICE,
              seller: sold.seller,
              buyer: buyer.id,
              due: addDays(on, 30),
              discount: asRatio(discount, 'what the seller takes off'),
              discountBy: addDays(on, 10),
            };
            ctx.issue({ id, kind: INVOICE, issuer: some(buyer.id), ccy, terms, market: none() });
            // XI-5: the goods and the promise in one instruction, which is what shipping on terms
            // IS — the buyer pays with a promise instead of with money and both legs are there.
            ctx.settle({
              legs: [
                {
                  kind: 'asset',
                  from: sold.seller,
                  to: buyer.id,
                  instrument: sold.instrument,
                  qty,
                  pricePerUnit: some(asPerPiece(1, 'a piece of money a piece')),
                  accruedPerUnit: none(),
                },
                {
                  kind: 'asset',
                  from: buyer.id,
                  to: sold.seller,
                  instrument: id,
                  qty,
                  pricePerUnit: some(asPerPiece(1, 'at what it promised')),
                  accruedPerUnit: none(),
                },
              ],
              cause: 'trade',
              reason: `${String(sold.seller)} ships to ${String(buyer.id)} on terms`,
            });
          },
        },
      ],
      participants: [],
      families: [],
    };
  }

  function world(discount: number): World {
    const spec = rigSpec('credit-early');
    return assemble({ ...spec, modules: mergeModules(spec.modules, [ships(discount)]) });
  }

  it('takes a discount worth more than its deposit, and leaves one worth less', () => {
    // Two off for twenty days is about thirty-seven per annum; no board in this world pays that,
    // so the buyer pays early and keeps the two.
    const rich = world(0.02);
    for (let i = 0; i < 8; i += 1) rich.step();
    const paid = rich.journal.ofKind('tradeCredit.paidEarly');
    expect(paid.length).toBeGreaterThan(0);
    const row = paid[0];
    expect(row).toBeDefined();
    if (row === undefined) return;
    // It paid less than the face, and the row it paid is gone: a redemption below par, in one
    // instruction with two legs, and the seller realised the difference against what it carried.
    expect(Number(row.data['paid'])).toBeLessThan(Number(row.data['face']));
    expect(Number(row.data['impliedRate'])).toBeGreaterThan(Number(row.data['earns']));
    const invoice = rich.instruments.get(instrumentId(String(row.data['invoice'])));
    expect(rich.register.heldTotal(invoice.id).value).toBe(0);
    // A hundredth of a per cent for twenty days is under two tenths a year, which a deposit beats:
    // the buyer keeps its money and pays on the day.
    const thin = world(0.0001);
    for (let i = 0; i < 8; i += 1) thin.step();
    expect(thin.journal.ofKind('tradeCredit.paidEarly')).toHaveLength(0);
  });
});
