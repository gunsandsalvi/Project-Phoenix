/**
 * Which firms have shares, and what the seed states about them (Law 15: all DATA in a registry).
 *
 * @spec Equity A6 Equity D2.c Seed A3 Seed B1 Seed B1.a Seed B4 Seed C4 Seed E1 Seed E2 Law 2 Law 15 Law 19
 *
 * A LISTED firm is one whose shares have a market. Not every firm has one — a firm nobody has ever
 * bought a share of is a real state, and it is the state most of the firms in this world are in —
 * so being listed is data about a firm, not a property every firm has. The rest arrive with the
 * market that lists them (worklist 13g).
 *
 * WHICH ONES, AND HOW MANY OF THEM, ARE DRAWN (Seed B1.a). It used to be a table of three, chosen
 * by hand as "the largest in each line", which is a claim about a world of twelve firms and says
 * nothing at all about one of three thousand. What is stated instead is the RULE a listing follows:
 * a firm is listed if it is large enough that somebody would buy a share of it, and how large that
 * is, is one number.
 */
import { instrumentId, marketId, paramId, type InstrumentId, type MarketId, type ParamId } from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import { betweenWhole, type Spread } from '../../rng/spread.js';

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
  /**
   * Dealer Desks A1, A3: which of this world's banks open holding its float, by name. A share line
   * has a few makers and not all of them — a bank that has never taken a view of a firm does not
   * quote it — and which few is drawn, so no name is written down beside another.
   */
  readonly makers: readonly string[];
  readonly why: string;
}

/** Law 9: a share is named by its issuer, and its line and market are named after the firm. */
export const equityLineOf = (firm: string): InstrumentId => instrumentId(`equity.${firm}`);
export const equityMarketOf = (firm: string): MarketId => marketId(`mkt.equity.${firm}`);
export const equityParam = (firm: string, what: string): ParamId => paramId(`equity.${what}.${firm}`);

/**
 * Equity D2.c, Firm E5: how fast a management takes its money out. Two managements sitting on the
 * same cash do not distribute it over the same number of periods, and which of them is holding on
 * to it is what a shareholder is deciding about.
 */
export const PAYOUT_PATIENCE: Spread = {
  low: 4,
  high: 16,
  why: 'Equity D2.c, Firm E5: over how many of its own periods a management distributes what it has spare. A quarter to a third of a year: at one end a management that pays out nearly everything it makes as it makes it, at the other one that keeps three quarters of it in the firm. It is the whole of how fast money leaves a firm for its owners, and two managements that are not equally patient distribute differently out of the same cash.',
};

/**
 * Seed B1.a: HOW BIG A FIRM HAS TO BE BEFORE ANYBODY BUYS A SHARE OF IT, as a multiple of the
 * smallest firm in its line. A share line exists because somebody wanted a claim on the business
 * and somebody else was willing to sell one, and neither happens for a firm that is one person and
 * a van: a listing is what a firm large enough to outlive its owner gets.
 *
 * It is a SHAPE and it dies with worklist 13g: a firm listing is a DECISION — a firm that needs
 * money it cannot borrow sells part of itself, and somebody buys — and once entry and listing are
 * mechanisms, which firms have a share line is an outcome of them and this number goes.
 */
export const LISTING_SIZE = 8;

/**
 * Dealer Desks A1, A3, E3: how many of this world's banks open holding a given line's float. A
 * market in one name has a few makers, not all of them and not one: with one, every session is that
 * bank facing the world and there is no interdealer market in the name at all (E3); with all of
 * them, every bank in the world has a view of every firm in it, which is not what a dealer is.
 */
export const MAKERS_PER_LINE = 3;

/** Dealer Desks C5: what a share line IS, as a bank's own row names the kinds it makes. */
const SHARE_KIND = 'equity.share';

/**
 * Seed B1.a, B4: WHICH FIRMS THIS WORLD LISTED, drawn from the firms it has.
 *
 * Law 19: it READS the firm rows rather than restating anything about them — a firm's size is the
 * firms module's number, and this decides only which of them cross the line and who makes each
 * market. Deterministic in the world's own seed value (Seed A5, Audit D3).
 */
export function drawListed(
  firms: readonly { readonly firm: string; readonly size: number }[],
  /**
   * Dealer Desks A1, A3: the banks of this world AND WHAT EACH OF THEM MAKES A MARKET IN. A line's
   * makers are drawn from the banks that deal shares and never from all of them: a bank that opens
   * holding a line it does not quote is holding inventory for a book it does not run, which is not
   * a dealer with a position — it is a position with nobody behind it (Law 4).
   *
   * A world where no bank deals shares lists its firms and none of them has a float. That is a real
   * state and not a gap: the line exists, the market exists, and nothing is outstanding until
   * somebody issues into it (Equity D1).
   */
  banks: readonly { readonly bank: string; readonly makes: readonly string[] }[],
  seed: string,
): readonly ListedDecl[] {
  const rng = prng(seed, 'equity');
  const out: ListedDecl[] = [];
  const dealers = banks.filter((b) => b.makes.includes(SHARE_KIND)).map((b) => b.bank);
  for (const f of firms) {
    if (f.size < LISTING_SIZE) continue;
    // A3: the makers, drawn without repeating a name — a bank cannot be two of a line's makers.
    const pool = [...dealers];
    const makers: string[] = [];
    while (makers.length < MAKERS_PER_LINE && pool.length > 0) {
      const at = rng.int(pool.length);
      const taken = pool[at];
      pool.splice(at, 1);
      if (taken !== undefined) makers.push(taken);
    }
    out.push({
      firm: f.firm,
      payoutPatience: betweenWhole(rng, PAYOUT_PATIENCE),
      makers: makers.sort(),
      why: `Equity A6, Seed B1.a: listed because it is ${f.size.toFixed(1)} times the smallest firm in its line, which is over the size at which somebody buys a share of a business. Nothing about it is stated one firm at a time.`,
    });
  }
  return out;
}

/** The listed row a firm is in, when this world listed it (Law 15: the data says). */
export function listedOf(rows: readonly ListedDecl[], firm: string): ListedDecl | undefined {
  return rows.find((r) => r.firm === firm);
}
