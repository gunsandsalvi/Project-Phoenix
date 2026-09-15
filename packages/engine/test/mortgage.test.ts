/**
 * A mortgage is a row per (lender, cell), asked for every period a cell is short of roofs, and
 * foreclosed on a default and nothing else (Housing C1, C2, C4, XI-1, 12a.4).
 *
 * @spec Housing C1 Housing C2 Housing C4 XI-1 XI-15 Register F2
 */
import { describe, expect, it } from 'vitest';
import { DWELLING, HOUSEHOLD, LOAN, assemble, none, some, type LoanTerms, type MechanismContext, type SystemModule } from '../src/index.js';
import { goodId } from '../src/registry/physical.js';
import { addMonths } from '../src/calendar/civil.js';
import { asPerPiece, asRatio } from '../src/core/measure.js';
import { mergeModules, rigFor, rigSpec, rigWorld } from './rig.js';

describe('a cell asks for a mortgage while it is short of roofs, and a lender forecloses only on a default (12a.4)', () => {
  it('asks in more than one period, and every foreclosure names a row a default was recorded on', () => {
    const w = rigWorld('mortgage');
    const periodsAsked = new Map<string, Set<number>>();
    for (let i = 0; i < 8; i += 1) {
      w.step();
      for (const e of w.journal.ofKindIn('credit.request', w.period)) {
        const who = e.subjects[0];
        if (who === undefined || w.parties.get(who as never).kind !== HOUSEHOLD) continue;
        if (!JSON.stringify(e.data['security'] ?? []).includes('dwelling')) continue;
        const had = periodsAsked.get(who) ?? new Set<number>();
        had.add(w.period);
        periodsAsked.set(who, had);
      }
    }
    // C1, XI-15: "one mortgage at a time" is not a cell's rule — a cell short of roofs asks again.
    expect([...periodsAsked.values()].some((s) => s.size > 1)).toBe(true);
    // C4, XI-1: a foreclosure follows a default on the row, never the borrower being gone.
    const defaulted = new Set(w.journal.ofKind('credit.default').map((e) => String(e.data['instrument'])));
    for (const e of w.journal.ofKind('housing.foreclosed')) expect(defaulted.has(String(e.data['loan']))).toBe(true);
  });
});

/**
 * 12a.9: A MORTGAGE IS SERVICED, AND A CELL THAT CANNOT FAILS AND ITS DWELLING PASSES THROUGH THE
 * LIEN. No bank in the scale model quotes a household name (12a.4), so the row is built here out
 * of the doors any module has — a lender, a borrower cell that holds roofs, a schedule and what it
 * is secured on — and the housing module charges it, the kernel presents it, and the lender
 * forecloses on the default the kernel records.
 */
const built = { loan: '', cell: '', bank: '', dwelling: '', drains: 6 };
const lend: SystemModule = {
  id: 'test.mortgageRow',
  spec: 'Housing C2',
  requires: ['housing'],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [],
  phases: [
    {
      name: 'test.mortgageRow',
      spec: 'Housing C2',
      anchor: { after: 'markets' },
      reads: [],
      writes: [],
      run: (ctx: MechanismContext) => {
        if (ctx.period === 2) {
          const cell = ctx.parties.ofKind(HOUSEHOLD).find((p) => {
            if (p.representation !== 'cell' || !p.status.alive || p.key['estate'] !== 'living') return false;
            const id = goodId(DWELLING, p.region);
            return ctx.instruments.has(id) && ctx.register.quantity(p.id, id) > 0;
          });
          if (cell === undefined) return;
          const ccy = ctx.registry.currencyOf(cell.region);
          const bank = ctx.accountOf(cell.id, ccy).issuer;
          const dwelling = goodId(DWELLING, cell.region);
          const drawn = ctx.calendar.startOf(ctx.period);
          const id = `loan:${String(bank)}:${String(cell.id)}:test` as never;
          const terms: LoanTerms = {
            kind: LOAN,
            originator: bank,
            borrower: cell.id,
            rate: asRatio(0.05, 'rate'),
            drawn,
            maturity: addMonths(drawn, 12),
            dayCount: 'ACT/365F',
            amortising: true,
            security: [{ instrument: dwelling, qty: ctx.register.quantity(cell.id, dwelling) }],
          };
          ctx.issue({ id, kind: LOAN, issuer: some(cell.id), ccy, terms, market: none() });
          const principal = 1_000_000 as never;
          ctx.settle({
            legs: [
              { kind: 'asset', from: cell.id, to: bank, instrument: id, qty: principal, pricePerUnit: some(asPerPiece(1, 'par')), accruedPerUnit: none() },
              { kind: 'money', from: { holder: bank, issuer: bank }, to: ctx.accountOf(cell.id, ccy), ccy, amount: principal },
            ],
            cause: 'issuance',
            reason: `${String(bank)} lends ${String(cell.id)} against its roofs`,
          });
          built.loan = String(id);
          built.cell = String(cell.id);
          built.bank = String(bank);
          built.dwelling = String(dwelling);
        }
        if (ctx.period === built.drains && built.cell !== '') {
          // The week before a payment falls due, the cell's money goes elsewhere.
          const cell = ctx.parties.resolve(built.cell as never);
          const ccy = ctx.registry.currencyOf(cell.region);
          const cash = ctx.participant(cell.id).cash(ccy);
          const other = ctx.parties.ofKind(HOUSEHOLD).find((p) => p.status.alive && p.id !== cell.id);
          if (cash <= 0 || other === undefined) return;
          ctx.settle({
            legs: [{ kind: 'money', from: ctx.accountOf(cell.id, ccy), to: ctx.accountOf(other.id, ccy), receipt: { of: 'transfer' }, ccy, amount: cash }],
            cause: 'transfer',
            reason: `${String(cell.id)} pays away what it holds`,
          });
        }
      },
    },
  ],
  participants: [],
  families: [],
};

describe('a mortgage is serviced, and a cell that cannot fails and its dwelling passes through the lien (12a.9)', () => {
  const { banks, firms } = rigFor('mortgage-row', { makes: ['coalRaw'] });
  const spec = rigSpec('mortgage-row', banks, firms);
  const w = assemble({ ...spec, modules: mergeModules(spec.modules, [lend]) });
  for (let i = 0; i < 10; i += 1) w.step();

  it('is charged on the roofs and serviced — interest and a slice of principal every period — while the cell has the money', () => {
    expect(built.loan).not.toBe('');
    const paid = w.ledger.all().filter((r) => r.outcome === 'settled' && r.instruction.period > 2 && r.instruction.period <= built.drains && r.instruction.reason.includes(built.loan));
    expect(paid.some((r) => r.instruction.cause === 'coupon')).toBe(true);
    expect(paid.some((r) => r.instruction.reason.startsWith('amortisation of'))).toBe(true);
    // C2, Law 19: the lien is the one record of what stands behind the row.
    const charged = w.ledger.all().find((r) => r.outcome === 'settled' && r.instruction.reason.startsWith(`${built.loan} is charged on`));
    expect(charged).toBeDefined();
  });

  it('when it cannot pay, the default is recorded on the row, the lender forecloses through the lien, and the cell fails', () => {
    const defaults = w.journal.ofKind('credit.default').filter((e) => String(e.data['instrument']) === built.loan || (String(e.data['party']) === built.cell && e.period > built.drains));
    expect(defaults.length).toBeGreaterThan(0);
    const foreclosed = w.journal.ofKind('housing.foreclosed').filter((e) => String(e.data['loan']) === built.loan);
    expect(foreclosed.length).toBeGreaterThan(0);
    expect(foreclosed.every((e) => e.data['settled'] === true)).toBe(true);
    expect(w.register.quantity(built.bank as never, built.dwelling as never)).toBeGreaterThan(0);
    // XI-3 (12a.5): and the people who could not pay are in probate, marked.
    const now = w.parties.resolve(built.cell as never);
    expect(now.representation === 'cell' && now.key['estate']).toBe('probate');
  });

  it('housing.shortfall no longer fires for every cell every period', () => {
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.representation === 'cell' && p.status.alive).length;
    for (let p = 3; p <= w.period; p += 1) {
      const short = new Set(w.journal.ofKindIn('housing.shortfall', p as never).filter((e) => Number(e.data['short']) > 0).map((e) => String(e.subjects[0])));
      expect(short.size).toBeLessThan(cells);
    }
  });
});
