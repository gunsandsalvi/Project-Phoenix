/**
 * The credit view: what ONE bank believes about every name it has a reason to price, formed once a
 * period from its own record and published under its own name.
 *
 * @spec Banks Lending B2 Banks Lending B2.a Banks Lending B2.b Banks Lending B2.c Banks Lending B2.d Banks Lending C1 Banks Lending C1.a Banks Lending C1.b Banks Lending C1.c Banks Lending C1.d Banks Lending C3 Banks Lending C4 Banks Lending F3 Corporate Credit A4 Corporate Credit A4.a Corporate Credit A4.b Corporate Credit E5 Corporate Credit E5.a Corporate Credit E5.b Corporate Credit E5.c Corporate Credit E7 Ratings C2 XI-4 Law 2 Law 4 Law 19
 *
 * Corporate Credit A4: creditworthiness is an OPINION HELD BY SOMEBODY, and here the somebody is a
 * bank. Its opinion of a name has two halves and both are its own record (C1.b, C4: one model per
 * borrower, used for the price, the provision and the reservation alike):
 *
 * - how often it has seen that name fail to pay, PER YEAR, over the memory it keeps — a frequency of
 *   events it observed, never a hazard that produces them (XI-1);
 * - what it has lost when a name it lent to went into an estate — its OWN recoveries, read off the
 *   claims it handed to estates and what those estates paid for them. A bank that has never been a
 *   creditor of an estate has seen nothing recovered and treats all of a claim as at risk, which is
 *   what ignorance honestly says; a bank that has been paid in full by every estate it met expects
 *   to lose little, and the two quote the same name apart (A4.b: the disagreement is the market).
 *
 * The third term is not its opinion but the regulation's use of somebody else's: the capital a claim
 * consumes is weighted by the GRADE the assessors publish on the name (Ratings C2), and a name nobody
 * has graded carries the weight an ordinary loan does. The fourth is what money costs this bank
 * (C1.a, E5.a) and the fifth what running a loan costs it (C1.d) — holding a bond is not work, so a
 * bank requires less to hold a name's paper than to lend to it.
 *
 * AND THREE THINGS IT MAY READ ABOUT A NAME BEYOND ITS OWN RECORD (17.0a, the owner's rule): the
 * statement the name PUBLISHED or SHOWED it — with the ask, as its lender of record, as the bank that
 * keeps its account — and never one it was not shown; the coverage in that statement (Corporate
 * Credit A3.b: what the name took in against what servicing its debt took, both real payments); and
 * what the MARKET requires of the name's own traded paper, a yield derived from the print (Law 3).
 * Each is a reason to DECLINE (Banks Lending C3, the credit decision), said by name so declined
 * volume is visible (C3.a): a name that prepared books and did not show them; a name whose earnings
 * did not cover its debt service; a name whose paper the market already prices above the rate this
 * bank would lend at — nobody lends at six to a name whose bonds yield fifteen, it buys the bonds
 * (XI-4: the alternative is the cost of capital). What it requires to HOLD the paper stays its own
 * (XI-13): a reservation read off the print is the fixed point that stopped the world at 13b.
 *
 * ONE DERIVATION, MANY READERS (Law 4). The loan quote, the overdraft's row, the provision, the desk's
 * view of a dated claim, the collateral a lender advances against and the reservation a bond book is
 * built from all read this — and it is computed once per bank per period, so nothing in a period can
 * see two answers from one bank about one name.
 */
import { yearFraction } from '../../calendar/daycount.js';
import { period } from '../../calendar/calendar.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import {
  asAmount,
  asCash,
  asRatio,
  atMostCash,
  type Cash,
  heldAsMoney,
  minus,
  noCash,
  type PerPiece,
  plus,
  type Ratio,
  ratioOf,
  scale,
  sumCash,
  valueAt,
} from '../../core/measure.js';
import { atMost } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { asQty, type Qty } from '../../core/tick.js';
import type { Event } from '../../journal/journal.js';
import { isAssetLeg } from '../../ledger/instruction.js';
import { couldLeave, liquidHeld } from '../../registry/banking.js';
import { type Grade, middleGrade, riskWeightParam } from '../../registry/grades.js';
import { gradesOn } from '../../registry/notices.js';
import { coverageOf, leverageOf, type Statement, statementVisibleTo } from '../../registry/statements.js';
import type { DayCount } from '../../calendar/daycount.js';
import { yieldOf } from '../../prices/curve.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import { bankParam, type BankDecl } from './data.js';
import { LENDING, roomFor } from './lines.js';

/** C1: the four terms, kept apart so a reader can see which one moved (B2.d, XI-4). */
export interface Quote {
  readonly bank: PartyId;
  /** Item 16: every one of these is a RATE PER ANNUM on a unit lent — a pure number, never money. */
  readonly costOfFunds: Ratio;
  readonly expectedLoss: Ratio;
  readonly capitalCharge: Ratio;
  readonly operatingCost: Ratio;
  readonly rate: Ratio;
}

/** What the standard-setter asks of every bank, and what running this bank's book costs it. */
export interface Regulation {
  readonly capitalRatio: Ratio;
  /** The weight an exposure carries when no assessor has said anything about the name. */
  readonly riskWeight: Ratio;
  readonly operatingCost: Ratio;
}

/** C3: why this bank will not lend to a name, when it will not. */
export type Declined = 'undisclosed' | 'coverage' | 'marketYield';

/** A4: one bank's opinion of one name, and every number the opinion is made of. */
export interface NameView extends Quote {
  /** Corporate Credit A3.b: what it earned against what its debt took, from the statement it showed; nothing without one. */
  readonly coverage: Option<Ratio>;
  /** Corporate Credit A2.b: what it owes against what it earns, from the same statement. */
  readonly leverage: Option<Ratio>;
  /** What the market requires of the name's own traded paper, or nothing where it has none priced. */
  readonly marketYield: Option<Ratio>;
  /** C3: the reason this bank declines the name for new lending, or nothing. */
  readonly declines: Option<Declined>;
  /** C1.b: how often per year this bank has seen the name fail, over its memory. */
  readonly probabilityOfDefault: Ratio;
  /** C1.b: what it expects to lose of a unit that fails, from its own recoveries. */
  readonly lossGivenDefault: Ratio;
  /** Ratings C2: the middle of what the assessors publish on the name, or nothing. */
  readonly grade: Option<Grade>;
  readonly riskWeight: Ratio;
  /** E5: what it requires to HOLD the name's paper — the loan rate without the cost of running a loan. */
  readonly required: Ratio;
}

/**
 * What one bank's own claims on estates came to: the pieces the estates paid and the pieces they
 * wrote off. Both are counts of the money the claims promised.
 */
export interface Recovered {
  readonly paid: Qty;
  readonly lost: Qty;
}

/** The inputs a bank's view is formed from. Each is a read, and each is named so a test can vary one. */
export interface CreditInputs {
  /** C1.a, E5.a: what this bank's funding costs it in the money the view is in, per annum. */
  readonly funds: Ratio;
  readonly reg: Regulation;
  /** C1.b: every default anybody has published. They are public, so this bank saw all of them (A2). */
  readonly defaults: readonly Event[];
  /** C1.b: what this bank's claims on estates came to. */
  readonly recovered: Recovered;
  /** Ratings C2: the grade the regulation weights a name by, or nothing where nobody has graded it. */
  readonly gradeOn: (name: PartyId) => Option<Grade>;
  /** C1.c: the weight a claim on this name carries under the standard — by grade, or by what it is. */
  readonly weightOf: (name: PartyId, grade: Option<Grade>) => Ratio;
  /** Observer A3, A4: the latest statement of the name this bank may read, or nothing. */
  readonly statementOf: (name: PartyId) => Option<Statement>;
  /** Reporting A1: whether the name has prepared any statement at all — a young company has not. */
  readonly prepared: (name: PartyId) => boolean;
  /** Law 3: the highest yield the market requires of the name's own priced, dated paper. */
  readonly marketYieldOn: (name: PartyId) => Option<Ratio>;
}

/** One day count for a yield read off a print here, stated once by the module that asks. */
export const CREDIT_DAY_COUNT: DayCount = 'ACT/365F';

/**
 * Law 3, XI-4: WHAT THE MARKET REQUIRES OF A NAME, read off the prints of the name's own dated paper
 * — the yield that discounts each line's remaining payments to its last print, and the highest of
 * them, because the line the market trusts least is the one that says most. Nothing where the name
 * has no priced dated claim: a name the market has never priced has no market opinion.
 */
export function marketYieldOn(view: ParticipantView, name: PartyId): Option<Ratio> {
  const on = view.calendar.startOf(view.period);
  let highest: Ratio | undefined;
  for (const i of view.instruments.issuedBy(name)) {
    if (!i.status.live || !i.market.some) continue;
    const profile = view.registry.instrumentKind(i.kind);
    if (!profile.liabilityOfIssuer || profile.pricing !== 'cleared') continue;
    const print = view.print(i.id);
    if (!print.some || print.value.price <= 0) continue;
    const flows = profile.cashFlows(i, on, view.calendar, view.registry);
    if (flows.length === 0) continue;
    const y = yieldOf(flows, print.value.price, on, CREDIT_DAY_COUNT, `what the market requires of ${i.id}`);
    if (y.some && (highest === undefined || y.value > highest)) highest = y.value;
  }
  return highest === undefined ? none<Ratio>() : some(highest);
}

/**
 * Corporate Credit A3.b: COVERAGE, from the statement — what the name earned before interest, tax,
 * wear and the marks (Clearing D4: a revaluation is nobody's payment), against what servicing its
 * debt took (A3.a: interest plus principal, both real payments). Nothing where the quarter had no
 * service to cover: a name that owed nothing has no ratio, which is a different answer from
 * covering it infinitely. The sums are the statement's own (Law 4).
 */
export function coverageIn(shown: Statement): Option<Ratio> {
  const c = coverageOf(shown);
  return c.some ? some(asRatio(c.value, 'what it earned against what its debt took')) : none<Ratio>();
}

/** Debt over what it earns before interest, tax, wear and the marks; nothing where that is not positive. */
export function leverageIn(shown: Statement): Option<Ratio> {
  const l = leverageOf(shown);
  return l.some ? some(asRatio(l.value, 'what it owes against what it earns')) : none<Ratio>();
}

/**
 * C1.b, C4: how often this bank has seen this borrower fail to pay, PER YEAR, over the memory it
 * keeps. Distinct periods with a default on the name, over the years those periods span — read off
 * the calendar's own dates rather than a periods-per-year (Law 8). A world too young to remember a
 * year remembers what it has; a borrower with no history has failed none of it.
 */
export function defaultFrequency(
  view: ParticipantView,
  decl: BankDecl,
  borrower: PartyId,
  defaults: readonly Event[],
): Ratio {
  const memory = view.params.periods(bankParam(decl.bank, 'credit.memory'));
  const remembered = atMost(view.period, memory, 'a world cannot remember before it began');
  if (remembered <= 0) return asRatio(0, 'a borrower with no history has failed none of it');
  const from = view.period - remembered;
  const failures = new Set(
    defaults
      .filter((e) => e.period >= from && e.subjects.includes(borrower))
      .map((e) => e.period),
  ).size;
  const years = yearFraction(
    'ACT/365F',
    view.calendar.startOf(period(from)),
    view.calendar.startOf(view.period),
  );
  if (years <= 0) return asRatio(0, 'a memory of no length has seen nothing fail');
  return asRatio(failures / years, 'how often per year this borrower has failed');
}

/**
 * C1.b, Corporate Credit E5.b: WHAT THIS BANK HAS LOST OF A CLAIM THAT WENT INTO AN ESTATE, over
 * every claim it has ever handed to one. A bank that has met no estate has seen nothing recovered
 * and treats all of a unit as at risk — not a stated recovery rate but the absence of a record
 * (Appendix B forbids the fixed one). The first estate it is paid by moves it.
 */
export function lossFromRecoveries(recovered: Recovered): Ratio {
  const met = plus(recovered.paid, recovered.lost, 'what its claims on estates came to');
  if (met <= 0) return asRatio(1, 'it has met no estate: all of a claim is at risk');
  return ratioOf(recovered.lost, met, 'the part of a claim on an estate it has lost');
}

/**
 * C1.b, C5.a (13d): what a SECURED claim would lose, at the market's own price of the security.
 * The security covers what it fetches; the part it does not cover is at risk as an unsecured claim
 * on the name is — this bank's own loss from recoveries. A pledge of something nobody prices covers
 * nothing, which is a real answer and not a zero anybody chose; a claim secured on more than it
 * lends loses nothing.
 */
export function lossGivenDefault(
  unsecured: Ratio,
  security: readonly { readonly instrument: InstrumentId; readonly qty: number }[],
  /** What the market last said a unit of the thing pledged is worth, or none because it has not said. */
  worthOf: (instrument: InstrumentId) => PerPiece | undefined,
  /** What is owed, in the same money. */
  owed: Cash,
): Ratio {
  return scale(unsecured, uncoveredShare(security, worthOf, owed), 'on the uncovered part');
}

/**
 * C5.a (17.7c): HOW MUCH OF A CLAIM ITS SECURITY DOES NOT STAND BEHIND, at the market's own price of
 * that security, as a share of what is owed.
 *
 * It is the whole of what a pledge does to a lender's arithmetic, and it is separated from the loss
 * itself because two readers need it and they need different things (Law 4). A lender provisioning a
 * row wants the loss: this share times what it expects to lose on the name. A desk pricing a claim
 * on a name it has already published an expected loss for wants the SHARE — its published number is
 * for the name, and what this claim is secured on is what makes this claim different from it.
 *
 * One with nothing pledged, nothing where the security is worth more than the claim, and everything
 * in between is arithmetic. A pledge of something nobody prices covers nothing, which is a real
 * answer and not a zero anybody chose.
 */
export function uncoveredShare(
  security: readonly { readonly instrument: InstrumentId; readonly qty: number }[],
  worthOf: (instrument: InstrumentId) => PerPiece | undefined,
  owed: Cash,
): Ratio {
  if (security.length === 0 || owed.pieces <= 0) return asRatio(1, 'nothing stands behind it');
  const behind = sumCash(
    owed.ccy,
    security.map((row) => {
      const price = worthOf(row.instrument);
      if (price === undefined) return noCash(owed.ccy);
      return valueAt(price, asQty(row.qty, 'the units pledged'), owed.ccy, 'what the security is worth');
    }),
    'what stands behind it',
  ).value;
  const uncovered = minus(owed, behind, 'the part the security does not cover');
  if (uncovered.pieces <= 0) return asRatio(0, 'covered: nothing of it is at risk');
  return ratioOf(uncovered, owed, 'of every unit lent');
}

/** Ratings C2, A5.a: the middle of what the assessors have published on a name, or nothing. */
export function gradeOn(reads: Parameters<typeof gradesOn>[0], name: PartyId): Option<Grade> {
  const grade = middleGrade([...gradesOn(reads, String(name)).values()]);
  return grade === undefined ? none<Grade>() : some(grade);
}

/**
 * Ratings C2: the weight the standard puts on a claim on a GRADED name, read from the register the
 * standard-setter declared it in; an ungraded name carries the weight the caller says an ordinary
 * exposure does. A supervisor's rule refers to somebody else's opinion, so an assessor's mistake
 * becomes a capital shortfall without anybody having mispriced anything — which is the point of C2.
 */
export function gradedWeight(view: ParticipantView, grade: Option<Grade>, ungraded: Ratio): Ratio {
  return grade.some ? view.params.ratio(riskWeightParam(grade.value)) : ungraded;
}

/** C1: this bank's view of one name, every term from its own record and the public one. */
export function nameView(
  view: ParticipantView,
  decl: BankDecl,
  name: PartyId,
  inputs: CreditInputs,
): NameView {
  const probabilityOfDefault = defaultFrequency(view, decl, name, inputs.defaults);
  const unsecuredLoss = lossFromRecoveries(inputs.recovered);
  const expectedLoss = scale(probabilityOfDefault, unsecuredLoss, 'expected loss');
  const grade = inputs.gradeOn(name);
  const riskWeight = inputs.weightOf(name, grade);
  // C1.c, E5.c: the capital the claim consumes, times what this bank needs to earn on it.
  const consumed = scale(riskWeight, inputs.reg.capitalRatio, 'capital consumed per unit');
  const capitalCharge = scale(
    consumed,
    view.params.perAnnum(bankParam(decl.bank, 'returnOnCapital')),
    'capital charge',
  );
  const required = plus(
    plus(inputs.funds, expectedLoss, 'funds and loss'),
    capitalCharge,
    'what it requires to hold the name',
  );
  const rate = plus(required, inputs.reg.operatingCost, 'the rate it quotes');
  // C3, Observer A4: what it may read about the name beyond its own record, and what each says.
  const shown = inputs.statementOf(name);
  const coverage = shown.some ? coverageIn(shown.value) : none<Ratio>();
  const leverage = shown.some ? leverageIn(shown.value) : none<Ratio>();
  const marketYield = inputs.marketYieldOn(name);
  const declines = ((): Option<Declined> => {
    // A name that has books and did not open them is a name it will not lend to blind; one too
    // young to have closed a quarter is priced on the record, which is all anybody has of it.
    if (!shown.some && inputs.prepared(name)) return some<Declined>('undisclosed');
    // A3.b: coverage below one is arithmetic — it did not take in what its debt took — not a line
    // anybody drew; a name in that state is one this bank expects to fail on the next payment.
    if (coverage.some && coverage.value < 1) return some<Declined>('coverage');
    // XI-4: it does not lend below where the market already prices the name; it would buy the paper.
    if (marketYield.some && marketYield.value > rate) return some<Declined>('marketYield');
    return none<Declined>();
  })();
  return {
    coverage,
    leverage,
    marketYield,
    declines,
    bank: view.self.id,
    costOfFunds: inputs.funds,
    expectedLoss,
    capitalCharge,
    operatingCost: inputs.reg.operatingCost,
    rate,
    probabilityOfDefault,
    lossGivenDefault: unsecuredLoss,
    grade,
    riskWeight,
    required,
  };
}

/** One bank's view this period: every name it is asked about, answered once (Law 4). */
export interface CreditView {
  readonly bank: PartyId;
  readonly ccy: CurrencyCode;
  readonly costOfFunds: Ratio;
  readonly lossGivenDefault: Ratio;
  of(name: PartyId): NameView;
  /** F3: what this name already owes it, read once a period. */
  exposureTo(name: PartyId): Cash;
  /** The names it has been asked about so far this period. */
  asked(): readonly PartyId[];
}

export function creditView(
  view: ParticipantView,
  decl: BankDecl,
  ccy: CurrencyCode,
  inputs: CreditInputs,
): CreditView {
  const names = new Map<string, NameView>();
  const exposures = new Map<string, Cash>();
  return {
    bank: view.self.id,
    ccy,
    costOfFunds: inputs.funds,
    lossGivenDefault: lossFromRecoveries(inputs.recovered),
    of: (name) => {
      const had = names.get(String(name));
      if (had !== undefined) return had;
      const made = nameView(view, decl, name, inputs);
      names.set(String(name), made);
      return made;
    },
    exposureTo: (name) => {
      const had = exposures.get(String(name));
      if (had !== undefined) return had;
      const made = exposureTo(view, name);
      exposures.set(String(name), made);
      return made;
    },
    asked: () => [...names.keys()].map((k) => k as PartyId),
  };
}

/**
 * C1.b: WHAT THIS PARTY'S CLAIMS ON ESTATES CAME TO, read off the settled ledger: every claim it
 * handed to a party of a terminal kind — an estate — was either paid for at what it promised or
 * written off at nothing, and the price on the leg says which. Walked once per period over the
 * periods that have closed (Law 18: a settled period cannot change) and kept as a working memo of
 * that walk, never as a second copy of the ledger (Law 19).
 */
interface RecoveryWalk {
  through: Option<number>;
  byParty: Map<string, { paid: number; lost: number }>;
}

export function recoveriesOf(ctx: MechanismContext, holder: PartyId): Recovered {
  const walk = ctx.state<RecoveryWalk>('banks.recoveries', () => ({
    through: none<number>(),
    byParty: new Map(),
  }));
  const first = walk.through.some ? walk.through.value + 1 : 0;
  for (let p = first; p < ctx.period; p += 1) {
    for (const r of ctx.ledger.inPeriod(period(p))) {
      if (r.outcome !== 'settled') continue;
      for (const leg of r.instruction.legs) {
        if (!isAssetLeg(leg) || !leg.pricePerUnit.some) continue;
        if (!ctx.parties.has(leg.to)) continue;
        if (ctx.registry.partyKind(ctx.parties.get(leg.to).kind).terminal !== true) continue;
        const paid = leg.qty * leg.pricePerUnit.value;
        const had = walk.byParty.get(String(leg.from));
        const next = had ?? { paid: 0, lost: 0 };
        next.paid += paid;
        next.lost += leg.qty - paid;
        if (had === undefined) walk.byParty.set(String(leg.from), next);
      }
    }
    walk.through = some(p);
  }
  const mine = walk.byParty.get(String(holder));
  return {
    paid: asAmount<'piece'>(mine === undefined ? 0 : mine.paid, 'what estates paid it'),
    lost: asAmount<'piece'>(mine === undefined ? 0 : mine.lost, 'what estates wrote off on it'),
  };
}

/** C1.b: the events of this world a bank's view is formed from, read by their public kinds. */
export function creditInputs(
  ctx: MechanismContext,
  view: ParticipantView,
  funds: Ratio,
  reg: Regulation,
  defaults: readonly Event[],
  weightOf: (name: PartyId, grade: Option<Grade>) => Ratio,
  /** Corporate Credit A4: the books borrowers opened with their asks this period, by name. */
  asked: ReadonlyMap<string, Statement>,
): CreditInputs {
  return {
    funds,
    reg,
    defaults,
    recovered: recoveriesOf(ctx, view.self.id),
    gradeOn: (name) => gradeOn(ctx.journal, name),
    weightOf,
    statementOf: (name) => {
      // What the name showed it or published, or what it opened with its ask — whichever is later.
      const seen = statementVisibleTo(view, name);
      const withAsk = asked.get(String(name));
      if (withAsk === undefined) return seen;
      if (!seen.some || withAsk.preparedIn >= seen.value.preparedIn) return some(withAsk);
      return seen;
    },
    prepared: (name) => ctx.latestReportOf(name).some,
    marketYieldOn: (name) => marketYieldOn(view, name),
  };
}

/**
 * B2: what stops it. Three constraints, separate because they are separate things, and which one
 * binds is an outcome (B2.d). B2.b — liquidity — is the funding room below: the deposit it creates
 * may be spent away and it must fund that.
 */
export interface Room {
  /** What its capital leaves it, as its own module published it. Missing before it has (B1). */
  readonly capital: Option<Cash>;
  readonly appetite: Cash;
  /**
   * Banks Funding D4: what its own funding leaves it able to lend. Missing where no funding market
   * has said anything about this bank yet — a world with none has no funding condition to transmit.
   */
  readonly funding: Option<Cash>;
  readonly most: Cash;
  readonly binds: 'capital' | 'appetite' | 'funding' | 'nothing';
}

/**
 * Banks Funding D4, D4.a: THE CREDIT CRUNCH, and it belongs here rather than in a rule about
 * lending. A bank writes a loan by creating a deposit (B1.a), and a deposit walks: the borrower
 * borrowed in order to pay somebody, and paying somebody who banks elsewhere moves liquid assets it
 * has to have. So what it will lend is what it holds beyond what could leave it — its own two
 * published sides (C1 against C2) — and a bank already short of that lends nothing at all.
 *
 * Both numbers are the funding module's (C1, C2.a) and they arrive the only way anything crosses a
 * module boundary: as the public thing that bank published about itself (F4). It is last period's
 * publication, which is what a lag IS — the credit decision is taken before this period's session
 * has told anybody anything.
 */
export function fundingRoom(view: ParticipantView): Option<Cash> {
  const liquid = liquidHeld(view);
  const exposed = couldLeave(view);
  if (!liquid.some || !exposed.some) return none<Cash>();
  // Item 16: its own published liquidity re-enters here — two moneys it said it had and could lose.
  return some(
    minus(
      asCash(
        liquid.value,
        view.registry.currencyOf(view.self.region),
        'what it published it holds liquid',
      ),
      asCash(
        exposed.value,
        view.registry.currencyOf(view.self.region),
        'what it published could leave',
      ),
      'what it can lend and still cover what could leave',
    ),
  );
}

/** Which of the three is the constraint: the smallest of them, named so a refusal says why (C3.a). */
function bindingOf(capital: Option<Cash>, appetite: Cash, funding: Option<Cash>): Room['binds'] {
  const cap = capital.some ? capital.value : appetite;
  if (funding.some && funding.value.pieces <= cap.pieces && funding.value.pieces <= appetite.pieces)
    return 'funding';
  return capital.some && cap.pieces < appetite.pieces ? 'capital' : 'appetite';
}

export function room(view: ParticipantView, decl: BankDecl, exposure: Cash): Room {
  const capital = view.equity();
  // Banks Capital B1, B1.b, B1.c: what its capital leaves it is the position its own module
  // published this period (`bank.capital`) — the weighted rule and the leverage backstop, whichever
  // of the two leaves it less, in units of the asset it is deciding about. It is READ and not
  // recomputed here, because a bank with two answers to how much capital it has would be a bank
  // that lends against one and reports the other (Law 4, Law 19). A bank that has published no
  // position has not decided anything about it (Appendix A) and is constrained by the rest.
  //
  // XI-4, Dealer Desks F2: and it is the room ITS TREASURY ALLOTTED THIS LINE, not the whole bank's.
  // The treasury allots to what earned (`bank.lines`), and a lending line behind the dealing line
  // in a period when the room ran out has none, which is what scarce capital means.
  const byCapital = roomFor(view, LENDING);
  // F3, B2.c: the most it will have out to one name, whatever its capital would allow.
  const limit = scale(
    capital,
    view.params.ratio(bankParam(decl.bank, 'limitPerBorrower')),
    'its limit for one name',
  );
  const byAppetite = minus(limit, exposure, 'room under its limit');
  const byFunding = fundingRoom(view);
  // F3, B2.b: three real constraints and the tightest is the one that binds. A constraint this
  // bank has not published a position for is not a constraint it has (Appendix A), so it is skipped
  // rather than treated as zero.
  const least = (a: Cash, b: Option<Cash>): Cash =>
    b.some
      ? atMostCash(a, b.value, 'it lends no more than the tightest of its own limits allows')
      : a;
  const most = least(least(byAppetite, byCapital), byFunding);
  return {
    capital: byCapital,
    appetite: byAppetite,
    funding: byFunding,
    most,
    binds: most.pieces <= 0 ? bindingOf(byCapital, byAppetite, byFunding) : 'nothing',
  };
}

/** F3: what it already has out to this name. */
export function exposureTo(view: ParticipantView, borrower: PartyId): Cash {
  const home = view.registry.currencyOf(view.self.region);
  let total = noCash(home);
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    // F3: EVERYTHING THAT NAME OWES IT, not everything of one kind. A limit that counted only loans
    // would be a limit a bank could go round by lending the same name money in another shape — a
    // week of unsecured money, a claim behind every other claim on it, a balance at it — and the
    // limit would bind on the one exposure it happened to be written about (Law 15, Law 19).
    if (!view.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    if (!i.issuer.some || view.parties.resolve(i.issuer.value).id !== borrower) continue;
    const units = view.quantity(h.instrument);
    if (units <= 0) continue;
    // F3, Law 19: WHAT THE NAME OWES IT is the FACE. Reading the lender's mark here was the wrong
    // quantity (mark vs face) and a cycle: a loan's mark asks the lender's own valuer, which reads
    // this exposure, which asks the mark.
    total = plus(
      total,
      view.inMoney(heldAsMoney(units, i.ccy, 'at the face it owes'), home),
      'exposure to one name',
    );
  }
  return total;
}
