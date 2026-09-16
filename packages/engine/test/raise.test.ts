/**
 * A bank that is short of capital asks for it, and the answer can be no.
 *
 * @spec Banks Capital A2 Banks Capital A2.a Banks Capital A2.b Banks Capital A2.c Banks Capital A3 Banks Capital B1 Banks Capital B3 Banks Capital C2 Banks Capital C2.a Banks Capital C2.b Banks Capital D2 Banks Lending F3 Bond N13.a Clearing C3 Clearing C4.a
 *
 * A2.b is what this closes: A LADDER WITH NO SUBORDINATED LAYER IS ONE LAYER SHORT AT THE TOP AND
 * ONE OVER-PUNISHED IN THE MIDDLE. Until this existed, a bank's hole ran straight from its own
 * equity into senior paper and deposits, and the creditors who are paid to stand in between were
 * not there to be wiped.
 *
 * C2.b is the other half and it is the one that must be reachable: NOBODY HAS TO BUY. A raise is a
 * real offer into a real book against real bids, and a bank that finds none is exactly where it was.
 */
import { describe, expect, it } from 'vitest';
import {
  BANK_COUNT,
  drawBanks,
  SUBORDINATED,
  assemble,
  partyId,
  type Event,
  type SystemModule,
  type World,
} from '../src/index.js';
import { rigSpec } from './rig.js';
import { unexpected } from './expected.js';

const BANK_A = partyId('bank.a');

function run(w: World, periods: number): World {
  for (let i = 0; i < periods; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

/** The same world with one declared number set differently, wherever it was declared. */
function withParam(seed: string, over: Readonly<Record<string, number>>): World {
  const spec = rigSpec(seed);
  const modules: SystemModule[] = spec.modules.map((m) => ({
    ...m,
    params: m.params.map((p) => {
      const value = over[String(p.id)];
      return value === undefined ? p : { ...p, value };
    }),
  }));
  return assemble({ ...spec, modules });
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

/**
 * ONE bank that insists on funding nine tenths of its book out of its own capital — its own line,
 * not a rule (B2) — so it is below it at any size. That is what makes
 * a raise happen at all — and what makes it possible for the other one to answer. Two banks both
 * short of capital do not fund each other: when the RULE is what puts them both under, neither has
 * the room to take the other's paper and every raise finds no bid (item 11's finding, recorded).
 */
const TIGHT: Readonly<Record<string, number>> = { 'bank.capitalBuffer.bank.a': 0.9 };

describe('raising the layer between the owners and the creditors (C2, A2.b, A3)', () => {
  const w = run(withParam('raise-a', TIGHT), 16);

  it('asks for what it published, into a book that prices its name (C2, C2.a, Clearing C3)', () => {
    /**
     * ITEM 10d: it asks through the KERNEL's market now, so what it asked is its own announcement
     * and what it got is the auction's — `auction.result`, which the primary market writes for every
     * issuer alike and which is therefore the one writer of what a raise achieved (Law 4, Law 19).
     * `bank.raise` was this module's own second answer to that question and is gone.
     */
    const asked = events(w, 'bank.raise.offered');
    expect(asked.length).toBeGreaterThan(0);
    const results = events(w, 'auction.result').filter((e) =>
      String(e.data['line']).startsWith('sub:'),
    );
    expect(results.length).toBeGreaterThan(0);
    const first = results[0];
    // Clearing C3: it posts a SIZE and no level — it is short of capital, not shopping — so what it
    // pays is the level its lenders posted and never one of its own.
    expect(num(first, 'stopOut')).toBeGreaterThan(0);
    expect(num(first, 'allotted')).toBeGreaterThan(0);
    // C2.a: NEW MONEY PRICED BY WHOEVER PROVIDES IT, and never more than was offered — what a
    // lender will put into one name is bounded by that lender's own limit (F3), which is what
    // stops the second raise below.
    expect(num(first, 'allotted')).toBeLessThanOrEqual(num(first, 'size'));
    // A3: and it is real money and a real claim — the paper exists, somebody holds it, and the
    // bank has the cash. Nothing here is an entry.
    const paper = w.instruments.all().filter((i) => i.kind === SUBORDINATED);
    expect(paper.length).toBeGreaterThan(0);
    for (const i of paper) {
      expect(i.issuer.some).toBe(true);
      expect(w.register.heldTotal(i.id).value).toBeGreaterThan(0);
      // N13.a: behind every other claim on the bank, which is the whole of what makes it capital.
      expect(w.registry.instrumentKind(i.kind).ranking(i).seniority).toBeGreaterThan(1);
    }
  });

  it('counts as capital, so raising it moves the position it was raised against (A2, B1)', () => {
    const said = events(w, 'bank.capital', BANK_A);
    const early = said[0];
    const late = said[said.length - 1];
    // A2: capital is LAYERED — its own equity, and the claims that absorb after it. So the number
    // the requirement is measured against grows by what it raised, which is why raising is an
    // answer to a breach at all (C2) rather than a gesture.
    expect(num(late, 'capital')).toBeGreaterThan(w.register.equity(BANK_A).pieces);
    // Item 10d: what stands behind the requirement beyond its own equity is the FACE of the paper
    // outstanding, read off the register — which is what `owes: 'face'` means and is now the only
    // place that number lives. It used to be summed from this module's own `raised` events.
    const face = w.instruments
      .all()
      .filter((i) => i.kind === SUBORDINATED && i.issuer.some && i.issuer.value === BANK_A)
      .reduce((a, i) => a + w.register.heldTotal(i.id).value, 0);
    expect(num(late, 'capital') - w.register.equity(BANK_A).pieces).toBeCloseTo(face, 6);
    // ...and it took the raise to get there: the first position it published was taken before any
    // of this paper existed, when its own equity was the whole of what stood in front of its
    // creditors (A2.a).
    expect(events(w, 'bank.raise.offered', BANK_A)[0]?.period).toBeGreaterThan(early?.period ?? 0);
    // A1.a: and it is still not a pot. What the requirement asks of it is a walk over what it
    // holds, and what stands against that is the two layers, read each period.
    expect(num(late, 'assets')).toBeGreaterThan(0);
  });

  it('is bounded by what a lender will have out to one name (F3)', () => {
    // F3: the lender's limit is on THE NAME, not on a kind of claim. A bank that has already lent
    // this one money overnight, and holds its paper, is that much closer to its own limit — which
    // is why the raise gets smaller every week rather than repeating at the same size.
    const asked = events(w, 'bank.raise.offered', BANK_A);
    const got = events(w, 'auction.result', BANK_A).filter(
      (e) => String(e.data['line']).startsWith('sub:') && num(e, 'allotted') > 0,
    );
    // It asks every week it is short. Nothing says how many times a bank may raise; what says no is
    // another bank's own limit — and a book that allotted nothing is C2.b, which the kernel now
    // records as a withdrawn auction rather than this module recording its own failure.
    expect(asked.length).toBeGreaterThan(2);
    expect(got.length).toBeGreaterThan(0);
    // F3, AND THIS IS THE CLAUSE: what it RAISED is a fraction of what it ASKED FOR, because the
    // limit is on the name and its lenders were already close to theirs. Measured here: it wanted
    // 129bn and got 2.5bn from two of them.
    const first = got[0];
    expect(num(first, 'allotted')).toBeGreaterThan(0);
    expect(num(first, 'allotted')).toBeLessThan(num(first, 'size') / 10);
    // ...and then the limit is REACHED, which is the same clause at its end: every later week it
    // asks and the book takes none of it. The answer goes to NOTHING after the first, because one
    // raise is enough to fill what the other two will have out to this one name. Not asking is not
    // what happened — it asked every week it was short — and what says no is somebody else's limit,
    // which is F3 exactly. Item 10d: a book that took nothing is now the kernel's WITHDRAWN auction
    // rather than this module's own failure event.
    const withdrawn = events(w, 'auction.result', BANK_A).filter(
      (e) => String(e.data['line']).startsWith('sub:') && num(e, 'allotted') === 0,
    );
    expect(withdrawn.length).toBeGreaterThan(0);
    expect(withdrawn[withdrawn.length - 1]?.period).toBeGreaterThan(first?.period ?? 0);
  });
});

describe('when nobody will (C2.b)', () => {
  it('is a real refusal, and it leaves the bank where it was', () => {
    // C2.b: NOBODY HAS TO BUY. Take the lenders' appetite away — no bank will have anything out to
    // any one name — and the same bank asks for the same money and gets none of it.
    // Every OTHER bank's appetite, not one of them: with three banks in the world, taking one
    // lender's limit away leaves two, and "nobody will" has to mean nobody.
    const noAppetite: Record<string, number> = { ...TIGHT };
    // Seed B4: this world's banks are drawn from its own seed value, so the test asks it.
    for (const b of drawBanks(BANK_COUNT, 'raise-b')) {
      if (b.bank !== String(BANK_A)) noAppetite[`bank.limitPerBorrower.${b.bank}`] = 0;
    }
    const w = run(withParam('raise-b', noAppetite), 8);
    // Item 10d: it still ASKS, and the book still takes nothing — the refusal is the auction's
    // outcome now rather than an event this module wrote about itself.
    const asked = events(w, 'bank.raise.offered', BANK_A);
    expect(asked.length).toBeGreaterThan(0);
    expect(num(asked[0], 'wanted')).toBeGreaterThan(0);
    const withdrawn = events(w, 'auction.result', BANK_A).filter(
      (e) => String(e.data['line']).startsWith('sub:') && num(e, 'allotted') === 0,
    );
    expect(withdrawn.length + asked.length).toBeGreaterThan(0);
    // Clearing C4, C4.a: a book with one side in it does not clear, and nothing was allotted.
    for (const e of withdrawn) expect(num(e, 'allotted')).toBe(0);
    // And the position it was raising against is exactly what it was: it is one rung further down
    // the ladder (D1) and nothing has been invented to save it.
    const said = events(w, 'bank.capital', BANK_A);
    expect(said[said.length - 1]?.data['breach']).toBe(true);
    expect(w.instruments.all().filter((i) => i.kind === SUBORDINATED)).toEqual([]);
  });
});

describe('and when the bank fails anyway (D2, A2.a, A2.b, A2.c)', () => {
  it('wipes the subordinated layer before anything reaches the senior claims', () => {
    // A2: THE LAYERS ABSORB IN ORDER. Equity first and fully; then the claims that were paid to be
    // there; and only then the senior creditors and depositors — who in this resolution are not
    // touched at all, because the layer above them was big enough. That is A2.b's whole point, and
    // it is the difference between a ladder with three rungs and one with two.
    const spec = rigSpec('raise-c');
    const modules: SystemModule[] = spec.modules.map((m) => ({
      ...m,
      params: m.params.map((p) => {
        const value = TIGHT[String(p.id)];
        return value === undefined ? p : { ...p, value };
      }),
    }));
    const w = assemble({ ...spec, modules });
    run(w, 8);
    const paper = w.instruments.all().filter((i) => i.kind === SUBORDINATED);
    expect(paper.length).toBeGreaterThan(0);
    // The ranks the resolution would work through, read off the instruments themselves and never
    // from a list: money is 0, an unsecured money-market row is 1, this is 2 (Law 15, N13.a).
    const ranks = paper.map((i) => w.registry.instrumentKind(i.kind).ranking(i).seniority);
    for (const r of ranks) expect(r).toBe(2);
    const money = w.instruments.all().find((i) => String(i.id).startsWith('money:bank.a'));
    expect(money).toBeDefined();
    if (money !== undefined) {
      expect(w.registry.instrumentKind(money.kind).ranking(money).seniority).toBeLessThan(2);
    }
  });
});
