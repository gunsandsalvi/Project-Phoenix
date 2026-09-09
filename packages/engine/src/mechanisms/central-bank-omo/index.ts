/**
 * The central bank in the sovereign market: what it buys, and what it hands back.
 *
 * @spec Central Bank C1 Central Bank C1.a Central Bank C1.b Central Bank C2 Central Bank C2.a Central Bank C3 Central Bank C4 Central Bank E1 Central Bank E3 Central Bank E3.a Central Bank E4 Central Bank E5 Sovereign H1 Sovereign H2 Sovereign H3 Sovereign H3.a Sovereign H4 Sovereign E2.c Treasury D3.a Clearing B4
 *
 * It buys in the SECONDARY market, from a seller, at a price (Treasury D3.a) — never in the primary,
 * where the seller would be the treasury itself and the purchase would be the advance that D3
 * forbids (C1.b). It posts a QUANTITY and no level (C3): a price-taker takes the level the book
 * gives it, and the size is policy's (C1.a), never an auction's weakness.
 *
 * Its purchases pay with money it creates, so there is no debit anywhere (C2, C2.a): settlement's
 * own routing does that when the payer's account is at its own issuer.
 *
 * REINVESTMENT is a separate decision (C4): with it on, the policy is a share of what is
 * outstanding and maturities are replaced; with it off, the book simply runs off and the base
 * shrinks with it — and the difference between the two is quantitative tightening.
 *
 * REMITTANCE (E3) is its net INCOME, not its revaluation (E3.a): the equity its own settled
 * instructions produced since it last remitted. Revaluation never passes through an instruction, so
 * it cannot reach this number. A loss is not remitted (E4): it stands in its equity.
 */
import type { Civil } from '../../calendar/civil.js';
import { compareCivil } from '../../calendar/civil.js';
import { period, type Period } from '../../calendar/calendar.js';
import { paramId, type PartyId } from '../../core/ids.js';
import { add, material, mul, sub, sum } from '../../core/num.js';
import { none } from '../../core/option.js';
import { months } from '../../core/rate.js';
import type { Order } from '../../clearing/solver.js';
import type { Leg } from '../../ledger/instruction.js';
import { CENTRAL_BANK, TREASURY } from '../../registry/profiles.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

export const CB_PARAMS = {
  targetShare: paramId('centralBank.omo.targetHoldingShare'),
  reinvest: paramId('centralBank.reinvest'),
  remittanceMonths: paramId('centralBank.remittance.periodicity'),
} as const;

export const centralBankOmo: SystemModule = {
  id: 'central-bank-omo',
  spec: 'Central Bank C, E; Sovereign H',
  requires: ['sovereign-instruments', 'sovereign-curve'],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: CB_PARAMS.targetShare,
      value: 0.25,
      unit: 'ratio of a line outstanding',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank C1, C1.a: it buys sovereign paper in a size IT chooses, set by policy and never by an auction’s weakness. The share of each line it wants to hold is that size, stated.',
    },
    {
      id: CB_PARAMS.reinvest,
      value: 1,
      unit: '1 to reinvest maturities, 0 to let the book run off',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank C4: reinvestment of maturities is a decision separate from new purchases, and the difference between the two is quantitative tightening.',
    },
    {
      id: CB_PARAMS.remittanceMonths,
      value: 12,
      unit: 'months',
      kind: 'policy',
      owner: 'model',
      why: 'Central Bank E3: its net income goes to the treasury because the treasury owns it. How often it settles up is placed on the one calendar by date (Money G3).',
    },
  ],
  phases: [
    {
      name: 'centralBank.remittance',
      spec: 'Central Bank E3 Central Bank E3.a Central Bank E4 Sovereign H3',
      cycle: 'anchor',
      anchor: { before: 'revaluation' },
      run: (ctx: MechanismContext): void => {
        for (const cb of ctx.parties.ofKind(CENTRAL_BANK)) {
          if (cb.status.alive) remit(ctx, cb.id);
        }
      },
    },
  ],
  participants: [
    {
      partyKind: CENTRAL_BANK,
      orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
        // C1.b: never in the primary market. The seller there is the issuer, and buying from it is
        // the advance the treasury is not allowed to have (Treasury D3, D3.a).
        if (view.offer(m.id).some) return [];
        const i = view.instruments.get(m.instrument);
        if (view.registry.instrumentKind(i.kind).pricing !== 'cleared') return [];
        if (i.ccy !== view.registry.region(view.self.region).ccy) return [];
        // C1: it buys SOVEREIGN paper. Everything else that clears in its money — a tonne of grain,
        // a share, a corporate line — is somebody else's market, and a central bank standing in it
        // with a size set by its own policy is the buyer of last resort this world does not have
        // (Appendix B). What makes paper sovereign is who promised it, which is public.
        if (!i.issuer.some) return [];
        const sovereigns = new Set(view.parties.ofKind(TREASURY).map((t) => t.id));
        if (!sovereigns.has(i.issuer.value)) return [];
        const held = view.quantity(i.id);
        // C4: with reinvestment off there is no target to restore, so the book runs off.
        if (view.params.get(CB_PARAMS.reinvest) === 0) return [];
        const target = mul(i.issued, view.params.get(CB_PARAMS.targetShare), 'target holding');
        const gap = sub(target, held, 'omo gap');
        // A gap smaller than the dust of the subtraction that produced it is not a policy decision.
        if (!material(gap, 2, add(Math.abs(target), Math.abs(held), 'gap magnitude'))) return [];
        if (gap > 0) return [{ party: view.self.id, side: 'buy', price: 'market', qty: gap }];
        const free = view.free(i.id);
        const size = -gap < free ? -gap : free;
        return size > 0 ? [{ party: view.self.id, side: 'sell', price: 'market', qty: size }] : [];
      },
    },
  ],
  families: [],
};

/** E3: everything its own instructions earned it since the last time it settled up. */
function remit(ctx: MechanismContext, cb: PartyId): void {
  if (!dueThisPeriod(ctx)) return;
  const previous = lastRemittance(ctx, cb);
  const terms: number[] = [];
  for (let p = previous; p <= ctx.period; p = period(p + 1)) {
    for (const r of ctx.ledger.inPeriod(p)) {
      if (r.outcome !== 'settled') continue;
      for (const e of r.equity) if (e.party === cb) terms.push(e.delta);
    }
  }
  const income = sum(terms).value;
  const treasuries = ctx.parties.ofKind(TREASURY).filter((t) => t.status.alive);
  const to = treasuries[0];
  if (to === undefined) return;
  if (income <= 0) {
    // E4: a loss is not remitted. It reduces its equity and stands there until income covers it.
    ctx.record('centralBank.loss', [cb], { income, since: previous }, true);
    return;
  }
  const ccy = ctx.registry.region(ctx.parties.get(cb).region).ccy;
  const leg: Leg = {
    kind: 'money',
    from: { holder: cb, issuer: cb },
    to: { holder: to.id, issuer: ctx.parties.get(to.id).bank },
    ccy,
    amount: income,
    fromCell: none(),
    toCell: none(),
  };
  const r = ctx.settle({ legs: [leg], cause: 'transfer', reason: `remittance to ${to.id}` });
  ctx.record(
    'centralBank.remittance',
    [cb, to.id],
    { income, since: previous, settled: r.outcome === 'settled' },
    true,
  );
}

/** Placed on the one calendar by date, like every other periodicity (Money G3.a). */
function dueThisPeriod(ctx: MechanismContext): boolean {
  const every = ctx.params.get(CB_PARAMS.remittanceMonths);
  const epoch = ctx.calendar.epoch;
  const end = ctx.calendar.endOf(ctx.period);
  let date: Civil = epoch;
  for (let k = 0; k < ctx.period + 1; k += 1) {
    date = ctx.calendar.advance(date, months(every));
    if (compareCivil(date, end) > 0) return false;
    if (ctx.calendar.place(date) === ctx.period) return true;
  }
  return false;
}

/** The period it last settled up in, read from the journal; nothing stores it. */
function lastRemittance(ctx: MechanismContext, cb: PartyId): Period {
  const events = [...ctx.journal.ofKind('centralBank.remittance'), ...ctx.journal.ofKind('centralBank.loss')]
    .filter((e) => e.subjects.includes(cb))
    .sort((a, b) => a.period - b.period);
  const last = events[events.length - 1];
  return last === undefined ? period(0) : period(add(last.period, 1, 'after the last remittance'));
}
