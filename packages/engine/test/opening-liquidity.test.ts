/**
 * The banking system opens meeting its own liquidity standard, and what it took to get there.
 *
 * @spec Banks Funding C1 Banks Funding C1.a Banks Funding C2 Banks Funding C2.a Banks Funding D1 Banks Funding D4 Banks Funding D4.a Banks Capital B1.b Banks Capital D4 Banks Capital D6 Central Bank C1 Central Bank C1.a Equity A1 Equity B3 Money Market C1 Register F2 Seed A3 Seed B1 Seed E1 Law 2 Law 8 XI-15
 *
 * A bank lends what it holds beyond what could leave it (D4, D4.a), so a bank that opens below that
 * line lends nothing on the first morning and everything downstream of credit is dark. Item 11.5's
 * whole content is that the opening satisfies the rule the world is then measured against — and
 * that no number was chosen to make it so.
 *
 * WHAT THE CAUSE TURNED OUT TO BE, and this file is where it is asserted: not the reserve-to-paper
 * mix the plan expected, but WHO HELD THE EQUITY FLOAT. A share raises nothing at the central bank's
 * window (C1.a), so a desk opening with the whole float of every line it makes put four times its
 * own capital of unfundable asset on the balance sheet whose liquidity turns on exactly that. With
 * the float where its holders are (B3: a saver holds a claim on a firm's earnings), every bank in
 * every seed opens above the line, at two banks and at twenty.
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  HOUSEHOLD,
  USD,
  assemble,
  moneyInstrumentId,
  none,
  partyId,
  some,
  type Event,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { defended } from '../src/mechanisms/banks/treasury.js';
import { rigWorld, rigSpec, rigDraw, listedIn, mergeModules } from './rig.js';
import { unexpected } from './expected.js';

/** What a bank published about itself this period, read back rather than recomputed (Law 19, F4). */
function liquidity(
  w: World,
  period: number,
): { bank: string; liquid: number; couldLeave: number }[] {
  return w.journal
    .ofKind('bank.liquidity')
    .filter((e: Event) => e.period === period)
    .map((e: Event) => ({
      bank: String(e.data['bank']),
      liquid: Number(e.data['liquid']),
      couldLeave: Number(e.data['couldLeave']),
    }));
}

describe('the opening liquidity position (Banks Funding C1, C2, D4)', () => {
  it('opens every bank with liquid assets that cover what could leave it, at two banks and at twenty', () => {
    // XI-15, Seed B1: THE COUNT OF BANKS IS NOT LOAD-BEARING. Before this item, a world of two or
    // three opened above the line and a world of six or ten did not, because which banks made a
    // market in a listed line was the draw's business — so whether the banking system opened able
    // to lend at all was luck. What decides it now is each bank's own capital line against the
    // haircut on the paper it holds, and neither of those is a count of banks.
    for (const banks of [2, 3, 6, 20]) {
      const w = rigWorld('opening-liquidity', banks, banks * 4);
      w.step();
      const rows = liquidity(w, 1);
      expect(rows.length).toBe(banks);
      for (const r of rows) {
        expect(r.couldLeave).toBeGreaterThan(0);
        expect(r.liquid).toBeGreaterThanOrEqual(r.couldLeave);
      }
    }
  });

  it('is the same at every seed, because nothing in it was chosen by reading an answer (Seed E1)', () => {
    for (const seed of ['probe', 'year', 'cap-a']) {
      const w = rigWorld(seed, 6, 24);
      w.step();
      for (const r of liquidity(w, 1)) expect(r.liquid).toBeGreaterThanOrEqual(r.couldLeave);
    }
  });

  it('quotes credit from period one, which is what having room MEANS (D4.a)', () => {
    const w = rigWorld('opening-credit', 6, 24);
    w.step();
    const quoted = w.journal.ofKind('credit.quoted').filter((e: Event) => e.period === 1);
    expect(quoted.length).toBeGreaterThan(0);
    // And the banks quoting are not one bank: a world where only the best-capitalised name lends is
    // a world with one lender in it, whatever its table says.
    expect(new Set(quoted.flatMap((e: Event) => e.subjects.map(String))).size).toBeGreaterThan(1);
  });
});

describe('who opens holding a listed line (Equity A1, B3; Seed E1)', () => {
  it('gives the float to the savers and none of it to the desks that make its market', () => {
    const draw = rigDraw('float', 6, 40);
    const line = listedIn(draw);
    const w = rigWorld('float', 6, 40);
    const instrument = w.instruments
      .all()
      .find((i) => String(i.id).includes(line) && String(i.kind) === 'equity.share');
    expect(instrument).toBeDefined();
    const id = instrument!.id;
    // Not one piece of it on a bank's book: a dealer's inventory is a position it takes by trading,
    // and at period zero nobody has traded anything (Seed E1).
    for (const p of w.parties.ofKind(BANK)) expect(w.register.quantity(p.id, id)).toBe(0);
    // And every piece of it has a named holder: what the members hold IS the line (Law 8).
    let held = 0;
    for (const cell of w.parties.ofKind(HOUSEHOLD)) {
      held += w.register.quantity(cell.id, id) * (cell.representation === 'cell' ? cell.weight : 1);
    }
    expect(held).toBe(w.instruments.get(id).issued);
    expect(held).toBeGreaterThan(0);
  });

  it('leaves every desk inside its own limit on the first morning (Dealer Desks D1, D4)', () => {
    const w = rigWorld('float', 6, 40);
    for (let i = 0; i < 3; i += 1) w.step();
    const books = w.journal.ofKind('bank.dealing').filter((e: Event) => e.period === w.period - 1);
    expect(books.length).toBeGreaterThan(0);
    // D5: room left is a number and it can be negative — a desk carrying more than its own limit
    // has less than none. Before this item every desk in every seed opened there, so D4's "shrinks
    // the bid to whichever limit binds" could only ever report the aggregate one.
    for (const b of books) expect(Number(b.data['roomLeft'])).toBeGreaterThanOrEqual(0);
  });
});

describe('what a bank pays to keep a class of deposit (Banks Funding D1, Banks Capital D4)', () => {
  it('stops defending at what that class is worth to it, net of the guarantee on it', () => {
    // The bank's all-in cost of an insured deposit is the rate plus the premium. A rival showing
    // 0.019 on money worth 0.020 costs this bank 0.021 all-in if it matches, so it does not: it
    // funds itself in the market instead, which is the alternative D1 names.
    const worth = 0.02;
    const premium = 0.002;
    const own = 0.0175;
    expect(defended(some(0.019), own, worth - premium)).toBe(own);
    // And it does match inside that point, because a deposit it can fund more cheaply than the
    // market is one it wants (B1.a).
    expect(defended(some(0.0179), own, worth - premium)).toBe(0.0179);
    // An uninsured class has no premium, so its stopping point is what the money is worth (B1.a).
    expect(defended(some(0.019), own, worth)).toBe(0.019);
  });
});

/** XI-1: a loss is an EVENT — a penalty in money, to a payee with a name, big enough to end a bank. */
function penalty(at: number, weeks: number, share: number): SystemModule {
  let each = 0;
  return {
    id: 'test.penalty',
    spec: 'XI-1',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.penalty',
        spec: 'XI-1',
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          if (ctx.period < at || ctx.period >= at + weeks) return;
          const bank = partyId('bank.b');
          const p = ctx.parties.get(bank);
          if (!p.status.alive) return;
          const cb = ctx.registry.centralBankOf(USD);
          if (each === 0) {
            let assets = 0;
            for (const h of ctx.register.holdingsOf(bank)) {
              assets += ctx.valuation.valueOfLots(h.instrument, h.lots, ctx.period);
            }
            each = Math.round(assets * share);
          }
          ctx.settle({
            legs: [
              {
                kind: 'money',
                from: { holder: bank, issuer: cb },
                to: { holder: partyId('treasury.us'), issuer: cb },
                ccy: USD,
                amount: each,
                fromCell: none(),
                toCell: none(),
              },
            ],
            cause: 'transfer',
            reason: 'a penalty this bank cannot pay',
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

describe('what a failed bank leaves behind (Banks Capital D6, Register F2, Money B3)', () => {
  it('takes its reserve overdraft with it, because a liability has a destination too', () => {
    // Appendix B: no death without a destination, applied to a LIABILITY. A bank whose reserve
    // account was overdrawn when it failed used to keep the overdraft: `money` reported a borrowing
    // with no lender row behind it and `names` a ceased party still holding something, every period
    // for the rest of the run, and neither number ever moved again.
    const spec = rigSpec('overdraft-destination', 4, 16);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [penalty(6, 4, 0.35)]) });
    for (let i = 0; i < 14; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const ceased = w.parties.all().filter((p) => !p.status.alive && p.kind === BANK);
    expect(ceased.length).toBeGreaterThan(0);
    for (const p of ceased) {
      for (const h of w.register.holdingsOf(p.id)) {
        expect(w.register.quantity(p.id, h.instrument)).toBe(0);
      }
    }
    const cb = w.registry.centralBankOf(USD);
    for (const p of ceased) expect(w.register.quantity(p.id, moneyInstrumentId(cb, USD))).toBe(0);
  });
});
