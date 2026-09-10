/**
 * What this world's money market is made of: the books it has, who counts as which kind of
 * depositor, and what each bank is like as a funder.
 *
 * @spec Money Market B6 Money Market B6.a Money Market C1 Money Market C2 Banks Funding A1 Banks Funding A1.a Banks Funding A1.b Banks Funding A1.c Banks Funding A1.d Banks Funding B1.a Banks Funding C2.a Central Bank B1 Central Bank D1 Law 2 Law 15
 *
 * Data only (Law 15). A1.d is the clause this file exists for: A MODEL WITH ONE DEPOSIT TYPE CANNOT
 * HAVE A RUN, so who banks where is classified here — by the kind of party the depositor is, in a
 * table, so that nothing anywhere branches on it — and a kind nobody classified has no deposit
 * class and is told so rather than given one.
 */
import { paramId, partyKindId, type ParamId, type PartyKindId } from '../../core/ids.js';
import { BANK, FIRM, HOUSEHOLD, SMALL_BUSINESS } from '../../registry/profiles.js';

/** B6: the books. A tenor and a security, which together say what a session strikes. */
export interface BookDecl {
  readonly id: string;
  readonly tenor: string;
  /** How many periods the row runs for (B6: overnight is this period's; term is longer). */
  readonly periods: number;
  readonly secured: boolean;
  readonly why: string;
}

export const BOOKS: readonly BookDecl[] = [
  {
    id: 'overnight.unsecured',
    tenor: 'overnight',
    periods: 1,
    secured: false,
    why: 'B2: the plainest thing in the market — cash to a name until the next close, priced by what the lender thinks of the name and nothing else.',
  },
  {
    id: 'overnight.secured',
    tenor: 'overnight',
    periods: 1,
    secured: true,
    why: 'B3: the same cash against paper. A name the market doubts can still borrow here while its paper lasts, which is what makes running out of collateral (B3.c) the thing that stops it.',
  },
  {
    id: 'term.unsecured',
    tenor: 'term',
    periods: 4,
    secured: false,
    why: 'B6, A2.a: a month of money, which is what a bank rolls. B6.a reads the gap against overnight as information about expected stress, so both books have to exist for either to mean anything.',
  },
  {
    id: 'term.secured',
    tenor: 'term',
    periods: 4,
    secured: true,
    why: 'B6 and B3 together: term money against paper, which is what a bank funds a securities book with when nobody will lend it a month unsecured.',
  },
];

/** A1: what kind of depositor a party is, which is a read of what the party IS. */
export interface DepositClassDecl {
  readonly id: string;
  readonly partyKinds: readonly PartyKindId[];
  /** A1.a: insured up to the limit, per member where the depositor is a cell (XI-15). */
  readonly insured: boolean;
  /**
   * A1.d, E1: what it costs a depositor of this class to move its account, per annum. THIS IS
   * WHERE STICKINESS LIVES, and it is a cost somebody bears rather than a stated stickiness: a
   * class that will not move for less than half a point is paid half a point less, and it is the
   * same number that decides whether it runs. A stated "retail is sticky" would be an outcome
   * written down; this is the reason behind it.
   */
  readonly switchingCost: number;
  readonly why: string;
}

export const DEPOSIT_CLASSES: readonly DepositClassDecl[] = [
  {
    id: 'retail',
    partyKinds: [HOUSEHOLD, SMALL_BUSINESS],
    insured: true,
    switchingCost: 0.006,
    why: 'A1.a: many, small, sticky and insured up to a limit — which is what a cell of members IS (XI-15), and what makes E4 break the loop for them and not for anybody else.',
  },
  {
    id: 'corporate',
    partyKinds: [FIRM, partyKindId('estate')],
    insured: false,
    switchingCost: 0.003,
    why: 'A1.b: fewer, larger, operational. A firm banks where it transacts, so this money moves because the firm is trading, not because a rate moved.',
  },
  {
    id: 'wholesale',
    partyKinds: [BANK, partyKindId('fund'), partyKindId('fundManager'), partyKindId('desk')],
    insured: false,
    switchingCost: 0.0002,
    why: 'A1.c: few, very large and rate-sensitive. This is the money that leaves first (E4.a), because nothing insures it and its holder is in the market all day anyway.',
  },
];

/** The class a party of this kind is in, or none: a kind nobody classified is not a depositor. */
export function classOf(kind: PartyKindId): DepositClassDecl | undefined {
  return DEPOSIT_CLASSES.find((c) => c.partyKinds.includes(kind));
}

/** What each bank is like as a funder: its own numbers, stated one bank at a time (Law 2, Seed B4). */
export interface FunderDecl {
  readonly bank: string;
  /**
   * B1.a, B3: what it keeps for itself out of what the money is worth to it — its margin on
   * funding, per annum. Its own: two banks that keep the same margin are one bank, and the one that
   * keeps less wins the deposit and earns less on it, which is what a net interest margin IS.
   */
  readonly depositMargin: number;
  /**
   * C2.a, A2.a: how far back it looks at its own account when it decides what buffer to hold. A
   * bank with a long memory holds against the worst week it has seen; a short one forgets it.
   */
  readonly bufferMemory: number;
  readonly why: string;
}

export const FUNDERS: readonly FunderDecl[] = [
  {
    bank: 'bank.a',
    depositMargin: 0.008,
    bufferMemory: 26,
    why: 'The cautious one: it holds against half a year of its own worst weeks, and it keeps a wide margin on the money it takes in rather than bidding for deposits it does not need.',
  },
  {
    bank: 'bank.b',
    depositMargin: 0.004,
    bufferMemory: 8,
    why: 'The keen one: a short memory of its own outflows, so a thinner buffer, and a thin margin on funding — it pays up for deposits and lives on the volume, which is the same disposition that makes it the keener lender.',
  },
];

export function funderOf(bank: string): FunderDecl | undefined {
  return FUNDERS.find((f) => f.bank === bank);
}

export const mmParam = (bank: string, what: string): ParamId => paramId(`bank.${what}.${bank}`);

export const switchingCost = (cls: string): ParamId => paramId(`deposits.switchingCost.${cls}`);

export const MM_PARAMS = {
  floorSpread: paramId('centralBank.corridor.floorSpread'),
  ceilingSpread: paramId('centralBank.corridor.ceilingSpread'),
  overdraftPenalty: paramId('centralBank.overdraftPenalty'),
  policyRate: paramId('centralBank.policyRate'),
  insuranceLimit: paramId('regulation.depositInsurance.limit'),
} as const;
