/**
 * A value that is neither a print nor a cost: a claim ON A BOOK, worth what the book comes to
 * divided by how many claims there are.
 *
 * @spec XI-6 Fund Shares B1 Fund Shares B2 Fund Shares B2.a Fund Shares F2 Law 3 Law 4 Law 19
 *
 * This is a kernel test and it uses a toy kind, deliberately: the rule is that a derived value is
 * read from the same marks everything else is valued at, never stored, and the same number for
 * every holder. What it is a claim on is the funds module's business.
 */
import { describe, expect, it } from 'vitest';
import {
  PHX,
  assemble,
  foundationSpec,
  instrumentId,
  instrumentKindId,
  none,
  partyId,
  some,
  unitId,
  type InstrumentKindProfile,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const BOOK = partyId('firm.1');
const HOLDER = partyId('firm.2');
const OTHER = partyId('firm.3');
const CLAIM_KIND = instrumentKindId('test.claim-on-a-book');
const CLAIM = instrumentId('test.claim');
const CLAIMS = unitId('claims');

/**
 * B1: what one claim is worth is what the book holds, less what the book owes anybody else,
 * divided by the claims outstanding. It reads the register at the moment it is asked and stores
 * nothing (Law 19), and every holder gets the same number.
 */
function claimKind(countsItsOwn = false): InstrumentKindProfile {
  return {
    id: CLAIM_KIND,
    pricing: 'derived',
    carry: 'cost',
    liabilityOfIssuer: true,
    fairValueThroughIncome: true,
    unit: () => CLAIMS,
    ranking: () => ({ seniority: 1, secured: [], claim: 'what is left of the book' }),
    validateTerms: () => undefined,
    displayName: (i) => String(i.id),
    due: () => [],
    accrued: () => 0,
    cashFlows: () => [],
    carriedAt: (_i, lot, marked) => (lot.qty === 0 ? none<number>() : marked),
    derive: (i, at, reads) => {
      const issued = reads.issued(i.id);
      if (issued <= 0) throw new Error('no claims outstanding');
      let assets = 0;
      for (const h of reads.holdingsOf(BOOK)) {
        if (h.instrument === i.id && !countsItsOwn) continue;
        const worth = reads.worthOf(BOOK, h.instrument, at);
        if (worth.some) assets += worth.value.value;
      }
      return assets / issued;
    },
  };
}

function aBookAndItsClaims(claims: number, countsItsOwn = false): SystemModule {
  return {
    id: 'test.book',
    spec: 'Fund Shares B1',
    requires: ['seed.foundation'],
    instrumentKinds: [claimKind(countsItsOwn)],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: CLAIMS, name: 'claims', perUnit: 1 }],
    params: [],
    phases: [],
    participants: [],
    families: [],
    seed(ctx) {
      ctx.instruments.add({
        id: CLAIM,
        kind: CLAIM_KIND,
        issuer: some(BOOK),
        ccy: PHX,
        terms: { kind: CLAIM_KIND },
        market: none(),
      });
      ctx.endowMoney(BOOK, PHX, 1000);
      ctx.endowUnits(HOLDER, CLAIM, claims / 2, 1);
      ctx.endowUnits(OTHER, CLAIM, claims / 2, 1);
    },
  };
}

/** A kind that says its value is derived and derives nothing. */
function derivesNothing(): SystemModule {
  const kind: Record<string, unknown> = { ...claimKind() };
  delete kind['derive'];
  return {
    ...aBookAndItsClaims(100),
    instrumentKinds: [kind as unknown as InstrumentKindProfile],
  };
}

/** A book that holds its own claim and counts it: its value depends on itself (F2). */
function holdsItsOwnClaim(): SystemModule {
  const base = aBookAndItsClaims(100, true);
  return {
    ...base,
    seed(ctx) {
      base.seed?.(ctx);
      ctx.endowUnits(BOOK, CLAIM, 10, 1);
    },
  };
}

function world(...extra: readonly SystemModule[]): World {
  const spec = foundationSpec('derived');
  const kernel = spec.modules.filter((m) =>
    ['sovereign-instruments', 'seed.foundation', 'banks', 'money-market'].includes(m.id),
  );
  return assemble({ ...spec, modules: [...kernel, ...extra] });
}

describe('a value that is derived (XI-6, Fund Shares B1)', () => {
  it('is the book divided by the claims, read at the ask and stored nowhere', () => {
    const w = world(aBookAndItsClaims(100));
    expect(unexpected(w.step().audit)).toEqual([]);
    // The book holds money and nothing else, so a claim is worth what it holds over how many
    // claims there are — the same money mark everyone else is valued at, not a second price system.
    const book = w.cash(BOOK, PHX);
    expect(book).toBeGreaterThan(0);
    expect(w.valuation.markPerUnit(CLAIM, w.period)).toBeCloseTo(book / 100, 9);
    expect(w.register.quantity(HOLDER, CLAIM)).toBe(50);
    // B1: nothing stored it. The price store has no print for a thing no market cleared (Law 3).
    expect(w.prices.latest(CLAIM, w.period).some).toBe(false);
  });

  it('moves when the book moves, and the holders carry the move (Clearing D4, Audit B5)', () => {
    const w = world(aBookAndItsClaims(100));
    const before = w.register.equity(HOLDER);
    w.step();
    // The claim was endowed at a basis of 1, so the first revaluation moves each holder's equity
    // by what its own claims turned out to be worth — and the book's by the mirror of both, because
    // a claim is its issuer's liability.
    const mark = w.valuation.markPerUnit(CLAIM, w.period);
    expect(w.register.equity(HOLDER) - before).toBeCloseTo(50 * (mark - 1), 6);
    expect(unexpected(w.step().audit)).toEqual([]);
  });

  it('refuses a kind that says its value is derived and derives nothing (XI-6)', () => {
    expect(() => world(derivesNothing())).toThrow(/derived and derives nothing/);
  });

  it('refuses to value a book that holds its own claim (F2)', () => {
    const w = world(holdsItsOwnClaim());
    expect(() => w.step()).toThrow(/depends on itself/);
  });
});
