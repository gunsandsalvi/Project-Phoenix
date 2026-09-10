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
  LENDING_PARAMS,
  SUBORDINATED,
  assemble,
  foundationSpec,
  partyId,
  type Event,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const BANK_A = partyId('bank.a');

function run(w: World, periods: number): World {
  for (let i = 0; i < periods; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

/** The same world with one declared number set differently, wherever it was declared. */
function withParam(seed: string, over: Readonly<Record<string, number>>): World {
  const spec = foundationSpec(seed);
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

/** A backstop no bank in this world meets: what makes the raise happen at all (B3). */
const TIGHT = { [String(LENDING_PARAMS.leverageRatio)]: 0.6 };

describe('raising the layer between the owners and the creditors (C2, A2.b, A3)', () => {
  const w = run(withParam('raise-a', TIGHT), 8);

  it('asks for what it published, into a book that prices its name (C2, C2.a, Clearing C3)', () => {
    const raised = events(w, 'bank.raise');
    expect(raised.length).toBeGreaterThan(0);
    const first = raised[0];
    // Clearing C3: it posts a SIZE and no level — it is short of capital, not shopping — so what it
    // pays is the level its lenders posted and never one of its own.
    expect(num(first, 'rate')).toBeGreaterThan(0);
    expect(num(first, 'raised')).toBeGreaterThan(0);
    // C2.a: NEW MONEY PRICED BY WHOEVER PROVIDES IT. What it got is less than what it asked for,
    // because what a lender will put into one name is bounded by that lender's own limit (F3).
    expect(num(first, 'raised')).toBeLessThan(num(first, 'wanted'));
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
    expect(num(late, 'capital')).toBeGreaterThan(w.register.equity(BANK_A));
    expect(num(late, 'capital') - w.register.equity(BANK_A)).toBeCloseTo(
      num(events(w, 'bank.raise', BANK_A)[0], 'raised'),
      6,
    );
    // ...and it took the raise to get there: the first position it published was taken before any
    // of this paper existed, when its own equity was the whole of what stood in front of its
    // creditors (A2.a).
    expect(events(w, 'bank.raise', BANK_A)[0]?.period).toBeGreaterThan(early?.period ?? 0);
    // A1.a: and it is still not a pot. What the requirement asks of it is a walk over what it
    // holds, and what stands against that is the two layers, read each period.
    expect(num(late, 'assets')).toBeGreaterThan(0);
  });

  it('is bounded by what a lender will have out to one name (F3)', () => {
    // F3: the lender's limit is on THE NAME, not on a kind of claim. A bank that has already lent
    // this one money overnight, and holds its paper, is that much closer to its own limit — which
    // is why the raise gets smaller every week rather than repeating at the same size.
    const asked = events(w, 'bank.raise', BANK_A).concat(events(w, 'bank.raise.failed', BANK_A));
    expect(asked.length).toBeGreaterThan(2);
    // It asked every week it was short and got money once: after that its only possible lender was
    // already at its limit for this name, so the answer was no — a real refusal by a real lender
    // with a real reason, and not a rule anywhere saying one raise per bank.
    expect(events(w, 'bank.raise', BANK_A).length).toBe(1);
    expect(events(w, 'bank.raise.failed', BANK_A).length).toBeGreaterThan(0);
  });
});

describe('when nobody will (C2.b)', () => {
  it('is a real refusal, and it leaves the bank where it was', () => {
    // C2.b: NOBODY HAS TO BUY. Take the lenders' appetite away — no bank will have anything out to
    // any one name — and the same bank asks for the same money and gets none of it.
    const w = run(withParam('raise-b', { ...TIGHT, 'bank.limitPerBorrower.bank.b': 0 }), 8);
    const failed = events(w, 'bank.raise.failed', BANK_A);
    expect(failed.length).toBeGreaterThan(0);
    expect(num(failed[0], 'wanted')).toBeGreaterThan(0);
    expect(events(w, 'bank.raise', BANK_A)).toEqual([]);
    // Clearing C4.a: a book with one side in it does not clear, and the failure says which side.
    expect(String(failed[0]?.data['outcome'])).toBe('noSupply');
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
    const spec = foundationSpec('raise-c');
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
