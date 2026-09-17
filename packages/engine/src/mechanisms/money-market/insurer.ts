/**
 * The deposit insurer: a fund the banks pay into, that pays when one of them fails.
 *
 * @spec Banks Funding A1.a Banks Capital D4 Banks Capital D5 Money A1 XI-3 Law 2 Law 15
 *
 * A1.a says a deposit is insured up to a limit per member, and D4 says the insurance PAYS when the
 * book cannot. A guarantee with no fund behind it is a guarantee somebody else silently makes: the
 * treasury, every time, which deletes D4 into D5 and with it the whole distinction the spec draws
 * between what the banking system pays for and what the public purse does.
 *
 * So the insurer is a party with an account and a real income: every bank pays a PREMIUM each
 * period on the insured part of its own deposit base — a real payment, to a named payee, out of the
 * bank's own money, which is what makes insurance a cost of taking retail deposits rather than a
 * free option. What it has when a bank fails is what it has actually collected, and the treasury
 * stands behind whatever that does not cover — last, and as a fiscal cost with a payer (D5).
 *
 * It fails on nothing, and that is a consequence rather than an exemption: it holds no assets but
 * cash and owes nothing until a bank fails, so neither trigger XI-3 names can fire on it. What
 * happens when its fund is empty is not death, it is the purse.
 */
import { noCash, sumCash } from '../../core/measure.js';
import { currencyUnit, moneyInstrumentId, paramId, partyId, partyKindId } from '../../core/ids.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { type Cash, scale, heldAsMoney } from '../../core/measure.js';
import type { PartyKindProfile } from '../../registry/kinds.js';
import type { MechanismContext } from '../../world/context.js';
import { MM_PARAMS } from './data.js';
import { insuredAt } from './deposits.js';

export const DEPOSIT_INSURER = partyKindId('depositInsurer');

/**
 * A1.a: ONE PER CURRENCY, because one currency has one guarantee behind it — one issuer of the
 * money the deposits are in, one window they can be turned into it at, one set of banks under one
 * rule. 13j: there used to be one in this world, and it was right when there was one banking system
 * in it; a world with four has four, and a Japanese depositor is not guaranteed by an American fund.
 */
export const insurerOf = (ccy: CurrencyCode): PartyId => partyId(`insurer.${ccy}`);

export const INSURER_PARAMS = {
  premium: paramId('regulation.depositInsurance.premium'),
} as const;

export const insurerKind: PartyKindProfile = {
  /** item 15: what somebody else set it up to do, and it does not get to change it. */
  objective: 'itsMandate',
  id: DEPOSIT_INSURER,
  representation: 'named',
  moneyIssuer: null,
  // XI-3: neither trigger can fire on it. It holds cash and owes nothing until a bank fails, and
  // what happens when its fund is short is D5's purse rather than its own death.
  fails: [],
  cannotFail:
    'XI-3, Banks Capital D5: it owes nothing until a bank fails, and when its fund is short what pays is the purse behind it, not its death',
  // Banks Lending A1: nobody lends to it. What stands behind it is the state, not a creditor.
  borrows: false,
  buysOnTerms: false,
  // 17.8: Banks Lending A1.a, D4.a: a loan is not a security and is never distributed outside the banking system, so nobody of this kind is ever owed one.
  banking: false,
  // Its account is at the central bank, like the treasury's: the guarantee behind a banking system
  // does not sit inside one of the banks it guarantees.
  depositClass: null,
};

/**
 * D4, A1.a: the premium. Each bank pays on the insured part of its OWN deposit base, per member —
 * so a bank funded by a cell of small households pays for the guarantee it gets, and one funded by
 * a few large wholesale accounts pays almost nothing because almost nothing of it is covered. It is
 * the same split A1.a draws for the cover, read once and used for both (Law 4).
 */
export function collectPremiums(ctx: MechanismContext, banks: readonly PartyId[]): void {
  const rate = ctx.params.perAnnum(INSURER_PARAMS.premium);
  if (rate <= 0) return;
  for (const bank of banks) {
    const p = ctx.parties.get(bank);
    if (!p.status.alive) continue;
    standBehind(ctx, bank);
    const ccy = ctx.registry.currencyOf(p.region);
    // 13j, A1.a: a bank pays the guarantee of ITS OWN money, which is the one that covers it.
    const insurer = insurerOf(ccy);
    if (!ctx.parties.has(insurer) || !ctx.parties.get(insurer).status.alive) continue;
    const limit = ctx.params.amount(MM_PARAMS.insuranceLimit, currencyUnit(ccy));
    const covered: Cash[] = [];
    for (const holder of ctx.register.holdersOf(moneyInstrumentId(bank, ccy))) {
      if (holder === bank) continue;
      covered.push(insuredAt(ctx, bank, holder, ccy, limit));
    }
    const base = sumCash(ccy, covered, 'what is covered').value;
    const due = ctx.registry.payable(scale(base, rate, 'the premium on what is covered'));
    if (due <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          // 0i.5: what a bank pays for deposit cover.
          receipt: { of: 'premium' },
          from: ctx.accountOf(bank, ccy),
          to: ctx.accountOf(insurer, ccy),
          ccy,
          amount: due,
        },
      ],
      cause: 'transfer',
      reason: `${bank} pays the deposit guarantee for what it covers`,
    });
    ctx.record(
      'insurance.premium',
      [insurer, bank],
      { insurer, bank, covered: base.pieces, rate, due, paid: r.outcome === 'settled', ccy },
      true,
    );
  }
}

/** What the fund has to meet a guarantee with, for the reads that need it (D4). */
export function fundOf(ctx: MechanismContext, ccy: CurrencyCode): Cash {
  const insurer = insurerOf(ccy);
  if (!ctx.parties.has(insurer)) return noCash(ccy);
  const p = ctx.parties.get(insurer);
  // 0f.1: the register holds the cell's TOTAL.
  return heldAsMoney(
    ctx.register.quantity(insurer, moneyInstrumentId(p.bank, ccy)),
    ccy,
    'the fund',
  );
}

/**
 * A1.a, item 13: THE GUARANTEE ITSELF, said out loud once per bank.
 *
 * Deposit insurance worked before this and it worked by being an ORDERING OF PAYMENTS written into
 * the resolution path — the insurer pays, then the purse — rather than a thing anybody holds. So
 * nothing could be asked who stood behind a bank, a guaranteed deposit ranked in an estate exactly
 * like an unguaranteed one, and a second guarantee would have had to be written into a second place.
 *
 * It is given when the bank first pays a premium, which is when the cover starts: a bank that has
 * never paid is not insured, and that is the same fact read from the other end (Law 19). The
 * beneficiary is `whoeverHolds`, because a guarantee of DEPOSITS is given to whoever holds one and
 * naming every holder would be naming a set that changes every period.
 *
 * The limit is the one this world already declares — `regulation.depositInsurance.limit`, per
 * member — and it is a TERM of the promise rather than a clamp on a number (Law 6).
 */
function standBehind(ctx: MechanismContext, bank: PartyId): void {
  const ccy = ctx.registry.currencyOf(ctx.parties.get(bank).region);
  const insurer = insurerOf(ccy);
  if (!ctx.parties.has(insurer)) return;
  const already = ctx.guarantees.behind(bank);
  if (already.some((g) => g.guarantor === insurer && g.basis === 'insurance')) return;
  ctx.guarantee({
    guarantor: insurer,
    obligor: bank,
    beneficiary: 'whoeverHolds',
    what: `the insured part of what ${bank} owes its depositors`,
    ccy,
    limit: null,
    basis: 'insurance',
    why: 'Banks Funding A1.a: a deposit is insured up to a limit per member, and D4 says the insurance PAYS when the book cannot. What the fund cannot meet is the purse (D5), which is why this promise has no limit of its own: the LIMIT IS PER MEMBER AND PER DEPOSIT, declared as `regulation.depositInsurance.limit` and applied where the cover is worked out, not to the scheme as a whole.',
  });
}
