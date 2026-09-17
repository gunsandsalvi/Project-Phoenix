/**
 * The polity: the constitution's own numbers, and the rule that turns votes into seats.
 *
 * @spec Polity A1 Polity A4 Polity C1 Polity C2 Polity C4 Polity D1 Polity D5 XI-17 Law 15
 */
import { describe, expect, it } from 'vitest';
import { ALLOTMENT_RULES, POLITY_PARAMS, allotmentBy } from '../src/mechanisms/polity/index.js';
import { TREASURY_PARAMS } from '../src/mechanisms/treasury/index.js';
import { rigWorld } from './rig.js';
import { PLATFORMS } from '../src/mechanisms/polity/platforms.js';
import { platformPositions, distanceBetween, spreadAcross } from '../src/registry/platforms.js';
import { ballotOf, tally, turnoutOf } from '../src/mechanisms/polity/vote.js';
import { asCash } from '../src/core/measure.js';
import { USD, type ParamId } from '../src/index.js';

describe('the constitution states its own numbers (Polity A1, A4, C1, C2, C4, D5)', () => {
  const w = rigWorld('polity');

  it('names a seat count, a term in months, a rule, a distance and a lag — all the constitution’s', () => {
    for (const id of Object.values(POLITY_PARAMS)) {
      const d = w.params.decl(id);
      // D5: a primitive has an OWNER, and these are the constitution's — not parliament's, because
      // a body does not get to rewrite the rule that elected it.
      expect(d.kind).toBe('policy');
      expect(d.owner).toBe('constitution');
    }
    expect(w.params.count(POLITY_PARAMS.seats)).toBeGreaterThan(1);
    // A4: the term is in MONTHS, so the election falls on a day and not on a remainder.
    expect(w.params.decl(POLITY_PARAMS.termMonths).dimension).toBe('months');
    expect(w.params.periods(POLITY_PARAMS.mandateLag)).toBeGreaterThan(0);
  });

  it('turns votes into whole seats by the stated rule, and the house is always full (C1)', () => {
    const votes = new Map([
      ['a', 1000],
      ['b', 600],
      ['c', 401],
    ]);
    const seats = 100;
    for (const rule of ALLOTMENT_RULES) {
      const out = rule.allot(votes, seats);
      let total = 0;
      for (const n of out.values()) {
        expect(Number.isInteger(n)).toBe(true);
        expect(n).toBeGreaterThanOrEqual(0);
        total += n;
      }
      // A seat is a person: whole ones, and all of them sit.
      expect(total).toBe(seats);
    }
    // Proportional gives the shares; first past the post gives the lot to the largest. The two are
    // different parliaments from the SAME votes, which is what makes the rule a primitive (C1).
    const p = allotmentBy('proportional').allot(votes, seats);
    const f = allotmentBy('firstPastThePost').allot(votes, seats);
    expect(p.get('a')).toBeLessThan(seats);
    expect(p.get('b')).toBeGreaterThan(0);
    expect(f.get('a')).toBe(seats);
    expect(f.get('b')).toBe(0);
    // Determinism: the same votes give the same house, every time, with ties broken by name.
    expect([...allotmentBy('proportional').allot(votes, seats)]).toEqual([...p]);
  });

  it('has four tax bases with rates, and every one of them is parliament’s (D1, §30 C1)', () => {
    const bases = [
      TREASURY_PARAMS.taxProfits,
      TREASURY_PARAMS.taxInterest,
      TREASURY_PARAMS.taxGains,
      TREASURY_PARAMS.taxConsumption,
    ];
    for (const id of bases) {
      const d = w.params.decl(id);
      expect(d.kind).toBe('policy');
      expect(d.owner).toBe('parliament');
      expect(w.params.ratio(id)).toBeGreaterThan(0);
    }
    // 19.2: a gain is not a wage. The two rates are separate numbers and may differ, which is one
    // of the things an election is actually about.
    expect(String(TREASURY_PARAMS.taxGains)).not.toBe(String(TREASURY_PARAMS.taxIncome));
  });
});

describe('every party states a position on every number parliament owns (Polity A2, D5, 19.3)', () => {
  const w = rigWorld('platforms');
  const mine = w.params.all().filter((d) => d.kind === 'policy' && d.owner === 'parliament');

  it('covers all of them, in every platform, and the world would not have opened otherwise', () => {
    expect(PLATFORMS.length).toBeGreaterThan(1);
    expect(mine.length).toBeGreaterThan(0);
    const said = platformPositions(PLATFORMS, w.params.all());
    expect(said.size).toBe(PLATFORMS.length);
    for (const positions of said.values()) {
      // A2: EVERY one. A party that says nothing about a number has not given a cell enough to
      // vote on, and whatever it did about it afterwards would arrive from nowhere.
      expect(positions.size).toBe(mine.length);
    }
  });

  it('refuses a missing position, one on something parliament does not own, and a duplicate', () => {
    const [first] = PLATFORMS;
    expect(first).toBeDefined();
    if (first === undefined) return;
    const short = { ...first, positions: first.positions.slice(1) };
    expect(() => platformPositions([short], w.params.all())).toThrow();
    // D5, F2: a party cannot promise the central bank's rate — it is not parliament's to set.
    const overreach = {
      ...first,
      positions: [...first.positions, { on: 'centralBank.policyRate.USD', value: 0, why: 'it cannot.' }],
    };
    expect(() => platformPositions([overreach], w.params.all())).toThrow();
    // And two positions on one number is two answers to one question (Law 4).
    const twice = {
      ...first,
      positions: [...first.positions, { on: 'treasury.tax.income', value: 0.9, why: 'and also this.' }],
    };
    expect(() => platformPositions([twice], w.params.all())).toThrow();
    // Two platforms with one name is two parties nobody can tell apart.
    expect(() => platformPositions([first, first], w.params.all())).toThrow();
  });

  it('measures how far apart two platforms are against how far apart they all are (B2)', () => {
    const said = platformPositions(PLATFORMS, w.params.all());
    const spread = spreadAcross(said);
    expect(spread.size).toBeGreaterThan(0);
    const [a, b, c] = [...said.values()];
    expect(a).toBeDefined();
    expect(b).toBeDefined();
    expect(c).toBeDefined();
    if (a === undefined || b === undefined || c === undefined) return;
    // A platform is no distance at all from itself, and the two that differ most are further apart
    // than either is from the one between them — which is what "between" means here.
    expect(distanceBetween(a, a, spread)).toBe(0);
    expect(distanceBetween(a, b, spread)).toBeGreaterThan(distanceBetween(a, c, spread));
    expect(distanceBetween(a, b, spread)).toBeGreaterThan(distanceBetween(c, b, spread));
  });
});

describe('a cell votes from its own state, or stays home (Polity B1, B2, B2.b, B3, 19.4)', () => {
  const w = rigWorld('vote');
  const said = platformPositions(PLATFORMS, w.params.all());
  const on = {
    income: 'treasury.tax.income' as ParamId,
    consumption: 'treasury.tax.consumption' as ParamId,
    transfers: 'treasury.outlays.transfers.perMember' as ParamId,
    pension: 'pensions.contribution.employeeShare' as ParamId,
  };

  it('prefers the platform that leaves it best off, and a poor cell and a rich one differ', () => {
    // B2: each platform applied to the cell's OWN state. The two cells here differ in one thing —
    // what they expect to be paid — and that is enough to make them want different governments,
    // which is B4's mechanism seen from one cell: the result turns on the distribution.
    const poor = ballotOf(
      { cell: 'cell.poor' as never, weight: 10, expects: asCash(400, USD, 'what it expects') },
      said,
      on,
    );
    const rich = ballotOf(
      { cell: 'cell.rich' as never, weight: 10, expects: asCash(40_000, USD, 'what it expects') },
      said,
      on,
    );
    expect(poor.voted).toBeDefined();
    expect(rich.voted).toBeDefined();
    // The transfer is worth more than the taxes to a household with little pay, so the poor cell
    // wants the broad state. The rich cell does NOT, and which of the other two it wants is the
    // arithmetic's to say rather than mine: the lean state's low income tax is paid for by a high
    // consumption tax, and on a big income that trade is close. Nothing here is a preference
    // parameter — the two cells differ in ONE number, what they expect to be paid, and that is
    // enough to make them want different governments (B4's mechanism, seen from one cell).
    expect(poor.voted).toBe('broad');
    expect(rich.voted).not.toBe('broad');
    expect(rich.voted).not.toBe(poor.voted);
    // B3: a cell casts ALL its votes the same way, because a cell is one household with a
    // multiplicity and one household votes once (A3, XI-15).
    expect(poor.votes).toBe(10);
  });

  it('stays home when every platform leaves it in the same place, and turnout is the read', () => {
    // B2.b: abstention is a DECISION and not a rate. A cell with no expectation of pay has nothing
    // to compare and casts nothing; the turnout that results is a read of who could and who did.
    const nothing = ballotOf(
      { cell: 'cell.new' as never, weight: 7, expects: undefined },
      said,
      on,
    );
    expect(nothing.voted).toBeUndefined();
    expect(nothing.votes).toBe(0);
    expect(nothing.positions.length).toBe(0);
    const one = ballotOf(
      { cell: 'cell.a' as never, weight: 3, expects: asCash(1_000, USD, 'what it expects') },
      said,
      on,
    );
    const out = turnoutOf([nothing, one]);
    // The cell that could not compare is not counted as ABLE: it had no ballot to cast, which is a
    // different fact from having one and staying home, and the two are not averaged together.
    expect(out.able).toBe(3);
    expect(out.cast).toBe(3);
    // B2.a: there is no turnout parameter anywhere — this is arithmetic over what the cells did.
    expect(w.params.has('polity.turnout' as ParamId)).toBe(false);
  });

  it('sums the ballots weighted, and the tally is what the seats are allotted from (B3)', () => {
    const ballots = [
      ballotOf({ cell: 'a' as never, weight: 5, expects: asCash(400, USD, 'pay') }, said, on),
      ballotOf({ cell: 'b' as never, weight: 2, expects: asCash(400, USD, 'pay') }, said, on),
      ballotOf({ cell: 'c' as never, weight: 4, expects: asCash(40_000, USD, 'pay') }, said, on),
    ];
    const votes = tally(ballots);
    expect(votes.get('broad')).toBe(7);
    const other = [...votes].find(([id]) => id !== 'broad');
    expect(other?.[1]).toBe(4);
    // Σ vote(xᵢ)·wᵢ and never a sector's mean voter: the mean of these three cells' pay would have
    // voted one way, and what they actually did was 7 to 4 the other.
    expect([...votes.values()].reduce((x, y) => x + y, 0)).toBe(11);
  });
});
