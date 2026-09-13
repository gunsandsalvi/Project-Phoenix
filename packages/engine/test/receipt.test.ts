import { describe, expect, it } from 'vitest';
import { RECEIPT_KINDS } from '../src/ledger/instruction.js';
import { rigWorld } from './rig.js';

/** Every `treasury.receipts` event this world has published, newest last. */
function receipts(w: ReturnType<typeof rigWorld>) {
  return w.journal.ofKind('treasury.receipts');
}

describe('a receipt says what money IS to whoever gets it (Treasury C1)', () => {
  it('the vocabulary is closed and the ledger owns it', () => {
    expect([...RECEIPT_KINDS].sort()).toEqual([
      'borrowing',
      'disposal',
      'dividend',
      'interest',
      'rent',
      'returnOfCapital',
      'transfer',
      'wage',
    ]);
  });

  it('every penny reaching a household is named, and the state taxes only what was named', () => {
    const w = rigWorld('receipt');
    for (let i = 0; i < 6; i += 1) w.step();
    const said = receipts(w);
    expect(said.length).toBeGreaterThan(0);
    const last = said[said.length - 1];
    if (last === undefined) return;
    const bases = last.data['bases'] as Record<string, number>;
    // The gap this item exists to close, and it is closed in this world: nothing arrives at a
    // household without the payer saying what it is.
    expect(bases['unclassified']).toBe(0);
    expect(bases['interest']).toBeGreaterThan(0);
  });

  it('a transfer, an inheritance and a returned principal are not income', () => {
    /**
     * The measurement that made the case. Before this, EVERY money leg into a household that was
     * not a coupon was taxed at the wage rate, and in the scale model that came to 11,519,429 of
     * base — all of it state transfers under the standing mandate and estates being divided by
     * probate. Not one penny of it was a wage, because no household in this world is paid one.
     *
     * So the income tax was a tax on benefits and inheritance, and its base is now what payers
     * actually called income: zero, here, and the zero is the finding (A-46, A-37).
     */
    const w = rigWorld('receipt');
    for (let i = 0; i < 6; i += 1) w.step();
    const all = receipts(w);
    const last = all[all.length - 1];
    if (last === undefined) return;
    expect((last.data['bases'] as Record<string, number>)['income']).toBe(0);
    // And the state still collects, on the base that IS real — interest received.
    expect(last.data['total']).toBeGreaterThan(0);
  });

  it('the wire keeps its own vocabulary and the two do not merge', () => {
    // `Cause` is the settlement layer's label for why bytes moved, and it stays nine values. A
    // coupon instruction carries `cause: 'coupon'` AND `receipt: { of: 'interest' }`: one says how
    // the wire moved, the other what the money is to whoever got it (Law 4, one writer each).
    const w = rigWorld('receipt');
    for (let i = 0; i < 4; i += 1) w.step();
    const coupons = w.ledger.all().filter((r) => r.instruction.cause === 'coupon');
    expect(coupons.length).toBeGreaterThan(0);
    const legs = coupons.flatMap((r) => r.instruction.legs).filter((l) => l.kind === 'money');
    expect(legs.length).toBeGreaterThan(0);
    for (const l of legs) expect(l.receipt?.of).toBe('interest');
  });
});
