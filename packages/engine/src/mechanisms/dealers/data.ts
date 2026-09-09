/**
 * The desks in this world: whose they are, what they make a market in, and what each of them is
 * like (Law 15: all DATA in a registry).
 *
 * @spec Dealer Desks A1 Dealer Desks A3 Dealer Desks D1 Dealer Desks D2 Dealer Desks E3 Seed A3 Law 2 Law 15
 *
 * TWO of them, one inside each bank, because E3 asks for an interdealer market and one desk cannot
 * face itself. They are not alike: a desk that will carry twice as much as another quotes tighter
 * out of the same view, and a desk that wants more on its capital quotes wider — which is why the
 * spread is an output and not a number anybody states (C5).
 */
import { paramId, type ParamId } from '../../core/ids.js';

export interface DeskDecl {
  readonly desk: string;
  readonly name: string;
  /** A1: the bank whose trading arm it is. Its account is there and it pays that bank its rent. */
  readonly bank: string;
  /** C5: the instrument kinds it quotes. One posted quote belongs in every book it makes. */
  readonly makes: readonly string[];
  /** Seed A3: the money its bank put into it when it opened. */
  readonly cash: number;
  /** Seed A3: units of each line it opens holding, by the kind of the line (A3: inventory). */
  readonly opens: Readonly<Record<string, number>>;
  /** D1: the most it will be long of one line, in units. Its own risk function's number. */
  readonly limitPerInstrument: number;
  /** D1: the most its whole book may be worth, in its own money. */
  readonly limitAggregate: number;
  /** D2: what it needs to earn on the capital its inventory consumes, per annum. Its own. */
  readonly requiredReturnOnCapital: number;
  readonly why: string;
}

export const deskParam = (desk: string, what: string): ParamId => paramId(`desk.${what}.${desk}`);

/** D2: the capital a trading position consumes, as a rule somebody wrote rather than a fact. */
export const TRADING_BOOK_RISK_WEIGHT: ParamId = paramId('regulation.riskWeight.tradingBook');
export const TRADING_BOOK_CAPITAL_RATIO: ParamId = paramId('regulation.capitalRatio');

export const DESKS: readonly DeskDecl[] = [
  {
    desk: 'desk.a',
    name: 'Bank A trading',
    bank: 'bank.a',
    makes: ['equity.share', 'sovereign.bond', 'sovereign.bill', 'fund.share'],
    cash: 400,
    // Seed A3: it opens holding shares and no paper. A share line has no other holder at period
    // zero — nobody has founded anything and nobody has bought anything (Seed E1) — so what a desk
    // opens holding IS the line, and it is the float the rest of the world buys from. Sovereign
    // paper already has holders the seed states, and a desk that opened holding more of it would
    // be a seed deciding how much the sovereign owes: it builds that book by trading, from nothing.
    opens: { 'equity.share': 120 },
    limitPerInstrument: 400,
    limitAggregate: 1200,
    requiredReturnOnCapital: 0.1,
    why: 'The larger book: it will carry more of a line and wants a tenth on the capital that ties up. It quotes the tighter of the two out of the same view, and it is the one that is still there when the other has filled up.',
  },
  {
    desk: 'desk.b',
    name: 'Bank B trading',
    bank: 'bank.b',
    makes: ['equity.share', 'sovereign.bond', 'sovereign.bill', 'fund.share'],
    cash: 250,
    opens: { 'equity.share': 80 },
    limitPerInstrument: 220,
    limitAggregate: 700,
    requiredReturnOnCapital: 0.15,
    why: 'The smaller and dearer book: half the room and half again the required return, so it skews harder as it fills and steps back sooner. It is the desk whose stopping is what a thin market looks like.',
  },
];

/** The desk a named party is, if this world has one by that name (Law 15: the data says). */
export function deskOf(rows: readonly DeskDecl[], party: string): DeskDecl | undefined {
  return rows.find((r) => r.desk === party);
}
