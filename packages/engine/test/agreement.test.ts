import { describe, expect, it } from 'vitest';
import { currencyCode, partyId } from '../src/core/ids.js';
import { period } from '../src/calendar/calendar.js';
import { Agreements } from '../src/register/agreements.js';
import { assemble, type World } from '../src/index.js';
import { mergeModules, rigSpec, rigWorld } from './rig.js';

/**
 * A world whose state takes MORE THAN THERE IS. The income tax rate is a POLICY (Law 2) and what
 * happens when one is set past what a payer holds is an OUTCOME — the levy fails, which is the
 * answer Money E1 wants from a payer that cannot pay. Nothing else about the scale model changes,
 * so what this shows is the arrears mechanism and not a rigged world.
 *
 * At the rate the seed carries, no levy in this world fails and none ever has: `D-1` was measured
 * at 855 failed tax legs in an earlier world and today's reaches zero. So the mechanism needs a
 * world that exercises it, and this is the smallest one — one number, declared where the parliament
 * declares it.
 */
function confiscatory(seed: string, rate: number): World {
  const spec = rigSpec(seed);
  const modules = spec.modules.map((m) => ({
    ...m,
    params: m.params.map((x) => (String(x.id) === 'treasury.tax.income' ? { ...x, value: rate } : x)),
  }));
  return assemble({ ...spec, modules: mergeModules(modules, []) });
}

const USD = currencyCode('USD');
const A = partyId('a');
const B = partyId('b');

describe('an agreement: what one party owes another that is not an instrument (XI-8)', () => {
  it('needs two parties and something owed, or it is not one', () => {
    const book = new Agreements();
    // Law 5: an agreement with one side is not an agreement.
    expect(() => book.open({ debtor: A, creditor: A, ccy: USD, owed: 1, what: 'x', why: 'y' }, period(0))).toThrow(
      /cannot owe itself/,
    );
    // Money E1: owing nothing is not owing.
    expect(() => book.open({ debtor: A, creditor: B, ccy: USD, owed: 0, what: 'x', why: 'y' }, period(0))).toThrow(
      /owing nothing/,
    );
    expect(() => book.open({ debtor: A, creditor: B, ccy: USD, owed: 5, what: '', why: 'y' }, period(0))).toThrow(
      /does not say what it is/,
    );
  });

  it('a part payment leaves what is left, and does not discharge (Money E1)', () => {
    const book = new Agreements();
    const row = book.open({ debtor: A, creditor: B, ccy: USD, owed: 100, what: 'tax', why: 'assessed' }, period(0));
    expect(row.state).toBe('performing');
    const part = book.paid(row.id, 40);
    expect(part.owed).toBe(60);
    expect(part.state).toBe('performing');
    const rest = book.paid(row.id, 60);
    expect(rest.owed).toBe(0);
    expect(rest.state).toBe('discharged');
    // And nothing is paid twice.
    expect(() => book.paid(row.id, 1)).toThrow(/discharged/);
  });

  it('refuses a payment bigger than what is owed', () => {
    const book = new Agreements();
    const row = book.open({ debtor: A, creditor: B, ccy: USD, owed: 10, what: 'wages', why: 'w' }, period(0));
    expect(() => book.paid(row.id, 11)).toThrow(/paid against/);
  });

  it('a write-off says so rather than quietly becoming a discharge (XI-8)', () => {
    const book = new Agreements();
    const row = book.open({ debtor: A, creditor: B, ccy: USD, owed: 10, what: 'wages', why: 'w' }, period(0));
    const dead = book.terminate(row.id);
    expect(dead.state).toBe('terminated');
    // Ten is still owed and nobody is going to pay it. Calling that discharged would say somebody
    // was paid who was not — which is the difference between a write-off and a settlement.
    expect(dead.owed).toBe(10);
  });

  it('answers both halves, which is why an estate can divide it', () => {
    const book = new Agreements();
    book.open({ debtor: A, creditor: B, ccy: USD, owed: 10, what: 'wages', why: 'w' }, period(0));
    book.open({ debtor: A, creditor: B, ccy: USD, owed: 20, what: 'tax', why: 't' }, period(0));
    expect(book.owedBy(A).length).toBe(2);
    expect(book.owedTo(B).length).toBe(2);
    expect(book.owedBy(B).length).toBe(0);
  });
});

describe('what used to evaporate (D-1, A-41)', () => {
  it('a levy that failed is a claim the state holds, and it adds up to what it says (Law 4)', () => {
    /**
     * `treasury.receipts` carried an `unpaid` number and NOTHING carried the claim: the cell did
     * not owe it next period, the treasury did not chase it, and no account was short by it. A tax
     * that failed was a hole between two balance sheets that only the journal knew about.
     */
    const w = confiscatory('estate', 20);
    for (let i = 0; i < 14; i += 1) w.step();
    const arrears = w.agreements.all().filter((a) => a.what === 'tax in arrears');
    expect(arrears.length).toBeGreaterThan(0);
    for (const a of arrears) {
      // Law 5: two named sides and a size. The debtor is the cell that could not pay and the
      // creditor is the treasury that assessed it — which is the pair `unpaid` never wrote down.
      expect(a.debtor).not.toBe(a.creditor);
      expect(a.owed).toBeGreaterThan(0);
      expect(a.state).toBe('performing');
    }
    /**
     * Law 4 and the whole point: the treasury's `unpaid` and the claims it now holds are THE SAME
     * MONEY, reached two ways and equal to the piece. If they ever differ, one of them is a second
     * writer of one fact — so this is the check that keeps the number in the event honest, and it
     * is exact because both sides are whole pieces of one currency (Law 7: no band).
     */
    const said = w.journal
      .ofKind('treasury.receipts')
      .reduce((t, e) => t + Number(e.data['unpaid']), 0);
    const owed = arrears.reduce((t, a) => t + a.owed, 0);
    expect(owed).toBe(said);
  });

  it('the arrears are on the payer as well, which is what an estate divides (XI-8)', () => {
    const w = confiscatory('estate', 20);
    for (let i = 0; i < 14; i += 1) w.step();
    const arrears = w.agreements.all().filter((a) => a.what === 'tax in arrears');
    expect(arrears.length).toBeGreaterThan(0);
    // Indexed both ways, which is the read an estate needs: what this party owes, by name.
    for (const a of arrears) {
      expect(w.agreements.owedBy(a.debtor).some((x) => x.id === a.id)).toBe(true);
      expect(w.agreements.owedTo(a.creditor).some((x) => x.id === a.id)).toBe(true);
    }
  });
});

describe('a fee nobody could pay is a claim somebody holds', () => {
  it('an issuer that cannot pay for its own rating owes the assessor for it', () => {
    /**
     * Found while measuring this item: in a world where no bank will lend, EIGHTEEN rating fees
     * failed over fourteen periods and every one of them was written as `rating.unpaid` — a number
     * in an event that carried nothing. The assessor had no claim, the issuer's book was not short
     * by it, and an estate dividing either of them would have found nothing between them.
     *
     * It is the same shape as the unpaid tax and the unpaid wage, so it goes through the same door
     * rather than growing a fourth private book (Law 4).
     */
    const spec = rigSpec('estate');
    // Banks Lending B3.c: no bank in this world will lend a penny, so a party that cannot pay
    // simply does not pay. The refusal is the answer and the failure is a real state (Money E1).
    const modules = spec.modules.map((m) => ({
      ...m,
      params: m.params.map((x) =>
        String(x.id).startsWith('bank.limitPerBorrower.') ? { ...x, value: 0 } : x,
      ),
    }));
    const w = assemble({ ...spec, modules: mergeModules(modules, []) });
    for (let i = 0; i < 14; i += 1) w.step();
    const fees = w.agreements.all().filter((a) => a.what === 'a rating fee in arrears');
    expect(fees.length).toBeGreaterThan(0);
    // The number the event says and the claims that now exist are the same money (Law 4).
    const said = w.journal.ofKind('rating.unpaid').reduce((t, e) => t + Number(e.data['due']), 0);
    expect(fees.reduce((t, a) => t + a.owed, 0)).toBe(said);
    for (const a of fees) {
      expect(w.agreements.owedTo(a.creditor).some((x) => x.id === a.id)).toBe(true);
      expect(w.agreements.owedBy(a.debtor).some((x) => x.id === a.id)).toBe(true);
    }
  });
});

describe('a probate that could not be handed over (A-20)', () => {
  it('reads what settlement returned instead of throwing the world down', () => {
    /**
     * `handToProbate` discarded the `SettlementRecord`. Settlement is atomic, so on a fail NOTHING
     * moved, the estate still held everything, and the next line — `cells.die` — met "no death
     * without a destination" and threw a `PhoenixError` the engine never catches. The same happened
     * with no failure at all whenever the dead held anything ENCUMBERED: the asset legs are built
     * from what is FREE, a lien is not free, and the units stayed.
     *
     * A failed estate transfer stopped the world. It is an outcome now: what did not arrive is owed
     * by the estate to the office, and an estate that still holds does not die — it is `winding`.
     */
    const w = rigWorld('agreement');
    for (let i = 0; i < 12; i += 1) w.step();
    const deaths = w.journal.ofKind('households.lifecycle').filter((e) => e.data['event'] === 'died');
    expect(deaths.length).toBeGreaterThan(0);
    // Every death now says whether the estate reached the office, which is the fact that used to be
    // discarded. In the scale model all 24 of them do, so the world exercises the read and not the
    // branch — and that is a statement about the world, not about the mechanism (Law 11).
    for (const e of deaths) expect(typeof e.data['handedOver']).toBe('boolean');
    const stuck = deaths.filter((e) => e.data['handedOver'] === false);
    // Whatever did not hand over is `winding` and still on the books by name: nothing is a residual
    // with no holder, which is the whole reason it may not die (Appendix B).
    for (const e of stuck) {
      const estate = String(e.subjects[0]);
      const st = w.parties.get(partyId(estate)).status;
      expect(st.alive && st.standing).toBe('winding');
      expect(w.register.holdingsOf(partyId(estate)).length).toBeGreaterThan(0);
    }
  });
});
