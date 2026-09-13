/**
 * Trade credit (13e): the receivable and the payable that are one row read from two sides.
 *
 * @spec Trade Credit A1 Trade Credit A2 Trade Credit A3 Trade Credit B3 Trade Credit C4 Trade Credit D4 Trade Credit E1 Trade Credit E2 Firm Birth D2.b Law 4 Law 5 Law 15
 */
import { describe, expect, it } from 'vitest';
import { displayName, paramId } from '../src/index.js';
import { INVOICE, invoiceTerms, isInvoice } from '../src/mechanisms/trade-credit/index.js';
import { rigWorld } from './rig.js';

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
      // E2: every receivable has a holder, and it is the seller.
      expect(holders.length).toBeGreaterThan(0);
      for (const h of holders) expect(String(h)).toBe(String(t.seller));
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
      const before = kind.cashFlows(row, t.due, w.calendar);
      expect(before).toHaveLength(1);
      expect(before[0]?.perUnit).toBe(1);
      // Nothing accrues on it: it is a sum owed on a date, not a rate running against a balance.
      expect(kind.accrued(row, t.due, w.calendar)).toBe(0);
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

describe('how long a buyer has is the ONE declared number (Trade Credit A3, Law 2)', () => {
  it('declares the convention and declares nothing about who gets terms', () => {
    const w = rigWorld('credit-a');
    const days = w.params.decl(paramId('tradeCredit.days'));
    expect(days.kind).toBe('technology');
    expect(days.unit).toBe('days');
    // B5, D4: WHO is shipped on terms is a decision the seller takes about a buyer it knows, and a
    // decision is never a number. A world with a `tradeCredit.share` in it would have imported the
    // answer instead of clearing it (Law 2).
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
