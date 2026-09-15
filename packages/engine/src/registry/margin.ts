/**
 * BOTH SIDES MARKED, AND THE DIFFERENCE CALLED — one mechanism, and every party that has cover
 * against an exposure is a caller of it.
 *
 * @spec Prime Brokerage C1 Prime Brokerage C2 Prime Brokerage C3 Prime Brokerage C3.b Securities Lending C1 Securities Lending C2 Securities Lending C2.a Law 4 Law 6 Law 19
 *
 * §15 C2 and §14 C2 are the same sentence about two different contracts. A broker marks a client's
 * portfolio against what it requires of it; a stock lender marks the paper it lent against the
 * collateral it holds. **Both are: what this exposure requires, against what is actually there, at
 * today's marks — and the difference is real money between two named parties, now.** Written twice
 * they would be two definitions of what a margin call IS, and the day one of them gained a floor the
 * other would not (Law 4).
 *
 * IT IS NEVER FLOORED (§15 C3.b, and it is the clause that says so outright): *"the available line
 * is never floored at zero. A client drawn past its line is over the line, and the shortfall is what
 * forces the sale. Flooring it makes the whole path unreachable."* So this is a SUBTRACTION and the
 * sign is the answer: positive is a call to meet, negative is cover to give back, and the second is
 * not an optional courtesy — a mechanism that took margin and never returned it would be a one-sided
 * flow that nothing ever failed on (Law 5).
 *
 * NOTHING HERE DECIDES WHAT IS REQUIRED. That is the party at risk's own view of it — a broker's
 * portfolio margin out of its own outlook (§15 C1.b: *"a decision by the broker, not a formula the
 * client can rely on"*), a lender's haircut out of its own surprises — and this only puts two
 * numbers that party produced against each other.
 */
import { minus, type Cash } from '../core/measure.js';

/** What one exposure requires as cover, and what is actually covering it, both at today's marks. */
export interface Exposure {
  /** What the party at risk requires against this position, from its own view of it. */
  readonly required: Cash;
  /** What is there covering it now: the portfolio at the marks, the collateral at the marks. */
  readonly covering: Cash;
}

/**
 * §15 C2, C3.b, §14 C2: THE CALL — and it is a subtraction, so it comes out negative when there is
 * cover to give back and that is the answer rather than a case.
 *
 * A caller that wants only the shortfall reads the sign; a caller that moves money both ways moves
 * it both ways. Neither is allowed to floor it here, which is why there is nowhere here to do it.
 */
export const callFor = (e: Exposure): Cash =>
  minus(e.required, e.covering, 'what is short of what this exposure requires');
