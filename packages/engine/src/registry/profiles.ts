/**
 * The kernel's own kinds: money, and the parties money needs (a central bank, a bank) plus the
 * party kinds every world has. Everything else is registered by a module.
 *
 * @spec Money A1 Money A1.b Money A1.c Money D2 Money B3.a Money B3.b Money B3.c XI-15 Households A2.e Small-Business Pools A6
 */
import { InvalidRegistry } from '../core/errors.js';
import { currencyUnit, instrumentKindId, partyKindId } from '../core/ids.js';
import type { InstrumentKindProfile, OverdraftDecision, PartyKindProfile } from './kinds.js';

export const MONEY_KIND = instrumentKindId('money');

export const moneyKind: InstrumentKindProfile = {
  id: MONEY_KIND,
  pricing: 'money',
  liabilityOfIssuer: true,
  unit: (ccy) => currencyUnit(ccy),
  validateTerms: (t) => {
    if (t.kind !== MONEY_KIND) throw new InvalidRegistry('Money D2', 'money terms carry no fields');
  },
  displayName: (i, issuerName) => `${issuerName} money ${i.ccy}`,
  due: () => [],
};

export const CENTRAL_BANK = partyKindId('centralBank');
export const TREASURY = partyKindId('treasury');
export const BANK = partyKindId('bank');
export const FIRM = partyKindId('firm');
export const HOUSEHOLD = partyKindId('household');
export const SMALL_BUSINESS = partyKindId('smallBusiness');

/** The party kinds the kernel needs or every world has (XI-15: institutions named, populations as cells). */
export const KERNEL_PARTY_KINDS: readonly PartyKindProfile[] = [
  {
    id: CENTRAL_BANK,
    representation: 'named',
    moneyIssuer: {
      // B3.b: a bank overdrawn at the central bank is borrowing from it and the corridor prices it.
      // Until the corridor exists (worklist 11, Central Bank D3.b) the overdraft is allowed and
      // recorded as a reserve overdraft; the Money audit family reports every one as unpriced.
      overdraft: (): OverdraftDecision => ({ allow: true, recordedAs: 'reserveOverdraft' }),
    },
  },
  {
    id: BANK,
    representation: 'named',
    moneyIssuer: {
      // B3.a: a customer overdrawn is a credit decision by its bank. The credit decision is Banks
      // Lending (worklist 6); until then the bank refuses and the refusal is recorded.
      overdraft: (): OverdraftDecision => ({ allow: false }),
    },
  },
  { id: TREASURY, representation: 'named', moneyIssuer: null },
  { id: FIRM, representation: 'named', moneyIssuer: null },
  { id: HOUSEHOLD, representation: 'cell', moneyIssuer: null },
  { id: SMALL_BUSINESS, representation: 'cell', moneyIssuer: null },
];
