/**
 * A mortgage is a row per (lender, cell), asked for every period a cell is short of roofs, and
 * foreclosed on a default and nothing else (Housing C1, C2, C4, XI-1, 12a.4).
 *
 * @spec Housing C1 Housing C2 Housing C4 XI-1 XI-15 Register F2
 */
import { describe, expect, it } from 'vitest';
import { rate as perAnnum } from '../src/core/rate.js';
import { DWELLING, HOUSEHOLD, LOAN, assemble, none, some, type LoanTerms, type MechanismContext, type SystemModule } from '../src/index.js';
import { goodId } from '../src/registry/physical.js';
import { addMonths } from '../src/calendar/civil.js';
import { asPerPiece, asRatio, valueAt } from '../src/core/measure.js';
import { asQty, downTick } from '../src/core/tick.js';
import { weightOf, type Party } from '../src/parties/party.js';
import { LANDLORD } from '../src/registry/property.js';
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
          const living = (p: Party): boolean => p.representation === 'cell' && p.status.alive && p.key['estate'] === 'living' && ctx.instruments.has(goodId(DWELLING, p.region));
          let cell = ctx.parties.ofKind(HOUSEHOLD).find((p) => living(p) && ctx.register.quantity(p.id, goodId(DWELLING, p.region)) > 0);
          if (cell === undefined) {
            // 15.5: a household here rents and never buys (21.39), so the roof this row stands on is
            // bought by hand — one a member from a landlord of the place, at the last print, delivery
            // against payment — the instruction any buyer of a roof would write.
            // The smallest cell whose money reaches a piece of a roof: a cell of a few households, so
            // that what it holds is what a wage or two comes to and a drained account stays drained.
            const cells = ctx.parties.ofKind(HOUSEHOLD).filter(living).sort((a, b) => weightOf(a) - weightOf(b));
            for (const p of cells) {
              const id = goodId(DWELLING, p.region);
              const print = ctx.prices.latest(id, ctx.period);
              if (!print.some) continue;
              const ccy = ctx.registry.currencyOf(p.region);
              const cash = ctx.participant(p.id).cash(ccy);
              // Law 8, XI-15: the pieces of a roof one member's money reaches, whole pieces and DOWN, for every member alike.
              const perMember = downTick(cash / weightOf(p) / print.value.price);
              if (perMember <= 0) continue;
              const pieces = asQty(perMember * weightOf(p), 'what the cell buys');
              const cost = ctx.registry.payable(valueAt(print.value.price, pieces, ccy, 'what that costs it'));
              if (cash < cost) continue;
              const seller = ctx.parties.ofKind(LANDLORD).find((l) => l.status.alive && l.region === p.region && ctx.register.free(l.id, id) >= pieces);
              if (seller === undefined) continue;
              const bought = ctx.settle({
                legs: [
                  { kind: 'asset', from: seller.id, to: p.id, instrument: id, qty: pieces, pricePerUnit: some(print.value.price), accruedPerUnit: none() },
                  { kind: 'money', from: ctx.accountOf(p.id, ccy), to: ctx.accountOf(seller.id, ccy), receipt: { of: 'sale' }, ccy, amount: cost },
                ],
                cause: 'transfer',
                reason: `${String(p.id)} buys a roof a member from ${String(seller.id)}`,
              });
              if (bought.outcome === 'settled') {
                cell = p;
                break;
              }
            }
          }
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
            // 17d.2: a margin over the fixing, and a fixed row where the book has never traded.
            margin: asRatio(0.05, 'rate'),
            floatsOver: none<string>(),
            coupon: perAnnum(0.05, { kind: 'annual' }),
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
      },
    },
    {
      name: 'test.mortgageDrain',
      spec: 'Housing C4',
      // 15.5: the coupon falls due in the corporate actions at the top of the period, and the money
      // goes elsewhere just before — after every wage and rent of the period before has landed, so
      // nothing refills the account between the drain and the payment.
      anchor: { before: 'corporateActions' },
      reads: [],
      writes: [],
      run: (ctx: MechanismContext) => {
        if (ctx.period === built.drains + 1 && built.cell !== '') {
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
