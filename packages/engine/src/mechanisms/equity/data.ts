/**
 * Every firm's share line, and whether this world opened it to a market (Law 15: all DATA in a registry).
 *
 * @spec Equity A6 Equity C2.e Equity D1.b Equity D2.c Equity E3 Seed A3 Seed B1 Seed B1.a Seed B4 Seed C4 Seed E1 Seed E2 Private Equity A5 Law 2 Law 15 Law 19
 *
 * EVERY FIRM HAS A RESIDUAL AND SOMEBODY OWNS IT. What separates a public company from a private one
 * is not that the private one has no shares — it is that nobody may buy them in a market. So every
 * firm in this world has a share line, and `listed` says whether that line has a market.
 *
 * WHAT USED TO STAND HERE was a size threshold: a firm was listed if it was eight times the smallest
 * in its line, and a firm below that had no share line at all. Two things were wrong with it, and the
 * second is the worse. **Going public is a FUNDING CHOICE and not a size** — a firm sells part of
 * itself because it needs money it would rather not borrow (D1.b), and there are very large private
 * companies and very small public ones for exactly that reason. And a firm with no share line has a
 * residual **nobody holds**, which is a defect Appendix B names outright; at nine thousand firms the
 * threshold left more than eight thousand of them unowned.
 *
 * So what is drawn is an OPENING CONDITION (Seed B1.a) and not a rule about size: some of the firms
 * this world opens with had already gone public before it started, and which ones is a draw. From
 * period one it is a decision — an IPO and a take-private are both real events (E3, 10f.2) — and
 * nothing reads the draw again.
 */
import { instrumentId, marketId, paramId, type InstrumentId, type MarketId, type ParamId } from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import { betweenWhole, type Spread } from '../../rng/spread.js';

/**
 * Seed C4, Law 2, Equity D4: HOW FINELY A SHARE LINE IS CUT at the opening — what one share of one
 * is worth before any session has run.
 *
 * It is a RESOLUTION and not a shape, and D4 is the proof: a split multiplies the count and divides
 * the size of every unit, and it moves no value, no flow and no decision. Double this number and
 * halve every count below it and the world is the same world — which is exactly the invariance a
 * resolution is tested by, and it is a test in this item rather than a claim about it.
 *
 * IT IS THE FINEST THE GRID ALLOWS, and that is what a resolution should be. A share is indivisible
 * and a cell holds whole pieces per member (XI-15), so a line can only reach as many people as it
 * has pieces: at a dollar a share a firm's whole line came to fewer pieces than this world has
 * savers, and every firm below thirty million dollars opened with a line **nobody held at all**. A
 * penny a share cuts the same book into a hundred times as many pieces and reaches a hundred times
 * as many owners, and by D4 that is the only thing it changes.
 *
 * What the seed does state is who opens HOLDING how many, and that is endowment state like the
 * money a bank opens with (Seed A3), not a price and not a claim about what anything is worth.
 */
export const OPENING_SHARE = paramId('equity.openingShare');

/** A firm's share line, and what this world states about it (one row per firm, Law 15). */
export interface EquityDecl {
  /** The firm whose residual claim it is (Equity A1). */
  readonly firm: string;
  /**
   * Equity E3, Private Equity A5: whether its line has a MARKET. A public company's shares clear a
   * price every period; a private one's do not trade at all, so the holding is a mark and never a
   * price (§29 C5.a). It is the whole difference between the two, and it is one field because it is
   * one fact.
   */
  readonly listed: boolean;
  /** D2.c, Firm E5: over how many of its own periods this management distributes what it has spare. */
  readonly payoutPatience: number;
  /**
   * Dealer Desks A1, A3: which of this world's banks open holding its float, by name. A share line
   * has a few makers and not all of them — a bank that has never taken a view of a firm does not
   * quote it — and which few is drawn, so no name is written down beside another.
   *
   * A private line has NONE, and it is empty rather than absent: nobody makes a market in something
   * that has no market, and saying so is not the same as saying nothing is known.
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
 * Seed B1.a, Equity D1.b, E3: HOW MANY OF THE FIRMS THIS WORLD OPENS WITH HAD ALREADY GONE PUBLIC.
 *
 * One in twelve, and it is drawn INDEPENDENTLY OF SIZE. A firm is public because at some point it
 * wanted money it preferred not to borrow and sold part of itself to get it (D1.b), and that is a
 * choice a small firm can make and a very large one can decline — which is why the largest companies
 * in a real economy include private ones and the exchange is full of small ones. A rule that listed
 * the large and only the large would be stating the answer to the question 10f.2 exists to ask.
 *
 * It is a SHAPE and it is read exactly once, at the seed: from period one a firm decides for itself
 * whether to float (10f.2) and a buyer decides whether to take it private (E3), and what fraction of
 * this world is public is then the outcome of those two. Nothing reads this number again.
 */
export const PUBLIC_AT_THE_OPENING = 1 / 12;

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
 * Seed B1.a, B4: A SHARE LINE FOR EVERY FIRM THIS WORLD HAS, and which of them opened public.
 *
 * Law 19: it READS the firm rows rather than restating anything about them — a firm's size is the
 * firms module's number, and this decides only which of them this world had already floated and who
 * makes each market. Deterministic in the world's own seed value (Seed A5, Audit D3).
 */
export function drawEquity(
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
): readonly EquityDecl[] {
  const rng = prng(seed, 'equity');
  const out: EquityDecl[] = [];
  const dealers = banks.filter((b) => b.makes.includes(SHARE_KIND)).map((b) => b.bank);
  for (const f of firms) {
    const listed = rng.next() < PUBLIC_AT_THE_OPENING;
    const patience = betweenWhole(rng, PAYOUT_PATIENCE);
    // A3: the makers, drawn without repeating a name — a bank cannot be two of a line's makers.
    const pool = listed ? [...dealers] : [];
    const makers: string[] = [];
    while (makers.length < MAKERS_PER_LINE && pool.length > 0) {
      const at = rng.int(pool.length);
      const taken = pool[at];
      pool.splice(at, 1);
      if (taken !== undefined) makers.push(taken);
    }
    out.push({
      firm: f.firm,
      listed,
      payoutPatience: patience,
      makers: makers.sort(),
      why: listed
        ? 'Equity A6, D1.b, Seed B1.a: public at the opening because at some point before this world started it wanted money it preferred not to borrow and sold part of itself. Drawn, and not from its size: nothing about it is stated one firm at a time.'
        : 'Equity A6, Seed B1.a, §29 C5: private at the opening — it has a residual and named owners and no market, so what its holders carry it at is a mark and never a price. Drawn, and not from its size.',
    });
  }
  return out;
}
