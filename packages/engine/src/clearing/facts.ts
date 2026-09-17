/**
 * What a market says when a session ends (0i).
 *
 * @spec Clearing C4.b Clearing E1 Clearing F1.a Clearing F2 Law 4 Law 8 Appendix A
 *
 * ONE PRINT, ONE SHAPE. It was written at four sites in four shapes — a cleared session, a trip's
 * settlement, a session that printed nothing, and a stale carry-forward — and its only reader asks
 * `data['stale'] === true`, which was `undefined` for three of the four and worked by not matching.
 * A session that printed nothing now says so in the same field every other session fills.
 */
import { fact } from '../registry/facts.js';

export const PRINT = fact('print', 'what a market session came to, and whether it printed at all', {
  printed: { is: 'flag', what: 'whether real supply met real demand here this period' },
  price: { is: 'price', what: 'what it printed, per unit', orNone: true },
  volume: { is: 'count', what: 'the units the session matched' },
  settledVolume: { is: 'count', what: 'the units whose instructions actually settled' },
  failedTrades: { is: 'count', what: 'the matches that did not settle' },
  /**
   * Clearing C3: WHICH SIDE WAS CUT PRO RATA — `buy`, `sell` or `none`, which is what the solver
   * answers. The cleared session wrote that side and the other three wrote `false`: a boolean where
   * the one reader that mattered would have found a string, and nothing in the world could say so.
   */
  rationed: { is: 'text', what: 'which side posted more at the clearing price and was cut' },
  /**
   * Clearing E4: A STALE PRINT IS A MARK AND SAYS HOW OLD IT IS. `carriedFrom` is the period the
   * price it carries was really struck in, so nothing has to infer the age of a number.
   */
  stale: { is: 'flag', what: 'whether the price standing is carried from an earlier session' },
  carriedFrom: { is: 'period', what: 'the period the carried price was struck in', orNone: true },
  reason: { is: 'text', what: 'why it did not clear, where it did not', orNone: true },
  /** Freight, 16.5: the legs of a trip settle beside the book's own trades and print with them. */
  transact: { is: 'flag', what: 'whether this print is a trip settling in the book' },
});
