/**
 * Prime brokerage: the bank that holds a client's book, lends against it, and decides every period
 * how much of it the client may finance.
 *
 * @spec Prime Brokerage A1 Prime Brokerage A2 Prime Brokerage A3 Prime Brokerage A4 Prime Brokerage B1 Prime Brokerage B2 Prime Brokerage B3 Prime Brokerage B4 Prime Brokerage C1 Prime Brokerage C1.a Prime Brokerage C1.b Prime Brokerage C2 Prime Brokerage C3 Prime Brokerage C3.b Prime Brokerage C4 Prime Brokerage C4.a Prime Brokerage C5 Prime Brokerage E1 Hedge Funds B1 Hedge Funds B3 XI-2 Law 3 Law 6 Law 19
 *
 * IT IS A BANK'S FIFTH BUSINESS LINE, which is why it lives here beside lending, funding, capital
 * and the dealing desk (ARCHITECTURE 4.11b) rather than in a module of its own: what it does is
 * WRITE A LOAN, and the loan, its rate, the capital it consumes and the room the bank has for it
 * are all already the bank's own economics. What is new is only the decision about how much.
 *
 * C1 IS WHAT THE SPEC CALLS THE CORE, and it is a decision and not a formula: *"the broker sets a
 * requirement on the whole portfolio from its own view of the risk, accounting for offsetting
 * positions — a decision by the broker, not a formula the client can rely on"*. So the number is
 * built out of the BROKER'S OWN OUTLOOKS: what it requires against a line is how wide its own recent
 * surprises about that line have been (§46 B3, `Outlook.confidence`). Two brokers looking at one
 * portfolio want different amounts, because they have seen different things and been wrong by
 * different amounts — which is what makes it a view rather than a rule.
 *
 * AND C4.a FALLS OUT OF THAT RATHER THAN BEING WRITTEN. *"Raising margin into a falling market
 * amplifies the fall — the mechanism behind most of what looks like contagion."* A market that moves
 * violently makes every observer's recent surprises wider; wider surprises are a larger requirement;
 * a larger requirement on a book that is already worth less is a call, and meeting a call means
 * selling into the fall. Nothing states a margin rate, so nothing had to remember to raise it.
 *
 * C3.b IS A REFUSAL TO FLOOR, and it is stated as a rule because it is the one place the whole path
 * can be silently deleted: *"a client drawn past its line is over the line, and the shortfall is
 * what forces the sale. Flooring it makes the whole path unreachable — and lending the shortfall
 * straight back at a penalty, from the same broker, makes it unreachable twice."* `available` is a
 * subtraction and it is allowed to come out negative. That negative number IS the call.
 *
 * WHAT IS NOT MARGINED HERE. The client's CONTRACT positions are margined by the derivative layer
 * with the counterparty that holds them (`derivative-layer/margin.ts`), per pair, as the layer's own
 * C1 requires. Asking for margin on them here as well would be one exposure collateralised twice,
 * which is what Appendix B means by no collateral counted twice — so this is a requirement over the
 * REGISTER: the securities the broker holds for the client, which is what A3 says its knowledge is.
 */
import { atMostCash, noCash, sumCash } from '../../core/measure.js';
import {
  agreementKindId,
  type AgreementId,
  type CurrencyCode,
  type PartyId,
  type PartyKindId,
} from '../../core/ids.js';
import { type Cash, minus, plus, valueAt, asPerPiece, heldAsMoney } from '../../core/measure.js';
import { callFor } from '../../registry/margin.js';
import { none, type Option, some } from '../../core/option.js';
import type { Agreement, AgreementTerms } from '../../register/agreements.js';
import type { Terms } from '../../register/instruments.js';
import { about, type MechanismContext, type ParticipantView } from '../../world/context.js';
import { wantedToDraw } from '../../registry/notices.js';

/** A1: a named bank and a named client, with a contract that can be ended. */
export const PRIME = agreementKindId('banks.prime');

export interface PrimeTerms extends AgreementTerms {
  readonly kind: typeof PRIME;
  /**
   * C1: WHAT THE BROKER REQUIRED of this portfolio when it last looked, and it is the ONE thing a
   * relationship has to carry — because it is the only number here that is nobody else's to know.
   * A requirement is a decision, taken out of a view its holder has and nothing else can see.
   *
   * WHAT IS FINANCED IS NOT ON IT (Law 19). That is a fact about the register — the rows this
   * broker holds — and a copy of it here would be a mirror that the two could disagree about, which
   * is exactly what Law 4 hunts. Every reader asks the book.
   */
  readonly required: Cash;
}

/** Law 15: a structural narrowing, the idiom every agreement kind in this world uses. */
export const isPrime = (t: AgreementTerms): t is PrimeTerms => 'required' in t;

/** A1: the relationship this client is under, or none because nobody has taken it on. */
export function primeOf(ctx: MechanismContext, client: PartyId): Option<Agreement> {
  for (const a of ctx.agreements.ofKind(PRIME)) {
    if (a.state === 'performing' && a.debtor === client) return some(a);
  }
  return none<Agreement>();
}

/**
 * A3, C1: WHAT THE BROKER CAN SEE, which is the whole of why it can lend at all — *"the broker holds
 * the client's assets and knows the whole position; that knowledge is what lets it lend"*.
 *
 * It is the securities, at the marks everything else in this world is valued at (XI-6), in the
 * broker's own money. MONEY IS NOT IN IT, and that is not a detail: a broker that counted the cash
 * it had just lent as part of the book it was lending against would lend against its own loan, and
 * the arithmetic would not converge — which is a missing mechanism wearing a bound's clothes
 * (Law 6). The cash a client holds is its EQUITY, and it appears on that side below.
 */
export interface Position {
  readonly instrument: Parameters<MechanismContext['valuation']['worthOf']>[1];
  readonly worth: Cash;
}

function positionsOf(ctx: MechanismContext, client: PartyId, broker: PartyId): readonly Position[] {
  const out: Position[] = [];
  for (const h of ctx.register.holdingsOf(client)) {
    if (!ctx.instruments.has(h.instrument)) continue;
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live) continue;
    // Money D2: a balance is not a position. It is what the client has to meet a call WITH.
    if (ctx.registry.instrumentKind(i.kind).pricing === 'money') continue;
    const worth = ctx.valuation.worthOf(client, h.instrument, ctx.period);
    if (!worth.some || worth.value.value.pieces <= 0) continue;
    out.push({
      instrument: h.instrument,
      // Currency C4.a: on the BROKER's book, because the line and the loan are in its money.
      worth: ctx.valuation.inOwnMoney(broker, worth.value.value, ctx.period),
    });
  }
  return out;
}

/**
 * C1, C1.b, C4.a: THE REQUIREMENT, and it is the broker's own view of the risk.
 *
 * Against each line, what it requires is HOW WIDE ITS OWN RECENT SURPRISES ABOUT THAT LINE HAVE BEEN
 * (§46 B3) — a read of its own history of being wrong, never a rate anybody stated. A broker that
 * has watched a line move violently wants more against it; one that has watched it sit still wants
 * less; and two brokers want different amounts because they have seen different things (§46 A3).
 *
 * C1.b, OFFSETTING POSITIONS: what the requirement is measured on is the client's NET position in
 * each line, which is what a register holding IS — a long and a short of the same thing is not two
 * exposures, and nothing here adds them up as though it were. Across DIFFERENT lines nothing offsets
 * here, and that is deliberate rather than missing: an offset between two lines is a claim that they
 * move together, which is a correlation, and a correlation nobody measured is a number invented to
 * make a requirement smaller.
 *
 * A LINE IT HAS NO VIEW OF IS NOT FINANCED AT ALL. The broker requires the whole of it — which is a
 * refusal and not a bound: a lender that cannot say how wrong it has been about a thing has no basis
 * for lending against it, and `Missing` is missing (App A).
 */
export function requirementOn(broker: ParticipantView, positions: readonly Position[]): Cash {
  // Currency B1: the broker's book is reported in its home money, and every position here is (positionsOf).
  const home = broker.registry.currencyOf(broker.self.region);
  const terms = positions.map((p) => {
    const view = broker.outlook(about({ on: 'price', instrument: p.instrument }));
    if (!view.some) return p.worth;
    // 0h.2: and a line it has a view of but has never been SCORED on is the same refusal — it has
    // no width to lend against, and an untested outlook read as a width of zero would have financed
    // the whole position for nothing (§46 B3, App A).
    const wide = view.value.confidence;
    if (!wide.some) return p.worth;
    const held = broker.quantity(p.instrument);
    if (held <= 0) return p.worth;
    // §46 B3: the width of its own surprises about this line, per piece, over what the client holds.
    const wideValue = valueAt(
      asPerPiece(wide.value, 'how wide its own surprises about this line have been'),
      held,
      p.worth.ccy,
      'what a move the size of its own surprises would cost',
    );
    return atMostCash(wideValue, p.worth, 'a line cannot fall by more than the whole of it');
  });
  return sumCash(home, terms, 'what it requires against the book').value;
}

/** What the broker's line arithmetic came to, in the order §15 C states it. */
export interface Line {
  /** A3, C1: the securities it holds for the client, at the marks. Money is not in it. */
  readonly portfolio: Cash;
  /** C1: what it requires against them, from its own view of the risk. */
  readonly required: Cash;
  /** B1, E1: what it has already financed — its exposure to this client. */
  readonly financed: Cash;
  /** C2: what the client has of its own in the account: the book, less what it owes on it. */
  readonly equity: Cash;
  /**
   * C3, C3.b: what is left to draw — AND IT IS ALLOWED TO BE NEGATIVE. A client drawn past its line
   * is over the line, and that number is the shortfall that forces the sale.
   */
  readonly available: Cash;
}

/**
 * C1–C3: the whole of it, as three reads and two subtractions. Nothing here is stored: the portfolio
 * is the register at the marks, the requirement is the broker's own view today, and what is financed
 * is the loan the register says the broker holds (Law 19).
 */
export function lineFor(
  ctx: MechanismContext,
  broker: PartyId,
  client: PartyId,
  financed: Cash,
): Line {
  const positions = positionsOf(ctx, client, broker);
  const portfolio = sumCash(
    financed.ccy,
    positions.map((p) => p.worth),
    'what the book is worth',
  ).value;
  const required = requirementOn(ctx.participant(broker), positions);
  return {
    portfolio,
    required,
    financed,
    equity: minus(portfolio, financed, 'what the client has of its own in the account'),
    // C3.b: a subtraction, and it is allowed to come out negative. Flooring it here is the one
    // change that would make the whole forced-sale path unreachable.
    available: minus(
      minus(portfolio, required, 'what it will finance of this book'),
      financed,
      'what is left to draw on it',
    ),
  };
}

/** B1, E1: what this broker has actually lent this client — the rows it holds, read off the book. */
export function financedFor(
  ctx: MechanismContext,
  broker: PartyId,
  client: PartyId,
  /** Law 15: the module that declared the kind narrows a row back to it; this only asks. */
  isLoanTerms: (t: Terms) => boolean,
  ccy: CurrencyCode,
): Cash {
  const terms: Cash[] = [];
  for (const i of ctx.instruments.issuedBy(client)) {
    if (!i.status.live || !isLoanTerms(i.terms) || i.ccy !== ccy) continue;
    const held = ctx.register.quantity(broker, i.id);
    if (held > 0) terms.push(heldAsMoney(held, ccy, 'what this broker has lent it'));
  }
  return sumCash(ccy, terms, 'what this broker has lent it').value;
}

/**
 * C3: what a call is — the part of the shortfall that is real money, now. Nothing, when there is none.
 *
 * Item 13.8: it asks `registry/margin.ts`, which is the ONE definition of what a margin call is in
 * this world (Law 4). A broker marking a portfolio against what it requires and a stock lender
 * marking lent paper against its collateral are the same sentence about two contracts, and written
 * twice they would be two definitions — the day one of them gained a floor the other would not.
 *
 * What is required of this client is its requirement PLUS what the broker has already financed:
 * both have to be covered by the book, which is what `available` subtracts in the other order.
 */
export const callOf = (line: Line): Cash => {
  const short = callFor({
    required: plus(line.required, line.financed, 'what this book has to cover'),
    covering: line.portfolio,
  });
  return short.pieces > 0 ? short : noCash(line.portfolio.ccy);
};

/** The terms a relationship carries after this period's look at it. */
export const termsOf = (line: Line): PrimeTerms => ({ kind: PRIME, required: line.required });

/** B1: the two sides of the account, for the record that publishes it (C2, E1). */
export const asData = (line: Line): Record<string, number> => ({
  portfolio: line.portfolio.pieces,
  required: line.required.pieces,
  financed: line.financed.pieces,
  equity: line.equity.pieces,
  available: line.available.pieces,
});

/* --------------------------------------------------------------------------------------------
 * THE BROKER'S PERIOD
 * ------------------------------------------------------------------------------------------ */

/** What `runPrime` is allowed to do with the bank's own lending machinery, and nothing more. */
export interface PrimeDeps {
  /** C9: the one row this broker has with this client, and what it is owed on it. */
  financed(ctx: MechanismContext, broker: PartyId, client: PartyId, ccy: CurrencyCode): Cash;
  /**
   * B1, B3: lend it, at the rate THIS broker charges THIS client — never the keenest quote in the
   * world. A prime loan is secured on a book only this broker holds (A1, A3), so the client cannot
   * take it elsewhere, and pricing it off somebody else's offer would be a rate nobody quoted.
   */
  lend(
    ctx: MechanismContext,
    broker: PartyId,
    client: PartyId,
    amount: Cash,
    ccy: CurrencyCode,
  ): boolean;
  /** C3: take the money back. A call met is a real payment that reduces what is owed. */
  repay(
    ctx: MechanismContext,
    broker: PartyId,
    client: PartyId,
    amount: Cash,
    ccy: CurrencyCode,
  ): Cash;
}

/**
 * A1, B1, C1-C3, E1: the broker's period, and it is the same four questions every period.
 *
 * WHO IS A CLIENT. A pool whose own module says it may borrow (`mayBorrow`, item 13.3) and whose
 * account is here — because A3's knowledge is what lets a broker lend, and in this world the party
 * that holds your assets is the bank you bank at. A pool that may not borrow is not offered a
 * relationship, which is the difference between a permission and a loan (B1).
 *
 * A3's MULTI-BROKER BLIND SPOT is therefore not reachable yet and is not pretended to be: *"the
 * client can have more than one broker, and then no broker sees the whole position"*. One account,
 * one broker. When a pool can bank in two places the blind spot arrives by itself, and E3's
 * "each broker underestimates" arrives with it.
 */
export function runPrime(ctx: MechanismContext, deps: PrimeDeps, clients: PartyKindId): void {
  for (const client of ctx.parties.ofKind(clients)) {
    if (!client.status.alive) continue;
    const broker = client.bank;
    if (!ctx.parties.has(broker) || !ctx.parties.get(broker).status.alive) continue;
    const ccy = ctx.registry.currencyOf(client.region);
    const held = primeOf(ctx, client.id);
    // B1, item 13.3: the PERMISSION is the client's own module's answer, never this one's. A broker
    // that decided for itself which pools may be levered would be deciding a term of somebody
    // else's mandate (Law 4).
    if (!ctx.participant(client.id).mayBorrow()) {
      // A1: a contract that can be ended, and this is one of the two ways it ends — the client
      // stopped being one. (The other is the broker deciding it will not carry it, which is D-side.)
      if (held.some) ctx.endAgreement(held.value.id, `${client.id} may no longer be levered`);
      continue;
    }
    const financed = deps.financed(ctx, broker, client.id, ccy);
    const line = lineFor(ctx, broker, client.id, financed);
    // A1: appointed on the first look. The terms are restated at the END of the period's work, so
    // what a relationship says it has financed is what the register says it has financed — a claim
    // that disagreed with the book would be a second answer to one question, and the `names` family
    // below is what would have to catch it (Law 4, Law 19).
    const row = held.some
      ? held.value.id
      : ctx.owes({
          debtor: client.id,
          creditor: broker,
          ccy,
          owed: 0,
          terms: termsOf(line),
          why: `${broker} is prime broker to ${client.id}`,
        }).id;
    /**
     * C1, C2, E1: WHAT THE BROKER DECIDED, said out loud under both names. It is private between
     * them — a client's line is nobody else's business (Observer A4) — and the CLIENT reads its own,
     * which is how it knows what it may draw. Everything in it is a read taken today.
     */
    ctx.record(
      'prime.line',
      [client.id, broker],
      { client: client.id, broker, ...asData(line) },
      false,
    );
    const call = callOf(line);
    if (call.pieces > 0) {
      /**
       * C3, C5: A SHORTFALL IS A CALL — REAL MONEY, NOW, and this is what stops margin being *"only
       * a number"*. The broker takes what the client can pay out of its account; what it cannot pay
       * is recorded and stays owed, which is the state D1 starts from.
       *
       * Nothing is lent back to cover it (C3.b), nothing is written off, and the requirement is not
       * relaxed because the client cannot meet it — all three would make the forced sale unreachable.
       */
      const paid = deps.repay(ctx, broker, client.id, call, ccy);
      ctx.record(
        'prime.call',
        [client.id, broker],
        {
          client: client.id,
          broker,
          called: call.pieces,
          paid: paid.pieces,
          unmet: minus(call, paid, 'what is still owed').pieces,
          ccy: call.ccy,
        },
        false,
      );
      settle(ctx, row, line);
      continue;
    }
    // B1, B3: and what it asked for, up to what the line allows. The ASK is the client's (its own
    // target, published under its own name); the ALLOWANCE is the broker's, and it changes every
    // period as the book moves (B3: *"the amount available is the lender's decision"*).
    const wants = wantedBy(ctx, client.id);
    if (!wants.some || wants.value.pieces <= 0 || line.available.pieces <= 0) continue;
    const draw = ctx.registry.payable(
      atMostCash(
        wants.value,
        line.available,
        'it lends what the line leaves, never what was asked for',
      ),
    );
    if (draw > 0) deps.lend(ctx, broker, client.id, heldAsMoney(draw, ccy, 'what it draws'), ccy);
    settle(ctx, row, line);
  }
}

/** C1: the requirement this period's look arrived at, which is the one thing the row carries. */
function settle(ctx: MechanismContext, row: AgreementId, line: Line): void {
  ctx.restate(row, termsOf(line));
}

/**
 * B3, Law 19: WHAT THE CLIENT ASKED FOR, read off what the client published under its own name.
 *
 * The broker does not decide how levered its client wants to be — that is the client's own target,
 * a term of the mandate its investors agreed to — and it does not read the client's mandate either
 * (it may not). It reads a request, which is what a request is for, and it answers with a number of
 * its own. A client that has asked for nothing is not lent anything.
 */
function wantedBy(ctx: MechanismContext, client: PartyId): Option<Cash> {
  return wantedToDraw(ctx.journal, String(client));
}
