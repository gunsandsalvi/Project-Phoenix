/**
 * The investment universe and the blueprint language (item 10e).
 *
 * @spec Fund Shares A4 Corporate Credit A4 Corporate Credit A4.b Ratings A3 Bond N4 Bond N13.a Law 2 Law 4 Law 15
 *
 * The owner's test for the language: *"a schedule of these should be able to fully define each asset
 * in the investment universe (like securitization, CP, etc)."* So this file takes the assets this
 * world actually issues and asks whether three structural reads and a handful of bands describe them
 * — with no table anywhere mapping an instrument to what it is.
 */
import { describe, expect, it } from 'vitest';
import { civil } from '../src/calendar/civil.js';
import { currencyCode, instrumentId, partyId } from '../src/core/ids.js';
import { none, some } from '../src/core/option.js';
import { admits, classify, lowestGrade, type Blueprint } from '../src/index.js';
import type { Instrument } from '../src/register/instruments.js';
import type { UniverseReads } from '../src/index.js';

const USD = currencyCode('USD');
const TODAY = civil(2026, 1, 1);

/** An asset stated by the three things the classification actually reads, and nothing else. */
function asset(o: {
  issuer?: string;
  kind?: string;
  promises?: boolean;
  last?: ReturnType<typeof civil>;
  seniority?: number;
  secured?: boolean;
  listed?: boolean;
  grades?: readonly string[];
}): { i: Instrument; reads: UniverseReads } {
  const i = {
    id: instrumentId('x'),
    ccy: USD,
    issuer: o.issuer === undefined ? none<ReturnType<typeof partyId>>() : some(partyId(o.issuer)),
    market: o.listed === false ? none() : some('mkt.x'),
  } as unknown as Instrument;
  const reads: UniverseReads = {
    on: TODAY,
    partyKind: () => o.kind ?? 'firm',
    promises: () => o.promises !== false,
    lastFlow: () => (o.last === undefined ? none() : some(o.last)),
    ranking: () =>
      o.seniority === undefined
        ? none()
        : some({ seniority: o.seniority, secured: o.secured === true }),
    gradesOn: () => o.grades ?? [],
  };
  return { i, reads };
}

const of = (o: Parameters<typeof asset>[0]) => {
  const { i, reads } = asset(o);
  return classify(i, reads);
};

describe('three reads give the classes, and there is no table (Law 15)', () => {
  it('classifies every asset this world issues, by what it structurally IS', () => {
    // Goods A1: nobody issued a tonne of wheat.
    expect(of({ promises: false }).what).toBe('thing');
    // Equity A4: it is PERPETUAL, which is why equity is not a long bond.
    expect(of({ issuer: 'firm.1' }).what).toBe('residual');
    expect(of({ issuer: 't.1', kind: 'treasury', last: civil(2031, 1, 1) }).what).toBe('government');
    // Commercial paper: a dated promise by a firm, and its shortness is the duration band's job.
    expect(of({ issuer: 'firm.1', last: civil(2026, 4, 1) }).what).toBe('corporate');
    // XI-11: a vehicle's note is a claim on a POOL and on nothing else.
    expect(of({ issuer: 'spv.1', kind: 'vehicle', last: civil(2031, 1, 1) }).what).toBe('structured');
  });

  it('measures duration from TODAY, so a five-year bond becomes a three-year bond', () => {
    const long = of({ issuer: 'firm.1', last: civil(2031, 1, 1) });
    const short = of({ issuer: 'firm.1', last: civil(2026, 4, 1) });
    expect(long.durationYears.some && long.durationYears.value).toBeGreaterThan(4);
    expect(short.durationYears.some && short.durationYears.value).toBeLessThan(1);
    // A residual has no duration to have, which is not a duration of zero (App A).
    expect(of({ issuer: 'firm.1' }).durationYears.some).toBe(false);
  });

  it('reads public against private off the market, as securitisable does (10c)', () => {
    expect(of({ issuer: 'firm.1', listed: false }).listed).toBe(false);
    expect(of({ issuer: 'firm.1' }).listed).toBe(true);
  });
});

describe('the grade is the LOWEST anybody published (Ratings A3, A4.b)', () => {
  it('takes the worst of the opinions, which is a SELECTION and never a blend', () => {
    const g = lowestGrade(['aa', 'bbb', 'a']);
    expect(g.some && g.value).toBe('bbb');
    // Every candidate is one named assessor's own view; what comes out is one of them, not a
    // number none of them holds (Law 2, Appendix B: no decision at an average).
    expect(['aa', 'bbb', 'a']).toContain(g.some ? g.value : '');
  });

  it('says NOTHING about a name nobody has assessed, which is not the worst grade (App A)', () => {
    expect(lowestGrade([]).some).toBe(false);
  });

  it('lets ONE assessor’s downgrade push a name below a mandate’s line', () => {
    const line: Blueprint = { classes: [], currencies: [], worstGrade: 'bbb' };
    const held = of({ issuer: 'firm.1', last: civil(2031, 1, 1), grades: ['a', 'a', 'a'] });
    const cut = of({ issuer: 'firm.1', last: civil(2031, 1, 1), grades: ['a', 'a', 'bb'] });
    expect(admits(line, held, () => undefined)).toBe(true);
    // That is the whole point of the lowest-of convention: a downgrade TRANSMITS.
    expect(admits(line, cut, () => undefined)).toBe(false);
  });
});

describe('a blueprint is bands, and an unstated band is silence (A4)', () => {
  const anySize = () => undefined;

  it('describes a money fund and a credit fund in one language', () => {
    const mmf: Blueprint = {
      classes: ['government', 'corporate'],
      currencies: [USD],
      duration: { to: 1 },
      worstGrade: 'a',
    };
    const credit: Blueprint = { classes: ['corporate'], currencies: [USD], duration: { from: 1, to: 10 } };
    const cp = of({ issuer: 'firm.1', last: civil(2026, 4, 1), grades: ['aa'] });
    const bond = of({ issuer: 'firm.1', last: civil(2031, 1, 1), grades: ['bbb'] });
    expect(admits(mmf, cp, anySize)).toBe(true);
    // The bond is too long for the money fund and too good-for-nothing about grade for it.
    expect(admits(mmf, bond, anySize)).toBe(false);
    expect(admits(credit, bond, anySize)).toBe(true);
    expect(admits(credit, cp, anySize)).toBe(false);
  });

  it('admits everything when it states nothing, which is a real mandate and not an absence', () => {
    const macro: Blueprint = { classes: [], currencies: [] };
    expect(admits(macro, of({ promises: false }), anySize)).toBe(true);
    expect(admits(macro, of({ issuer: 'firm.1', last: civil(2031, 1, 1) }), anySize)).toBe(true);
  });

  it('REFUSES an asset that cannot answer a band it stated (App A: missing is not a pass)', () => {
    // A fund that banded on duration is a fund that holds dated claims. A share has no duration,
    // so it is refused rather than admitted by silence — which is the `?? 0` this codebase forbids.
    const dated: Blueprint = { classes: [], currencies: [], duration: { to: 5 } };
    expect(admits(dated, of({ issuer: 'firm.1' }), anySize)).toBe(false);
  });

  it('separates public from private equity on the one read that distinguishes them', () => {
    const publicEq: Blueprint = { classes: ['residual'], currencies: [], listed: true };
    const privateEq: Blueprint = { classes: ['residual'], currencies: [], listed: false };
    expect(admits(publicEq, of({ issuer: 'firm.1' }), anySize)).toBe(true);
    expect(admits(privateEq, of({ issuer: 'firm.1' }), anySize)).toBe(false);
    expect(admits(privateEq, of({ issuer: 'firm.1', listed: false }), anySize)).toBe(true);
  });
});
