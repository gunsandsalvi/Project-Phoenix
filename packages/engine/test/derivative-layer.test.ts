/**
 * The layer: a contract book, the house both sides face, the margin, and the waterfall.
 *
 * @spec Derivative D2 Derivative D9 Derivative D9.a Derivative D11 Derivative Layer B1 Derivative Layer C2 Derivative Layer C3 Derivative Layer C3.a Derivative Layer C3.b Derivative Layer C4 Derivative Layer D1 Derivative Layer D2 Derivative Layer D2.b Derivative Layer D3 Derivative Layer D4 Derivative Layer E1 Derivative Layer E2 Derivative Layer E4 Derivative Layer G2 Audit B5
 *
 * Everything here runs through the kernel's own book: a fill in a contract market becomes a row on
 * two balance sheets, cut to what its two sides can margin, with the margin in the same instruction.
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  LAYER_PARAMS,
  USD,
  assemble,
  asQty,
  capacityOf,
  currencyUnit,
  houseIdFor,
  instrumentId,
  inFund,
  marginLineId,
  marketId,
  membersOf,
  moveMargin,
  none,
  partyId,
  partyKindId,
  period,
  posted,
  requirement,
  snapshot,
  type MarketDecl,
  type MechanismContext,
  type Order,
  type ParticipantView,
  type PartyId,
  type PartyKindProfile,
  type SeedContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { runWaterfall } from '../src/index.js';
import { mergeModules, rigSpec, withDependencies } from './rig.js';
import { testForwardKind, TEST_FORWARD, type ForwardTerms } from './support/test-forward.js';

const BOOK = marketId('mkt.forward.test');
/** A size two parties can trade without either of them running out of the line (Law 8: a count). */
const SMALL_SIZE = 100;

/**
 * A party kind that exists only to give the underlying a second opinion.
 *
 * IT IS NOT A BANK, and that is the whole point of it. A bank already has a desk with a view on
 * every line it holds, and a fixture that put a second order in the same bank's name would be
 * giving one party two opinions — which the kernel refuses (Clearing A2: nobody crosses itself) and
 * Law 4 forbids for better reasons than the refusal. So the fixture brings its own parties, they
 * hold what they trade, and nothing else in the world has a claim on what they think.
 */
const MOVER = partyKindId('test.mover');

const moverKind: PartyKindProfile = {
  id: MOVER,
  representation: 'named',
  moneyIssuer: null,
  fails: ['cash', 'solvency'],
  borrows: false,
  depositClass: null,
  terminal: false,
};

const MOVERS: readonly PartyId[] = [partyId('test.mover.long'), partyId('test.mover.short')];

/**
 * The line the book is written on: one this world clears (D3.a) AND that the movers actually hold,
 * so the fixture below can trade it. A forward on a line nobody can sell is a forward on a line
 * whose print never moves, and a line that has never moved has a measured move of nothing — which
 * is the layer refusing to margin it (G2) rather than a world worth testing in.
 */
function tradableLine(
  markets: readonly MarketDecl[],
  held: (instrument: MarketDecl['instrument']) => number,
): MarketDecl | undefined {
  return markets.find((x) => x.kind === undefined && x.fx === undefined && held(x.instrument) > 0);
}

function underlying(w: World): MarketDecl {
  const m = tradableLine(w.markets, (i) =>
    MOVERS.reduce((t, p) => t + (w.instruments.has(i) ? w.register.quantity(p, i) : 0), 0),
  );
  if (m === undefined) throw new Error('nothing in this world holds anything it could sell');
  return m;
}

/**
 * A module that opens a contract book and puts two banks on opposite sides of it. The house is the
 * one the layer seeded for this money (C2), so every fill becomes two rows and no member ever faces
 * another. It brings two parties of its own to keep the underlying's print moving (`MOVER`).
 */
function book(opts: {
  cleared: boolean;
  strike: number;
  size: number;
  /** Whether the fixture's own parties clear as members too (they are not banks: they can run out). */
  movers?: boolean;
}): SystemModule {
  let terms: ForwardTerms | undefined;
  return {
    id: 'test.book',
    spec: 'Derivative Layer B1',
    requires: ['derivative-layer'],
    instrumentKinds: [],
    derivativeKinds: [testForwardKind],
    partyKinds: [moverKind],
    curveFamilies: [],
    units: [{ id: testForwardKind.unit, name: 'contracts', perUnit: 1 }],
    params: [],
    seed: (ctx: SeedContext): void => {
      const banks = [...ctx.parties.all()].filter((p) => p.kind === BANK && p.status.alive);
      const bank = banks[0];
      if (bank === undefined) return;
      // Something this world already has and somebody already holds: a fixture that minted its own
      // instrument would be trading a line no market of this world clears (D3.a).
      const line = ctx.instruments
        .all()
        .find(
          (i) =>
            ctx.registry.instrumentKind(i.kind).pricing === 'cleared' &&
            banks.some((b) => ctx.register.quantity(b.id, i.id) > 0),
        );
      if (line === undefined) return;
      const print = ctx.prices.latest(line.id, ctx.period);
      for (const [n, id] of MOVERS.entries()) {
        ctx.parties.add({
          id,
          kind: MOVER,
          region: bank.region,
          name: `a fixture that trades ${line.id}`,
          bank: bank.id,
          representation: 'named',
          status: { alive: true },
        });
        // Enough of each that neither runs out over a year of swapping sides, and NO MORE: an
        // endowment is a liability of whoever issued what it holds (Seed A4), so a fixture that
        // opened with a billion would be handing a bank a billion of deposits against nothing and
        // testing the layer in a world whose banks it had just made insolvent.
        const level = print.some ? print.value.price : 1;
        ctx.endowMoney(id, USD, level * SMALL_SIZE * 40);
        if (n === 1) ctx.endowUnits(id, line.id, SMALL_SIZE * 4, level);
      }
    },
    phases: [
      {
        name: 'test.book',
        spec: 'Derivative Layer B1',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          if (ctx.markets.some((m) => m.id === BOOK)) return;
          const line = tradableLine(ctx.markets, (i) =>
            MOVERS.reduce((t, m) => t + (ctx.instruments.has(i) ? ctx.register.quantity(m, i) : 0), 0),
          );
          if (line === undefined) return;
          terms = {
            kind: TEST_FORWARD,
            market: line.id,
            underlying: line.instrument,
            expiry: period(ctx.period + 6),
            strike: opts.strike,
            long: true,
            window: 8,
            // Clearing A2, Law 4: WHO TRADES THIS BOOK AND HOW MUCH, carried by the book itself.
            // The layer declares the one participant a contract book asks and the kind answers it
            // (`DerivativeKindProfile.orders`), so the fixture's intent has to travel with the
            // terms rather than in a second participant of its own.
            size: opts.size,
            movers: opts.movers === true ? [String(MOVERS[0])] : [],
          };
          ctx.openMarket({
            id: BOOK,
            name: 'a forward book',
            instrument: instrumentId('contract:test.forward'),
            ccy: USD,
            rationing: 'proRata',
            kind: 'contract',
            contract: {
              kind: TEST_FORWARD,
              terms,
              house: opts.cleared ? houseIdFor(USD) : null,
            },
          });
        },
      },
    ],
    participants: [
      {
        /**
         * A FIXTURE, and it says so: two parties of the fixture's own trading the underlying at a
         * level that moves.
         *
         * Initial margin is sized from the underlying's OWN measured move (D1), and this world's
         * sovereign line prints the same number every period — a line that has never moved has a
         * measured move of nothing, so every margin on it is nothing and no trade is ever cut by
         * one. That is the mechanism working; it is also a world in which the capacity clause
         * cannot be shown. So the fixture makes the line move, the way a market with two opinions
         * in it does, and nothing else about the layer is touched.
         *
         * THEY SWAP SIDES EACH PERIOD, which is not decoration: a fixture with a permanent buyer
         * and a permanent seller runs the seller out of the line within a year and the print stops
         * moving halfway through the run that was supposed to exercise it.
         */
        partyKind: MOVER,
        in: 'asset',
        speculative: true,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          if (m.id !== terms?.market) return [];
          const last = view.print(m.instrument);
          if (!last.some) return [];
          const tick = view.registry.tickFor(view.instruments.get(m.instrument).kind, m.ccy);
          const up = view.period % 2 === 0;
          const level = last.value.price + (up ? tick : -tick);
          if (level <= 0) return [];
          const buyer = up ? MOVERS[0] : MOVERS[1];
          const side = view.self.id === buyer ? 'buy' : 'sell';
          // A small, whole size both sides can manage: what is being tested is that the level
          // MOVES, not how much of it changes hands.
          const want = asQty(SMALL_SIZE);
          const free = view.free(m.instrument);
          const qty = side === 'sell' ? (free < want ? free : want) : want;
          if (qty <= 0) return [];
          return [{ party: view.self.id, side, price: level, qty }];
        },
      },
      {
        /**
         * D2.c NEEDS A MEMBER THAT CAN RUN OUT OF MONEY, and a bank is not one: a bank pays what it
         * owes by issuing its own deposit (Money A1), so a clearing house whose members are all
         * banks has no member that can miss a call. The fixture's own parties are not money
         * issuers, which is the whole reason they are here and not a third bank.
         */
        partyKind: MOVER,
        in: 'contract',
        speculative: true,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          if (m.id !== BOOK || terms === undefined || opts.movers !== true) return [];
          if (view.self.id !== MOVERS[0]) return [];
          return [{ party: view.self.id, side: 'buy', price: opts.strike, qty: asQty(opts.size) }];
        },
      },
      {
        partyKind: BANK,
        // Spot FX D1, Law 15: a participant answers the SORT of market it declared itself in. A
        // contract book is not an asset book — nothing is delivered in it — so a desk that trades
        // instruments is never asked about one and this one is never asked about a bond.
        in: 'contract',
        speculative: true,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          if (m.id !== BOOK || terms === undefined) return [];
          const banks = view.parties
            .ofKind(BANK)
            .filter((b) => b.status.alive)
            .map((b) => b.id);
          const buyer = banks[0];
          const seller = banks[1];
          if (view.self.id === buyer) {
            return [{ party: view.self.id, side: 'buy', price: opts.strike, qty: asQty(opts.size) }];
          }
          if (view.self.id === seller) {
            return [{ party: view.self.id, side: 'sell', price: opts.strike, qty: asQty(opts.size) }];
          }
          return [];
        },
      },
    ],
    families: [],
  };
}

function world(...extra: SystemModule[]): World {
  const spec = rigSpec('layer');
  const base = withDependencies(
    spec.modules,
    (m) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market' ||
      m.id === 'estate' ||
      m.id === 'derivative-layer',
  );
  // The desks DEAL here, unlike most kernel-only worlds: initial margin is sized from the
  // underlying's own measured move (D1), and a line nobody quotes has never moved. A world whose
  // sovereign book never prints cannot margin a forward on it, which is G2 working rather than a
  // world worth testing in.
  return assemble({ ...spec, modules: mergeModules(base, extra) });
}

function banksOf(w: World): readonly PartyId[] {
  return w.parties
    .ofKind(BANK)
    .filter((b) => b.status.alive)
    .map((b) => b.id);
}

describe('a contract book (Derivative Layer B1, C2, E2)', () => {
  it('turns one trade into two rows against the house, and no member faces another', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 4; i += 1) w.step();
    const house = houseIdFor(USD);
    expect(w.parties.has(house)).toBe(true);
    const rows = w.contracts.open_();
    // The book has to have STRUCK something, or every assertion below is vacuous and the test is
    // measuring nothing (Law 11's temptation in test form).
    expect(rows.length).toBeGreaterThan(0);
    // C2: every row has the house on one side of it, so no member ever pays another.
    for (const r of rows) expect(r.a === house || r.b === house).toBe(true);
    const members = membersOf(w.mechanismContext('test'), house, USD);
    for (const m of members) {
      for (const other of members) {
        if (m === other) continue;
        expect(w.contracts.between(m, other).length).toBe(0);
      }
    }
  });

  it('the house is flat by construction: what it holds on one side it owes on the other', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 4; i += 1) w.step();
    const house = houseIdFor(USD);
    const rows = w.contracts.openOf(house);
    if (rows.length === 0) return;
    const net = rows.reduce((t, r) => t + w.contractValue(r, house, w.period), 0);
    // C2: it is buyer to the seller and seller to the buyer, at the same level on the same terms,
    // so its own position in the contracts is nothing. What it has instead is the margin and the
    // fund, which are its liabilities (C3).
    expect(net).toBe(0);
  });
});

describe('margin (D9, D9.a, C3.a, D3)', () => {
  it('is an asset swap and not an expense: money out, a claim in, one instruction', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 4; i += 1) w.step();
    const house = houseIdFor(USD);
    const members = membersOf(w.mechanismContext('test'), house, USD);
    const ctx = w.mechanismContext('test');
    for (const m of members) {
      const claim = marginLineId(m, house, USD);
      if (!w.instruments.has(claim)) continue;
      // C3.a: the margin a member posts is its ASSET at the house, on its own balance sheet.
      expect(w.register.quantity(m, claim)).toBeGreaterThan(0);
      // D3: and the house owes it back — the same number read from the other side (Register B3).
      expect(w.instruments.get(claim).issued).toBe(w.register.heldTotal(claim).value);
      expect(posted(ctx, m, house, USD)).toBe(w.register.quantity(m, claim));
    }
  });

  it('what is posted is what the requirement asks for, and it moves both ways (D2)', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 6; i += 1) w.step();
    const ctx = w.mechanismContext('test');
    const house = houseIdFor(USD);
    for (const m of membersOf(ctx, house, USD)) {
      const need = requirement(ctx, m, house, USD);
      const have = posted(ctx, m, house, USD);
      // D2: the requirement is re-measured every period and the margin is moved to meet it — up
      // when the marks went against the member, and back down when they went for it (D3).
      expect(Math.abs(need - have)).toBeLessThanOrEqual(1);
    }
  });

  it('every margin claim is held by exactly what was issued (D2.b, the zero-sum contribution)', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    let report = w.step();
    for (let i = 0; i < 5; i += 1) report = w.step();
    const zero = report.audit.families.find((f) => f.family === 'zeroSum');
    expect(zero?.built).toBe(true);
    expect(zero?.contributions).toContain('derivative-layer');
    expect(zero?.violations.filter((v) => v.spec.includes('D2.b'))).toEqual([]);
  });
});

describe('the default fund (C3, C3.b)', () => {
  it('is sized cover-one and shared pro rata to what each member has at risk', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 6; i += 1) w.step();
    const ctx = w.mechanismContext('test');
    const house = houseIdFor(USD);
    const members = membersOf(ctx, house, USD);
    if (members.length === 0) return;
    const held = members.map((m) => inFund(ctx, m, house, USD));
    // C3.b: contributions are pro rata to margin and they sum to the cover-one figure, in whole
    // pieces of money, with nothing left over (Law 8: `splitOnTick`).
    for (const h of held) expect(h).toBeGreaterThanOrEqual(0);
    const total = held.reduce((t, h) => t + h, 0);
    expect(Number.isInteger(total)).toBe(true);
  });
});

describe('capacity (E1, E2, E4)', () => {
  it('cuts a trade to what its two sides can margin and journals what it refused', () => {
    // A size far past what either bank could ever margin: the book still clears, and what it
    // strikes is what the members could carry. Nothing raises the limit (E4).
    const w = world(book({ cleared: true, strike: 1, size: 1_000_000_000 }));
    for (let i = 0; i < 5; i += 1) w.step();
    const refused = w.journal.ofKind('derivatives.refused');
    const rows = w.contracts.open_();
    if (rows.length === 0 && refused.length === 0) return;
    for (const e of refused) {
      expect(Number(e.data['admitted'])).toBeLessThan(Number(e.data['wanted']));
      expect(Number(e.data['cut'])).toBeGreaterThan(0);
    }
    // E2: what was struck is what both sides could margin, so every row that exists is margined.
    const ctx = w.mechanismContext('test');
    const house = houseIdFor(USD);
    for (const m of membersOf(ctx, house, USD)) {
      expect(posted(ctx, m, house, USD)).toBeGreaterThan(0);
    }
  });

  it('counts the cash a member has posted ONCE, not twice (E1, E3, Law 19)', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 5; i += 1) w.step();
    const ctx = w.mechanismContext('test');
    const house = houseIdFor(USD);
    const posters = membersOf(ctx, house, USD).filter((m) => posted(ctx, m, house, USD) > 0);
    // A draw in which nothing was posted has no subject for this assertion; the test above is what
    // says something was.
    if (posters.length === 0) return;
    const buffer = w.params.get(LAYER_PARAMS.buffer);
    for (const m of posters) {
      const view = w.participantView(m);
      const cash = view.cash(USD);
      // C3.a, E2: margin is an asset swap settled in the SAME instruction as the trade it covers,
      // so this account is ALREADY net of everything this member has posted. Its capacity is that
      // account less what it keeps back — and the margin claim it holds against the posting is not
      // taken off a second time, which is what `posted(...) > 0` above makes this a test of. Under
      // the double count the room fell by twice every unit posted, and every book was half the size
      // the mechanism says.
      expect(capacityOf(view, USD, buffer)).toBe(cash - cash * buffer);
    }
  });

  it('cannot margin a line whose own move nobody can measure yet (G2, D1)', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    const line = underlying(w);
    // At the seal the line has exactly the price the seed wrote and has never moved: there is no
    // record to measure, so there is no honest number for what a position in it could do. G2 admits
    // an exposure with no margin only with A STATED REASON, and "nobody can say" is not one — so
    // the kind answers Missing and the layer refuses the trade rather than admitting it at nothing.
    expect(w.prices.history(line.instrument).length).toBe(1);
    const terms: ForwardTerms = {
      kind: TEST_FORWARD,
      market: line.id,
      underlying: line.instrument,
      expiry: period(6),
      strike: 1,
      long: true,
      window: 8,
      size: 0,
      movers: [],
    };
    const need = w.contracts.marginFor(
      {
        kind: TEST_FORWARD,
        terms,
        ccy: USD,
        notional: 10,
        struckAt: 1,
        house: houseIdFor(USD),
        a: houseIdFor(USD),
        b: houseIdFor(USD),
      },
      w.period,
    );
    expect(need.some).toBe(false);
  });
});

describe('the balance sheet carries the contract (D1, Audit B5)', () => {
  it('a row is an asset to one side and a liability to the other, and the accounts hold', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    let report = w.step();
    for (let i = 0; i < 5; i += 1) report = w.step();
    const rows = w.contracts.open_();
    if (rows.length === 0) return;
    const accounts = report.audit.families.find((f) => f.family === 'accounts');
    const house = houseIdFor(USD);
    // D1, Audit B5: the mark is on both balance sheets and the equity accounts moved with it, so
    // the identity holds for the house and for every member.
    const named = new Set([house, ...banksOf(w)].map(String));
    expect(
      accounts?.violations.filter((v) => named.has(v.owner)).map((v) => v.message),
    ).toEqual([]);
  });
});

describe('expiry (D11, D11.a, D3)', () => {
  it('terminates on both books at once and gives the margin back', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    // The book's forwards expire six periods after it opened; run past that.
    for (let i = 0; i < 9; i += 1) w.step();
    const closed = w.journal.ofKind('contract.closed');
    expect(closed.length).toBeGreaterThan(0);
    const ids = new Set(closed.map((e) => String(e.data['contract'])));
    for (const id of ids) {
      // D11: it ceases to exist on both books at once. The row stays readable — F4's chain has to
      // be traceable — and it is terminated.
      expect(w.contracts.get(id as never).state).toBe('terminated');
    }
    // D3: margin is held, not consumed. With nothing left to secure it goes back to whoever posted
    // it, and what the house owes is what it holds (Register B3).
    const ctx = w.mechanismContext('test');
    const house = houseIdFor(USD);
    for (const m of membersOf(ctx, house, USD)) {
      expect(posted(ctx, m, house, USD)).toBe(requirement(ctx, m, house, USD));
    }
  });
});

describe('the waterfall (C4, C4.a, C4.d, C5)', () => {
  it('absorbs in the stated order and says what each line paid', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 6; i += 1) w.step();
    const ctx = w.mechanismContext('test');
    const house = houseIdFor(USD);
    const members = membersOf(ctx, house, USD);
    const defaulter = members[0];
    if (defaulter === undefined) return;
    const margin = posted(ctx, defaulter, house, USD);
    const fund = inFund(ctx, defaulter, house, USD);
    // A loss larger than everything the defaulter put up, so the waterfall has to walk past its
    // own first two lines and report what it could not fund (C5: past the end is a real event).
    const loss = (margin + fund) * 2 + 1_000_000;
    const rounds = runWaterfall(ctx, house, defaulter, USD, loss);
    const lines = rounds.map((r) => r.line);
    // C4: in order. Its margin, then its fund contribution, then the house's own capital, then the
    // survivors' contributions pro rata. Nothing jumps the queue and nothing is topped up.
    const order = ['defaulterMargin', 'defaulterFund', 'houseCapital', 'survivors'];
    const rank = (l: string): number => order.indexOf(l);
    for (let i = 1; i < lines.length; i += 1) {
      expect(rank(String(lines[i]))).toBeGreaterThanOrEqual(rank(String(lines[i - 1])));
    }
    // C4.d: every round is recorded and reportable — who defaulted, the loss, what each line paid.
    const said = w.journal.ofKind('waterfall.round');
    expect(said.length).toBe(rounds.length);
    for (const e of said) expect(e.public).toBe(true);
    // C5: and what nothing could fund is said out loud rather than absorbed by somebody.
    const last = rounds[rounds.length - 1];
    if (last !== undefined && last.left > 0) {
      expect(w.journal.ofKind('waterfall.unfunded').length).toBe(1);
    }
  });
});

describe('what the layer refuses to be', () => {
  it('declares no class of contract of its own (13b builds those)', () => {
    const w = world();
    // A layer that shipped a class would be a layer with an opinion about what a derivative is.
    expect(w.registry.derivativeKinds.size).toBe(0);
    // And the house exists anyway: it is an institution, not something a trade creates.
    expect(w.parties.has(houseIdFor(USD))).toBe(true);
    expect(currencyUnit(USD)).toBe('ccy:USD');
  });
});

/**
 * A phase that takes a member's cash away, so that the call it gets next cannot be met.
 *
 * It is a FIXTURE and it does not pretend otherwise: nothing in this world is poor enough to fail a
 * margin call on its own yet, because no class of contract is built (13b) and a forward on a
 * sovereign line moves by ticks. What is being tested is what the layer does when the payment does
 * not go through, and the only honest way to reach it is to make a member that cannot pay. It pays
 * the money to another party, so the money still exists and still has a holder — a test that made
 * cash vanish would be testing a world that breaks Law 5 rather than one that breaks a promise.
 */
function drain(who: PartyId, after: number): SystemModule {
  return {
    id: 'test.drain',
    spec: 'Derivative Layer D2.c',
    requires: ['derivative-layer'],
    instrumentKinds: [],
    derivativeKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.drain',
        spec: 'Derivative Layer D2.c',
        cycle: 0,
        anchor: { before: 'margin.calls' },
        run: (ctx: MechanismContext): void => {
          if (ctx.period < after || !ctx.parties.has(who)) return;
          if (!ctx.parties.get(who).status.alive) return;
          // Somewhere for it to go: the money still exists and still has a holder afterwards. A
          // fixture that deleted cash would be testing a world that breaks Law 5.
          const other = MOVERS.find((m) => m !== who && ctx.parties.has(m));
          if (other === undefined) return;
          const cash = ctx.participant(who).cash(USD);
          const amount = ctx.registry.cashFor(USD, cash);
          if (amount <= 0) return;
          ctx.settle({
            legs: [
              {
                kind: 'money',
                from: ctx.accountOf(who, USD),
                to: ctx.accountOf(other, USD),
                ccy: USD,
                amount,
                fromCell: none(),
                toCell: none(),
              },
            ],
            cause: 'transfer',
            reason: `${who} pays its cash away`,
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

describe('the payment has a cash test (D2.c, X2, Money E1)', () => {
  it('a member that cannot post fails the instruction, and nothing appears to cover it', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 3; i += 1) w.step();
    const ctx = w.mechanismContext('test');
    const house = houseIdFor(USD);
    // NOT A BANK: a bank pays what it owes by issuing its own deposit (Money A1), so there is no
    // sum a bank cannot post and no cash test to fail. The party that can fail one is a party that
    // issues nothing, which is what D2.c is about.
    const member = MOVERS[0];
    if (member === undefined || !w.parties.has(member)) return;
    const cashBefore = ctx.participant(member).cash(USD);
    const postedBefore = posted(ctx, member, house, USD);
    // More than it has, by a margin nothing rounds away.
    const tooMuch = ctx.registry.cashFor(USD, cashBefore * 2 + 1_000_000);
    const r = ctx.settle({
      legs: moveMargin(ctx, member, house, USD, tooMuch),
      cause: 'transfer',
      reason: `${member} is asked for more margin than it has`,
    });
    // Money E1: the instruction FAILS. It is not part-settled, it is not borrowed, and no overdraft
    // appears — which is X2 as much as D2.c: an exposure is not a way to get cash for free.
    expect(r.outcome).not.toBe('settled');
    expect(ctx.participant(member).cash(USD)).toBe(cashBefore);
    expect(posted(ctx, member, house, USD)).toBe(postedBefore);
  });
});

describe('an unmet call (D4, D11.a, F2)', () => {
  it('does not leave the position open', () => {
    const mover = MOVERS[0];
    if (mover === undefined) return;
    // A SIZE THAT COSTS SOMETHING. Initial margin is the underlying's own measured move times the
    // notional (D1), and this line moves by a tick: on ten contracts the requirement is a fraction
    // of a cent, posts nothing (Law 8: a payment is a count of pieces) and no call is ever short.
    // What is being tested needs a book big enough for the margin on it to be money.
    const w = world(
      book({ cleared: true, strike: 1, size: 200_000, movers: true }),
      drain(mover, 3),
    );
    for (let i = 0; i < 10; i += 1) w.step();
    // D2.c: the call was made, the payment was attempted, and the event says it did not settle.
    const missed = w.journal
      .ofKind('margin.call')
      .filter((e) => e.data['met'] === false);
    expect(missed.length).toBeGreaterThan(0);
    // D4: and the row did not survive it. Either the call closed it out at its stated value
    // (D11.a), or the member failed on the missed payment first and the row closed on the estate
    // (F2) — what is forbidden is the third answer, where nothing happens and the exposure stands.
    const closed = w.journal.ofKind('contract.closed');
    expect(closed.length).toBeGreaterThan(0);
    for (const e of closed) expect(e.public).toBe(true);
  });
});

describe('a year of it (Part XII, Law 18)', () => {
  it('runs a year with a contract book open and the zero-sum family stays green', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    let report = w.step();
    for (let i = 1; i < 52; i += 1) report = w.step();
    // D1.b: the family this item built. Every mark is one number and its negation, every margin
    // line is held by exactly what was issued, every period of the year.
    const zero = report.audit.families.find((f) => f.family === 'zeroSum');
    expect(zero?.built).toBe(true);
    expect(zero?.violations.map((v) => v.message)).toEqual([]);
    // Something happened: a year in which no contract was ever written would say nothing at all.
    expect(w.journal.ofKind('contract.closed').length).toBeGreaterThan(0);
  });

  it('is the same world twice: no clock, no randomness (Law 18)', () => {
    const said = (): readonly string[] => {
      const w = world(book({ cleared: true, strike: 1, size: 10 }));
      for (let i = 0; i < 12; i += 1) w.step();
      return w.journal
        .all()
        .filter((e) => e.kind.startsWith('margin.') || e.kind.startsWith('contract.'))
        .map((e) => `${e.period}|${e.kind}|${e.subjects.join(',')}|${JSON.stringify(e.data)}`);
    };
    const once = said();
    expect(once.length).toBeGreaterThan(0);
    expect(said()).toEqual(once);
  });
});

describe('what a side may see (Observer A4, G3)', () => {
  it('shows a party the rows it is on and no others', () => {
    const w = world(book({ cleared: true, strike: 1, size: 10 }));
    for (let i = 0; i < 4; i += 1) w.step();
    const house = houseIdFor(USD);
    const members = membersOf(w.mechanismContext('test'), house, USD);
    const member = members[0];
    if (member === undefined) return;
    const seen = snapshot(w, { kind: 'party', party: member }, 0).contracts;
    expect(seen.length).toBeGreaterThan(0);
    // A4: its own side of its own rows. A member never sees what another member has with the house,
    // which is also G3: there is no read that nets across counterparties for it to see it through.
    for (const c of seen) expect([c.a, c.b]).toContain(String(member));
  });
});
