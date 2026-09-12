/**
 * FX forwards, FX swaps and cross-currency swaps: the moneys, carried across time.
 *
 * @spec FX Forwards A1 FX Forwards A1.a FX Forwards A1.b FX Forwards A1.c FX Forwards A1.d FX Forwards A2 FX Forwards A3 FX Forwards A4 FX Forwards B1 FX Forwards B2 FX Forwards B2.a FX Forwards B2.b FX Forwards B3 FX Forwards B3.a FX Forwards B3.b FX Forwards B4 FX Forwards C1 FX Forwards C1.a FX Forwards C2 FX Forwards C3 FX Forwards C4 FX Forwards D1 FX Forwards D2 FX Forwards D2.a FX Forwards D3 FX Forwards D4 FX Forwards E1 FX Forwards E2 FX Forwards E3 FX Forwards E4 Derivative Layer B1 Derivative Layer C2 XI-12 Law 3 Law 4 Law 15
 *
 * A4: AN FX SWAP IS NOT A THIRD INSTRUMENT. It is spot one way and a forward back, written together
 * by a party that wants a secured loan of one money against another — so it is booked as what it
 * is, two rows in two books, and nothing here invents a kind for it. D3 is who does it: a bank
 * funding a book in a money it has not raised, which is the mechanism 12d's finding was waiting on.
 */
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { pairOf } from '../../clearing/market.js';
import { fxPairId } from '../../core/ids.js';
import { addYears } from '../../calendar/civil.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import {
  BASE_MONEY,
  FX_FORWARD,
  XCCY,
  fxForwardKind,
  xccyKind,
  type FxForwardTerms,
  type XccyTerms,
  fxForwardClass,
  xccyClass,} from './contract.js';
import {
  FX_PARAMS,
  forwardLineOf,
  forwardMarketOf,
  xccyLineOf,
  xccyMarketOf,
} from './data.js';
import { overnightRate } from './participants.js';

export * from './contract.js';
export * from './data.js';
export * from './participants.js';

function params(): ParamDecl[] {
  return [
    {
      id: FX_PARAMS.tenors,
      value: 3,
      unit: 'years',
      dimension: 'years',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'FX Forwards C1: the longest tenor a forward is written to here. The forward curve is the set of books from one year out to this one — a convention of the market, not a choice anybody in it makes.',
    },
    {
      id: FX_PARAMS.window,
      value: 8,
      unit: 'periods',
      dimension: 'periods',
      kind: 'resolution',
      owner: 'model',
      why: "Derivative Layer D1: how much of a book's own record the initial margin is measured over. A resolution: the answer must not turn on it.",
    },
    {
      id: FX_PARAMS.payEvery,
      value: 13,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'FX Forwards C1.a: how often each leg of a cross-currency swap pays interest on its own money. A convention stated with the contract.',
    },
  ];
}

/** C1: the tenors this world's forward curve has points at. */
export function fxTenorsOf(ctx: Pick<MechanismContext, 'params'>): readonly number[] {
  const longest = ctx.params.years(FX_PARAMS.tenors);
  const out: number[] = [];
  for (let y = 1; y <= longest; y += 2) out.push(y);
  return out;
}

/** The pairs this world quotes, as the spot market already names them (Law 4: one list). */
function pairsHere(ctx: MechanismContext): readonly { base: CurrencyCode; quote: CurrencyCode }[] {
  const out: { base: CurrencyCode; quote: CurrencyCode }[] = [];
  for (const m of ctx.markets) {
    const pair = pairOf(m);
    if (pair === undefined) continue;
    out.push({ base: pair.base, quote: pair.quote });
  }
  return out;
}

/**
 * A1, C1, Derivative Layer B1, C2: one forward book and one basis book per pair per tenor.
 *
 * The pairs are the spot market's own (Law 4: a second list of which moneys trade against which is
 * a second answer to one question), and the books clear through the house of the money they are
 * quoted in.
 */
function openBooks(ctx: MechanismContext, house: (ccy: CurrencyCode) => PartyId): void {
  const open = new Set(ctx.markets.map((m) => String(m.id)));
  const window = ctx.params.periods(FX_PARAMS.window);
  const payEvery = ctx.params.periods(FX_PARAMS.payEvery);
  const alive = (p: PartyId): boolean => ctx.parties.has(p) && ctx.parties.get(p).status.alive;
  for (const { base, quote } of pairsHere(ctx)) {
    // C2, and a real one: a pair is cleared through the house of the money it is QUOTED in,
    // because that is the money the margin moves in. A world with no house in that money clears
    // this pair BILATERALLY — which the layer already does, and which is what a market without a
    // central counterparty in it actually looks like (C2.a: clearing is a choice with a cost).
    const clearer = alive(house(quote)) ? house(quote) : null;
    const spot = fxPairId(base, quote);
    for (const tenorYears of fxTenorsOf(ctx)) {
      const maturity = ctx.calendar.periodOf(addYears(ctx.calendar.startOf(ctx.period), tenorYears));
      const forwardId = forwardMarketOf(base, quote, tenorYears);
      if (!open.has(String(forwardId))) {
        const terms: FxForwardTerms = {
          kind: FX_FORWARD,
          base,
          quote,
          book: forwardLineOf(base, quote, tenorYears),
          spot,
          maturity,
          tenorYears,
          buysBase: true,
          window,
        };
        ctx.openMarket({
          id: forwardId,
          name: `${base}/${quote} ${tenorYears}y forward`,
          instrument: forwardLineOf(base, quote, tenorYears),
          ccy: quote,
          rationing: 'proRata',
          kind: 'contract',
          contract: { kind: FX_FORWARD, terms, house: clearer },
        });
      }
      const basisId = xccyMarketOf(base, quote, tenorYears);
      if (open.has(String(basisId))) continue;
      const anyone = ctx.participant(ctx.parties.all()[0]?.id ?? house(quote));
      const rateBase = overnightRate(anyone, base);
      const rateQuote = overnightRate(anyone, quote);
      const spotPrint = ctx.prices.latest(spot, ctx.period);
      // C1, C3: a basis book needs a rate on each leg and a rate to exchange the notionals at.
      // A world that has not printed one of them has no such book, and nothing is assumed.
      if (!rateBase.some || !rateQuote.some || !spotPrint.some) continue;
      const terms: XccyTerms = {
        kind: XCCY,
        base,
        quote,
        book: xccyLineOf(base, quote, tenorYears),
        spot,
        exchangedAt: spotPrint.value.price,
        started: ctx.period,
        maturity,
        tenorYears,
        paysBase: true,
        baseRate: rateBase.value,
        quoteRate: rateQuote.value,
        payEvery,
        window,
      };
      ctx.openMarket({
        id: basisId,
        name: `${base}/${quote} ${tenorYears}y cross-currency basis`,
        instrument: xccyLineOf(base, quote, tenorYears),
        ccy: quote,
        rationing: 'proRata',
        kind: 'contract',
        contract: { kind: XCCY, terms, house: clearer },
      });
    }
  }
}

export function fxDerivatives(house: (ccy: CurrencyCode) => PartyId): SystemModule {
  return {
    id: 'fx-derivatives',
    spec: 'FX Forwards',
    requires: ['derivative-layer', 'spot-fx', 'money-market', 'indices'],
    instrumentKinds: [],
    derivativeKinds: [fxForwardKind, xccyKind],
    derivativeClasses: [fxForwardClass, xccyClass],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: BASE_MONEY, name: 'of the base money', perUnit: 1 }],
    params: params(),
    phases: [
      {
        name: 'fx.books',
        spec: 'FX Forwards A1 FX Forwards C1',
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
