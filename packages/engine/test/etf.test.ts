/**
 * A fund whose shares TRADE: two values for one claim, and what somebody has to want for the gap
 * between them to close.
 *
 * @spec Fund Shares E1 Fund Shares E2 Fund Shares E3 Fund Shares E3.a Fund Shares E4 Fund Shares G1.a Fund Shares B1 Fund Shares B3 Fund Shares F3 Dealer Desks E3 Clearing E4 XI-2 XI-5 XI-6 Law 3 Law 5 Law 6
 *
 * E2 is what this file is for. Every other claim in this world has ONE value: what the market said,
 * or what the book says. An exchange-traded fund has both at once and they are different numbers,
 * and nothing anywhere ties them together — so the thing to assert is not that they agree, but that
 * a party who could turn one into the other does it when it is worth its while (E3, G1.a), does
 * NOT when it is not, and that a gap nobody will close simply stays open and is reported (E3.a, E4).
 */
import { describe, expect, it } from 'vitest';
import {
  USD,
  assemble,
  etfMarketOf,
  etfVenue,
  funds,
  instrumentId,
  partyId,
  shareLineOf,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { rigDraw, rigSpec, mergeModules, rigShapeFor } from './rig.js';
import { unexpected } from './expected.js';
import { asQty, type Qty } from '../src/core/tick.js';

const FUND = partyId('etf.us');
const MANAGER = partyId('manager.etf.us');
const SHARE = shareLineOf('etf.us');
const MARKET = etfMarketOf('etf.us');
const VENUE = etfVenue('etf.us');
const LINE_4 = instrumentId('equity.firm.4');

/**
 * A world whose households have somewhere to put their savings, which is what puts a third party in
 * the fund's book at all. A cushion of nothing is not a claim about households: it is the smallest
 * world in which the fund's own market has a side that is not a bank's dealing line (§46 A3).
 */
function saversWorld(seed: string, extra: readonly SystemModule[] = []): World {
  // Seed B1.a: a world with an exchange-traded fund in it, ASKED FOR rather than assumed. Which
  // firms list is a draw, and a world that listed none launches none — so a test that took the
  // default rig would be measuring a fund that is not there.
  const shape = rigShapeFor(seed, { etfs: 1 });
  const spec = rigSpec(seed, shape.banks, shape.firms);
  const modules = spec.modules.map((m) =>
    m.id === 'households'
      ? { ...m, params: m.params.map((p) => (p.id === 'households.buffer.periods' ? { ...p, value: 0 } : p)) }
      : m,
  );
  return assemble({ ...spec, modules: mergeModules(modules, extra) });
}

/** Somebody who brings a basket in or gives one back, at a period the test chooses (G1.a). */
function bringsABasket(party: string, side: 'buy' | 'sell', shares: Qty, at: number): SystemModule {
  return {
    id: 'test.creation',
    spec: 'Fund Shares E3 Fund Shares G1.a',
    requires: ['funds', 'banks'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.creation',
        spec: 'Fund Shares E3',
        cycle: 0,
        anchor: { before: 'funds.etf' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          ctx.post(VENUE, { party: partyId(party), side, price: 'market', qty: shares });
        },
      },
    ],
    participants: [],
    families: [],
    seed: () => undefined,
  };
}

describe('two values for one claim (Fund Shares E1, E2)', () => {
  it('has a market that clears and a book that is read, and they are not the same number', () => {
    const w = saversWorld('etf-two');
    for (let i = 0; i < 10; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // E1: its shares trade. A session in them printed a price out of a queue of orders, which is
    // the whole of what makes this an exchange-traded fund rather than another fund.
    const traded = w.journal
      .ofKind('etf.struck')
      .filter((e) => e.data['stale'] === false && typeof e.data['price'] === 'number');
    expect(traded.length).toBeGreaterThan(0);
    // E2: and the same event carries what the book comes to per share, read off the fund's own
    // holdings (B1). Neither is the other's approximation and neither is derived from the other.
    const last = traded[traded.length - 1];
    const nav = Number(last?.data['perShare']);
    const price = Number(last?.data['price']);
    expect(nav).toBeGreaterThan(0);
    expect(price).toBeGreaterThan(0);
    expect(price).not.toBe(nav);
    // E4: the premium is a READ of the two, published beside them, and it is exactly their ratio.
    expect(Number(last?.data['premium'])).toBeCloseTo((price - nav) / nav, 12);
  });

  it('reads its book over the claims on it, at every ask (B1, XI-6)', () => {
    const w = saversWorld('etf-nav');
    for (let i = 0; i < 8; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const struck = w.journal.ofKind('etf.struck').filter((e) => e.period === w.period);
    const nav = Number(struck[0]?.data['perShare']);
    const issued = w.instruments.get(SHARE).issued;
    // B1, A3: what the fund holds is what its shares are claims on, so the two sides of its own
    // balance sheet are the same number and its equity is zero. A residual here would be money
    // with no holder (Law 2).
    let held = 0;
    for (const h of w.register.holdingsOf(FUND)) {
      if (h.instrument === SHARE) continue;
      held += w.valuation.valueAtMark(h.instrument, w.register.quantity(FUND, h.instrument), w.period);
    }
    expect(nav * issued).toBeCloseTo(held, 6);
  });
});

describe('in kind (Fund Shares E3, G1.a, XI-2)', () => {
  it('takes a slice of its own book and gives shares against it, in one instruction', () => {
    const w = saversWorld('etf-create', [bringsABasket('bank.a', 'buy', asQty(10), 3)]);
    const before = w.instruments.get(SHARE).issued;
    const basketBefore = w.register.quantity(FUND, LINE_4) / before;
    for (let i = 0; i < 3; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const made = w.journal.ofKind('etf.created').filter((e) => e.period === w.period);
    expect(made.length).toBeGreaterThan(0);
    expect(made.every((e) => e.data['settled'] === true)).toBe(true);
    expect(w.instruments.get(SHARE).issued).toBeGreaterThan(before);
    // E3: a creation cannot change what the fund IS — it took a pro-rata slice of its own book, so
    // one share is a claim on exactly what one share was a claim on before (Law 19: read the book).
    const after = w.instruments.get(SHARE).issued;
    expect(w.register.quantity(FUND, LINE_4) / after).toBeCloseTo(basketBefore, 9);
    // G1.a: no cash moved and no market was touched. Every leg of it was units of something.
    const legs = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.instruction.reason.includes('creates') && r.instruction.reason.includes(String(SHARE)))
      .flatMap((r) => r.instruction.legs);
    expect(legs.length).toBeGreaterThan(0);
    expect(legs.every((l) => l.kind === 'asset')).toBe(true);
  });

  it('gives the slice back and takes the shares, and sells nothing to do it (XI-2)', () => {
    const w = saversWorld('etf-redeem', [bringsABasket('bank.a', 'sell', asQty(5), 4)]);
    for (let i = 0; i < 4; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const back = w.journal.ofKind('etf.redeemed').filter((e) => e.period === w.period);
    expect(back.length).toBeGreaterThan(0);
    expect(back.every((e) => e.data['settled'] === true)).toBe(true);
    // XI-2: this is why an exchange-traded fund is NOT the forced seller. The fund posted nothing
    // into any market to meet it — the money fund is the vehicle that has to (Fund Shares C2.b).
    const sold = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.instruction.cause === 'trade')
      .flatMap((r) => r.instruction.legs)
      .filter((l) => l.kind === 'asset' && l.from === FUND);
    expect(sold).toEqual([]);
  });
});

describe('the gap, and what it takes to close it (E3.a, E4)', () => {
  it('is closed by a bank when it is worth more than carrying it costs, and only then', () => {
    const w = saversWorld('etf-arb');
    for (let i = 0; i < 26; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const acted = w.journal.ofKind('bank.arbitrage');
    expect(acted.length).toBeGreaterThan(0);
    for (const e of acted) {
      const gap = Number(e.data['premium']);
      // E3.a: the reason is the bank's own, and it is arithmetic it can state — the difference
      // against what a period of carrying the position costs it at its own rate (Dealer Desks D3).
      expect(Math.abs(gap)).toBeGreaterThan(Number(e.data['worth']));
      expect(e.data['side']).toBe(gap > 0 ? 'create' : 'redeem');
      expect(Number(e.data['shares'])).toBeGreaterThan(0);
    }
    // Both directions are reachable: dear, it delivers the basket; cheap, it delivers the shares.
    expect(new Set(acted.map((e) => e.data['side']))).toEqual(new Set(['create', 'redeem']));
  });

  it('never acts on a mark nobody traded at (Clearing E4, Law 3)', () => {
    const w = saversWorld('etf-stale');
    const printedIn = new Set<number>();
    for (let i = 0; i < 20; i += 1) {
      const r = w.step();
      expect(unexpected(r.audit)).toEqual([]);
      const m = r.markets.find((x) => x.market === MARKET);
      if (m?.outcome === 'cleared') printedIn.add(r.period);
    }
    // A price carried forward because nobody traded is not a level it could sell into: a
    // creation against one would be a derivative on an uncleared price, and the gap it closed
    // would be the carry's own artefact. So every act of arbitrage follows a session that traded.
    const acted = w.journal.ofKind('bank.arbitrage');
    expect(acted.length).toBeGreaterThan(0);
    for (const e of acted) expect(printedIn.has(e.period - 1)).toBe(true);
  });

  it('stays open when nobody will close it, and the read says how big it is (E3.a, E4)', () => {
    // NOBODY IS AN AUTHORISED PARTICIPANT here. A fund's shares are created and given back by the
    // banks that make a market in them (G1.a), and this fund has none — a real state, and the only
    // one in which E4's "the gap can simply persist" is a claim about the model rather than about
    // how long the test happened to run. Everything else in the world is untouched: the fund still
    // strikes, the market still trades, and the two numbers still differ.
    const spec = rigSpec('etf-persist');
    const noParticipant = spec.modules.map((m) =>
      m.id === 'banks' ? { ...m, phases: m.phases.filter((p) => p.name !== 'banks.arbitrage') } : m,
    );
    const w = assemble({
      ...spec,
      modules: noParticipant.map((m) =>
        m.id === 'households'
          ? { ...m, params: m.params.map((p) => (p.id === 'households.buffer.periods' ? { ...p, value: 0 } : p)) }
          : m,
      ),
    });
    for (let i = 0; i < 30; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const struck = w.journal.ofKind('etf.struck');
    const last = struck[struck.length - 1];
    const premium = Number(last?.data['premium']);
    // The gap is bigger than what closing it would earn — which is the module's OWN condition for
    // acting (E3.a: a period of carry) and so is the size that says the gap is open rather than a
    // number this test chose — and nobody acted on it.
    const books = w.journal.ofKind('bank.dealing');
    const carry = Number(books[books.length - 1]?.data['ratePerPeriod']);
    expect(carry).toBeGreaterThan(0);
    expect(Math.abs(premium)).toBeGreaterThan(carry);
    expect(w.journal.ofKind('bank.arbitrage')).toEqual([]);
    // E4, Law 6: it is a finding about liquidity and never a number to adjust. Nothing moved it,
    // and the audit — which is where a finding belongs — did not report it as a violation.
    expect(unexpected(w.step().audit)).toEqual([]);
    // Clearing E4: and the half of it that is a price says out loud that it is a carried mark.
    expect(last?.data['stale']).toBe(true);
  });
});

describe('what the manager takes (B3, F3)', () => {
  it('is a real payment out of the fund, and the book is smaller for it', () => {
    const w = saversWorld('etf-fee');
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const accrued = w.journal.ofKind('fund.fee').filter((e) => e.data['fund'] === String(FUND));
    const fees = accrued.filter((e) => e.data['paid'] === true);
    expect(fees.length).toBeGreaterThan(0);
    // F3: the manager is a separate party and the fee is its income, so it has the money.
    expect(w.cash(MANAGER, USD)).toBeGreaterThan(0);
    // B3, Law 5: what left the fund left it. Every fee is a numbered instruction with two sides,
    // out of the fund's own account and into the manager's — never a subtraction from a number.
    const instructions = w.ledger
      .all()
      .filter((r) => r.instruction.reason === `${FUND} pays its manager`);
    // Appendix B, one payment convention: EVERY accrual goes to the wire, for the whole of what is
    // owed. A fund short of the money does not pay a smaller fee — the instruction is refused and
    // the refusal is the record. This fund is short in one period of the six and pays in the other
    // five, which is a fund that ran out of cash on the day and not a fund that cannot pay at all.
    expect(instructions.length).toBe(accrued.length);
    expect(instructions.filter((r) => r.outcome !== 'settled').length).toBe(
      accrued.length - fees.length,
    );
    // And nothing half-settled: what the manager received is the sum of the fees that were paid,
    // with no part-payment standing in for the one that was not (Money E1, Law 6).
    const legs = instructions
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs);
    const total = fees.reduce((t, e) => t + Number(e.data['amount']), 0);
    expect(legs.reduce((t, l) => t + (l.kind === 'money' ? l.amount : 0), 0)).toBeCloseTo(total, 12);
    expect(legs.every((l) => l.kind === 'money' && l.from.holder === FUND && l.to.holder === MANAGER)).toBe(true);
  });
});

describe('a world whose launch nobody joined (Seed A3, Law 15)', () => {
  it('is a smaller fund and not a broken one: the basket backs the shares that were taken', () => {
    // Only the sponsor turned up — the desks that would make its market are not in this world, so
    // what they were down to take is not what anybody took.
    const drew = rigDraw('etf-small');
    const sponsorOnly = drew.etfs.map((e) => ({ ...e, launchedBy: { 'manager.etf.us': 20 } }));
    const spec = rigSpec('etf-small');
    const w = assemble({
      ...spec,
      modules: spec.modules.map((m) => (m.id === 'funds' ? funds(drew.funds, sponsorOnly) : m)),
    });
    for (let i = 0; i < 4; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // A basket backing shares nobody holds would be a NAV that was a multiple of what the fund
    // owed, and a fund with more assets than claims on them is money with no holder (Law 2).
    expect(w.instruments.get(SHARE).issued).toBe(20);
    expect(w.register.quantity(FUND, LINE_4)).toBeCloseTo(20, 9);
    const struck = w.journal.ofKind('etf.struck').filter((e) => e.period === w.period);
    expect(Number(struck[0]?.data['perShare'])).toBeGreaterThan(0);
  });
});
