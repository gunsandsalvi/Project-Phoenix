/**
 * The leveraged buyout: the company borrows, its own shares are bought back with what it borrowed,
 * and the buyer's cheque pays for the rest.
 *
 * @spec Private Equity B1 Private Equity B2 Private Equity B3 Private Equity B4 Private Equity B5 Private Equity E1 M&A A1 M&A A2 Banks Lending A1 Banks Lending B1 Register B3 Law 5 Law 19
 *
 * §29 B2.a is the sentence the whole design turns on: *"the debt is the TARGET's liability, not the
 * fund's — which is why a failed buyout kills the firm and not the fund."* The only two-sided way
 * for a company's own borrowing to reach its own shareholders is for it to get its shares back for
 * the money, so the tender has TWO PAYERS: the company, drawing what a lender committed, whose
 * shares come back to their issuer and cease, and the buyer, whose shares move to it.
 *
 * A test never names a party (CLAUDE.md): the target, the line and the bank are asked of the world
 * the draw made, and what is asserted is the arithmetic of the deal rather than who was in it.
 */
import { describe, expect, it } from 'vitest';
import {
  assemble,
  downTick,
  FIRM,
  instrumentId,
  mul,
  partyId,
  type InstrumentId,
  type MechanismContext,
  type PartyId,
  type SeedContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { REGION, USD } from '../src/seeds/foundation.js';
import { asCash, asPerPiece } from '../src/core/measure.js';
import { mergeModules, rigSpec } from './rig.js';
import { equityLines, runTender, type Bid } from '../src/mechanisms/control/index.js';
import { askToFund, DEAL_MONTHS } from '../src/mechanisms/control/deal.js';
import { facilityLoanId, isLoan, LOAN } from '../src/registry/credit.js';

const BUYER = partyId('buyer.lbo');

/** How far over what a holder's books carry a share the bid is struck, so that every holder sells. */
const OVER = 4;
/** What share of the price the buyer brings itself, so that the rest has to be borrowed (B3). */
const CHEQUE = 0.2;

/**
 * How far the world runs: the deal is found in period 1, its lender decides in period 2 and the
 * tender is in period 3, and the rest is room for what follows it (the owner's hand at 17b.6).
 */
const PERIODS = 6;

/**
 * Seed B1.a: THE COMPANY THE DRAW MADE, asked for rather than named — the equity line with the most
 * holders that are not the buyer, which is the one a tender has anybody to tender into.
 */
function targetIn(ctx: MechanismContext): { line: InstrumentId; target: PartyId } | undefined {
  let best: { line: InstrumentId; target: PartyId; holders: number } | undefined;
  for (const i of equityLines(ctx)) {
    if (!i.issuer.some || i.ccy !== USD) continue;
    const holders = ctx.register.holdersOf(i.id).filter((h) => h !== BUYER).length;
    if (holders < 2) continue;
    if (best !== undefined && holders <= best.holders) continue;
    best = { line: i.id, target: i.issuer.value, holders };
  }
  return best === undefined ? undefined : { line: best.line, target: best.target };
}

/** Law 19: the dearest basis anybody carries a unit of this line at, read off their own lots. */
function dearestBasis(ctx: MechanismContext, line: InstrumentId): number {
  let most = 0;
  for (const h of ctx.register.holdersOf(line)) {
    for (const holding of ctx.register.holdingsOf(h)) {
      if (holding.instrument !== line) continue;
      for (const lot of holding.lots) if (lot.basisPerUnit > most) most = lot.basisPerUnit;
    }
  }
  return most;
}

/**
 * The buyer, and the deal it does: it asks the company's lenders to commit in period 1 and runs the
 * tender in period 3, by which time a bank has decided. It brings a fifth of the price itself.
 */
function buysACompany(): SystemModule {
  /**
   * The deal's own clock. A lender will not commit against accounts that do not exist (Reporting
   * A2.a: terms nobody can test are not terms), and this world's companies close their books once a
   * quarter — thirteen weeks — so the deal starts when the target has PUBLISHED and not on a period
   * number written here.
   */
  const chosen: {
    line?: InstrumentId;
    target?: PartyId;
    needs?: number;
    asked?: number;
    done?: number;
  } = {};
  return {
    id: 'test.lbo',
    spec: 'Private Equity B2',
    requires: ['control', 'banks', 'equity', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        /**
         * §29 C2, C3: THE OWNER NEEDS MONEY, and it says so where a firm says so — before the
         * channels that read it run (an equity float reads this period's, at 51). What the company
         * it controls does about the need is what the test is for.
         */
        name: 'test.lboOwnerNeeds',
        spec: 'Private Equity C2',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [{ kind: 'event', name: 'firms.funding' }],
        run: (ctx: MechanismContext) => {
          if (chosen.done === undefined || ctx.period !== chosen.done + 1) return;
          ctx.record(
            'firms.funding',
            [BUYER],
            {
              short: 1e9,
              shortNow: 1e9,
              shortTerm: 0,
              owed: 0,
              programme: 0,
              working: 0,
              ccy: USD,
            },
            true,
          );
        },
      },
      {
        name: 'test.lbo',
        spec: 'Private Equity B2 Private Equity B3',
        // Before the banks turn this period's asks into rows, which is where the tender sits too.
        anchor: { before: 'lending.book' },
        reads: [],
        writes: [
          { kind: 'event', name: 'control.acquired' },
          { kind: 'event', name: 'control.advisory' },
          { kind: 'event', name: 'control.combined' },
          { kind: 'event', name: 'control.failed' },
          { kind: 'event', name: 'control.financing' },
          { kind: 'event', name: 'control.owned' },
          { kind: 'event', name: 'control.tender' },
          { kind: 'event', name: 'credit.request' },
          { kind: 'event', name: 'tender.unfilled' },
        ],
        run: (ctx: MechanismContext) => {
          if (chosen.asked === undefined) {
            const found = targetIn(ctx);
            if (found === undefined) return;
            const outstanding = ctx.register.heldTotal(found.line).value;
            const price = mul(dearestBasis(ctx, found.line), OVER, 'well over what anybody paid');
            const needs = downTick(outstanding);
            const cost = mul(price, needs, 'what all of it would cost at that price');
            chosen.line = found.line;
            chosen.target = found.target;
            chosen.needs = needs;
            chosen.asked = ctx.period;
            // B2: it brings a fifth and asks the company's lenders to commit the rest.
            askToFund(
              ctx,
              found.target,
              asCash(cost * (1 - CHEQUE), USD, 'what the deal is short of'),
              asCash(cost * CHEQUE, USD, 'what the buyer brings itself'),
              BUYER,
              ctx.params.months(DEAL_MONTHS),
            );
            return;
          }
          const { line, target } = chosen;
          if (line === undefined || target === undefined || chosen.needs === undefined) return;
          // It asked in one period, its lender decided in the next, and it tenders in the one after.
          if (chosen.done !== undefined || ctx.period < chosen.asked + 2) return;
          chosen.done = ctx.period;
          const bid: Bid = {
            buyer: BUYER,
            target,
            line,
            price: asPerPiece(
              mul(dearestBasis(ctx, line), OVER, 'well over what anybody paid'),
              'what it offers for one',
            ),
            // A1: control, and it asks for the whole of it so that a take-private is reachable.
            needs: downTick(ctx.register.heldTotal(line).value),
            ccy: USD,
          };
          runTender(ctx, bid, [bid]);
        },
      },
    ],
    seed(ctx: SeedContext) {
      ctx.parties.add({
        id: BUYER,
        kind: FIRM,
        region: REGION,
        name: 'A buyout fund',
        bank: partyId('bank.a'),
        representation: 'named',
        status: { alive: true, standing: 'good' },
      });
      ctx.endowMoney(BUYER, USD, asCash(1e12, USD, 'its own money, and it is not the deal'));
    },
    participants: [],
    families: [],
  };
}

function buyoutWorld(seed = 'lbo'): World {
  const spec = rigSpec(seed);
  return assemble({ ...spec, modules: mergeModules(spec.modules, [buysACompany()]) });
}

describe('the tender has two payers (Private Equity B2, B2.a, B3, B4, B5)', () => {
  it('the company borrows, buys its own shares back, and the buyer pays the rest', () => {
    const w = buyoutWorld();
    for (let i = 0; i < PERIODS; i += 1) w.step();
    const done = w.journal.ofKind('control.acquired');
    expect(done.length).toBeGreaterThan(0);
    const e = done[0];
    if (e === undefined) return;
    const drawn = Number(e.data['drawn']);
    const cheque = Number(e.data['cheque']);
    const paid = Number(e.data['paid']);
    // B2: most of the price is debt raised against the target itself.
    expect(drawn).toBeGreaterThan(0);
    // B3: and the equity cheque is the rest.
    expect(cheque).toBeGreaterThan(0);
    // B5: sources and uses balance exactly — what the sellers were paid is what was drawn plus what
    // the buyer put up, leg by leg, with nothing left over anywhere.
    expect(drawn + cheque).toBeCloseTo(paid, 6);
  });

  it('the debt is the TARGET’s liability and nobody else’s (B2.a)', () => {
    const w = buyoutWorld();
    for (let i = 0; i < PERIODS; i += 1) w.step();
    const e = w.journal.ofKind('control.acquired')[0];
    if (e === undefined) throw new Error('the deal did not happen');
    const target = partyId(String(e.data['target']));
    // The deal's own row, which is the one the commitment names — the company may well have an
    // ordinary working-capital line beside it, and that one is nothing to do with this.
    const row = w.instruments
      .all()
      .find(
        (i) =>
          i.kind === LOAN &&
          i.issuer.some &&
          i.issuer.value === target &&
          isLoan(i.terms) &&
          String(i.id) === String(facilityLoanId(i.terms.originator, target)),
      );
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    // The company owes it; the buyer owes nothing at all, which is why a failed buyout would kill
    // the firm and not the fund.
    expect(row.terms.borrower).toBe(target);
    // It was drawn on the commitment made for this deal, and the row is the one the lender's own
    // headroom read names — one row, two readers (Law 4). What is OUTSTANDING on it now is a moving
    // number: a borrower with money over what it needs pays its dearest line down (17.9a).
    expect(row.terms.originator).not.toBe(BUYER);
    expect(
      w.instruments.all().filter((i) => i.kind === LOAN && i.issuer.some && i.issuer.value === BUYER),
    ).toHaveLength(0);
  });

  it('what the company paid for ceases, so the buyer holds a majority of what is left (B4)', () => {
    const w = buyoutWorld();
    for (let i = 0; i < PERIODS; i += 1) w.step();
    const e = w.journal.ofKind('control.acquired')[0];
    if (e === undefined) throw new Error('the deal did not happen');
    const line = instrumentId(String(e.subjects[2]));
    const held = w.register.quantity(BUYER, line);
    const inIssue = w.instruments.get(line).issued;
    // A1, B4: ownership changed in the register, and the majority is arithmetic — the buyer bought
    // some of the shares and the company retired the rest, so the denominator is what is left.
    expect(held).toBeGreaterThan(0);
    expect(held * 2).toBeGreaterThan(inIssue);
  });
});

describe('sources and uses are measured, not assumed (Private Equity B5)', () => {
  it('the family reports nothing on a deal that balanced', () => {
    const w = buyoutWorld();
    let said: string[] = [];
    for (let i = 0; i < PERIODS; i += 1) {
      const report = w.step();
      said = report.audit.families
        .filter((f) => f.family === 'flows')
        .flatMap((f) => f.violations)
        .filter((v) => v.spec === 'Private Equity B5')
        .map((v) => v.message);
    }
    // It balances by construction — every fill is one instruction paying a named seller out of one
    // of two named accounts — and the family is what would say so the day it stopped.
    expect(w.journal.ofKind('control.acquired').length).toBeGreaterThan(0);
    expect(said).toEqual([]);
  });

  it('is BUILT, and the deal leaves it the three numbers it reads', () => {
    // Audit C2: an unbuilt family reports "not built" and never green, so a family that is going to
    // hold has to say it is built — and it has to have something to read. The deal's own record
    // carries what the sellers were paid and the two places it came from, under those names.
    const w = buyoutWorld();
    let built = false;
    for (let i = 0; i < PERIODS; i += 1) {
      const report = w.step();
      built = report.audit.families.some((f) => f.family === 'flows' && f.built);
    }
    expect(built).toBe(true);
    const e = w.journal.ofKind('control.acquired')[0];
    if (e === undefined) throw new Error('the deal did not happen');
    for (const key of ['paid', 'drawn', 'cheque']) expect(typeof e.data[key]).toBe('number');
  });
});

describe('the owner’s hand (Private Equity C2, C3)', () => {
  it('a company its owner needs money from asks for it, in its own name', () => {
    const w = buyoutWorld();
    for (let i = 0; i < PERIODS + 6; i += 1) w.step();
    const asks = w.journal
      .ofKind('control.financing')
      .filter((e) => Number(e.data['cheque']) === 0);
    expect(asks.length).toBeGreaterThan(0);
    const e = asks[0];
    if (e === undefined) return;
    // C2, C3: the borrowing is the COMPANY's — a request says who will owe it — and the reason is
    // its OWNER's, which is why the cheque is nothing: nobody is buying anything, the owner already
    // owns it. The buyer named beside it is the controller whose need this is.
    expect(String(e.data['buyer'])).toBe(String(BUYER));
    expect(Number(e.data['wanted'])).toBeGreaterThan(0);
    const target = String(e.data['target']);
    expect(w.control.controllerOf(partyId(target))?.controller).toBe(BUYER);
  });

  it('and whether it gets it is the credit market’s answer, not the owner’s (B2.b)', () => {
    const w = buyoutWorld();
    for (let i = 0; i < PERIODS + 6; i += 1) w.step();
    // 17b.6 said this would happen and it does: the company that has just been levered to buy
    // itself asks for more, and its lender reads the accounts it has just prepared — no earnings
    // over the periods it was bought in — and does not commit. A recapitalisation is not a thing an
    // owner can take; it is a thing a lender agrees to, which is B2.b reaching back into C3.
    const funded = w.journal.ofKind('control.recapitalised');
    const committed = w.journal.ofKind('credit.committed');
    // One commitment in this world and it is the BUYOUT's: the deal that was fundable was funded.
    expect(committed).toHaveLength(1);
    expect(committed[0]?.data['increased']).toBe(false);
    expect(funded).toHaveLength(0);
  });
});

