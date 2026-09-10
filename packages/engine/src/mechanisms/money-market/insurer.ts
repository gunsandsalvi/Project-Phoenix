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
import { currencyUnit, moneyInstrumentId, paramId, partyId, partyKindId } from '../../core/ids.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { mul, sum } from '../../core/num.js';
import { none } from '../../core/option.js';
import { weightOf } from '../../parties/party.js';
import type { PartyKindProfile } from '../../registry/kinds.js';
import type { MechanismContext } from '../../world/context.js';
import { MM_PARAMS } from './data.js';
import { insuredAt } from './deposits.js';

export const DEPOSIT_INSURER = partyKindId('depositInsurer');

/** One in this world, because one currency has one guarantee behind it (A1.a). */
export const INSURER: PartyId = partyId('insurer.north');

export const INSURER_PARAMS = {
  premium: paramId('regulation.depositInsurance.premium'),
} as const;

export const insurerKind: PartyKindProfile = {
  id: DEPOSIT_INSURER,
  representation: 'named',
  moneyIssuer: null,
  // XI-3: neither trigger can fire on it. It holds cash and owes nothing until a bank fails, and
  // what happens when its fund is short is D5's purse rather than its own death.
  fails: [],
  // Banks Lending A1: nobody lends to it. What stands behind it is the state, not a creditor.
  borrows: false,
  // Its account is at the central bank, like the treasury's: the guarantee behind a banking system
  // does not sit inside one of the banks it guarantees.
  choosesBank: false,
};

/**
 * D4, A1.a: the premium. Each bank pays on the insured part of its OWN deposit base, per member —
 * so a bank funded by a cell of small households pays for the guarantee it gets, and one funded by
 * a few large wholesale accounts pays almost nothing because almost nothing of it is covered. It is
 * the same split A1.a draws for the cover, read once and used for both (Law 4).
 */
export function collectPremiums(ctx: MechanismContext, banks: readonly PartyId[]): void {
  if (!ctx.parties.has(INSURER) || !ctx.parties.get(INSURER).status.alive) return;
  const rate = ctx.params.get(INSURER_PARAMS.premium);
  if (rate <= 0) return;
  for (const bank of banks) {
    const p = ctx.parties.get(bank);
    if (!p.status.alive) continue;
    const ccy = ctx.registry.region(p.region).ccy;
    const limit = ctx.params.amount(MM_PARAMS.insuranceLimit, currencyUnit(ccy));
    const covered: number[] = [];
    for (const holder of ctx.register.holdersOf(moneyInstrumentId(bank, ccy))) {
      if (holder === bank) continue;
      covered.push(insuredAt(ctx, bank, holder, ccy, limit));
    }
    const base = sum(covered).value;
    const due = ctx.registry.payable(ccy, mul(base, rate, 'the premium on what is covered'));
    if (due <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: { holder: bank, issuer: p.bank },
          to: { holder: INSURER, issuer: ctx.parties.get(INSURER).bank },
          ccy,
          amount: due,
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'transfer',
      reason: `${bank} pays the deposit guarantee for what it covers`,
    });
    ctx.record(
      'insurance.premium',
      [INSURER, bank],
      { insurer: INSURER, bank, covered: base, rate, due, paid: r.outcome === 'settled', ccy },
      true,
    );
  }
}

/** What the fund has to meet a guarantee with, for the reads that need it (D4). */
export function fundOf(ctx: MechanismContext, ccy: CurrencyCode): number {
  if (!ctx.parties.has(INSURER)) return 0;
  const p = ctx.parties.get(INSURER);
  return mul(
    ctx.register.quantity(INSURER, moneyInstrumentId(p.bank, ccy)),
    weightOf(p),
    'the fund',
  );
}
