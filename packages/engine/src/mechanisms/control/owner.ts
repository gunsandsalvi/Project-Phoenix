/**
 * What an owner does with a company it controls: it makes it borrow, and the money leaves as a
 * distribution.
 *
 * @spec Private Equity C1 Private Equity C2 Private Equity C3 M&A A4 Equity D3 Corporate Credit A1 Law 2 Law 4 Law 12
 *
 * §29 C3: *"it can RECAPITALISE — raise more debt to pay itself a distribution, which is a real
 * transfer from the firm's future to the owner's present."* Every word of that is already a
 * mechanism in this world except the REASON, and the reason is the only thing here.
 *
 * A company borrows by publishing what it is short of (Corporate Credit A1); a company with money
 * over what it must pay distributes it to whoever holds it (Equity D3, `decideEquity`). Put the two
 * together and a recapitalisation is a company borrowing money IT DOES NOT NEED — and then the
 * distribution happens on its own, because the firm's own spare rose and the ordinary decision pays
 * it out. Nothing here writes a dividend, nothing here moves money to an owner, and there is no
 * second writer of either (Law 4).
 *
 * WHOSE REASON IT IS is the whole of C2: *"the owner influences the firm — investment, costs,
 * distributions."* What the company asks for is what its CONTROLLER published it must find — a
 * pool's shortfall to its own investors, a company's own funding gap — read off the wire like any
 * other public fact (Law 19). A company whose owner needs nothing borrows nothing extra.
 *
 * AND IT IS STILL THE CREDIT MARKET'S DECISION (B2.b). The ask is for a COMMITMENT, priced by a
 * bank against the company as it stands — which by now is a company carrying the buyout's debt — so
 * a firm already levered to the hilt is refused, and the recapitalisation that would have finished
 * it does not happen. That is C1 and C4 reaching back into C3 without a line saying so.
 */
import { asCash, minus, noCash, type Cash } from '../../core/measure.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { asQty, downTick, type Qty } from '../../core/tick.js';
import { asPerPiece } from '../../core/measure.js';
import { none, some } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import type { MechanismContext } from '../../world/context.js';
import { fundingPublishedBy, strikeOf } from '../../registry/funding.js';
import { askToFund, DEAL_MONTHS, facilityFor, facilityRow } from './deal.js';

/**
 * C2: WHAT THIS OWNER MUST FIND, in its own words. Two kinds of owner publish it and the question is
 * the same for both (Law 4): a POOL publishes what it owes its investors and could not pay
 * (`fund.struck`), a COMPANY publishes what falls due that it cannot meet (`firms.funding`). An
 * owner that published neither wants nothing out of what it owns this period.
 */
function whatItsOwnerMustFind(ctx: MechanismContext, owner: PartyId, ccy: CurrencyCode): Cash {
  const pool = strikeOf(ctx.journal, String(owner));
  if (pool.some && pool.value.shortfall > 0) {
    return asCash(pool.value.shortfall, ccy, 'what its owner published it must find');
  }
  const firm = fundingPublishedBy(ctx.journal, String(owner), ctx.period);
  if (firm.some && firm.value.shortNow > 0) {
    return asCash(firm.value.shortNow, ccy, 'what its owner published it must find');
  }
  return noCash(ccy);
}

/**
 * C3: THE COMPANY ASKS FOR MONEY ITS OWNER WANTS. It is the same door every borrower uses and the
 * same commitment a buyout is funded by; what differs is whose need it is, which is C2.
 *
 * It brings no cheque, because nobody is buying anything: the owner already owns it.
 */
export function askForTheOwner(ctx: MechanismContext): void {
  for (const row of ctx.control.all()) {
    const company = ctx.parties.get(row.subject);
    if (!company.status.alive || !ctx.parties.get(row.controller).status.alive) continue;
    const ccy = ctx.registry.currencyOf(company.region);
    const wants = whatItsOwnerMustFind(ctx, row.controller, ccy);
    if (wants.pieces <= 0) continue;
    // C3 (17b.8): over the years the company will service it, like the buyout's own debt.
    askToFund(ctx, company.id, wants, noCash(ccy), row.controller, ctx.params.months(DEAL_MONTHS));
  }
}

/**
 * C3, Banks Lending B1 (17b.6): AND WHEN THE LENDER HAS AGREED, IT DRAWS.
 *
 * The row is the same row a buyout draws (`facilityRow`), the money is the bank's own created
 * against it, and it lands in the company's own account — which is where B1.b says a drawing goes,
 * and it is the difference between this and a buyout: a buyout's proceeds pay the sellers and never
 * touch the company, and a recapitalisation's ARE the company's, to distribute.
 *
 * What happens next is not written here and must not be: the firm's own spare has risen, and what a
 * company does with money it does not need is `decideEquity`'s decision (Equity D3) — it pays its
 * holders, and its holders are mostly its owner. *"A real transfer from the firm's future to the
 * owner's present"* is then two ordinary mechanisms and no third one.
 */
export function drawForTheOwner(ctx: MechanismContext): void {
  for (const row of ctx.control.all()) {
    const company = ctx.parties.get(row.subject);
    if (!company.status.alive) continue;
    const ccy = ctx.registry.currencyOf(company.region);
    // C2: only where THIS company asked for its owner — a commitment made for a buyout of it is
    // drawn by the tender that completes, not here, and the two are told apart by the cheque the
    // ask published: a buyer brings one and an owner brings none.
    if (!askedForItsOwner(ctx, company.id)) continue;
    const facility = facilityFor(ctx, company.id, ccy);
    if (!facility.some) continue;
    const line = facilityRow(ctx, company.id, facility.value, ccy);
    const drawn = asCash(ctx.register.heldTotal(line).value, ccy, 'what it has already drawn');
    const room = downTick(minus(facility.value.limit, drawn, 'its headroom').pieces);
    if (room <= 0) continue;
    const amount = asQty(room, 'what it draws on the line its owner had it commit');
    const r = ctx.settle({
      legs: legsFor(ctx, company.id, facility.value.bank, line, ccy, amount),
      cause: 'issuance',
      reason: `${String(company.id)} draws ${room} for ${String(row.controller)}`,
    });
    ctx.record(
      'control.recapitalised',
      [String(company.id), String(row.controller)],
      {
        company: String(company.id),
        owner: String(row.controller),
        bank: String(facility.value.bank),
        drawn: room,
        ccy,
        settled: r.outcome === 'settled',
      },
      true,
    );
  }
}

/** Whether what this company was last asked to commit a lender to was its OWNER's need, not a buyer's. */
function askedForItsOwner(ctx: MechanismContext, company: PartyId): boolean {
  const said = ctx.journal.lastOf('control.financing', String(company));
  if (said === undefined || said.period < ctx.period - 2) return false;
  return said.data['cheque'] === 0;
}

/** B1: the row issued by the company, and the bank's own money created against it into its account. */
function legsFor(
  ctx: MechanismContext,
  company: PartyId,
  bank: PartyId,
  line: InstrumentId,
  ccy: CurrencyCode,
  amount: Qty,
): Leg[] {
  return [
    {
      kind: 'asset',
      from: company,
      to: bank,
      instrument: line,
      qty: amount,
      pricePerUnit: some(asPerPiece(1, 'at what it promised')),
      accruedPerUnit: none(),
    },
    {
      kind: 'money',
      from: { holder: bank, issuer: bank },
      to: ctx.accountOf(company, ccy),
      ccy,
      amount,
    },
  ];
}
