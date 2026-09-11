/**
 * A claim that promises nothing: what a share is, what it is worth to whoever holds it, and what
 * the firm does with it.
 *
 * @spec Equity A1 Equity A1.a Equity A1.b Equity A2 Equity A2.a Equity A3 Equity A4 Equity A5 Equity A6 Equity B1 Equity B2 Equity B3 Equity B4 Equity B4.a Equity B6 Equity C1 Equity C1.a Equity C1.b Equity C3 Equity C4 Equity D1 Equity D1.a Equity D1.c Equity D2 Equity D2.a Equity D2.b Equity D3 Equity D3.a Equity D4 Equity E4 Equity F1 Equity F3 Equity F4 Equity G3 Register E4 Register E5 XI-13 Law 8
 *
 * B3 is what these tests are for. A price that came out of an earnings multiple is the opinion
 * restated, so what has to be true is that the number in the book is one PARTICIPANT'S reservation,
 * that two participants with different histories bring different ones, and that what prints is
 * neither of them but what the session made of both.
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  HOUSEHOLD,
  REGION,
  OPENING_SHARE,
  PHX,
  SHARE,
  assemble,
  equityLineOf,
  equityMarketOf,
  goodId,
  instrumentId,
  isShare,
  moneyInstrumentId,
  partyId,
  shareTerms,
  snapshot,
  votesOf,
  type MarketDecl,
  type MechanismContext,
  type Order,
  type ParticipantView,
  type SystemModule,
  type World,
} from '../src/index.js';
import { listedIn, rigFor, rigSpec, rigWorld } from './rig.js';
import { unexpected } from './expected.js';
import { perTonne, phx, tonnes } from './units.js';

/**
 * Seed B1.a, B4: WHICH FIRMS THIS WORLD LISTED IS DRAWN. A listing is what a firm large enough to
 * outlive its owner gets (`equity.LISTING_SIZE`), so the test asks for a world with two of them and
 * then asks which two they are — it never writes `firm.4` down, because a world of three thousand
 * firms lists two hundred and a world of twelve may list none.
 */
const DREW = rigFor('equity', { listed: 2 });
const RIG = { banks: 3, firms: DREW.firms };
const BIG = listedIn(DREW.draw, 0);
const SMALL = listedIn(DREW.draw, 1);
const FIRM_6 = partyId(SMALL);
const LINE_4 = equityLineOf(BIG);
const LINE_6 = equityLineOf(SMALL);
const BANK_A = partyId('bank.a');
/** A firm this world did NOT list: a firm nobody has bought a share of is a real state. */
const UNLISTED = DREW.draw.firms.find((f) => !DREW.draw.listed.some((l) => l.firm === f.firm))?.firm ?? 'firm.1';

/** A test-only paymaster: a bank creates money into named cells, so a saver has something to save. */
function pays(
  rows: readonly { readonly to: string; readonly amount: number }[],
  at: number,
  name = 'test.pays',
): SystemModule {
  return {
    id: name,
    spec: 'Money C4',
    requires: ['households'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name,
        spec: 'Money C4',
        cycle: 0,
        anchor: { before: 'households.decide' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          for (const row of rows) {
            const holder = partyId(row.to);
            const party = ctx.parties.get(holder);
            if (party.representation !== 'cell') continue;
            ctx.settle({
              legs: [
                {
                  kind: 'money',
                  from: { holder: BANK_A, issuer: BANK_A },
                  to: { holder, issuer: party.bank },
                  ccy: PHX,
                  amount: row.amount * party.weight,
                  fromCell: { some: false },
                  toCell: { some: true, value: { perMember: row.amount, weight: party.weight } },
                },
              ],
              cause: 'transfer',
              reason: `the test pays ${holder}`,
            });
          }
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/** A test-only corporate action: the firm splits its line, through the kernel's own door. */
function splits(line: string, ratio: number, at: number): SystemModule {
  return {
    id: 'test.split',
    spec: 'Equity D4 Register E4',
    requires: ['equity'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.split',
        spec: 'Equity D4',
        cycle: 0,
        anchor: { after: 'equity.decide' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          // Register E5, D4: the invariance is about the SPLIT, so it is measured ACROSS the split
          // and not across the week the split was in. A week has other named events in it — the
          // issuer buys some of itself back, the session prints a new level — and a measurement
          // that spans them is measuring those instead (Law 19: read it where it happens).
          const id = instrumentId(line);
          const worth = (): Record<string, number> => {
            const out: Record<string, number> = {};
            for (const h of ctx.register.holdersOf(id)) {
              const v = ctx.valuation.worthOf(h, id, ctx.period);
              out[String(h)] = v.some ? v.value.value : 0;
            }
            return out;
          };
          const was = worth();
          ctx.split(id, ratio);
          ctx.record('test.split.across', [id], { was, now: worth() }, false);
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/**
 * Goods C3, XI-3: a buyer of one good far bigger than the line that makes it. The firms in the line
 * below bid their own input up against each other until one of them is paying more for it than its
 * output fetches — which is a firm failing on its own decisions rather than on anything this test
 * did to its balance directly.
 */
function hungryFor(subUnit: string): SystemModule {
  const BUYER = partyId('buyer.1');
  const instrument = goodId(subUnit, REGION);
  return {
    id: 'test.buyer',
    spec: 'Goods C3',
    requires: ['goods', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    seed(ctx) {
      ctx.parties.add({
        id: BUYER,
        kind: FIRM,
        region: REGION,
        name: 'A buyer',
        bank: BANK_A,
        representation: 'named',
        status: { alive: true },
      });
      ctx.endowMoney(BUYER, PHX, phx(100_000_000));
      ctx.endowMoney(BANK_A, PHX, phx(100_000_000));
    },
    participants: [
      {
        partyKind: FIRM,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] =>
          view.self.id === BUYER && m.instrument === instrument
            ? [{ party: BUYER, side: 'buy', price: perTonne(1200), qty: tonnes(400) }]
            : [],
      },
    ],
    families: [],
  };
}

function worldWith(seed: string, extra: readonly SystemModule[]): World {
  const spec = rigSpec(seed, RIG.banks, RIG.firms);
  return assemble({ ...spec, modules: [...spec.modules, ...extra] });
}

/**
 * The same world with savers who keep NO cushion: what they do not spend goes to work the same
 * period (Households C1.d is a preference, and this is a world in which it is nothing). It is the
 * world in which a household's own schedule is visible at all, because a cell that keeps four
 * periods of its income in its account has nothing over it to put anywhere in the weeks these
 * tests run for.
 */
function saversWorld(seed: string, extra: readonly SystemModule[] = []): World {
  const spec = rigSpec(seed, RIG.banks, RIG.firms);
  const modules = spec.modules.map((m) =>
    m.id === 'households'
      ? {
          ...m,
          params: m.params.map((p) =>
            p.id === 'households.buffer.periods' ? { ...p, value: 0 } : p,
          ),
        }
      : m,
  );
  return assemble({ ...spec, modules: [...modules, ...extra] });
}

describe('what a share is (Equity A)', () => {
  it('is a residual claim and not a liability of the firm that issued it (A1, A1.a)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    const kind = w.registry.instrumentKind(SHARE);
    expect(kind.liabilityOfIssuer).toBe(false);
    // A1.a: it ranks below everything the issuer owes. Money is 0, an unsecured loan is 1.
    const line = w.instruments.get(LINE_4);
    expect(kind.ranking(line).seniority).toBeGreaterThan(
      w.registry.instrumentKind(w.instruments.get(moneyInstrumentId(BANK_A, PHX)).kind).ranking(
        w.instruments.get(moneyInstrumentId(BANK_A, PHX)),
      ).seniority,
    );
  });

  it('is perpetual and promises nothing dated, so nothing can discount it (A4, B3)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    const kind = w.registry.instrumentKind(SHARE);
    const line = w.instruments.get(LINE_4);
    expect(kind.due(line, w.period, w.calendar)).toHaveLength(0);
    expect(kind.cashFlows(line, w.calendar.startOf(w.period), w.calendar)).toHaveLength(0);
    expect(kind.accrued(line, w.calendar.startOf(w.period), w.calendar)).toBe(0);
    // A4: nothing falls due on it, so there is no promise to break and it cannot default (N12).
    expect(kind.defaultOn).toBeUndefined();
  });

  it('is named by its issuer and carries a vote per share (A5, A6, F3, XI-15)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    const line = w.instruments.get(LINE_4);
    expect(isShare(line.terms)).toBe(true);
    const terms = shareTerms(line);
    expect(terms.votesPerShare).toBeGreaterThan(0);
    expect(w.registry.instrumentKind(SHARE).displayName(line, {
      issuer: { some: true, value: 'Broadacre Farm' },
      region: () => 'North',
    })).toContain('Broadacre Farm');
    // F3, XI-15: a cell casts weight x member x votes, so representation disenfranchises nobody.
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    expect(cell).toBeDefined();
    if (cell?.representation === 'cell') {
      expect(votesOf(cell, 2, terms)).toBe(2 * cell.weight * terms.votesPerShare);
    }
  });

  it('opens at a level that is a resolution and not a shape (Seed C4, Law 2)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    const decl = w.params.decl(OPENING_SHARE);
    expect(decl.kind).toBe('resolution');
    // Law 2: the shapes this world declares are the four goods' opening prices, the opening yield
    // and one preference width, and equity added none of them. The two management fees used to be
    // counted here; they name the item that deletes them, so they are placeholders (pre12).
    expect(w.last?.audit.reads.shapes).toBe(6);
    expect(w.last?.audit.reads.placeholders).toBe(2);
  });
});

describe('the count (Equity A2.a, D4, Register E4, E5)', () => {
  it('changes only by a named event, and a split moves no value at all', () => {
    const w = worldWith('equity', [splits(String(LINE_4), 2, 4)]);
    for (let i = 0; i < 3; i += 1) w.step();
    const holders = w.register.holdersOf(LINE_4);
    const heldBefore = holders.map((h) => w.register.totalQuantity(h, LINE_4));
    const printBefore = w.prices.latest(LINE_4, w.period);

    const r = w.step();
    expect(unexpected(r.audit)).toEqual([]);
    // D4: the count doubles — READ OFF THE SPLIT'S OWN EVENT (Law 19), which says what the line was
    // and what it became at the moment it happened. Inferring it from the period's opening and
    // closing counts would be asking a question the period cannot answer: a split is one named
    // event among the several a listed firm has in a week, and a firm that also bought some of
    // itself back that week closes on a count that is not twice anything.
    const split = w.journal
      .ofKind('instrument.split')
      .filter((e) => e.subjects.includes(String(LINE_4)))
      .pop();
    expect(split).toBeDefined();
    expect(Number(split?.data['issued'])).toBe(Number(split?.data['issuedBefore']) * 2);
    /** Law 19: the units this holder moved in the line this period, read off the wire. */
    const traded = (holder: string): number =>
      w.ledger
        .inPeriod(w.period)
        .filter((x) => x.outcome === 'settled')
        .flatMap((x) => x.instruction.legs)
        .reduce((t, leg) => {
          if (leg.kind !== 'asset' || leg.instrument !== LINE_4) return t;
          if (leg.to === holder) return t + leg.qty;
          if (leg.from === holder) return t - leg.qty;
          return t;
        }, 0);
    holders.forEach((h, i) => {
      // A2.a: THE COUNT CHANGES ONLY BY A NAMED EVENT, and there are two of them in this week. The
      // split doubled what each holder held; anything else it holds, it traded for, and the trade
      // is on the wire under its own name. So what a holder ends the week with is exactly twice
      // what it started it with plus what it bought less what it sold, and nothing else.
      const now = w.register.totalQuantity(h, LINE_4);
      expect(now).toBeCloseTo((heldBefore[i] ?? 0) * 2 + traded(String(h)), 6);
    });
    // D4: and NOTHING else changes. What each holder's position is worth is what it was, and the
    // equity account did not move, because a split is not a payment (Register E5). It is measured
    // across the split itself: what the week's other named events did to a holder is theirs.
    const acrossAll = w.journal.ofKind('test.split.across');
    const across = acrossAll[acrossAll.length - 1];
    const was = across?.data['was'] as Record<string, number> | undefined;
    const now = across?.data['now'] as Record<string, number> | undefined;
    expect(was).toBeDefined();
    expect(now).toBeDefined();
    for (const [holder, value] of Object.entries(was ?? {})) {
      expect(now?.[holder]).toBeCloseTo(value, 6);
    }
    // ...and no instruction carried it: there was no second side for the wire to name (Money D1).
    // ...and it is the SPLIT that carried none. A dividend on the same line in the same week is a
    // different corporate action, it moves money, and it has both sides on the wire like anything
    // else — so what this asks is that nothing on the wire is a split.
    expect(
      w.ledger
        .inPeriod(w.period)
        .filter((x) => x.instruction.reason.toLowerCase().includes('split')),
    ).toHaveLength(0);
    // The price is per NEW share from now on, and the history is restated with it.
    const printAfter = w.prices.latest(LINE_4, w.period);
    expect(printAfter.some).toBe(true);
    if (printAfter.some && printBefore.some) {
      expect(printAfter.value.price).toBeCloseTo(printBefore.value.price / 2, 8);
    }
    // E5: an event that moved a register and moved no money, saying so out loud.
    const said = w.journal.ofKind('instrument.split').filter((e) => e.subjects.includes(LINE_4));
    expect(said).toHaveLength(1);
    expect(said[0]?.public).toBe(true);
    expect(String(said[0]?.data['movedNoMoney'])).toContain('moves no value');
  });

  it('refuses to restate a line whose kind does not split (Law 15)', () => {
    const w = worldWith('equity', [splits('gov.north.2036-03-15', 2, 2)]);
    w.step();
    expect(() => w.step()).toThrow(/not a kind whose count a split may change/);
  });
});

describe('what the firm does with it (Equity D)', () => {
  it('declares a dividend publicly and it reaches the accounts of whoever holds it (D3, D3.a)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const declared = w.journal.ofKind('payout.declared');
    expect(declared.length).toBeGreaterThan(0);
    const one = declared[0];
    expect(one?.public).toBe(true);
    expect(Number(one?.data['perShare'])).toBeGreaterThan(0);
    // D3.a: it left the firm and arrived at a named holder — the flows family checks it every
    // period, and every period above was green.
    expect(Number(one?.data['paid'])).toBeGreaterThan(0);
    expect(Number(one?.data['failedPayments'])).toBe(0);
  });

  it('sells new shares when it is short and the market is dear, and the count rises (D1, D1.a)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    const issued = new Map<string, number>();
    for (const row of DREW.draw.listed.map((l) => l.firm)) {
      issued.set(row, w.instruments.get(equityLineOf(row)).issued);
    }
    let sold = false;
    for (let i = 0; i < 30; i += 1) {
      expect(unexpected(w.step().audit)).toEqual([]);
      for (const e of w.journal.ofKind('equity.plan').filter((x) => x.period === w.period)) {
        if (Number(e.data['issue']) > 0) sold = true;
      }
    }
    expect(sold).toBe(true);
    // D1.a: whoever bought them holds them, and the count rose by exactly what settled.
    const line = instrumentId(String(
      w.journal.ofKind('equity.plan').find((e) => Number(e.data['issue']) > 0)?.data['line'],
    ));
    const held = w.register.heldTotal(line);
    expect(held.value).toBeCloseTo(w.instruments.get(line).issued, 6);
  });

  it('never sells below its own reservation: a failed issue is a real outcome (D1.c, Clearing C4.a)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    for (let i = 0; i < 30; i += 1) w.step();
    const auctions = w.journal
      .ofKind('auction.result')
      .filter((e) => String(e.data['line']).startsWith('equity.'));
    // Whatever it placed, it placed at or above what it said it would take, and what nobody bid
    // for was withdrawn rather than absorbed (Treasury D5.a's rule, applied to a share).
    for (const a of auctions) {
      const stop = a.data['stopOut'];
      const plan = w.journal
        .ofKind('equity.plan')
        .find((e) => e.period === a.period && e.data['line'] === a.data['line']);
      if (stop === null || plan === undefined) continue;
      expect(Number(stop)).toBeGreaterThanOrEqual(Number(plan.data['reservation']) - 1e-9);
      expect(Number(a.data['allotted'])).toBeLessThanOrEqual(Number(a.data['size']) + 1e-9);
    }
  });

  it('buys its own back only when the market is below its own book, and the cash is gone (D2, D2.b)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    for (let i = 0; i < 40; i += 1) w.step();
    for (const e of w.journal.ofKind('equity.plan')) {
      const buyback = Number(e.data['buyback']);
      const dividend = Number(e.data['dividendPerShare']);
      // D2.c: it does one or the other and the choice has a reason. Never both in one period.
      expect(buyback > 0 && dividend > 0).toBe(false);
      if (buyback <= 0) continue;
      const print = w.prices.latest(instrumentId(String(e.data['line'])), e.period);
      if (print.some) expect(print.value.price).toBeLessThanOrEqual(Number(e.data['bookPerShare']));
    }
  });
});

describe('what the holder gets (Equity F)', () => {
  it('gets no income from earnings that were not distributed (F4)', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    for (let i = 0; i < 8; i += 1) w.step();
    // F4: retained earnings reach a holder through the PRICE and never as income credited to it.
    // What reaches a holder's account is a settled money leg, so a period in which a firm retained
    // and paid nothing is a period in which nothing reached anybody on account of holding it.
    const dividends = w.journal.ofKind('payout.declared').filter((e) => e.period === w.period);
    const lines = new Set(dividends.map((e) => String(e.data['line'])));
    for (const row of DREW.draw.listed.map((l) => l.firm)) {
      const line = equityLineOf(row);
      if (lines.has(String(line))) continue;
      // It retained: nothing left it this period on account of that line at all.
      const paid = w.ledger
        .inPeriod(w.period)
        .filter((r) => r.outcome === 'settled' && r.instruction.reason.startsWith(`payout on ${line}`));
      expect(paid).toHaveLength(0);
    }
  });

  it('is wiped by the waterfall when the firm fails, and not by a special case (E4, F2)', () => {
    // A firm dying is an outcome, and waiting for the foundation world to produce one at a listed
    // firm would be a test about how long that takes rather than about what happens when it does.
    // So this world has a reason in it: a buyer of bread far bigger than the ovens that bake it,
    // so the bakeries bid the flour price up against each other until one of them is paying more
    // for the flour than the bread fetches. That is a firm dying of its own decisions in a world
    // otherwise running normally, and the run is long enough for the winding-up to close as well.
    const w = worldWith('equity', [hungryFor('bread')]);
    for (let i = 0; i < 40; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const died = w.journal.ofKind('estate.opened').find((e) => e.data['dead'] === FIRM_6);
    expect(died).toBeDefined();
    // The module stopped pricing the line and said why, publicly.
    const said = w.journal.ofKind('equity.succeeded').filter((e) => e.subjects.includes(SMALL));
    expect(said).toHaveLength(1);
    expect(said[0]?.public).toBe(true);
    // E4: the register went to zero rather than to a recovery. It got there by ranking last: the
    // estate reached it after every creditor and wrote off what it could not pay, at zero.
    const line = w.instruments.get(LINE_6);
    expect(line.issued).toBeCloseTo(0, 6);
    expect(w.register.heldTotal(LINE_6).value).toBeCloseTo(0, 6);
  });
});

describe('the reads (Equity B4, B4.a, C1.b, G3)', () => {
  it('publishes the count, the float, the votes and what the market says it is all worth', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    for (let i = 0; i < 4; i += 1) w.step();
    const read = w.journal.ofKind('equity.reads').filter((e) => e.subjects.includes(BIG)).pop();
    expect(read).toBeDefined();
    expect(read?.public).toBe(true);
    const line = w.instruments.get(LINE_4);
    expect(Number(read?.data['shares'])).toBeCloseTo(line.issued, 8);
    // C1.b: the float is the count less what is BOUND — a read of the register (D5.a). Nobody has
    // bound any yet: the party that does is a founder, and a founder funds a firm's entry (13g).
    expect(Number(read?.data['strategic'])).toBe(0);
    expect(Number(read?.data['freeFloat'])).toBeCloseTo(line.issued, 8);
    // B4: shares times price. B4.a: nothing here compares it against shares times price and calls
    // that a check — this asserts it was PUBLISHED, which is a different thing entirely.
    const print = w.prices.latest(LINE_4, w.period);
    expect(print.some).toBe(true);
    expect(read?.data['marketCapitalisation']).not.toBeNull();
  });
});

describe('what a share is worth to one holder (Equity B1, B3, XI-13, §46 A3)', () => {
  it('is that holder own number: two cells with different histories want different prices', () => {
    // The one thing that differs between the two cells is what has happened to their incomes. A
    // cell that has been surprised wants more for holding a claim that promises it nothing (§46 B3),
    // so it wants to pay less for it — and the disagreement is what gives the book two sides.
    const steady = 'hh.working.bank.a.0';
    const jolted = 'hh.working.bank.a.1';
    const w = saversWorld('equity', [
      // The second cell is paid something it was not expecting, and it remembers being surprised.
      // Law 8: a real payment, so a whole number of cents to every member of the cell.
      pays([{ to: jolted, amount: phx(500) }], 6),
    ]);
    for (let i = 0; i < 12; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const bidsOf = (cell: string): Map<string, number> => {
      const plans = w.journal.ofKind('households.plan').filter((e) => e.subjects.includes(cell));
      const last = plans[plans.length - 1];
      const orders = last?.data['orders'];
      const out = new Map<string, number>();
      if (!Array.isArray(orders)) return out;
      for (const o of orders as { market: string; side: string; price: unknown }[]) {
        if (!o.market.startsWith('mkt.equity.') || o.side !== 'buy') continue;
        if (typeof o.price !== 'number') continue;
        // The top rung of its ladder is the most it will pay, which is its opinion's edge.
        const top = out.get(o.market);
        if (top === undefined || o.price > top) out.set(o.market, o.price);
      }
      return out;
    };
    const a = bidsOf(steady);
    const b = bidsOf(jolted);
    const shared = [...a.keys()].filter((m) => b.has(m));
    expect(shared.length).toBeGreaterThan(0);
    for (const market of shared) {
      // XI-13: neither of them is the price. What prints is what the session made of both.
      const mine = a.get(market);
      const theirs = b.get(market);
      expect(mine).not.toBe(theirs);
      // §46 B3: the cell whose income surprised it requires more of a claim that promises it
      // nothing, so it will pay less for the very same share.
      expect(theirs).toBeLessThan(mine ?? 0);
    }
  });

  it('posts a schedule, not a point, and never trades with itself (B1, B6, Clearing A2)', () => {
    const w = saversWorld('equity');
    for (let i = 0; i < 12; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    let ladders = 0;
    let books = 0;
    for (const plan of w.journal.ofKind('households.plan').filter((e) => e.period === w.period)) {
      const orders = plan.data['orders'];
      if (!Array.isArray(orders)) continue;
      const rows = (orders as { market: string; side: string; price: unknown }[]).filter((o) =>
        o.market.startsWith('mkt.equity.'),
      );
      for (const market of new Set(rows.map((o) => o.market))) {
        const here = rows.filter((o) => o.market === market);
        const buys = here.filter((o) => o.side === 'buy');
        const sells = here.filter((o) => o.side === 'sell');
        books += 1;
        // A2.a: how much at each price, not one quantity at one price.
        if (buys.length > 1) ladders += 1;
        // One line, one side: a bid and an ask from the same party in the same book is a party
        // trading with itself, and what printed out of it would be a trade that moved nothing.
        expect(buys.length > 0 && sells.length > 0).toBe(false);
      }
    }
    expect(books).toBeGreaterThan(0);
    expect(ladders).toBeGreaterThan(0);
  });
});

describe('the ownership identity (Equity C1.a)', () => {
  it('holds exactly what is outstanding, every period of a year', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    for (let i = 0; i < 52; i += 1) {
      expect(unexpected(w.step().audit)).toEqual([]);
      for (const row of DREW.draw.listed.map((l) => l.firm)) {
        const line = w.instruments.get(equityLineOf(row));
        if (!line.status.live) continue;
        const held = w.register.heldTotal(line.id);
        expect(Math.abs(held.value - line.issued)).toBeLessThanOrEqual(held.dust + line.issuedDust);
      }
    }
  });

  it('gives the same world twice from the same seed (Seed A5, Observer E3)', () => {
    const a = rigWorld('equity', RIG.banks, RIG.firms);
    const b = rigWorld('equity', RIG.banks, RIG.firms);
    for (let i = 0; i < 12; i += 1) {
      a.step();
      b.step();
    }
    expect(snapshot(a, { kind: 'inspector' }, 50)).toEqual(snapshot(b, { kind: 'inspector' }, 50));
  });
});

describe('the firm that has no shares (Law 15)', () => {
  it('is a real state: a firm this world did not list has no line and no market', () => {
    const w = rigWorld('equity', RIG.banks, RIG.firms);
    expect(w.instruments.has(equityLineOf(UNLISTED))).toBe(false);
    expect(w.markets.some((m) => m.id === equityMarketOf(UNLISTED))).toBe(false);
    expect(w.parties.ofKind(FIRM).length).toBeGreaterThan(3);
  });
});
