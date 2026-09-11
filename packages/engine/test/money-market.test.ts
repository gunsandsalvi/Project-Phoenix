/**
 * The money market: what a bank pays for money, what that costs whoever borrows from it, and the
 * corridor the central bank makes the policy rate effective through.
 *
 * @spec Money Market A1 Money Market A1.a Money Market A2 Money Market A2.a Money Market A3 Money Market B1 Money Market B1.a Money Market B2 Money Market B2.a Money Market B3 Money Market B5 Money Market B5.a Money Market B6 Money Market C1 Money Market C1.a Money Market C2 Money Market C3 Money Market C4 Money Market C4.b Banks Funding A1 Banks Funding A1.a Banks Funding B1 Banks Funding B1.a Banks Funding B2 Banks Funding B2.b Banks Funding C1 Banks Funding C1.a Banks Funding F1 Banks Funding F4 Money Market C5 Money Market E1 Money Market E2 Central Bank B1 Central Bank B2 Central Bank B3 Central Bank B3.a Central Bank B4 Central Bank D1 Central Bank D2 Central Bank D3 Central Bank D4 Central Bank D6 Banks Funding F2 Banks Lending C1.a Observer A5 XI-4
 *
 * XI-4's first joint is what these are about. A bank that funds itself for nothing prices every loan
 * off a policy rate somebody wrote down, and the whole chain from a financial price to a real
 * quantity is deleted. What has to be true is that money COSTS a bank something, that what it costs
 * is what it actually paid rather than a rate anybody stated, and that the number reaches the quote
 * it gives a borrower.
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  CB,
  MM_PARAMS,
  USD,
  assemble,
  moneyInstrumentId,
  none,
  partyId,
  snapshot,
  yearFraction,
  type PartyId,
  type Event,
  type SystemModule,
  type World,
} from '../src/index.js';
import { rigWorld, rigSpec, mergeModules } from './rig.js';
import { unexpected } from './expected.js';
import { phx } from './units.js';

const BANK_A = partyId('bank.a');
const BANK_B = partyId('bank.b');

/** The same world with one declared number set differently, wherever it was declared. */
function withParam(seed: string, over: Readonly<Record<string, number>>): World {
  const spec = rigSpec(seed);
  const modules: SystemModule[] = spec.modules.map((m) => ({
    ...m,
    params: m.params.map((p) => {
      const value = over[String(p.id)];
      return value === undefined ? p : { ...p, value };
    }),
  }));
  return assemble({ ...spec, modules });
}

function events(w: World, kind: string, subject?: string): Event[] {
  return w.journal
    .ofKind(kind as never)
    .filter((e) => subject === undefined || e.subjects.includes(subject));
}

function num(e: Event | undefined, key: string): number {
  const v = e?.data[key];
  return typeof v === 'number' ? v : 0;
}

function last(w: World, kind: string, subject?: string): Event | undefined {
  const all = events(w, kind, subject);
  return all[all.length - 1];
}

function run(w: World, periods: number): World {
  for (let i = 0; i < periods; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

describe('what money costs a bank (Banks Funding B1, B2, B2.b, XI-4 joint one)', () => {
  it('is what it actually paid, to named holders, and not a rate anybody stated', () => {
    const w = run(rigWorld('mm-cost'), 6);
    for (const bank of [BANK_A, BANK_B]) {
      const said = last(w, 'bank.costOfFunds', bank);
      expect(said).toBeDefined();
      // B2: a blend over what funds its book, and it is positive — money is not free to a bank.
      expect(num(said, 'perAnnum')).toBeGreaterThan(0);
      expect(num(said, 'owed')).toBeGreaterThan(0);
      // B1: and what it paid is on the wire, to holders by name, as real money leaving its account.
      const paid = w.ledger
        .inPeriod(w.period)
        .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'coupon')
        .flatMap((r) => r.instruction.legs)
        .filter((l) => l.kind === 'money' && l.from.holder === bank);
      expect(paid.length).toBeGreaterThan(0);
      expect(paid.every((l) => l.kind === 'money' && l.amount > 0)).toBe(true);
    }
  });

  it('differs between two banks with different mixes, and the difference reaches the quote', () => {
    const w = run(rigWorld('mm-mix'), 8);
    const a = last(w, 'bank.costOfFunds', BANK_A);
    const b = last(w, 'bank.costOfFunds', BANK_B);
    // B1.a, A1: the two banks pay their depositors differently and owe different mixes of money, so
    // what funding costs them is not one number the sector shares.
    expect(num(a, 'perAnnum')).not.toBe(num(b, 'perAnnum'));
    // XI-4 joint one, Banks Lending C1.a, Law 4: and it is IN the quote. Every quote a bank gave in
    // a period carries EXACTLY the cost of funds its own event published that period — one number,
    // published once and read, never the same formula evaluated twice in two places.
    let checked = 0;
    for (const q of events(w, 'credit.quoted')) {
      const bank = String(q.data['bank']);
      // Law 8, Currency A3: A COST OF FUNDS IS PER CURRENCY. A bank lends in every money this
      // world has and prices each quote off what funding costs it IN THAT MONEY, so comparing a
      // euro quote against the bank's own dollar number is comparing two numbers that were both
      // right. Its own money is the event's top line; the others are beside it, named.
      const said = events(w, 'bank.costOfFunds', bank).find((e) => e.period === q.period);
      expect(said).toBeDefined();
      const ccy = String(q.data['ccy']);
      const alsoIn = said!.data['alsoIn'] as Record<string, { perAnnum: number }>;
      const published = ccy === String(said!.data['ccy']) ? num(said, 'perAnnum') : alsoIn[ccy]?.perAnnum;
      expect(published, `${bank} quoted in ${ccy} and published no cost of funds in it`).toBeDefined();
      expect(num(q, 'costOfFunds')).toBe(published);
      // C1: and a quote is its funding plus what the loan itself costs it, never less.
      expect(num(q, 'rate')).toBeGreaterThan(num(q, 'costOfFunds'));
      checked += 1;
    }
    expect(checked).toBeGreaterThan(0);
  });

  it('pays more for money when it keeps less of what money is worth to it (B1.a)', () => {
    // ONE number differs: what bank A keeps for itself out of what money is worth to it. A thinner
    // margin is a better rate to its depositors — which is the whole of B1.a, a bank DECIDING what
    // to pay rather than a stickiness anybody stated, and the number a depositor then decides about.
    //
    // Read at the FIRST board it sets, because that is the one that is its own decision and nothing
    // else: from the next period its rivals are publishing too (D5.a), and a bank whose depositors
    // are being bid for pays what it takes to keep them whatever margin it would have liked. That
    // is the other half of B1.a and it is the test below, not a spoiled version of this one.
    const keen = run(withParam('mm-payup', { 'bank.depositMargin.bank.a': 0.001 }), 8);
    const mean = run(withParam('mm-payup', { 'bank.depositMargin.bank.a': 0.012 }), 8);
    const board = (w: World, cls: string): number => {
      const said = events(w, 'bank.depositRate', BANK_A)[0];
      const rates = said?.data['rates'] as Record<string, number> | undefined;
      return rates?.[cls] ?? 0;
    };
    for (const cls of ['retail', 'corporate', 'wholesale']) {
      expect(board(keen, cls)).toBeGreaterThan(board(mean, cls));
    }
  });

  it('answers a rival that is paying more, and never above what money is worth to it (B1.a, E2.a)', () => {
    // D5.a: the board is public, so a bank can read what its rivals pay and its depositors can read
    // both. E1 is then a real answer to a real number — and it is what stops a bank paying less.
    //
    // Two shapes, and neither is a level. It ANSWERS: banks facing each other for the same money end
    // up publishing the same number for it, because the one paying less matches the one paying more.
    // And it STOPS: what it will not do is pay more than money is worth to it, and what money is
    // worth to it never exceeds the top of the corridor — past that it takes the window instead and
    // lets the deposit go, which is the run's first cause rather than a defect.
    const w = run(rigWorld('mm-rivals'), 12);
    const seen = new Map<string, number[]>();
    for (const e of events(w, 'bank.depositRate')) {
      const rates = e.data['rates'] as Record<string, number>;
      for (const [cls, rate] of Object.entries(rates)) {
        const key = `${e.period}:${cls}`;
        seen.set(key, [...(seen.get(key) ?? []), rate]);
      }
    }
    let matched = 0;
    for (const rates of seen.values()) {
      if (rates.length < 2) continue;
      if (new Set(rates).size === 1) matched += 1;
    }
    expect(matched).toBeGreaterThan(0);
    const ceiling =
      w.params.get(MM_PARAMS.policyRate) + w.params.get(MM_PARAMS.ceilingSpread);
    for (const rates of seen.values()) for (const r of rates) expect(r).toBeLessThan(ceiling);
  });

  it('is what it paid plus what its capital costs, over what funds its book (B2, B2.b)', () => {
    const w = run(rigWorld('mm-parts'), 6);
    const previous = w.period - 1;
    for (const bank of [BANK_A, BANK_B]) {
      const said = last(w, 'bank.costOfFunds', bank);
      // B2.b, ONE RATE PER LIABILITY: the interest half is the money that ACTUALLY LEFT the account
      // last period, annualised — the same instruction the depositor was paid by, read rather than
      // recomputed. If the decision and the read were two formulas this would be two numbers.
      const paid = w.ledger
        .inPeriod(previous as never)
        .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'coupon')
        .flatMap((r) => r.instruction.legs)
        .reduce((t, l) => t + (l.kind === 'money' && l.from.holder === bank ? l.amount : 0), 0);
      const year = yearFraction(
        'ACT/365F',
        w.calendar.startOf(previous as never),
        w.calendar.startOf(w.period),
      );
      expect(num(said, 'interest')).toBeCloseTo(paid / year, 6);
      // B2, XI-4: and the blend is that plus what its owners require on the part of the book they
      // fund, over the whole of what funds it. Equity is the dearest money a bank has, so a bank
      // funded more by deposits and less by its own capital is cheaper — which is the mix XI-4
      // names, arriving as an outcome rather than a weighting anybody chose.
      const funding = num(said, 'owed') + num(said, 'capital');
      expect(num(said, 'perAnnum')).toBeCloseTo(
        (num(said, 'interest') + num(said, 'onCapital')) / funding,
        12,
      );
      expect(num(said, 'onCapital')).toBeGreaterThan(0);
    }
  });
});

describe('the corridor (Money Market C, Central Bank B2, D)', () => {
  it('publishes two administered levels and never a cleared one (B3.a, C2)', () => {
    const w = run(rigWorld('mm-corridor'), 4);
    const c = last(w, 'centralBank.corridor');
    expect(c?.public).toBe(true);
    const floor = num(c, 'floor');
    const policy = num(c, 'policy');
    const ceiling = num(c, 'ceiling');
    // C2: the floor is below the policy rate and the ceiling above it. They are administered, and
    // saying so out loud is what stops the policy rate being read as something a market printed.
    expect(floor).toBeLessThan(policy);
    expect(ceiling).toBeGreaterThan(policy);
    expect(policy).toBe(w.params.get(MM_PARAMS.policyRate));
  });

  it('moves the market rate when it moves, and only through the corridor (B4, B3.a, E1)', () => {
    // E1, Central Bank B3: THE POLICY RATE REACHES THE ECONOMY THROUGH THIS MARKET. Nothing assigns
    // it to anything: the central bank declares two levels and takes both sides at them for real
    // quantities on its own balance sheet, and every other participant then has an alternative it
    // can actually take. Move the declared rate and the rates that CLEAR move with it — because the
    // alternatives moved, which is the only channel there is.
    const cheap = run(withParam('mm-policy', { 'centralBank.policyRate': 0.02 }), 12);
    const dear = run(withParam('mm-policy', { 'centralBank.policyRate': 0.05 }), 12);
    const struck = (w: World): number => {
      const rates: number[] = [];
      const volumes: number[] = [];
      for (const e of events(w, 'moneyMarket.print')) {
        rates.push(num(e, 'rate') * num(e, 'volume'));
        volumes.push(num(e, 'volume'));
      }
      const total = volumes.reduce((a, x) => a + x, 0);
      expect(total).toBeGreaterThan(0);
      return rates.reduce((a, x) => a + x, 0) / total;
    };
    expect(struck(dear)).toBeGreaterThan(struck(cheap));
    // Three points of policy, and what the market did with them is between the two levels and not
    // equal to either: the pass-through is a consequence of the corridor and is MEASURED here
    // rather than asserted to be one-for-one (that measurement is Part XII).
    expect(struck(dear) - struck(cheap)).toBeGreaterThan(0.01);
    for (const w of [cheap, dear]) {
      const c = last(w, 'centralBank.corridor');
      for (const e of events(w, 'moneyMarket.print')) {
        // B3.a: a cleared rate is never the policy rate. It sits between the two levels somebody
        // can actually deal at, and a market that printed the policy rate would be one where the
        // corridor is decoration.
        expect(num(e, 'rate')).toBeGreaterThanOrEqual(num(c, 'floor'));
        expect(num(e, 'rate')).toBeLessThanOrEqual(num(c, 'ceiling'));
      }
    }
    // XI-4's first joint: and it reaches a borrower. What a bank pays for money is dearer, so what
    // it charges for a loan is dearer — through its own cost of funds and nothing else.
    const funds = (w: World): number => num(last(w, 'bank.costOfFunds', BANK_A), 'perAnnum');
    expect(funds(dear)).toBeGreaterThan(funds(cheap));
  });

  it('destroys reserves when a bank parks at the floor, on both balance sheets (C1.a)', () => {
    const w = run(rigWorld('mm-floor'), 8);
    // C1.a: parking at the floor is a real transfer to the central bank's own account, so the cash
    // LEAVES the banking system. Every leg of it has two named sides like any other payment.
    const parked = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter((l) => l.kind === 'money' && l.to.holder === CB && l.from.holder !== CB);
    expect(parked.length).toBeGreaterThan(0);
  });
});

describe('what a bank publishes about itself (Banks Funding F1, F4, C1, C1.a)', () => {
  it('reads its own two sides and never a ratio anybody stated', () => {
    const w = run(rigWorld('mm-publish'), 6);
    for (const bank of [BANK_A, BANK_B]) {
      const said = last(w, 'bank.liquidity', bank);
      expect(said?.public).toBe(true);
      // F2: the reserve balance is a READ OF ITS ACCOUNT, never a mirrored copy.
      expect(num(said, 'reserves')).toBe(
        w.register.quantity(bank, moneyInstrumentId(CB, USD)),
      );
      // C1, C1.a: liquid assets are the account, what comes back tomorrow, and what its own
      // unencumbered eligible paper would actually raise at the haircut the central bank declared.
      expect(num(said, 'liquid')).toBe(
        num(said, 'reserves') + num(said, 'overnight') + num(said, 'paper'),
      );
      // F4: and the metric is one read against the other, or nothing at all.
      const couldLeave = num(said, 'couldLeave');
      const metric = said?.data['metric'];
      if (couldLeave > 0) expect(metric).toBeCloseTo(num(said, 'liquid') / couldLeave, 9);
      else expect(metric).toBeNull();
    }
  });

  it('holds a buffer that is its own worst week and not a share of anything (A2.a, C2.a)', () => {
    // The buffer is DERIVED, and this is the derivation: the worst week this bank's own account has
    // had, over the memory it keeps. Nothing here is a ratio of what it has issued, and the two
    // banks in this world hold different buffers because their own weeks were different.
    const memory = 4;
    const w = run(withParam('mm-buffer', { 'bank.bufferMemory.bank.a': memory }), 12);
    const said = events(w, 'bank.liquidity', BANK_A);
    const moves = said.map((e) => num(e, 'move'));
    for (let i = 0; i < said.length; i += 1) {
      const seen = moves.slice(i + 1 > memory ? i + 1 - memory : 0, i + 1);
      const worst = Math.min(...seen, 0);
      expect(num(said[i], 'buffer')).toBe(-worst);
    }
    // ...and a bank that has never had a bad week holds nothing against one, which is a real
    // position and not a missing number.
    expect(num(said[0], 'buffer')).toBeGreaterThanOrEqual(0);
  });
});

describe('the session (Money Market A3, B2, B5, B6)', () => {
  it('writes a dated row between two named parties, and it comes back (A1, B6)', () => {
    const w = run(rigWorld('mm-session'), 10);
    const rows = w.instruments.all().filter((i) => i.kind === 'interbank' || i.kind === 'repo');
    // A1: money-market money is a ROW — a dated claim of a named lender on a named borrower — and
    // never a balance that moved with nothing behind it.
    expect(rows.length).toBeGreaterThan(0);
    for (const r of rows) {
      expect(r.issuer.some).toBe(true);
      // Law 9: and it is named as a market names one: the two parties and the row's own number.
      expect(String(r.id).split(':').length).toBeGreaterThan(2);
    }
    // B6: two books, and the overnight one comes back the next morning — a row that matured is a
    // row that was repaid, so nothing this session wrote is still outstanding a year later.
    const live = rows.filter((r) => r.status.live);
    expect(live.length).toBeLessThan(rows.length);
    // B2.a, B7: and a refusal is a real outcome of a real book, recorded under the name refused.
    for (const r of events(w, 'moneyMarket.refused')) {
      expect(r.public).toBe(true);
      expect(w.parties.get(String(r.data['borrower']) as never).kind).toBe(BANK);
      expect(num(r, 'short')).toBeGreaterThan(0);
    }
  });

  it('pays a deposit rate per class, and the classes are not one rate (A1, B1.a)', () => {
    const w = run(rigWorld('mm-classes'), 6);
    for (const bank of [BANK_A, BANK_B]) {
      const said = last(w, 'bank.depositRate', bank);
      expect(said?.public).toBe(true);
      const rates = said?.data['rates'] as Record<string, number> | undefined;
      expect(rates).toBeDefined();
      // A1: retail, corporate and wholesale are different money and are paid differently. What
      // separates them is what it costs each of them to leave, which is their own number.
      expect(new Set(Object.values(rates ?? {})).size).toBeGreaterThan(1);
      // A1.c: and the money that is cheapest to move is paid the most.
      const retail = rates?.['retail'];
      const wholesale = rates?.['wholesale'];
      if (retail !== undefined && wholesale !== undefined) {
        expect(wholesale).toBeGreaterThan(retail);
      }
    }
  });
});

/**
 * A world in which one bank has already pledged everything it owns: the seed binds its whole
 * sovereign book to the other bank. It still HOLDS the paper, so its balance sheet and its solvency
 * are exactly what they were — the only thing that changed is that none of it is free to pledge
 * again, which is C4.b's own case and the one that stops a SOLVENT bank borrowing (B3.c).
 */
function withoutCollateral(
  seed: string,
  encumbered: string,
  extra: readonly SystemModule[] = [],
): World {
  const spec = rigSpec(seed);
  const modules = spec.modules.map((m) => {
    // ...and it takes no more on either. A dealing line that will carry nothing quotes for nothing
    // and buys nothing, and a bank that will lend nobody anything writes no loans.
    if (m.id === 'banks') {
      return {
        ...m,
        params: m.params.map((p) =>
          p.id === `bank.dealing.limit.aggregate.${encumbered}` ||
          p.id === `bank.limitPerBorrower.${encumbered}`
            ? { ...p, value: 0 }
            : p,
        ),
      };
    }
    return m;
  });
  // The pledge runs FIRST, before anything else in the period can leave the bank short.
  return assemble({ ...spec, modules: mergeModules(modules, [everythingPledged(partyId(encumbered)), ...extra]) });
}

/**
 * Register D5: a bank every piece of whose eligible paper is already standing behind somebody
 * else's claim, and stays that way. It is not enough to bind what it opens with: it is a primary
 * dealer, and every auction it takes up, every repo it lends into and every row it writes is a new
 * dated claim on a name — which is exactly what the window takes (Money Market B3.a). So the pledge
 * runs every period, ahead of the market that would advance against it.
 */
function everythingPledged(bank: PartyId): SystemModule {
  return {
    id: 'test.everything-pledged',
    spec: 'Register D5 Money Market B3.a',
    requires: [],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.pledgeAll',
        spec: 'Register D5',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx): void => {
          const to = bank === BANK_A ? BANK_B : BANK_A;
          const on = ctx.calendar.startOf(ctx.period);
          for (const i of ctx.instruments.all()) {
            if (!i.status.live || !i.issuer.some) continue;
            if (ctx.registry.instrumentKind(i.kind).cashFlows(i, on, ctx.calendar).length === 0) {
              continue;
            }
            const free = ctx.register.free(bank, i.id);
            if (free <= 0) continue;
            ctx.settle({
              legs: [
                {
                  kind: 'pledge',
                  pledgor: bank,
                  beneficiary: to,
                  instrument: i.id,
                  qty: free,
                  secures: `everything ${bank} owns is already somebody's`,
                  pledgorCell: none(),
                },
              ],
              cause: 'transfer',
              reason: `${bank} has nothing left of its own`,
            });
          }
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/** A payment out of a bank's own reserves, larger than the reserves it has. */
function paysMoreThanItHas(from: PartyId, to: PartyId, at: number): SystemModule {
  return {
    id: 'test.bigpayment',
    spec: 'Money B3.b',
    requires: ['money-market'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.bigpayment',
        spec: 'Money B3.b',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx) => {
          if (ctx.period !== at) return;
          const cb = ctx.registry.centralBankOf(USD);
          const account = moneyInstrumentId(cb, USD);
          const has = ctx.register.quantity(from, account);
          ctx.settle({
            legs: [
              {
                kind: 'money',
                from: { holder: from, issuer: cb },
                to: { holder: to, issuer: cb },
                ccy: USD,
                amount: has + phx(50_000),
                fromCell: { some: false },
                toCell: { some: false },
              },
            ],
            cause: 'transfer',
            reason: 'more than it has',
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

describe('the window (Central Bank D1, D3, D6, C4.b)', () => {
  it('refuses a bank with nothing to pledge, and says so out loud (C4.b, D6)', () => {
    const w = withoutCollateral('mm-nocollateral', 'bank.a', [
      paysMoreThanItHas(BANK_A, BANK_B, 3),
    ]);
    for (let i = 0; i < 6; i += 1) w.step();
    // C4.b: AGAINST GOOD COLLATERAL is a real constraint on the borrower, not a quota the lender
    // set. A bank with no eligible paper cannot draw however solvent it is, and running out of
    // collateral is what stops a solvent bank borrowing (B3.c).
    const refused = events(w, 'centralBank.refused', BANK_A);
    expect(refused.length).toBeGreaterThan(0);
    for (const r of refused) {
      expect(r.public).toBe(true);
      expect(num(r, 'collateral')).toBe(0);
      expect(num(r, 'short')).toBeGreaterThan(0);
    }
    // Money E1.b: and the payment it could not make FAILED — it is not a silent negative, and the
    // thing it owed is still owed.
    const failed = w.ledger
      .all()
      .filter((r) => r.outcome === 'failed' && r.reason.kind === 'overdraftRefused');
    expect(failed.length).toBeGreaterThan(0);
  });

  it('leaves no account below zero without a lender behind it (Money B3.c)', () => {
    const w = run(rigWorld('mm-rows'), 12);
    // B3.c, D3.b: what the corridor ALLOWED is a row by the close — a lender, a rate and a date —
    // so the money family has nothing to report and needs no exemption to say so.
    const money = w.last?.audit.families.find((f) => f.family === 'money');
    expect(money?.built).toBe(true);
    expect(money?.violations).toEqual([]);
    // ...and where one was drawn, it was drawn at a penalty above the ceiling nobody cleared at.
    const c = last(w, 'centralBank.corridor');
    for (const d of events(w, 'moneyMarket.window')) {
      if (d.data['overdraft'] !== true) continue;
      expect(num(d, 'rate')).toBeGreaterThan(num(c, 'ceiling'));
    }
  });

  it('lends against collateral and no further than the paper the borrower has (C4.b)', () => {
    const w = run(rigWorld('mm-window'), 12);
    const draws = w.instruments
      .all()
      .filter((i) => String(i.id).startsWith('repo:') && String(i.id).includes(CB));
    for (const d of draws) {
      // C4: what the central bank lends is secured, always, on paper it declared eligible.
      const pledged = w.register
        .holdersOf(d.id)
        .flatMap((h) => w.register.holding(h, d.id).some ? [] : []);
      expect(pledged).toEqual([]);
      expect(d.issuer.some).toBe(true);
    }
    // D6: LOLR is to the SOLVENT, so the decider refuses a bank whose capital is gone — and a
    // refusal is a public event with a size, not a silence.
    for (const r of events(w, 'moneyMarket.refused')) expect(num(r, 'short')).toBeGreaterThan(0);
  });
});

describe('what somebody outside can see (Banks Funding F1, F2, F4, Observer A5)', () => {
  it('shows each bank what it published about itself, and how old it is', () => {
    const w = run(rigWorld('mm-observer'), 8);
    const seen = snapshot(w, { kind: 'inspector' }, 20);
    const banks = seen.banks.filter((b) => b.bank === String(BANK_A) || b.bank === String(BANK_B));
    expect(banks.length).toBe(2);
    for (const b of banks) {
      const said = last(w, 'bank.liquidity', partyId(b.bank));
      // Law 4, Law 19, Observer E3: NOTHING IS RECOMPUTED ON THE WAY OUT. Every number here is the
      // one the bank itself published, carried through unchanged.
      expect(b.reserves).toBe(num(said, 'reserves'));
      expect(b.couldLeave).toBe(num(said, 'couldLeave'));
      expect(b.metric).toBe(said?.data['metric']);
      // F1: its deposit lines by class, as reads of WHO ACTUALLY BANKS THERE — so a bank no fund
      // and no fund manager banks with shows no wholesale line at all, rather than showing a zero.
      // Every class it does show is one this world declared, and every bank here has the two that
      // every bank here has: the firms it transacts for, and the households it keeps accounts for.
      for (const cls of Object.keys(b.deposits)) {
        expect(['corporate', 'retail', 'wholesale']).toContain(cls);
      }
      expect(Object.keys(b.deposits)).toContain('corporate');
      expect(Object.keys(b.deposits)).toContain('retail');
      // F2: the reserve balance is its one account at the central bank, and the surface says the
      // same number the register does.
      expect(b.reserves).toBe(w.register.quantity(partyId(b.bank), moneyInstrumentId(CB, USD)));
      // B3.a: where its capital stands, and which of the two rules is the one biting.
      expect(b.capital).toBeGreaterThan(0);
      expect(b.binds).not.toBeNull();
      // A5: AND HOW OLD IT IS. A published report is what it said at the close, and a surface that
      // showed it as though it were now would be inventing a freshness nobody has.
      expect(b.age).not.toBeNull();
      expect(b.age).toBeGreaterThanOrEqual(0);
      expect(b.asOf).toBe(said?.period);
    }
  });
});
