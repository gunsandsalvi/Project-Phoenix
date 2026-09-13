/**
 * A third party standing behind a second: the one relation in this world with three sides.
 *
 * @spec Banks Funding A1.a Banks Capital D4 Banks Capital D5 XI-8 Law 5 Law 6 Law 15
 */
import { describe, expect, it } from 'vitest';
import { currencyCode, partyId } from '../src/core/ids.js';
import { period } from '../src/calendar/calendar.js';
import { Guarantees } from '../src/register/guarantees.js';
import { ranWorld } from './rig.js';

const USD = currencyCode('USD');
const FUND = partyId('fund');
const BANK = partyId('bank');

const decl = (over: Partial<Parameters<Guarantees['give']>[0]> = {}) => ({
  guarantor: FUND,
  obligor: BANK,
  beneficiary: 'whoeverHolds' as const,
  what: 'the insured part of what the bank owes',
  ccy: USD,
  limit: 100,
  basis: 'insurance' as const,
  why: 'A1.a',
  ...over,
});

describe('three sides, and the one that pays is not the one that owes', () => {
  it('refuses the two things that are not guarantees', () => {
    const b = new Guarantees();
    // Law 5: a party standing behind itself is not a third side.
    expect(() => {
      b.give(decl({ guarantor: BANK }), period(0));
    }).toThrow(/cannot stand behind itself/);
    // Law 2: a promise to pay nothing should be said by not making one.
    expect(() => {
      b.give(decl({ limit: 0 }), period(0));
    }).toThrow(/guarantee of nothing/);
    // And a guarantee with NO limit is a real answer, not a large number (Law 6).
    expect(b.give(decl({ limit: null }), period(0)).limit).toBeNull();
  });

  it('answers who stands behind a party, which is the first question a lender has', () => {
    const b = new Guarantees();
    const g = b.give(decl(), period(1));
    expect(b.behind(BANK).map((x) => x.id)).toEqual([g.id]);
    expect(b.given(FUND).map((x) => x.id)).toEqual([g.id]);
    expect(b.behind(FUND)).toEqual([]);
    expect(g.state).toBe('standing');
    expect(g.paid).toBe(0);
  });

  it('a call records what was paid against what was promised', () => {
    const b = new Guarantees();
    const g = b.give(decl(), period(1));
    expect(b.headroom(g.id)).toBe(100);
    const part = b.called(g.id, 40);
    expect(part.paid).toBe(40);
    expect(part.state).toBe('called');
    expect(b.headroom(g.id)).toBe(60);
    /**
     * D4, D5: IT REFUSES A CALL PAST ITS OWN PROMISE. A guarantor that paid more than it promised
     * would be paying somebody else's obligation, and what the guarantee could not meet is what
     * the next thing behind it is FOR — the purse — rather than something absorbed quietly here.
     */
    expect(() => b.called(g.id, 61)).toThrow(/past its limit/);
    const rest = b.called(g.id, 60);
    // The promise was kept and ran out, which is not the same as being released.
    expect(rest.state).toBe('exhausted');
    expect(b.headroom(g.id)).toBe(0);
    expect(() => b.called(g.id, 1)).toThrow(/exhausted/);
  });

  it('a guarantee with no limit is called without ever running out', () => {
    const b = new Guarantees();
    const g = b.give(decl({ limit: null }), period(1));
    expect(b.headroom(g.id)).toBeNull();
    expect(b.called(g.id, 1e9).state).toBe('called');
    expect(b.called(g.id, 1e9).paid).toBe(2e9);
  });

  it('released is not exhausted, and a released one cannot be called', () => {
    const b = new Guarantees();
    const g = b.give(decl(), period(1));
    expect(b.release(g.id).state).toBe('released');
    expect(() => b.called(g.id, 1)).toThrow(/was released/);
  });
});

describe('what stands behind a bank in this world (A1.a)', () => {
  it('is said out loud, once, and is public', () => {
    /**
     * Deposit insurance worked before this and it worked by being an ORDERING OF PAYMENTS written
     * into the resolution path — the insurer pays, then the purse. Nothing could be asked who stood
     * behind a bank; a guaranteed deposit ranked in an estate exactly like an unguaranteed one.
     */
    const w = ranWorld('guarantee', 8);
    const given = w.journal.ofKind('guarantee.given');
    expect(given.length).toBeGreaterThan(0);
    for (const e of given) {
      expect(e.public).toBe(true);
      expect(String(e.data['basis'])).toBe('insurance');
    }
    // Once per bank: a guarantee said again every period would be a new promise every period.
    const obligors = given.map((e) => String(e.data['obligor']));
    expect(new Set(obligors).size).toBe(obligors.length);
    // And every bank that pays a premium has one, which is the same fact from the other end.
    for (const e of given) {
      const bank = partyId(String(e.data['obligor']));
      expect(w.guarantees.behind(bank).length).toBe(1);
      expect(w.guarantees.behind(bank)[0]?.state).toBe('standing');
    }
  });
});
