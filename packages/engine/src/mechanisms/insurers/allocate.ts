/**
 * An institution does not invest. It hands its assets to somebody whose business that is.
 *
 * @spec Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Fund Shares A1 Fund Shares A4 Fund Shares D5 Law 3 Law 4 Law 19
 *
 * ITEM 14.0, AND IT IS THE OWNER'S LARGEST SIMPLIFICATION: *"insurance companies and pension funds
 * don't invest themselves. Their assets are always third party managed."* So there is no portfolio
 * mechanism here, no allocation rule, and no decision about which bond to buy. There is a LIABILITY
 * SCHEDULE and a choice of manager, and B2.b's duration matching is the whole of the choice.
 *
 * *"Nothing in this world invests except a fund, and every fund has a manager"* — so an insurer
 * reaches the markets through the same door a household does, at the same venue, under the same
 * refusals. What is different is only WHICH door it picks and WHY, and both of those are §27's.
 *
 * WHY IT IS INSERTED HERE, AHEAD OF THE REST OF ITEM 13 (Law 10). Items 13.2 and 13.3 built a
 * strategy house and a prime broker, and item 10e built a credit fund — and none of them is offered
 * to the public, so the only investors this world has (household cells) cannot reach any of them.
 * A private-equity fund (13.5) is the same shape and would have been the third. **The money those
 * sectors run on is institutional, and nothing in this world allocated any**: an insurer wrote
 * cover, took premiums, and sat on the cash for ever. This is where that money comes from.
 *
 * WHAT IT DOES NOT DO. It does not pick a bond, a tenor or a name; it does not rebalance; it holds
 * no view about a price. Its whole decision is: what can I put to work, and whose mandate is shaped
 * like what I promised. Everything after that is the manager's (Fund Shares A4).
 */
import type { CurrencyCode, PartyId, VenueId } from '../../core/ids.js';
import { moneyInstrumentId } from '../../core/ids.js';
import {
  absolute,
  amountOf,
  asCash,
  asPerPiece,
  type Cash,
  heldAsMoney,
  minus,
  type PerPiece,
} from '../../core/measure.js';
import { downTick } from '../../core/tick.js';
import { compareCivil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { MechanismContext } from '../../world/context.js';
import { isPolicy } from './index.js';

/** What a pool published about itself, as an allocator reads it off the tape (Observer A3). */
interface Door {
  readonly venue: VenueId;
  readonly fund: string;
  readonly perShare: PerPiece;
  /** B2.b: what its mandate says about duration, and none where it says nothing. */
  readonly years: number | undefined;
  /** Item 10e.6: the policy an entrant must clear, and none where it is offered to the public. */
  readonly asks: string | undefined;
}

/**
 * Observer A3, Law 19: every pool this institution could put money into, read off the venues and
 * what each of them last struck. It knows nothing private about any of them — the same two public
 * facts a household reads, and the door's own key.
 */
function doors(ctx: MechanismContext): readonly Door[] {
  const out: Door[] = [];
  for (const v of ctx.venues) {
    if (v.key['kind'] !== 'fund') continue;
    const fund = v.key['fund'];
    if (fund === undefined) continue;
    const said = ctx.journal.lastOf('fund.struck', fund);
    if (said === undefined) continue;
    const perShare = said.data['perShare'];
    const years = said.data['durationYears'];
    if (typeof perShare !== 'number' || perShare <= 0) continue;
    out.push({
      venue: v.id,
      fund,
      // Item 16: what the pool published a share is worth, re-entering as the level it is.
      perShare: asPerPiece(perShare, 'what the fund said a share is worth'),
      years: typeof years === 'number' ? years : undefined,
      asks: v.key['asks'],
    });
  }
  return out;
}

/**
 * B2, B2.b, Law 19: THE LONGEST PROMISE IT HAS MADE, in years from today.
 *
 * It is a SELECTION over real dates and never a weighted average of them: an average duration is a
 * number no promise in the book actually has, and matching to it would be deciding at an average
 * (Appendix B). The furthest date it has promised anything on is a fact about one real schedule,
 * and it is the one that decides whether this institution can hold a long asset at all.
 *
 * An institution that has promised nothing yet has no duration to match, and gets none rather than
 * a zero — a life company with no policies is not a company that needs its money tomorrow.
 */
export function longestPromise(ctx: MechanismContext, insurer: PartyId): number | undefined {
  const now = ctx.calendar.startOf(ctx.period);
  let furthest: typeof now | undefined;
  for (const i of ctx.instruments.issuedBy(insurer)) {
    if (!i.status.live || !isPolicy(i.terms) || i.issued <= 0) continue;
    for (const flow of i.terms.schedule) {
      if (compareCivil(flow.date, now) <= 0) continue;
      if (furthest === undefined || compareCivil(flow.date, furthest) > 0) furthest = flow.date;
    }
  }
  return furthest === undefined ? undefined : yearFraction('ACT/365F', now, furthest);
}

/**
 * B2, A4.c: WHAT IT CAN PUT TO WORK — its account, less what a period of its own claims costs it.
 *
 * The buffer is its OWN experience and not a ratio anybody stated: what it has actually paid out on
 * claims is a fact about its own book (A4.c), and an institution keeps enough to meet what its book
 * has been costing it. What is left is money that should be earning, and an insurer sitting on it is
 * an insurer whose promises are unfunded — which is what B2 is about.
 */
export function investable(ctx: MechanismContext, insurer: PartyId, ccy: CurrencyCode): Cash {
  const account = ctx.accountOf(insurer, ccy);
  const cash = heldAsMoney(
    ctx.register.quantity(insurer, moneyInstrumentId(account.issuer, ccy)),
    'what is in its account',
  );
  const claim = ctx.journal.lastOf(CLAIM_PAID_KIND, String(insurer));
  const keep =
    claim === undefined || typeof claim.data['amount'] !== 'number'
      ? asCash(0, 'an insurer that has paid no claim has none to keep against')
      : asCash(claim.data['amount'], 'what its last claim cost it');
  return minus(cash, keep, 'what it can put to work');
}

/** The one name the claims event goes under; read, never re-derived (Law 4). */
const CLAIM_PAID_KIND = 'insurer.claim';

/**
 * B2.b: WHOSE MANDATE IS SHAPED LIKE WHAT IT PROMISED — the pool whose stated duration is nearest
 * the furthest thing this institution owes, and NO pool at all where nothing matches.
 *
 * A pool that states no duration cannot be matched to a liability schedule, and that is a refusal
 * rather than a fallback: an equity fund is not a place to put money you have promised somebody on
 * a date. Ties break on the fund's own name, so two identical mandates are not separated by the
 * order a venue list happened to be built in.
 */
export function matchFor(doorsOpen: readonly Door[], years: number): Door | undefined {
  let best: Door | undefined;
  let closest = 0;
  for (const d of doorsOpen) {
    if (d.years === undefined) continue;
    const away = absolute(asCash(d.years - years, 'how far its mandate is from what it promised'), 'how far');
    if (best === undefined || away < closest || (away === closest && d.fund < best.fund)) {
      best = d;
      closest = away;
    }
  }
  return best;
}

/**
 * B2, Fund Shares C1: the institution's whole period. It reads what it owes, reads what it has,
 * picks whose mandate matches, and subscribes — one posting, at the same door a household uses.
 *
 * It CERTIFIES first where the door asks (item 10e.6). A named party is an institution and needs no
 * wealth test, so this is only for a door that asks something a named party could fail — and saying
 * so is cheaper than knowing which doors those are.
 */
export function allocate(ctx: MechanismContext, insurer: PartyId, ccy: CurrencyCode): void {
  const spare = investable(ctx, insurer, ccy);
  if (spare <= 0) return;
  const years = longestPromise(ctx, insurer);
  if (years === undefined) return;
  const door = matchFor(doors(ctx), years);
  if (door === undefined) return;
  // Law 8: whole shares, and DOWN — what its money buys, never a share it cannot pay for.
  const shares = downTick(amountOf(spare, door.perShare, 'shares its money buys'));
  if (shares <= 0) return;
  ctx.post(door.venue, { party: insurer, side: 'buy', price: 'market', qty: shares });
  ctx.record(
    'insurer.allocated',
    [insurer, door.fund],
    {
      insurer,
      fund: door.fund,
      // B2.b: the two numbers the choice was made on, both of them public.
      promisedYears: years,
      mandateYears: door.years,
      shares,
      putToWork: spare,
    },
    false,
  );
}
