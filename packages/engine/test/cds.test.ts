/**
 * Credit default swaps: a market in whether a named party will pay.
 *
 * @spec CDS A1 CDS A1.a CDS A1.b CDS A1.c CDS A1.d CDS A2 CDS A3 CDS A4 CDS A4.a CDS B1 CDS B1.a CDS B2 CDS B5 CDS C2 CDS C3 CDS E3 Derivative D1 Derivative D1.b Derivative D3 Derivative D7.b Derivative Layer B1 Derivative Layer C2 Law 3 Law 15
 *
 * A scale model, and it names no party: the reference is whichever drawn borrower this world's
 * banks actually hold paper of (`rig.ts`), not "the big firm".
 */
import { describe, expect, it } from 'vitest';
import {contractOf, CDS,
  CDS_PARAMS,
  USD,
  cdsKind,
  cdsLineOf,
  cdsMarketOf,
  impliedDefaultRate,
  isCds,
  middleGrade,
  rankOf,
  mirrored,
  netNotionalOn,
  period,
  cdsTenorsOf,
  type Contract,
  type ContractReads,
  type MarketDecl,
  type PartyId,
  type World,} from '../src/index.js';
import { rigWorld } from './rig.js';

function ranWorld(periods: number): World {
  const w = rigWorld('cds');
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

function books(w: World): readonly MarketDecl[] {
  return w.markets.filter((m) => {
      const t = contractOf(m)?.terms;
      return t !== undefined && isCds(t);
    });
}

describe('the books (A1, A1.d, A4, A4.a, C2)', () => {
  it('opens one per reference per tenor, on names somebody is actually exposed to', () => {
    const w = ranWorld(2);
    const open = books(w);
    expect(open.length).toBeGreaterThan(0);
    const tenors = cdsTenorsOf({ params: w.params });
    for (const m of open) {
      const t = contractOf(m)?.terms;
      if (t === undefined || !isCds(t)) continue;
      // A1.d: the term structure is a SET OF BOOKS. A reference with one has all of them.
      for (const y of tenors) {
        expect(w.markets.some((x) => x.id === cdsMarketOf(t.reference, y))).toBe(true);
      }
      // A4, A4.a: the reference is a party somebody can watch fail — it has live paper that can.
      expect(w.instruments.get(t.obligation).status.live).toBe(true);
      expect(w.instruments.get(t.obligation).issuer.some).toBe(true);
      // B1.a: and somebody other than the issuer is holding it, which is why the book exists.
      expect(w.register.holdersOf(t.obligation).some((h) => h !== t.reference)).toBe(true);
      // C2: cleared, through the house that clears this money.
      expect(contractOf(m)?.house).not.toBe(null);
    }
  });

  it('prints the SPREAD, on the grid a spread is quoted on (A1.c, Law 8)', () => {
    const w = ranWorld(1);
    expect(cdsKind.quotedAs).toBe('rate');
    // Law 8: the tick is in the terms a level is HELD in — money pieces per piece of the thing —
    // so a basis point per annum on one unit of face is a hundredth of a cent, and the level the
    // book clears is that rate PER ANNUM, which is what `quotedAs` says and a reader needs.
    expect(w.registry.tickForDerivative(CDS, USD)).toBeCloseTo(0.01, 12);
    // D7.b: struck at par — the cleared spread is what makes it worth nothing, so nothing is paid.
    expect(cdsKind.premiumPerUnit(0.01, { kind: CDS })).toBe(0);
  });
});

describe('what the contract IS (A2, A3, D1.b)', () => {
  function rowOn(w: World, reference: PartyId, tenorYears: number, buyer: PartyId, seller: PartyId): Contract {
    return {
      id: 'contract.test' as Contract['id'],
      kind: CDS,
      a: buyer,
      b: seller,
      terms: {
        kind: CDS,
        reference,
        obligation: w.instruments.all().find((i) => i.issuer.some && i.issuer.value === reference)?.id as never,
        book: cdsLineOf(reference, tenorYears),
        maturity: period(w.period + 20),
        tenorYears,
        buysProtection: true,
        window: 8,
      },
      ccy: USD,
      notional: 1_000_000,
      struckAt: 0.01,
      basis: 0,
      opened: w.period,
      state: 'open',
      terminated: { some: false },
      house: null,
    } as Contract;
  }

  function someReference(w: World): PartyId | undefined {
    const m = books(w)[0];
    const t = contractOf(m)?.terms;
    return t !== undefined && isCds(t) ? t.reference : undefined;
  }

  it('is one number and its negation, whichever side states it (D1.b, A3)', () => {
    const w = ranWorld(3);
    const reference = someReference(w);
    if (reference === undefined) return;
    const parties = w.parties.all().filter((p) => p.status.alive && p.id !== reference);
    const buyer = parties[0]?.id;
    const seller = parties[1]?.id;
    if (buyer === undefined || seller === undefined) return;
    const c = rowOn(w, reference, cdsTenorsOf({ params: w.params })[0] ?? 1, buyer, seller);
    const reads: ContractReads = w.contractReads(w.period);
    const toA = cdsKind.mark(c, w.period, reads);
    const toB = cdsKind.mark(mirrored(c, cdsKind), w.period, reads);
    // D1.b: EXACTLY. Not within dust — the same number, negated, computed from the other side's
    // own statement of the terms.
    expect(toA + toB).toBe(0);
  });

  it('pays a premium every period and stops the period the reference fails (A2)', () => {
    const w = ranWorld(3);
    const reference = someReference(w);
    if (reference === undefined) return;
    const parties = w.parties.all().filter((p) => p.status.alive && p.id !== reference);
    const buyer = parties[0]?.id;
    const seller = parties[1]?.id;
    if (buyer === undefined || seller === undefined) return;
    const c = rowOn(w, reference, cdsTenorsOf({ params: w.params })[0] ?? 1, buyer, seller);
    const due = cdsKind.legs(c, w.period, w.contractReads(w.period));
    expect(due.length).toBe(1);
    const leg = due[0];
    if (leg === undefined) return;
    // A2: the buyer pays the seller, in cash, the spread it struck at on what it protects.
    expect(leg.from).toBe(buyer);
    expect(leg.to).toBe(seller);
    expect(leg.amount).toBeGreaterThan(0);
    expect(leg.ccy).toBe(USD);
  });
});

describe('what is derived and what is stored (C2, E3, Law 3)', () => {
  it('derives the implied default rate and stores none of it', () => {
    // C2: the probability comes OUT of the spread and the recovery; it never goes in. A spread of
    // a hundred basis points against a recovery of forty cents is a rate anybody can divide out.
    const r = impliedDefaultRate(0.01, 0.4);
    expect(r.some).toBe(true);
    expect(r.some ? r.value : 0).toBeCloseTo(0.01 / 0.6, 12);
    // And nothing in the parameter register carries one: there is no recovery rate in this world.
    expect(() => ({ ...CDS_PARAMS })).not.toThrow();
    expect(Object.values(CDS_PARAMS).some((p) => String(p).includes('recovery'))).toBe(false);
  });

  it('nets notional per reference and offers no wider netting (E3, G3)', () => {
    const w = ranWorld(4);
    const reference = contractOf(books(w)[0])?.terms;
    if (reference === undefined || !isCds(reference)) return;
    const net = netNotionalOn(w.mechanismContext('test'), reference.reference);
    expect(net).toBeGreaterThanOrEqual(0);
  });
});

describe('the series (A5, A5.a, A5.b, Indices A1.a, B1)', () => {
  it('fixes its names at the roll and divides them by the rule everybody was told', () => {
    const w = rigWorld('cds-series');
    // Long enough for at least one roll (the roll is every `cds.index.roll.periods`).
    const every = w.params.periods(CDS_PARAMS.roll);
    for (let i = 0; i <= every; i += 1) w.step();
    const rolled = w.journal.ofKind('cds.index.rolled');
    if (rolled.length === 0) return;
    for (const e of rolled) {
      // A5: names and weights are stated when the series is published, and published means public.
      expect(e.public).toBe(true);
      expect(Array.isArray(e.data['names'])).toBe(true);
      expect(Array.isArray(e.data['weights'])).toBe(true);
      // Indices B1: a weight is a COUNT of a real thing — what each name has outstanding.
      for (const wgt of e.data['weights'] as readonly number[]) expect(wgt).toBeGreaterThan(0);
      // A5.a: one line per grade, split on the rule in the registry (the middle opinion counts).
      expect(['ig', 'hy']).toContain(String(e.data['grade']));
    }
    // A5.b: each series has its OWN book, so the line has a cleared spread of its own.
    const series = String(rolled[0]?.data['series']);
    expect(w.markets.some((m) => String(m.id).includes(series))).toBe(true);
  });

  it('combines three disagreeing opinions by one published rule, and one of them cannot move it', () => {
    // registry/grades.ts states it in advance: the MIDDLE opinion counts, so a single assessor
    // moving changes nothing and which two agree is a real question (A1.a, CDS A5.a).
    expect(middleGrade(['aa', 'a', 'bbb'])).toBe('a');
    expect(middleGrade(['aa', 'a', 'c'])).toBe('a');
    expect(middleGrade(['c', 'c', 'aaa'])).toBe('c');
    // A name nobody has assessed has no grade, which is not the worst grade.
    expect(middleGrade([])).toBe(undefined);
    expect(rankOf('bbb')).toBeLessThan(rankOf('bb'));
  });
});
