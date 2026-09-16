/**
 * Pensions: the payroll carries the contributions, the fund promises its retired members a
 * schedule the kernel marks at the curve, and a shortfall is called from the sponsors
 * (Insurers A2, A4, B1, B2.a, B3, D1, D3, Households F3, Law 5, 14.6).
 *
 * @spec Insurers A2 Insurers A4 Insurers B1 Insurers B2.a Insurers B3 Insurers D1 Insurers D3 Households F3 Law 5 Law 8 Law 19
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD, assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { FUNDED, PENSION, PENSION_PAID, SPONSOR_CALLED, pensionFundIdFor, pensionSchedule, presentValueOf } from '../src/mechanisms/insurers/index.js';
import { PENSION_DIM, PENSION_MEMBER, PENSION_PARAMS, isPensionTerms, sponsorshipOf } from '../src/registry/insurance.js';
import { arrearTerms, isArrear } from '../src/register/arrears.js';
import { asPerPiece, asRatio } from '../src/core/measure.js';
import { asQty, downTick } from '../src/core/tick.js';
import { isMoneyLeg } from '../src/ledger/instruction.js';
import { weightOf } from '../src/parties/party.js';
import type { Civil } from '../src/calendar/civil.js';
import { mergeModules, rigSpec, rigWorld } from './rig.js';

const contributionsTo = (w: ReturnType<typeof rigWorld>, fund: string, upTo: number): number => {
  let total = 0;
  for (let p = 1; p <= upTo; p += 1) {
    for (const r of w.ledger.inPeriod(p as never)) {
      if (r.outcome !== 'settled') continue;
      for (const l of r.instruction.legs) {
        if (l.kind === 'money' && l.receipt?.of === 'contribution' && String(l.to.holder) === fund) total += l.amount;
      }
    }
  }
  return total;
};

describe('the payroll carries the contributions (14.6)', () => {
  it('a wage instruction has the member’s and the employer’s legs to the fund of the place, the payer sponsors it, the worker is a member, and the fund holds what arrived', () => {
    const w = rigWorld('pensions');
    const periods = 4;
    for (let i = 0; i < periods; i += 1) w.step();
    const funds = w.parties.ofKind(PENSION).filter((p) => p.status.alive);
    expect(funds.length).toBeGreaterThan(0);
    let seen = 0;
    for (let p = 1; p <= periods; p += 1) {
      for (const r of w.ledger.inPeriod(p as never)) {
        if (r.outcome !== 'settled') continue;
        const legs = r.instruction.legs.filter(isMoneyLeg);
        const wage = legs.find((l) => l.receipt?.of === 'wage');
        const into = legs.filter((l) => l.receipt?.of === 'contribution');
        if (wage === undefined || into.length === 0) continue;
        seen += 1;
        // Law 5: one instruction — the wage net of the member's share, and both shares to the fund
        // of the payer's place, all out of the payer's account.
        expect(into).toHaveLength(2);
        const payer = w.parties.get(wage.from.holder);
        const fund = pensionFundIdFor(payer.region);
        for (const leg of into) {
          expect(String(leg.from.holder)).toBe(String(wage.from.holder));
          expect(String(leg.to.holder)).toBe(String(fund));
          expect(leg.amount).toBeGreaterThan(0);
        }
        // Law 8: the member's share is whole pieces a member, so it is at most the share of the
        // gross wage.
        const employee = into[0]?.amount ?? 0;
        const gross = wage.amount + employee;
        expect(employee).toBeLessThanOrEqual(gross * w.params.ratio(PENSION_PARAMS.employeeShare));
        // D3: the payer stands behind the fund its payroll feeds.
        const sponsorship = sponsorshipOf(w.agreements, wage.from.holder);
        expect(sponsorship).toBeDefined();
        expect(String(sponsorship?.creditor)).toBe(String(fund));
        // A2: whoever was paid the wage is a member, for life.
        const worker = w.parties.resolve(wage.to.holder);
        expect(worker.representation === 'cell' ? worker.key[PENSION_DIM] : undefined).toBe(PENSION_MEMBER);
      }
    }
    expect(seen).toBeGreaterThan(0);
    // Law 19, E2: nothing reached the fund that was not a contribution, and what it holds is what
    // came in less what it paid or put to work — every piece traceable on the ledger.
    for (const fund of funds) {
      const ccy = w.registry.currencyOf(fund.region);
      let inflow = 0;
      let outflow = 0;
      for (let p = 1; p <= periods; p += 1) {
        for (const r of w.ledger.inPeriod(p as never)) {
          if (r.outcome !== 'settled') continue;
          const money = r.instruction.legs.filter(isMoneyLeg);
          // A change of bank moves the fund's own balance from the old account to the new in one
          // instruction: the one unclassified money that may reach it, and neither in nor out.
          const ownMove = money.some((l) => l.from.holder === fund.id) && money.some((l) => l.to.holder === fund.id);
          if (ownMove) continue;
          for (const l of money) {
            if (l.to.holder === fund.id) {
              expect(l.receipt?.of).toBe('contribution');
              inflow += l.amount;
            }
            if (l.from.holder === fund.id) outflow += l.amount;
          }
        }
      }
      expect(inflow).toBe(contributionsTo(w, String(fund.id), periods));
      expect(inflow).toBeGreaterThan(0);
      expect(w.participantView(fund.id).cash(ccy)).toBe(inflow - outflow);
    }
    // Nobody the seed placed is a member until a payroll enrols them: every member cell has a hire behind it.
    for (const cell of w.parties.ofKind(HOUSEHOLD)) {
      if (cell.representation !== 'cell' || cell.key[PENSION_DIM] !== PENSION_MEMBER) continue;
      expect(w.journal.ofKind('pension.enrolled').length).toBeGreaterThan(0);
    }
  });
});

describe('the promise, the pension, the mark and the call (14.6)', () => {
  it('a retired cell of members is owed a row the kernel marks, is paid its pension each period, and the shortfall is called from the sponsors', () => {
    let retired: string | undefined;
    let members = 0;
    const probe: SystemModule = {
      id: 'test.retire',
      spec: 'Households F3',
      requires: ['insurers', 'households', 'labour'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.retire',
          spec: 'Households F3',
          anchor: { before: 'pensions.promise' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 3) return;
            // The crossing, written by hand: a cell of members reaches the last cohort this period
            // rather than whenever the draw's ages next carry one over the boundary.
            const last = ctx.registry.cohorts[ctx.registry.cohorts.length - 1];
            const cell = ctx.parties.ofKind(HOUSEHOLD).find(
              (p) => p.representation === 'cell' && p.status.alive && p.key[PENSION_DIM] === PENSION_MEMBER && p.key['estate'] === 'living' && last !== undefined && p.key['cohort'] !== String(last.id),
            );
            if (cell === undefined || last === undefined) return;
            members = weightOf(cell);
            retired = String(ctx.cells.reKey(cell.id, members, { cohort: String(last.id) }, 'retired by hand'));
          },
        },
      ],
      participants: [],
      families: [],
    };
    const spec = rigSpec('pensions-promise');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 6; i += 1) w.step();
    expect(retired).toBeDefined();
    if (retired === undefined) return;
    const cell = w.parties.resolve(retired as never);
    const fund = pensionFundIdFor(cell.region);
    // A2, B1, E1: one row, from the fund to the named cell, a pension on its terms.
    const rows = w.agreements.owedTo(cell.id).filter((a) => a.state === 'performing' && a.debtor === fund && isPensionTerms(a.terms));
    expect(rows).toHaveLength(1);
    const row = rows[0];
    if (row === undefined) return;
    // A4, Households F3, Law 8: paid every period from the one after the promise — the members
    // alive times a whole number of pieces a member — and the money moved.
    const paid = w.journal.ofKind(PENSION_PAID).filter((e) => e.data['row'] === row.id);
    expect(paid.length).toBeGreaterThan(0);
    for (const e of paid) {
      expect(e.data['paid']).toBe(true);
      expect(Number(e.data['amount'])).toBe(Number(e.data['members']) * Number(e.data['perMember']));
      expect(Number(e.data['perMember'])).toBeGreaterThan(0);
      expect(Number.isSafeInteger(Number(e.data['perMember']))).toBe(true);
      expect(Number(e.data['members'])).toBeLessThanOrEqual(members);
    }
    // B2, E3: the kernel marked the row at the curve, and the mark is what the fund owes — the
    // funding read is assets at mark over promises at the curve, and the fund wears the gap (A2.a).
    const mark = w.agreements.markOf(row.id);
    expect(mark).toBeDefined();
    expect(mark ?? 0).toBeGreaterThan(0);
    const funded = w.journal.ofKind(FUNDED).filter((e) => e.data['fund'] === fund);
    const latest = funded[funded.length - 1];
    expect(latest).toBeDefined();
    expect(Number(latest?.data['promises'])).toBe(mark);
    expect(Number(latest?.data['equity'])).toBeLessThan(0);
    expect(Number(latest?.data['ratio'])).toBeLessThan(1);
    // D3, Law 5: the shortfall is called from the sponsors — a recovery period's share, apportioned
    // on what each payroll carried in — and every call is either money that moved or an arrear the
    // sponsor now owes the fund, ranking as a contribution.
    const calls = w.journal.ofKind(SPONSOR_CALLED).filter((e) => e.data['fund'] === fund && e.data['sponsor'] !== undefined);
    expect(calls.length).toBeGreaterThan(0);
    const byPeriod = new Map<number, number>();
    for (const c of calls) byPeriod.set(c.period, (byPeriod.get(c.period) ?? 0) + Number(c.data['amount']));
    for (const c of calls) {
      const share = Number(c.data['shortfall']) / w.params.periods(PENSION_PARAMS.recoveryPeriods);
      expect(byPeriod.get(c.period) ?? 0).toBeLessThanOrEqual(downTick(share));
      if (c.data['paid'] === true) continue;
      const arrears = w.instruments.issuedBy(String(c.data['sponsor']) as never).filter((i) => i.status.live && isArrear(i) && arrearTerms(i).class === 'contribution');
      expect(arrears.length).toBeGreaterThan(0);
    }
  });
});

describe('the schedule has duration (Insurers B1, B2.a, B3)', () => {
  const day = (n: number): Civil => ({ y: 2030 + Math.floor(n / 12), m: (n % 12) + 1, d: 1 });
  const dates = Array.from({ length: 24 }, (_, n) => day(n));
  it('a higher mortality expects fewer payments, and a lower rate makes the same promise worth more', () => {
    const pension = asQty(1_000, 'a pension');
    const certain = pensionSchedule(pension, asRatio(0, 'nobody dies'), dates);
    const mortal = pensionSchedule(pension, asRatio(0.01, 'one in a hundred a period'), dates);
    expect(certain.every((f) => f.perUnit === 1_000)).toBe(true);
    for (let n = 1; n < mortal.length; n += 1) {
      const before = mortal[n - 1];
      const now = mortal[n];
      if (before === undefined || now === undefined) continue;
      expect(now.perUnit).toBeLessThan(before.perUnit);
    }
    const at = (rate: number) => (d: Civil): ReturnType<typeof asRatio> => asRatio(1 / (1 + rate) ** (d.y - 2030 + (d.m - 1) / 12), 'discount');
    const dear = presentValueOf(certain, at(0.05));
    const cheap = presentValueOf(certain, at(0.01));
    expect(cheap).toBeGreaterThan(dear);
    expect(dear).toBeLessThan(asPerPiece(24_000, 'undiscounted'));
  });
});
