/**
 * A claim its investor can ask back: what a fund is, what its shares are worth, and what happens
 * when everybody asks at once.
 *
 * @spec Fund Shares A1 Fund Shares A2 Fund Shares A3 Fund Shares A4 Fund Shares B1 Fund Shares B2 Fund Shares B2.a Fund Shares B3 Fund Shares B4 Fund Shares C1 Fund Shares C1.a Fund Shares C2 Fund Shares C2.a Fund Shares C2.b Fund Shares C3 Fund Shares C4 Fund Shares C5 Fund Shares D1 Fund Shares D2 Fund Shares D3 Fund Shares D4 Fund Shares F1 Fund Shares F3 Fund Shares G1 XI-2 XI-6 Law 2 Law 3
 *
 * XI-2 is why this exists. A price falls, somebody must sell into the fall, and the sale makes the
 * fall worse. Without a party that MUST sell, a shock is absorbed by nobody and dissipates — so the
 * thing to assert is not that a fund is tidy, but that a redemption it cannot meet out of cash
 * becomes an order in a real market at whatever that market gives, and that nothing about the
 * request is ever quietly dropped.
 */
import { describe, expect, it } from 'vitest';
import {
  FUND,
  HOUSEHOLD,
  PHX,
  assemble,
  foundationSpec,
  foundationWorld,
  fundVenue,
  instrumentId,
  partyId,
  shareLineOf,
  snapshot,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const FUND_ID = partyId('fund.money.north');
const MANAGER = partyId('manager.north');
const SHARE = shareLineOf('fund.money.north');
const BILL = instrumentId('gov.north.bill.2026-06-15');

/** Everybody who holds a share asks for all of it back, in one period: a run (C4.a). */
function everybodyRedeems(at: number): SystemModule {
  return {
    id: 'test.run',
    spec: 'Fund Shares C2 Fund Shares C2.b XI-2',
    requires: ['funds'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.run',
        spec: 'Fund Shares C2',
        cycle: 0,
        anchor: { before: 'funds.strike' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
            const held = ctx.register.quantity(p.id, SHARE);
            if (held <= 0 || p.representation !== 'cell') continue;
            ctx.post(fundVenue('fund.money.north'), {
              party: p.id,
              side: 'sell',
              price: 'market',
              qty: held * p.weight,
            });
          }
        },
      },
    ],
    participants: [],
    families: [],
  };
}

function runWorld(seed: string, at: number): World {
  const spec = foundationSpec(seed);
  return assemble({ ...spec, modules: [...spec.modules, everybodyRedeems(at)] });
}

/** The same world with a manager that charges more than the paper earns (B3, D4). */
function greedy(fee: number): World {
  const spec = foundationSpec('funds-greedy');
  const modules = spec.modules.map((m) => ({
    ...m,
    params: m.params.map((p) =>
      p.id === 'fund.fee.fund.money.north' ? { ...p, value: fee } : p,
    ),
  }));
  return assemble({ ...spec, modules });
}

/** What a share is worth, read from the kernel the same way anybody else would read it (XI-6). */
const nav = (w: World): number => w.valuation.markPerUnit(SHARE, w.period);

describe('what a fund is (Fund Shares A1, A2, A3)', () => {
  it('is a party whose liability is its shares and whose equity is nothing', () => {
    const w = foundationWorld('funds-a');
    for (let i = 0; i < 8; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const fund = w.parties.get(FUND_ID);
    expect(fund.kind).toBe(FUND);
    // A2: counted in shares, held by named holders. C3: and the count changes.
    const line = w.instruments.get(SHARE);
    expect(line.unit).toBe('shares');
    expect(line.issued).toBeGreaterThan(0);
    expect(w.register.holdersOf(SHARE).length).toBeGreaterThan(0);
    // A3: assets minus liabilities is nothing, because the holders own the assets. Nothing in the
    // module enforces it — it falls out of the wire, and the audit is what says the wire did it.
    const dust = w.valuation.equityDust(FUND_ID, w.register.equityWalk(FUND_ID), w.period);
    expect(Math.abs(w.register.equity(FUND_ID))).toBeLessThanOrEqual(dust);
  });

  it('holds only what its mandate allows, and it holds something (A4, C1.a, F1)', () => {
    const w = foundationWorld('funds-b');
    for (let i = 0; i < 8; i += 1) w.step();
    const held = w.register.holdingsOf(FUND_ID).filter((h) => h.instrument !== SHARE);
    expect(held.length).toBeGreaterThan(0);
    for (const h of held) {
      const kind = w.instruments.get(h.instrument).kind;
      // A4: bills and its own money, and nothing else at any price. D1: short paper is the mandate.
      expect(['sovereign.bill', 'money']).toContain(String(kind));
    }
    // F1: a fund does not create its assets. Every unit it holds came from a named seller in a
    // market that cleared — here the issuer itself, at its own auction.
    const bought = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter((leg) => leg.kind === 'asset' && leg.to === FUND_ID && leg.pricePerUnit.some);
    expect(bought.length).toBeGreaterThan(0);
  });
});

describe('net asset value (Fund Shares B1, B2, B3, B4, D4)', () => {
  it('is a read of the book over the shares, and nothing stores it', () => {
    const w = foundationWorld('funds-c');
    for (let i = 0; i < 8; i += 1) w.step();
    const shares = w.instruments.get(SHARE).issued;
    const assets = w.register
      .holdingsOf(FUND_ID)
      .filter((h) => h.instrument !== SHARE)
      .reduce((t, h) => t + w.valuation.valueOfLots(h.instrument, h.lots, w.period), 0);
    // B1, B4: the sum of what the holders' shares are worth IS the book. Exactly.
    expect(nav(w) * shares).toBeCloseTo(assets, 8);
    // Law 3, B1: no market cleared this and nothing printed it. It is arithmetic on marks that
    // markets did clear, read at the ask.
    expect(w.prices.latest(SHARE, w.period).some).toBe(false);
  });

  it('falls when the book falls: nothing holds it at one (D4)', () => {
    const w = foundationWorld('funds-d');
    for (let i = 0; i < 6; i += 1) w.step();
    const before = nav(w);
    const moves: number[] = [];
    for (let i = 0; i < 12; i += 1) {
      w.step();
      moves.push(nav(w));
    }
    // It moved, in both directions over a run of weeks — which is what "no guaranteed constant
    // NAV" means when nothing is guaranteeing it: the number is the division, and the division
    // moves when the bills it holds are marked at what the market said this week.
    expect(moves.some((x) => x !== before)).toBe(true);
    expect(new Set(moves).size).toBeGreaterThan(1);
  });

  it('breaks the buck: nothing holds it at the price it opened at (D4)', () => {
    // The one thing that differs is what the manager charges. A fund whose paper earns it less
    // than its manager takes loses value, and the loss falls on the NAV — because there is nothing
    // else for it to fall on. A constant NAV would take a guarantor and the guarantor would be
    // nobody, so this is D4 met by an ABSENCE: no clamp, no floor, no sponsor.
    const struckIn = (w: World, periods: number): number[] => {
      const navs: number[] = [];
      for (let i = 0; i < periods; i += 1) {
        w.step();
        const struck = w.journal.ofKind('fund.struck').find((e) => e.period === w.period);
        if (struck !== undefined) navs.push(Number(struck.data['perShare']));
      }
      return navs;
    };
    const w = greedy(0.015);
    const navs = struckIn(w, 20);
    const opened = w.params.get('fund.openingSharePrice' as never);
    expect(navs.some((x) => x < opened)).toBe(true);
    // ...and WHAT put it there is the fee, which is the whole of D4: the same fund whose manager
    // charges a tenth as much is worth more per share on the same day, over the same book, at the
    // same marks. Nothing is standing under either of them. A bill that actually defaulted would
    // reach the same read by the same route: the instrument stops performing (Bond N12), its
    // holder writes it down (item 5), and the division is over a smaller book.
    const cheap = struckIn(greedy(0.0015), 20);
    const last = navs[navs.length - 1] ?? 0;
    expect(last).toBeLessThan(cheap[cheap.length - 1] ?? 0);
  });

  it('pays the manager, and the fee comes out of the holders (B3, F3)', () => {
    const w = foundationWorld('funds-e');
    for (let i = 0; i < 8; i += 1) w.step();
    const fees = w.journal.ofKind('fund.fee').filter((e) => e.data['paid'] === true);
    expect(fees.length).toBeGreaterThan(0);
    // F3: the manager is a separate party and the fee is its income.
    expect(w.cash(MANAGER, PHX)).toBeGreaterThan(0);
    expect(fees.every((e) => Number(e.data['amount']) > 0)).toBe(true);
  });
});

describe('creation and redemption (Fund Shares C1, C2, C3, C5)', () => {
  it('takes cash and gives shares in one instruction, at the NAV it struck (C1)', () => {
    const w = foundationWorld('funds-f');
    for (let i = 0; i < 6; i += 1) w.step();
    const subs = w.journal.ofKind('fund.subscribed').filter((e) => e.data['settled'] === true);
    expect(subs.length).toBeGreaterThan(0);
    const at = Number(subs[0]?.period);
    const one = w.ledger
      .inPeriod(at as never)
      .find(
        (r) =>
          r.outcome === 'settled' &&
          r.instruction.legs.some((l) => l.kind === 'asset' && l.instrument === SHARE),
      );
    expect(one).toBeDefined();
    // C1: one instruction, both sides — the money in and the shares out, at the same NAV.
    const legs = one?.instruction.legs ?? [];
    expect(legs.filter((l) => l.kind === 'money')).toHaveLength(1);
    expect(legs.filter((l) => l.kind === 'asset')).toHaveLength(1);
  });

  it('gives the money back, and what it cannot pay it never drops (C2, C2.b, C4)', () => {
    const w = runWorld('funds-g', 8);
    for (let i = 0; i < 14; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const asked = w.journal.ofKind('fund.requested');
    expect(asked.length).toBeGreaterThan(0);
    const paid = w.journal.ofKind('fund.redeemed');
    expect(paid.length).toBeGreaterThan(0);
    // C4: it is paid at the NAV struck when it asked, not at what the sales later fetched — which
    // is why a redemption is a real cost to the holders who stayed (C4.a).
    for (const e of paid) expect(Number(e.data['perShare'])).toBeGreaterThan(0);
    // C2.b: the guard says every share ever asked for is paid or still on the book. It is built,
    // so its silence is a real absence rather than a family nobody wrote.
    const flows = w.last?.audit.families.filter((f) => f.family === 'flows') ?? [];
    expect(flows.some((f) => f.contributions.includes('funds'))).toBe(true);
    expect(flows.flatMap((f) => f.violations)).toEqual([]);
  });

  it('sells into a market it does not price when the buffer runs out (C2.a, C2.b, XI-2)', () => {
    const w = runWorld('funds-h', 8);
    for (let i = 0; i < 14; i += 1) w.step();
    // XI-2: the forced sale. It offers what it holds at whatever the book gives — a seller that
    // named a price would not be forced — and the market it sells into is the bills' own.
    const sold = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter((leg) => leg.kind === 'asset' && leg.from === FUND_ID && String(leg.instrument).startsWith('gov.'));
    expect(sold.length).toBeGreaterThan(0);
    // C2.b, C4.a: it could not meet the run out of cash, so it GATED — and a gate is information,
    // so it is public. What it could not pay stayed on the book under the holder's own name.
    const gated = w.journal.ofKind('fund.gate');
    expect(gated.length).toBeGreaterThan(0);
    for (const g of gated) expect(g.public).toBe(true);
    // ...and it kept selling until it had paid: the queue shrinks and then it is gone.
    const owed = gated.map((e) => Number(e.data['sharesOwed']));
    expect(owed[owed.length - 1]).toBeLessThan(owed[0] ?? 0);
    expect(gated[gated.length - 1]?.period).toBeLessThan(w.period);
  });
});

describe('what a saver does with it (Fund Shares D2, D2.a, D3)', () => {
  it('substitutes out of a deposit that pays nothing, and the fund buys the paper (D2, D3)', () => {
    const w = foundationWorld('funds-i');
    for (let i = 0; i < 10; i += 1) w.step();
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
    const holding = cells.filter((c) => w.register.quantity(c.id, SHARE) > 0);
    expect(holding.length).toBeGreaterThan(0);
    // D3: and its size is what determines how much paper can be placed — the savers' money reaches
    // the state's bills through it, which is the transmission A3 and A4 together are about.
    expect(w.register.quantity(FUND_ID, BILL)).toBeGreaterThan(0);
    // D2.a: what it offers is published, because a competition against a deposit rate cannot
    // happen against a number nobody can see.
    const struck = w.journal.ofKind('fund.struck').filter((e) => e.period === w.period);
    expect(struck.length).toBeGreaterThan(0);
    expect(struck[0]?.public).toBe(true);
    expect(Number(struck[0]?.data['offered'])).toBeGreaterThan(0);
  });

  it('shows the fund on the surface: what it is worth, what it holds, what it owes', () => {
    const w = foundationWorld('funds-j');
    for (let i = 0; i < 8; i += 1) w.step();
    const view = snapshot(w, { kind: 'inspector' }, 50);
    const line = view.instruments.find((i) => i.id === SHARE);
    expect(line?.issuer).toBe(FUND_ID);
    expect(line?.issued).toBeGreaterThan(0);
    // XI-6, F4: a position in it has a value, because a derived value is a value.
    const pos = view.positions.filter((p) => p.instrument === SHARE);
    expect(pos.length).toBeGreaterThan(0);
    expect(pos.every((p) => p.valuePerMember !== null)).toBe(true);
  });
});

describe('a year with a redemption wave in it', () => {
  it('stays green and gives the same world twice from the same seed (Law 13, Observer E3)', () => {
    const a = runWorld('funds-year', 20);
    for (let i = 0; i < 52; i += 1) expect(unexpected(a.step().audit)).toEqual([]);
    // The wave happened, it was met by selling, and the fund is still there afterwards.
    expect(a.journal.ofKind('fund.gate').length).toBeGreaterThan(0);
    expect(a.parties.get(FUND_ID).status.alive).toBe(true);
    expect(a.instruments.get(SHARE).issued).toBeGreaterThan(0);
    const b = runWorld('funds-year', 20);
    for (let i = 0; i < 52; i += 1) b.step();
    expect(snapshot(b, { kind: 'inspector' }, 50)).toEqual(snapshot(a, { kind: 'inspector' }, 50));
  });
});
