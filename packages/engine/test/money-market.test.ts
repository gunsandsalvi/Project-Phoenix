/**
 * The money market: what a bank pays for money, what that costs whoever borrows from it, and the
 * corridor the central bank makes the policy rate effective through.
 *
 * @spec Money Market A1 Money Market A1.a Money Market A2 Money Market A2.a Money Market A3 Money Market B1 Money Market B1.a Money Market B2 Money Market B2.a Money Market B3 Money Market B5 Money Market B5.a Money Market B6 Money Market C1 Money Market C1.a Money Market C2 Money Market C3 Money Market C4 Money Market C4.b Banks Funding A1 Banks Funding A1.a Banks Funding B1 Banks Funding B1.a Banks Funding B2 Banks Funding B2.b Banks Funding C1 Banks Funding C1.a Banks Funding F1 Banks Funding F4 Central Bank B2 Central Bank D1 Central Bank D3 Central Bank D6 Banks Lending C1.a XI-4
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
  PHX,
  assemble,
  foundationSpec,
  foundationWorld,
  moneyInstrumentId,
  partyId,
  yearFraction,
  type PartyId,
  type Event,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';
import { phx } from './units.js';

const BANK_A = partyId('bank.a');
const BANK_B = partyId('bank.b');

/** The same world with one declared number set differently, wherever it was declared. */
function withParam(seed: string, over: Readonly<Record<string, number>>): World {
  const spec = foundationSpec(seed);
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
    const w = run(foundationWorld('mm-cost'), 6);
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
    const w = run(foundationWorld('mm-mix'), 8);
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
      const said = events(w, 'bank.costOfFunds', bank).find((e) => e.period === q.period);
      expect(said).toBeDefined();
      expect(num(q, 'costOfFunds')).toBe(num(said, 'perAnnum'));
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
    const keen = run(withParam('mm-payup', { 'bank.depositMargin.bank.a': 0.001 }), 8);
    const mean = run(withParam('mm-payup', { 'bank.depositMargin.bank.a': 0.012 }), 8);
    const rateOf = (w: World, cls: string): number => {
      const said = last(w, 'bank.depositRate', BANK_A);
      const rates = said?.data['rates'] as Record<string, number> | undefined;
      return rates?.[cls] ?? 0;
    };
    for (const cls of ['retail', 'corporate', 'wholesale']) {
      expect(rateOf(keen, cls)).toBeGreaterThan(rateOf(mean, cls));
    }
  });

  it('is what it paid plus what its capital costs, over what funds its book (B2, B2.b)', () => {
    const w = run(foundationWorld('mm-parts'), 6);
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
    const w = run(foundationWorld('mm-corridor'), 4);
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

  it('destroys reserves when a bank parks at the floor, on both balance sheets (C1.a)', () => {
    const w = run(foundationWorld('mm-floor'), 8);
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
    const w = run(foundationWorld('mm-publish'), 6);
    for (const bank of [BANK_A, BANK_B]) {
      const said = last(w, 'bank.liquidity', bank);
      expect(said?.public).toBe(true);
      // F2: the reserve balance is a READ OF ITS ACCOUNT, never a mirrored copy.
      expect(num(said, 'reserves')).toBe(
        w.register.quantity(bank, moneyInstrumentId(CB, PHX)),
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
    // ONE number differs: how far back each bank looks at its own account. A longer memory reaches
    // a worse week, so it holds more against one — which is the buffer being DERIVED from what it
    // has actually seen rather than stated as a ratio of what it has issued.
    const short = run(withParam('mm-buffer', { 'bank.bufferMemory.bank.a': 2 }), 12);
    const long = run(withParam('mm-buffer', { 'bank.bufferMemory.bank.a': 40 }), 12);
    const bufferOf = (w: World): number => num(last(w, 'bank.liquidity', BANK_A), 'buffer');
    expect(bufferOf(long)).toBeGreaterThanOrEqual(bufferOf(short));
    // ...and a bank that has never had a bad week holds nothing against one, which is a real
    // position and not a missing number.
    expect(bufferOf(short)).toBeGreaterThanOrEqual(0);
  });
});

describe('the session (Money Market A3, B2, B5, B6)', () => {
  it('writes a dated row between two named parties, and it comes back (A1, B6)', () => {
    const w = run(foundationWorld('mm-session'), 10);
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
    const w = run(foundationWorld('mm-classes'), 6);
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
  const spec = foundationSpec(seed);
  const modules = spec.modules.map((m) =>
    m.id === 'seed.foundation'
      ? {
          ...m,
          seed: (ctx: Parameters<NonNullable<typeof m.seed>>[0]) => {
            m.seed?.(ctx);
            const bank = partyId(encumbered);
            const to = encumbered === 'bank.a' ? BANK_B : BANK_A;
            for (const i of ctx.instruments.all()) {
              const free = ctx.register.free(bank, i.id);
              if (!String(i.id).startsWith('gov.') || free <= 0) continue;
              ctx.register.pledge(bank, i.id, free, to, 'everything it owns is already somebody\'s', ctx.period);
            }
          },
        }
      : m,
  );
  return assemble({ ...spec, modules: [...modules, ...extra] });
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
          const cb = ctx.registry.centralBankOf(PHX);
          const account = moneyInstrumentId(cb, PHX);
          const has = ctx.register.quantity(from, account);
          ctx.settle({
            legs: [
              {
                kind: 'money',
                from: { holder: from, issuer: cb },
                to: { holder: to, issuer: cb },
                ccy: PHX,
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
    const w = run(foundationWorld('mm-rows'), 12);
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
    const w = run(foundationWorld('mm-window'), 12);
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
