/**
 * Which firms have shares, and what the seed states about them (Law 15: all DATA in a registry).
 *
 * @spec Equity A6 Equity D2.c Seed A3 Seed B1 Seed C4 Seed E1 Seed E2 Law 2 Law 15
 *
 * A LISTED firm is one whose shares have a market. Not every firm has one — a firm nobody has ever
 * bought a share of is a real state, and it is the state the other six in this world are in — so
 * being listed is data about a firm, not a property every firm has. The rest arrive with the market
 * that lists them (worklist 13g).
 */
import { instrumentId, marketId, paramId, type InstrumentId, type MarketId, type ParamId } from '../../core/ids.js';

/**
 * Seed C4, Law 2: what a share of a line is worth before any session has run.
 *
 * It is a RESOLUTION and not a shape, and D4 is the proof: a split multiplies the count and divides
 * the size of every unit, and it moves no value, no flow and no decision. Double this number and
 * halve every count below it and the world is the same world — which is exactly the invariance a
 * resolution is tested by, and it is a test in this item rather than a claim about it.
 *
 * What the seed does state is who opens HOLDING how many, and that is endowment state like the
 * money a bank opens with (Seed A3), not a price and not a claim about what anything is worth.
 */
export const OPENING_SHARE = paramId('equity.openingShare');

export interface ListedDecl {
  /** The firm whose residual claim it is (Equity A1). */
  readonly firm: string;
  /** D2.c, Firm E5: over how many of its own periods this management distributes what it has spare. */
  readonly payoutPatience: number;
  readonly why: string;
}

/** Law 9: a share is named by its issuer, and its line and market are named after the firm. */
export const equityLineOf = (firm: string): InstrumentId => instrumentId(`equity.${firm}`);
export const equityMarketOf = (firm: string): MarketId => marketId(`mkt.equity.${firm}`);
export const equityParam = (firm: string, what: string): ParamId => paramId(`equity.${what}.${firm}`);

/**
 * The three firms in this world that have a share line: the largest in each of its three lines.
 *
 * WHO HOLDS THEM AT THE SEED is not stated here, because a holding is endowment state and belongs
 * to whoever opens holding it (Seed A3) — here the banks whose dealing lines make each market,
 * exactly as the banks open holding the sovereign's paper. What is stated here is which firms have
 * a line at all and how fast each management distributes.
 *
 * WHAT IS NOT HERE YET is a founder. C2.e's insider — a block that is not for sale and that a
 * takeover has to obtain the vote of (A5.a) — is a party that comes into existence by FUNDING A
 * FIRM'S ENTRY, which is Firm Birth A and worklist 13g; inventing one at the seed would be stating
 * who owns this world before anybody bought anything (Seed E1, E2). Until then the free float is the
 * whole of what is issued, and it says so.
 */
/**
 * Seed A3, Dealer Desks A1, A3: WHO OPENS HOLDING THE FLOAT, and how much of every line they hold.
 *
 * A share line has no other holder at period zero — nobody has founded anything and nobody has
 * bought anything (Seed E1) — so what somebody opens holding IS the line: the float the rest of the
 * world buys from. It is held by the banks, because a bank's dealing line is what makes a market in
 * a share here (A1: a dealer's balance sheet is inside a bank's), and a bank that opens making a
 * market with nothing to sell can only ever bid.
 */
export const FLOAT: Readonly<Record<string, number>> = {
  'bank.a': 120_000,
  'bank.b': 80_000,
};

export const LISTED: readonly ListedDecl[] = [
  {
    firm: 'firm.4',
    payoutPatience: 10,
    why: 'The largest farm in the region, and the most patient management in it.',
  },
  {
    firm: 'firm.5',
    payoutPatience: 6,
    why: 'The big mill, whose management takes its money out faster than a farm does.',
  },
  {
    firm: 'firm.6',
    payoutPatience: 8,
    why: 'A plant bakery with the biggest week of bread in the region: a management between the two.',
  },
];

/** The line a firm's shares are, when this world listed it (Law 15: the data says). */
export function listedOf(rows: readonly ListedDecl[], firm: string): ListedDecl | undefined {
  return rows.find((r) => r.firm === firm);
}
