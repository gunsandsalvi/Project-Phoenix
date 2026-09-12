/**
 * A bank that fails, and what happens to everything it owed.
 *
 * @spec Banks Capital A2 Banks Capital A2.a Banks Capital A2.c Banks Capital C1 Banks Capital C1.a Banks Capital C3 Banks Capital C3.b Banks Capital D1 Banks Capital D2 Banks Capital D2.a Banks Capital D3 Banks Capital D3.b Banks Capital D4 Banks Capital D5 Banks Capital D6 Banks Capital E3 Banks Funding D6 Banks Funding A1.a Central Bank D3.a Money Market B3.c Money Market E3 Banks Capital E1 Banks Capital A4 XI-3 XI-15 Law 2
 *
 * A bank does not go to an estate (C3.b): its deposits are money, and money whose issuer has stopped
 * existing is not money. So it goes somewhere else — a valuation at marks, a hole, a hierarchy that
 * absorbs it in order, an acquirer that takes the book over the wire, a guarantee for what the
 * hierarchy could not reach and a public purse behind that.
 *
 * The world here is the foundation world with ONE thing added: bank A pays a penalty it cannot
 * afford. That is all it takes — a real payment to a real payee — and everything below follows from
 * mechanisms that were already there: the payment overdraws it, the central bank lends against its
 * paper while it is still solvent (D3), the loss lands on its equity, and the period after that its
 * liabilities are past its assets and the resolution opens. Nothing here makes a bank fail; one
 * payment does, and the model does the rest.
 */
import { describe, expect, it } from 'vitest';
import { asQty } from '../src/core/tick.js';
import {
  INSURER,
  INSURER_PARAMS,
  USD,
  TREASURY_US,
  assemble,
  moneyInstrumentId,
  none,
  partyId,
  rowTerms,
  type Event,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { rigSpec } from './rig.js';
import { unexpected } from './expected.js';

const BANK_A = partyId('bank.a');
const BANK_B = partyId('bank.b');
/**
 * When the penalty starts, how many weeks of it there are, and how much of the bank's own capital
 * each week takes. It is a SHARE OF WHAT THE BANK HAS rather than an amount somebody tuned: a bank
 * can only pay what it holds plus what the window will lend against its paper (D3), so one payment
 * big enough to put it under is one it cannot make — and four that each take a third of the capital
 * it started with put it under between them, whatever this world's banks happen to be worth.
 */
const AT = 10;
const WEEKS = 4;
/**
 * XI-1, Banks Capital C1: HOW BIG THE LOSS IS, as a multiple of the bank's OWN CAPITAL, a week, for
 * a month. THERE ARE TWO OF THEM AND THAT IS THE POINT — a bank losing a third of its capital a
 * week and a bank losing all of it are different worlds, and the clauses in this file are about
 * different halves of what happens in them:
 *
 * - A THIRD is a bank that LIMPS. It runs short of cash before its capital is gone, asks the window,
 *   and is refused for being insolvent — which is D3.a, the one refusal a lender of last resort must
 *   make, and it can only be observed in a bank that gets as far as asking.
 * - ALL OF IT is a bank whose hole runs the hierarchy the whole way down. At a third the loss is
 *   absorbed entirely by the bank's own creditors and never reaches a depositor, so the guarantee
 *   never pays and D4 and D5 have nothing to show; at its whole capital, households are written
 *   down, the insurer pays, and raising the cover protects them.
 *
 * One number cannot be both, and picking whichever one makes the most tests pass is how a fixture
 * stops being about anything. Each `describe` asks for the world its own clause needs.
 */
const LIMPS = 0.35;
const RUNS_THE_HIERARCHY_DOWN = 1;

/** XI-1: a loss is an EVENT — here a penalty, paid in money, to a payee with a name. */
function penalty(at: number, share: number): SystemModule {
  let each = 0;
  return {
    id: 'test.penalty',
    spec: 'XI-1',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.penalty',
        spec: 'XI-1',
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          if (ctx.period < at || ctx.period >= at + WEEKS) return;
          // Money E4: a party that has ceased is not a payer. Once the resolution has taken it,
          // there is nobody to fine.
          if (!ctx.parties.get(BANK_A).status.alive) return;
          if (ctx.period === at) {
            each = ctx.registry.payable(USD, ctx.participant(BANK_A).equity() * share);
          }
          if (each <= 0) return;
          ctx.settle({
            legs: [
              {
                kind: 'money',
                from: { holder: BANK_A, issuer: ctx.parties.get(BANK_A).bank },
                to: { holder: TREASURY_US, issuer: ctx.parties.get(TREASURY_US).bank },
                ccy: USD,
                amount: asQty(each),
                fromCell: none(),
                toCell: none(),
              },
            ],
            cause: 'transfer',
            reason: 'a penalty it had to pay',
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/** The foundation world, one penalty, and any declared number set differently. */
function failing(
  seed: string,
  over: Readonly<Record<string, number>> = {},
  share: number = LIMPS,
): World {
  const spec = rigSpec(seed);
  const modules: SystemModule[] = [...spec.modules, penalty(AT, share)].map((m) => ({
    ...m,
    params: m.params.map((p) => {
      const value = over[String(p.id)];
      return value === undefined ? p : { ...p, value };
    }),
  }));
  const w = assemble({ ...spec, modules });
  for (let i = 0; i < AT + WEEKS + 2; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

function events(w: World, kind: string, subject?: string): Event[] {
  return w.journal
    .ofKind(kind as never)
    .filter((e) => subject === undefined || e.subjects.includes(subject));
}

function num(e: Event | undefined, key: string): number {
  const v = e?.data[key];
  return typeof v === 'number' ? v : 0;
}

describe('the valuation and the hole (Banks Capital D1)', () => {
  const w = failing('res-d1');
  const valued = events(w, 'bank.resolution.valued', BANK_A)[0];

  it('opens on the trigger that fired, and says which one it was (C1, C1.a)', () => {
    expect(valued).toBeDefined();
    // C1.a: BOTH triggers exist and the resolution NAMES THE ONE THAT FIRED. Which one this world
    // reaches is a fact about the world and not about the clause, so the clause is what is asserted.
    //
    // It used to say the cash trigger, with a note that a bank here could not become insolvent
    // while still liquid — the only loss big enough to eat its capital was a payment, and paying it
    // away took the cash with it. That stopped being true: this world now reaches the SOLVENCY
    // trigger ("its liabilities are past its assets by 1500372191"), which is the loss that is not
    // a payment the note said would need a bank whose capital is a tenth of its book. It has one.
    expect(String(valued?.data['why'])).toMatch(/could not pay|liabilities are past its assets/);
    // What matters for everything below is that the HOLE is real: it owes more than the book is
    // worth at marks, so there is something for the hierarchy to work through (D1, D2).
    expect(num(valued, 'hole')).toBeGreaterThan(0);
  });

  it('values the book at marks and takes the hole from what it owes (D1)', () => {
    // D1: at MARKS, not at what its book said. Every asset it holds, at the last thing a market
    // printed — which is why a resolution in a falling market finds a bigger hole than the bank's
    // own accounts would show, and why nothing here reads a carrying value.
    const assets = num(valued, 'assets');
    const owed = num(valued, 'deposits') + num(valued, 'borrowings');
    expect(assets).toBeGreaterThan(0);
    expect(num(valued, 'hole')).toBeCloseTo(owed - assets, 6);
    expect(num(valued, 'hole')).toBeGreaterThan(0);
    // A1: capital is the residual, so the hole IS the negative equity plus what the marks say the
    // book is short of. The bank's own equity absorbed everything it had, first and fully (A2.a).
    const done = events(w, 'bank.resolution.done', BANK_A)[0];
    expect(num(done, 'equityAbsorbed')).toBeLessThan(0);
  });

  it('is refused by the window because it is insolvent, and that is the one refusal that must exist (D3.a)', () => {
    const refused = events(w, 'centralBank.refused', BANK_A).filter(
      (e) => e.data['solvent'] === false,
    );
    expect(refused.length).toBeGreaterThan(0);
    // D6, D3.a: it had the collateral. What it did not have was capital, and a lender of last resort
    // that lends to an insolvent bank is a subsidy with the four conditions written above it.
    expect(num(refused[0], 'collateral')).toBeGreaterThan(num(refused[0], 'short'));
  });
});

describe('who bears it (Banks Capital A2, D2, D2.a, E3)', () => {
  const w = failing('res-d2');
  const valued = events(w, 'bank.resolution.valued', BANK_A)[0];
  const done = events(w, 'bank.resolution.done', BANK_A)[0];
  const down = events(w, 'bank.resolution.writtenDown', BANK_A);

  it('cuts every uninsured claim by the same proportion (D2, D2.a)', () => {
    expect(down.length).toBeGreaterThan(1);
    // A2.c, D2.a: PARI PASSU is not a rule applied afterwards. Every claim THAT RANKS TOGETHER takes
    // the same share of the same loss, which is what they would have got in a liquidation — a
    // depositor above the limit and a bank that lent it a month of money are the same creditor here.
    // The shares differ only by the whole piece each write-down was rounded to (Law 8).
    //
    // WITHIN A RANK, and the event says which rank each claim is in. This used to compare them all
    // against the first, which held while every claim in the world ranked together; a bank with
    // subordinated debt in its funding has two ranks, and one of them is wiped while the other is
    // barely touched — which is what being junior MEANS and not a failure of pari passu.
    const byRank = new Map<number, number[]>();
    for (const e of down) {
      const rank = num(e, 'rank');
      byRank.set(rank, [...(byRank.get(rank) ?? []), num(e, 'lost') / num(e, 'owed')]);
    }
    expect(byRank.size, 'nothing was written down at any rank').toBeGreaterThan(0);
    let compared = 0;
    for (const [, shares] of byRank) {
      const first = shares[0] ?? 0;
      expect(first).toBeGreaterThan(0);
      for (const share of shares) expect(share).toBeCloseTo(first, 4);
      compared += shares.length;
    }
    expect(compared).toBe(down.length);
  });

  it('leaves a secured lender alone for what its own paper covers (D2, B3.c)', () => {
    // Appendix B: no collateral counted twice. Bank B lent to bank A against paper and holds the
    // liens; the acquirer takes the book WITH them. So it is not in this pool — and if the paper
    // ever failed to cover the row, the shortfall would be, at the same share as everybody else.
    for (const e of down) {
      const claim = String(e.data['claim']);
      if (!claim.startsWith('repo:')) continue;
      const row = w.instruments.get(claim as never);
      const covered = rowTerms(row).collateral.reduce((a, c) => a + c.qty, 0);
      expect(num(e, 'owed')).toBeLessThan(covered);
    }
    // And the rows themselves are still there, still secured, now owed by the acquirer (D6).
    const rows = w.instruments
      .all()
      .filter((i) => i.status.live && (i.kind === 'repo' || i.kind === 'interbank'));
    for (const r of rows) {
      const terms = rowTerms(r);
      expect(w.parties.get(w.parties.resolve(terms.borrower).id).status.alive).toBe(true);
      expect(w.parties.get(w.parties.resolve(terms.lender).id).status.alive).toBe(true);
    }
  });

  it('conserves: what the hole was is what somebody bore (E3)', () => {
    // E3: acquirer paid + insurer paid + estate realised + holders lost = the hole. Every term is a
    // real payment or a real write-down, so the identity is the wire's and not a reconciliation —
    // what is left over is the fraction of a piece nobody can be paid (Law 8).
    const borne = num(done, 'holdersLost') + num(done, 'insurerPaid') + num(done, 'pursePaid');
    expect(borne).toBeLessThanOrEqual(num(valued, 'hole'));
    expect(num(valued, 'hole') - borne).toBeLessThan(down.length + 1);
  });
});

describe('the acquirer (Banks Capital D3, D6, C3.b)', () => {
  const w = failing('res-d3');
  // PLAN §7, D3: EVERY BANK BIDS FROM ITS OWN VIEW, so some of them decline — a book that would
  // leave the bidder's own equity underwater is a book it says no to, and that is the mechanism
  // working rather than a world without an acquirer. This read the FIRST bid and asserted it was
  // accepted, which is asserting that the bank that happens to bid first is the one with the
  // balance sheet for it. Which bank takes it is the outcome; that one does is the clause.
  const bids = events(w, 'bank.resolution.bid', BANK_A);
  const bid = bids.find((e) => e.data['declined'] === false);

  it('bids from its own view, and is paid to take a book with a hole in it (D3)', () => {
    expect(bids.length, 'nobody bid at all').toBeGreaterThan(0);
    expect(bid, 'every bidder declined, so there is no acquirer').toBeDefined();
    expect(bid?.data['declined']).toBe(false);
    // D3: and the ones that said no said why, in their own terms.
    for (const no of bids.filter((e) => e.data['declined'] === true)) {
      expect(String(no.data['why'])).not.toBe('');
    }
    // D3: what it will pay is what the book is worth to IT. A book whose liabilities exceed its
    // assets is worth less than nothing, so the bid is negative: the acquirer is PAID to take it,
    // out of the estate and then the guarantee, and that payment is the hole made visible.
    expect(num(bid, 'pays')).toBeLessThan(0);
  });

  it('takes the deposits and the book over the wire, and nothing is left behind (D6, C3.b)', () => {
    expect(w.parties.get(BANK_A).status.alive).toBe(false);
    // C3.b: THE DEPOSITS KEEP WORKING. Every depositor that banked at the failed bank now holds
    // money issued by the acquirer, and holds it in an account at the acquirer.
    const dead = moneyInstrumentId(BANK_A, USD);
    expect(w.register.holdersOf(dead).length).toBe(0);
    for (const p of w.parties.all()) {
      if (!p.status.alive) continue;
      expect(p.bank).not.toBe(BANK_A);
    }
    // Law 2, D6: and nothing of its own is left in a dead party's hands.
    expect(w.register.holdingsOf(BANK_A).filter((h) => h.lots.length > 0)).toEqual([]);
    // Register F2: a reference to it resolves to whoever succeeded it — and WHICH bank bid for the
    // book is an outcome of what each of their own views made of it (D3), so the successor is read
    // off the resolution rather than named here (PLAN §7).
    const took = events(w, 'bank.resolution.done', BANK_A)[0];
    expect(took, 'nobody took the book, so there is no successor to resolve to').toBeDefined();
    expect(String(w.parties.resolve(BANK_A).id)).toBe(String(took?.data['acquirer']));
  });
});

describe('the guarantee (Banks Funding A1.a, Banks Capital D4, D5)', () => {
  it('makes the insured whole and lets the uninsured take it (D4, A1.a, XI-15)', () => {
    // A1.a: the cover is per MEMBER, so what it protects is a number of people rather than an
    // amount. Raise it past what a member of a household cell holds and no household is written
    // down at all, while the firms, the desks and the funds — which are nobody's members — take
    // exactly the loss they took before. That is E4's break in the loop, seen from the other side.
    const covered = failing(
      'res-d4',
      { 'regulation.depositInsurance.limit': 100000000 },
      RUNS_THE_HIERARCHY_DOWN,
    );
    const bare = failing('res-d4', { 'regulation.depositInsurance.limit': 0 }, RUNS_THE_HIERARCHY_DOWN);
    const households = (w: World): string[] =>
      events(w, 'bank.resolution.writtenDown', BANK_A)
        .map((e) => String(e.data['holder']))
        .filter((h) => h.startsWith('hh.'));
    expect(households(bare).length).toBeGreaterThan(0);
    expect(households(covered)).toEqual([]);
    // And the ones that are nobody's members are written down in both worlds.
    const firms = (w: World): string[] =>
      events(w, 'bank.resolution.writtenDown', BANK_A)
        .map((e) => String(e.data['holder']))
        .filter((h) => h.startsWith('firm.') || h.startsWith('desk.'));
    expect(firms(covered).length).toBeGreaterThan(0);
  });

  it('pays out of the fund the banks paid into, and the purse only after it (D4, D5)', () => {
    // D4: the insurer pays what the hierarchy could not reach, and it pays the ACQUIRER — because
    // the acquirer is the one about to owe the depositors the money nobody took from them.
    const w = failing('res-d5', {}, RUNS_THE_HIERARCHY_DOWN);
    const done = events(w, 'bank.resolution.done', BANK_A)[0];
    expect(num(done, 'insurerPaid')).toBeGreaterThan(0);
    const paid = w.ledger
      .all()
      .filter((r) => r.instruction.reason.includes('the guarantee') && r.outcome === 'settled');
    expect(paid.length).toBe(1);
    // D5: and the purse is LAST. With premiums collected there is a fund, so here it pays nothing.
    expect(num(done, 'pursePaid')).toBe(0);

    // With no premium there is no fund, and the same guarantee is met by the treasury instead —
    // which is the whole distinction D4 and D5 draw, and it is a fiscal cost with a payer.
    const unfunded = failing('res-d5', { [String(INSURER_PARAMS.premium)]: 0 });
    const then = events(unfunded, 'bank.resolution.done', BANK_A)[0];
    expect(unfunded.register.quantity(INSURER, moneyInstrumentId(
      unfunded.parties.get(INSURER).bank,
      USD,
    ))).toBe(0);
    expect(num(then, 'insurerPaid')).toBe(0);
    expect(num(then, 'pursePaid')).toBeGreaterThan(0);
  });
});

describe('contagion by name (Banks Capital E3, Banks Funding E5)', () => {
  it('lands a failed bank losses on the banks that funded it, junior money first', () => {
    // E3: a bank's creditors are other banks, so a failure is not contained in the bank it happens
    // to. Here bank B holds two claims on bank A — the paper it took when A was raising capital
    // (A2.b) and the money it lends A in the session — and they are in different places in the
    // queue. When A fails, the junior one is wiped before the senior one is touched at all, and
    // both losses land on a named lender's balance sheet in the same period.
    // ONE bank runs far above its own line and the other does not (B2: the buffer is each bank's
    // own), so one of them is raising and the other has the room to take the paper. Two banks both
    // short of capital do not fund each other, which is its own finding and the reason this world
    // is set up this way rather than by moving the rule.
    const w = failing('res-e3', { 'bank.capitalBuffer.bank.a': 0.9 });
    const took = events(w, 'bank.raise', BANK_A);
    expect(took.length).toBeGreaterThan(0);
    const down = events(w, 'bank.resolution.writtenDown', BANK_A);
    const junior = down.filter((e) => e.data['holder'] === BANK_B && num(e, 'rank') === 2);
    expect(junior.length).toBeGreaterThan(0);
    for (const e of junior) expect(num(e, 'lost')).toBeGreaterThan(0);
    // A2.b, A2.c: AND NOTHING ELSE WAS TOUCHED. The layer that was paid to be there was big enough
    // for this hole, so the senior claims and the depositors — everything at a lower rank — are
    // not in the list at all. That is what the middle rung is for, and without it every one of
    // them would have taken a share of this (the resolution above, where there was no layer).
    // Law 8: what does reach them is the piece the arithmetic could not put anywhere else. A hole
    // is a real number and a payment is a whole number of pieces, so the junior rank absorbs it to
    // within a piece and the remainder falls to the next rank — one piece each, not a share.
    for (const e of down) {
      if (num(e, 'rank') >= 2) continue;
      expect(num(e, 'lost')).toBeLessThanOrEqual(1);
    }
    // It is not wiped either: a rank takes the hole and no more than the hole, pro rata within it.
    for (const e of junior) expect(num(e, 'lost')).toBeLessThanOrEqual(num(e, 'owed'));
  });
});
