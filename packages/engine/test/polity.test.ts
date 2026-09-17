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
import { formGovernment, mandateOf } from '../src/mechanisms/polity/government.js';
import { asCash } from '../src/core/measure.js';
import { USD, type ParamId } from '../src/index.js';
import type { ParamDecl } from '../src/registry/params.js';
import { whatParliamentControls } from '../src/registry/mandate.js';
import { POLICY_PARAMS } from '../src/mechanisms/money-market/policy.js';
import { policyRateOf } from '../src/mechanisms/money-market/data.js';

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

describe('who governs, and what the parliament then says (Polity C2, C2.a, C3, 19.5)', () => {
  const w = rigWorld('government');
  const said = platformPositions(PLATFORMS, w.params.all());

  it('is the largest party plus the nearest it may sit with, until it has a majority', () => {
    const seats = new Map([
      ['broad', 45],
      ['steady', 35],
      ['lean', 20],
    ]);
    const g = formGovernment(seats, said, 1, 100);
    expect(g.hung).toBe(false);
    // C2: the largest is in it, and it stopped as soon as it had a majority — a rule that kept
    // adding would be a grand coalition nobody's arithmetic asked for.
    expect(g.members[0]).toBe('broad');
    expect(g.seats).toBeGreaterThan(50);
    expect(g.members.length).toBeLessThan(3);
    // A majority is MORE than half: a party with exactly half the house does not govern alone.
    const split = formGovernment(new Map([['broad', 50], ['lean', 50]]), said, 1, 100);
    expect(split.members.length).toBeGreaterThan(1);
  });

  it('is HUNG when nobody may sit with anybody, and the standing mandate continues (C2.a)', () => {
    // Every platform too far from every other: the largest is short, there is nobody it may add,
    // and the answer is that there is no government — reported, not repaired.
    const seats = new Map([
      ['broad', 40],
      ['steady', 35],
      ['lean', 25],
    ]);
    const g = formGovernment(seats, said, 0, 100);
    expect(g.hung).toBe(true);
    expect(g.seats).toBeLessThanOrEqual(50);
    // C3: and a hung parliament produces NO mandate — not an empty one, which would be a mandate
    // setting every number to nothing. What governs is what was already standing.
    expect(mandateOf(g, seats, said, w.params.all())).toBeUndefined();
  });

  it('reads the mandate off the parliament, seat-weighted across the coalition (C3)', () => {
    const seats = new Map([
      ['broad', 40],
      ['steady', 30],
      ['lean', 30],
    ]);
    const g = formGovernment(seats, said, 1, 100);
    const mandate = mandateOf(g, seats, said, w.params.all());
    expect(mandate).toBeDefined();
    if (mandate === undefined) return;
    // Every number parliament owns has a value in it, and each one lies between what the parties in
    // government wanted — a coalition governs at what its seats between them come to, which is the
    // only honest reading of "what this coalition would do" when nobody may invent a deal.
    const mine = w.params.all().filter((d) => d.kind === 'policy' && d.owner === 'parliament');
    expect(mandate.size).toBe(mine.length);
    for (const [id, value] of mandate) {
      const wanted = g.members.map((m) => said.get(m)?.get(id) ?? Number.NaN);
      expect(value).toBeGreaterThanOrEqual(Math.min(...wanted));
      expect(value).toBeLessThanOrEqual(Math.max(...wanted));
    }
    // One party governing alone governs at its own platform, exactly.
    const alone = new Map([['lean', 60], ['broad', 40]]);
    const solo = formGovernment(alone, said, 1, 100);
    const its = mandateOf(solo, alone, said, w.params.all());
    expect(solo.members).toEqual(['lean']);
    expect(its?.get('treasury.tax.income' as never)).toBe(said.get('lean')?.get('treasury.tax.income' as never));
  });
});

describe('the mandate takes effect at the lag, and the register holds it (Polity C3.a, C3.b, C4, 19.6)', () => {
  /**
   * The constitution says forty-eight months, which is longer than any test world runs — so this
   * one shortens the TERM (the constitution's own number, moved on the register, which is where a
   * constitution is written) and lets the same mechanism run four times instead of never. Nothing
   * else is touched: the platforms, the vote, the coalition and the lag are the world's.
   */
  function elected(): ReturnType<typeof rigWorld> {
    const w = rigWorld('polity.mandate');
    w.params.setByMandate(POLITY_PARAMS.termMonths, 1, 'constitution', 'a scale model votes often.');
    for (let i = 0; i < 14; i += 1) w.step();
    return w;
  }

  it('journals what the parliament said with the period it starts from, and writes it then', () => {
    const w = elected();
    const given = w.journal.ofKind('polity.mandate');
    expect(given.length).toBeGreaterThan(0);
    const first = given[0];
    if (first === undefined) return;
    // C4: a government is formed and THEN it governs — the mandate names the period it starts from,
    // and it is the election's period plus the constitution's lag, not the election's own.
    const lag = w.params.periods(POLITY_PARAMS.mandateLag);
    expect(Number(first.data['from'])).toBe(Number(first.data['elected']) + lag);
    expect(String(first.data['government']).length).toBeGreaterThan(0);
    // C3.a: and the numbers moved in the period it starts from, through the one door — the event
    // says which, and the period it was recorded in is the period the mandate named.
    const taken = w.journal.ofKind('polity.mandate.inForce');
    expect(taken.length).toBeGreaterThan(0);
    const firstTaken = taken[0];
    if (firstTaken === undefined) return;
    expect(Number(firstTaken.period)).toBe(Number(first.data['from']));
    // A government that governs moves something: the platforms differ, so the mandate does too.
    expect(taken.some((e) => Number(e.data['moved']) > 0)).toBe(true);
  });

  it('measures that every number parliament owns stands where the mandate put it (C3.b)', () => {
    const w = elected();
    const family = (r: ReturnType<typeof w.step>): number => {
      const f = r.audit.families.find((x) => x.contributions.includes('polity'));
      expect(f?.built).toBe(true);
      return f?.violations.filter((v) => v.spec === 'Polity C3.b').length ?? -1;
    };
    // With the mandate standing and nothing else touching the numbers, there is nothing to find.
    expect(family(w.step())).toBe(0);
    // Move one of them off the mandate and the family says so — by name, with the size of the gap,
    // and without repairing it (Audit C4). This is C3.a's forbid guarded rather than remembered.
    const given = w.journal.ofKind('polity.mandate');
    const last = given[given.length - 1];
    if (last === undefined) return;
    const values = last.data['mandate'] as Record<string, number>;
    const [id] = Object.keys(values);
    if (id === undefined) return;
    // Off the STANDING mandate, which is what the register holds while nothing else touches it.
    const was = w.params.decl(id as ParamId).value;
    w.params.setByMandate(id as ParamId, was + 0.01, 'parliament', 'a hand on the dial.');
    const after = w.step();
    const found = after.audit.families
      .find((x) => x.contributions.includes('polity'))
      ?.violations.filter((v) => v.spec === 'Polity C3.b');
    expect(found?.length).toBe(1);
    expect(found?.[0]?.owner).toBe(id);
    expect(found?.[0]?.size).toBeCloseTo(0.01, 12);
    // And it is still off: the audit observed, and changed nothing (Audit C4).
    expect(w.params.decl(id as ParamId).value).toBeCloseTo(was + 0.01, 12);
  });
});

describe('what the parliament controls, and what no parliament may reach (Polity D1–D4, D3.a, 19.7)', () => {
  const w = rigWorld('polity.scope');
  const policies = (): ParamDecl[] => [...w.params.all()];

  it('names the power behind every number parliament owns, and the world would not open otherwise', () => {
    const powers = whatParliamentControls(policies());
    const mine = policies().filter((d) => d.kind === 'policy' && d.owner === 'parliament');
    expect(mine.length).toBeGreaterThan(0);
    expect(powers.size).toBe(mine.length);
    // Every one is an exercise of ONE of the four powers, and says which.
    for (const [, p] of powers) {
      expect(['Polity D1', 'Polity D2', 'Polity D3', 'Polity D4']).toContain(p.clause);
      expect(p.why.length).toBeGreaterThan(0);
    }
    // D1: the buffer is the parliament's, and so it is a thing the parties differ about (§30 D4.b).
    expect(w.params.decl(TREASURY_PARAMS.bufferPeriods).owner).toBe('parliament');
    expect(powers.get(TREASURY_PARAMS.bufferPeriods)?.clause).toBe('Polity D1');
    // D4: what the bank AIMS AT is parliament's, and what it does about it is the bank's.
    expect(powers.get(POLICY_PARAMS.target('USD'))?.clause).toBe('Polity D4');
    expect(powers.has(policyRateOf('USD'))).toBe(false);
  });

  it('refuses a price, the bank’s rate, a number no clause grants and a power over nothing', () => {
    const all = policies();
    const one = all.find((d) => d.kind === 'policy' && d.owner === 'parliament');
    expect(one).toBeDefined();
    if (one === undefined) return;
    // D3.a: a price is CLEARED (Law 3), whoever would rather it were not.
    expect(() =>
      whatParliamentControls([...all, { ...one, id: 'treasury.tax.aPrice' as ParamId, dimension: 'price' }]),
    ).toThrow(/price/);
    // D4, §31 A4: the bank's rate is not the parliament's, however it is declared.
    expect(() =>
      whatParliamentControls([
        ...all,
        { ...all.find((d) => String(d.id) === String(policyRateOf('USD'))) ?? one, owner: 'parliament' },
      ]),
    ).toThrow(/policyRate/);
    // A module that makes a number parliament's without naming the power: the world does not open.
    expect(() =>
      whatParliamentControls([...all, { ...one, id: 'someModule.aNumber' as ParamId }]),
    ).toThrow(/no clause of D1–D4 grants it/);
    // And a power over nothing is scope this world does not actually grant — a number renamed, a
    // module dropped — which reads like a parliament with a power it has not got.
    expect(() =>
      whatParliamentControls(all.filter((d) => !String(d.id).startsWith('land.planning.'))),
    ).toThrow(/land\.planning\./);
  });
});
