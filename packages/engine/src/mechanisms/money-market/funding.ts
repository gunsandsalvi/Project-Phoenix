/**
 * The rung of the funding ladder below the market and the window: it sells what it holds.
 *
 * @spec Banks Funding D1 Banks Funding D2 Banks Funding D3 Banks Funding D4 Banks Funding C1 Banks Funding C1.a Money Market B7 Money Market C4.b Law 6 Law 8 Law 19
 *
 * D1 names the order a bank goes through when it cannot fund itself, and every rung of it is a real
 * action with a counterparty: it PLEDGES its paper (the secured book and the window, priced in
 * `session.ts`), it BIDS UP for deposits (its rate is its own decision, `deposits.ts`), it STOPS
 * ORIGINATING (the lending module reads its published liquidity), it DRAWS THE FACILITY (the window
 * takes its seat in every session, and an account below zero becomes a priced row at the close),
 * and it FAILS (`world/failure.ts`, then resolution). The one rung that had nowhere to happen is
 * this one: SELLING.
 *
 * A bank that closed the week short of money and could not borrow it has one thing left that is
 * worth something — the paper it has not already pledged — and D1 says it sells it. What it gets
 * for it is what somebody will pay: the order carries a size and no level (Clearing C3), so it is
 * struck at the worst level the other side actually posted and never at a reserve of its own. That
 * is what makes a forced sale different from an ordinary one, and it is why the print a fire sale
 * makes reaches every other holder of the same paper through the revaluation (XI-2).
 *
 * It sells into the NEXT session, because the shortfall is only known when this one closes: the
 * refusal is a public event with a size on it (B7), and this reads its own.
 */
import type { Civil } from '../../calendar/civil.js';
import { nextPeriod } from '../../calendar/calendar.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { div, mul, sub, sum } from '../../core/num.js';
import { upTick } from '../../core/tick.js';
import type { ParticipantView } from '../../world/context.js';
import { eligible, type Advance } from './collateral.js';

/**
 * B7, D1, D2, Money Market A2.b: WHAT IT IS ACTUALLY SHORT OF, which is not what it asked for.
 *
 * A bank asks the session for two different things at once: the money it has to pay out tomorrow,
 * and the buffer it likes to keep behind that. Only the first is an obligation. Its buffer is the
 * thing a bank RUNS DOWN when the market says no — that is what a buffer is for, and A2.b says
 * missing it has a cost the bank can feel, which is the next rung of this ladder and not this one.
 * So the sale covers the refusal LESS the buffer inside it: a bank that could not top up its cushion
 * sells nothing, and a bank that cannot meet a maturity sells until it can.
 *
 * Both numbers are in the refusal it published (B7), and nothing older counts: a shortfall it has
 * since funded is not a reason to sell anything, so this asks for the one the last close wrote.
 */
export function stillShort(view: ParticipantView): number {
  const said = view.lastOwn('moneyMarket.refused');
  if (!said.some || nextPeriod(said.value.period) !== view.period) return 0;
  const short = said.value.data['short'];
  const buffer = said.value.data['buffer'];
  if (typeof short !== 'number' || typeof buffer !== 'number') return 0;
  const owed = sub(short, buffer, 'what it cannot pay, once the cushion is gone');
  return owed > 0 ? owed : 0;
}

/**
 * C1, C1.a: what it could actually sell — its own free eligible paper, at its own marks, in lines
 * that have a market to sell them in. Paper it has pledged is not here (it is somebody's
 * collateral, C4.b), and a line nothing trades cannot be sold however good it is.
 */
export function saleable(view: ParticipantView, on: Civil): readonly Advance[] {
  const out: Advance[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!eligible(view, i, on)) continue;
    if (!view.markets.some((m) => m.instrument === i.id)) continue;
    const mark = view.mark(i.id);
    const free = view.free(i.id);
    if (!mark.some || mark.value <= 0 || free <= 0) continue;
    out.push({
      instrument: i.id,
      free,
      valuePerUnit: mark.value,
      total: mul(free, mark.value, 'what this parcel would fetch'),
    });
  }
  return out;
}

/**
 * D1: the sale itself. It raises what it is short of across the lines it can sell, in proportion to
 * what each of them is worth to it — a treasurer liquidating a book takes it down evenly rather
 * than emptying one line first, and a line it holds nothing free of contributes nothing.
 *
 * Law 8: it delivers whole pieces, and it rounds the size UP because the point is to cover the
 * shortfall — bounded by what it actually has, which is not a clamp but the arithmetic of a
 * delivery (Law 6: you cannot sell paper you do not hold).
 */
export function forcedSale(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const short = stillShort(view);
  if (short <= 0) return [];
  const on = view.calendar.startOf(view.period);
  const lines = saleable(view, on);
  const total = sum(lines.map((x) => x.total)).value;
  const parcel = lines.find((x) => x.instrument === m.instrument);
  if (parcel === undefined || total <= 0) return [];
  const share = mul(short, div(parcel.total, total, 'what this line carries'), 'raised here');
  const want = upTick(div(share, parcel.valuePerUnit, 'units to sell'));
  const qty = want < parcel.free ? want : parcel.free;
  if (qty <= 0) return [];
  // Clearing C3: a size and no level. What it fetches is what the other side posted.
  return [{ party: view.self.id, side: 'sell', price: 'market', qty }];
}
