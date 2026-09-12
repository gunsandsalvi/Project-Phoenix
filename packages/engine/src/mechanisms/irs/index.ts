/**
 * Interest-rate swaps: a market in the fixed rate, and a curve that is the set of what it cleared.
 *
 * @spec IRS A1 IRS A1.a IRS A1.b IRS A1.c IRS A1.d IRS A2 IRS A3 IRS A4 IRS B1 IRS B3 IRS B4 IRS B5 IRS C1 IRS C1.a IRS C2 IRS C3 IRS C3.a IRS C4 IRS D1 IRS D2 IRS D3 IRS D3.a IRS D4 IRS E1 IRS E2 IRS E3 Derivative Layer B1 Derivative Layer C2 XI-13 Law 3 Law 4 Law 15 Law 19
 *
 * C1, C1.a: THE SWAP CURVE IS NOT FITTED AND NOT STORED. It is the set of fixed rates these books
 * cleared, one per tenor, and a tenor nobody traded has no point — the same rule the sovereign
 * curve lives by, arrived at from prints rather than from a fit (Sovereign D3.b).
 *
 * C3, C3.a: the SWAP SPREAD — the cleared fixed rate against the sovereign's own yield at the same
 * tenor — is a read of two curves and a consequence of what both markets did. Nothing targets it,
 * nothing checks it, and when it goes somewhere surprising that is a finding about a mechanism.
 */
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { addYears } from '../../calendar/civil.js';
import { sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { curveFamilyOf } from '../../prices/curve.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { IRS, irsKind, isIrs, NOTIONAL, type IrsTerms } from './contract.js';
import { irsMarketOf, irsLineOf, IRS_PARAMS } from './data.js';

export * from './contract.js';
export * from './data.js';
export * from './participants.js';

/** Law 15, Law 4: a party kind is a NAME. Naming one is not importing the module that declares it. */

function params(): ParamDecl[] {
  return [
    {
      id: IRS_PARAMS.tenors,
      value: 9,
      unit: 'years',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'IRS C1: the longest tenor this world writes a swap to. The curve is the set of books from one year out to this one; how many points it has is a fact about the market rather than about anybody in it.',
    },
    {
      id: IRS_PARAMS.window,
      value: 8,
      unit: 'periods',
      kind: 'resolution',
      owner: 'model',
      why: "Derivative Layer D1: how much of a book's own record the initial margin is measured over. A resolution: the answer must not turn on it.",
    },
    {
      id: IRS_PARAMS.fixedEvery,
      value: 13,
      unit: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'IRS A2, D6.a: how often the fixed leg pays. A convention of the market, stated with the contract, and not the same as the floating leg’s — which is the whole reason A2 says the two need not match.',
    },
    {
      id: IRS_PARAMS.floatEvery,
      value: 1,
      unit: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'IRS A3: how often the floating leg fixes and pays. It fixes on the overnight book, which prints every period it trades, so this is how often that fixing is turned into a payment.',
    },
  ];
}

/** C1: the tenors this world's swap curve has points at. */
export function irsTenorsOf(ctx: Pick<MechanismContext, 'params'>): readonly number[] {
  const longest = ctx.params.get(IRS_PARAMS.tenors);
  const out: number[] = [];
  for (let y = 1; y <= longest; y += 2) out.push(y);
  return out;
}

/** A1.a, E3: which overnight book a swap in this money fixes on — the secured one, named. */
export const benchmarkOf = (ccy: CurrencyCode): string => `${String(ccy)}:secured`;

/**
 * A1, C1, Derivative Layer B1, C2: one book per money per tenor, cleared through the house.
 *
 * A swap book needs no reference and no underlying line: what it settles against is the money
 * market's own fixing, which every money in this world has as soon as its overnight book trades.
 */
function openBooks(ctx: MechanismContext, house: (ccy: CurrencyCode) => PartyId): void {
  const open = new Set(ctx.markets.map((m) => String(m.id)));
  const window = ctx.params.get(IRS_PARAMS.window);
  const fixedEvery = ctx.params.get(IRS_PARAMS.fixedEvery);
  const floatEvery = ctx.params.get(IRS_PARAMS.floatEvery);
  for (const ccy of ctx.registry.currencies.keys()) {
    const clearer = house(ccy);
    if (!ctx.parties.has(clearer) || !ctx.parties.get(clearer).status.alive) continue;
    // D3.a: no fixing, no floating leg. A money whose overnight book has never traded has no
    // benchmark, and a swap on a benchmark that does not exist is a swap on nothing.
    if (!ctx.journal.ofKind('index.benchmark').some((e) => e.subjects.includes(benchmarkOf(ccy)))) {
      continue;
    }
    for (const tenorYears of irsTenorsOf(ctx)) {
      const id = irsMarketOf(ccy, tenorYears);
      if (open.has(String(id))) continue;
      const terms: IrsTerms = {
        kind: IRS,
        ccy,
        benchmark: benchmarkOf(ccy),
        book: irsLineOf(ccy, tenorYears),
        maturity: ctx.calendar.periodOf(addYears(ctx.calendar.startOf(ctx.period), tenorYears)),
        tenorYears,
        fixedEvery,
        floatEvery,
        paysFixed: true,
        window,
      };
      ctx.openMarket({
        id,
        name: `${ctx.registry.currency(ccy).name} ${tenorYears}y swap`,
        instrument: irsLineOf(ccy, tenorYears),
        ccy,
        rationing: 'proRata',
        kind: 'contract',
        contract: { kind: IRS, terms, house: clearer },
      });
    }
  }
}

/** C1, C1.a: the curve — the set of cleared fixed rates, in tenor order. Built at the read. */
export function swapCurve(
  ctx: MechanismContext,
  ccy: CurrencyCode,
): readonly { readonly tenorYears: number; readonly rate: number }[] {
  const out: { tenorYears: number; rate: number }[] = [];
  for (const tenorYears of irsTenorsOf(ctx)) {
    const p = ctx.prices.latest(irsLineOf(ccy, tenorYears), ctx.period);
    if (p.some) out.push({ tenorYears, rate: p.value.price });
  }
  return out;
}

/**
 * C2: FORWARD RATES, derived from the curve of cleared rates and stored nowhere.
 *
 * What the market is saying about the period between two tenors is the rate that makes holding the
 * longer one the same as holding the shorter one and then that — arithmetic on two prints, done at
 * the read, and never an input to either of them.
 */
export function forwardRate(
  curve: readonly { readonly tenorYears: number; readonly rate: number }[],
  from: number,
  to: number,
): Option<number> {
  const near = curve.find((p) => p.tenorYears === from);
  const far = curve.find((p) => p.tenorYears === to);
  if (near === undefined || far === undefined || to <= from) return none<number>();
  const grown = Math.pow(1 + far.rate, to) / Math.pow(1 + near.rate, from);
  return some(sub(Math.pow(grown, 1 / (to - from)), 1, 'the forward rate between the two'));
}

/** C3, C3.a: the swap spread — the cleared fixed rate against the sovereign's own yield. A READ. */
export function swapSpread(
  ctx: MechanismContext,
  ccy: CurrencyCode,
  tenorYears: number,
  sovereign: PartyId,
): Option<number> {
  const swap = ctx.prices.latest(irsLineOf(ccy, tenorYears), ctx.period);
  if (!swap.some) return none<number>();
  const risk = ctx.curve(curveFamilyOf(sovereign, ccy)).at(tenorYears);
  if (!risk.yield.some) return none<number>();
  return some(sub(swap.value.price, risk.yield.value, 'the swap against the sovereign'));
}

export function irs(house: (ccy: CurrencyCode) => PartyId): SystemModule {
  return {
    id: 'irs',
    spec: 'IRS',
    requires: ['derivative-layer', 'indices', 'money-market'],
    instrumentKinds: [],
    derivativeKinds: [irsKind],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: NOTIONAL, name: 'of notional', perUnit: 1 }],
    params: params(),
    phases: [
      {
        name: 'irs.books',
        spec: 'IRS A1 IRS C1',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx): void => {
          openBooks(ctx, house);
        },
      },
    ],
    // Clearing A2, Law 4: A CLASS DECLARES NO PARTICIPANT OF ITS OWN. Its reasons live on its own
    // profile (`DerivativeKindProfile.orders`) and the layer that owns contract books asks for
    // them — one module speaking for one party in one book, which is the assembly half of the
    // kernel's refusal to let anybody cross themselves.
    participants: [],
    families: [],
  };
}

/** E1: what a reader checks — no leg of a swap ever moves the notional. */
export const movesNotional = (amount: number, notional: number): boolean => amount === notional;

export { isIrs };
