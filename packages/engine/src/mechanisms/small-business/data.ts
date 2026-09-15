/**
 * The small-business tier: how many of them there are, and what they differ in (Law 15: all DATA).
 *
 * @spec Small-Business Pools A2 Small-Business Pools A2.a Small-Business Pools A3 Small-Business Pools A5 Small-Business Pools A6 Small-Business Pools A6.a Seed B1 Seed B1.a Seed B4 Law 2 Law 15 Law 19
 *
 * A2.a IS THE WHOLE REASON THIS FILE IS A DISTRIBUTION AND NOT A ROW. *"No representative small
 * firm. Default is a threshold event; with one average firm a mean-preserving spread causes no
 * defaults, and the entire credit content of the sector is gone."* So what is stated here is the
 * WIDTH — the shape a small firm's size is drawn from — and the cells are what that width comes out
 * as. Nothing about any one of them is written down.
 *
 * A6.a DECIDES THE DATA MODEL: *"every relationship that must be named is either a dimension of the
 * cell's key or a register row, never an attribute averaged inside it."* So the key carries the
 * three things a cell's members must all share for the cell to be honest — where it is, who it
 * banks with, and what line of business it is in — and its LENDER is not in it, because a lender is
 * a loan row per (lender, cell) and lifting it into the key would be averaging a relationship.
 *
 * WHY THE LINE IS IN THE KEY. A6.a names region and bank and says plainly that *"lifting a row into
 * the key is a data change"* — this is one, and it is forced: a cell is homogeneous, so a cell whose
 * members were in different lines would have a cost base, a customer and a labour venue that were
 * averages over lines, which is A2.a one level down.
 */
import { prng } from '../../rng/prng.js';
import { drawSize, type Tail } from '../../rng/spread.js';

/**
 * A5, Seed B1.a: HOW MANY SMALL FIRMS THERE ARE FOR EACH NAMED ONE.
 *
 * Stated as a multiple rather than as a count, because the count is a property of the WORLD and not
 * of this file: a world with three thousand named firms and one with nine thousand have different
 * numbers of corner shops, and a count written here would make the sector a fixed size the rest of
 * the seed grew away from (Seed B1.a).
 *
 * TWELVE. In a real economy the named tier — the firms big enough to have a name in a market and to
 * reach a bond market (A5) — is a small fraction of the businesses there are, and the rest is this.
 * It is a SHAPE and it dies with item 12: once firms are BORN into this sector (§42 A6) and
 * PROMOTED out of it (A6.c), how many small firms there are is the outcome of those two flows and
 * nothing reads this again.
 */
export const SMALL_PER_NAMED = 12;

/**
 * A2, A2.a, A3, Seed B4: HOW UNEQUAL THE SECTOR IS — the same long tail a named line has, because
 * it is the same fact about business: a few that are nearly large enough to leave (A6.c) and a very
 * long tail of one person and a van.
 *
 * It is what a default threshold bites against (B3), and B4's correlation is what makes the bite
 * arrive at several of them at once. A width here and a threshold there is the whole credit content
 * of the sector, which is what A2.a says is lost the moment this becomes a mean.
 */
export const SMALL_SIZE: Tail = {
  concentration: 1.1,
  why: 'Small-Business Pools A2, A2.a, A3: how big one small firm is beside the smallest in its line. A tail and never a width, and a FATTER tail than the named sector’s (1.2), because the small tier is where the distance between the biggest member and the median is greatest — the firm about to be promoted out of it (A6.c) and the one person with a van are both in here. Losses depend on the distribution of size, leverage and coverage and not on the mean (A3), so this is the thing a threshold event is measured against; flatten it and every cell defaults together or none does, which is the sector with its credit content removed.',
};

/**
 * A6, A6.a, 0f.9: ONE FIRM as drawn — who it banks with, what it does, and how big it is. It is
 * not a cell: the cells are what the LATTICE makes of these draws at the seed (`index.ts seed`),
 * one per occupied key, and a firm's size puts it in a band of that key. Nothing about any one of
 * them is stated; the width is (A2.a).
 */
export interface SmallFirmDecl {
  /**
   * A6.a: a dimension of the key. Who it banks with — and A5, because that is who lends to it.
   *
   * WHERE IT IS is the key's other dimension and it is NOT here: a cell lives where its bank books
   * (the same sentence the households seed makes), so the region is read off the bank when the
   * party is made rather than drawn beside it. Two sources for one fact is how they come to
   * disagree (Law 4, Law 19).
   */
  readonly bank: string;
  /** A6.a: a dimension of the key, and forced by homogeneity: what line of business it is in. */
  readonly line: string;
  /** A3: how big it is, beside the smallest in its line. Drawn (A2.a). */
  readonly size: number;
}

/**
 * Seed B1.a, B4: THE SECTOR, DRAWN — one population per (region, bank, line), cut into cells, each
 * with its own size.
 *
 * Law 19: it READS what this world already has — its regions, its banks and the lines a small firm
 * is in — rather than restating any of them. WHERE the small firms are follows from where the banks
 * are and how big they are (Seed B4: a bigger bank has more of them, which is what makes it bigger),
 * exactly as the population of households does, so no share of the sector is written down anywhere.
 */
export function drawSmallBusiness(
  /** The lines a small firm is in, decided by the seed, which is the only place that may know both. */
  lines: readonly string[],
  /** Seed B4: the banks, by name and size, because that is how the sector spreads over them. */
  banks: readonly { readonly bank: string; readonly size: number }[],
  /** How many small firms this world has, which is a count of its named ones times a multiple. */
  population: number,
  seed: string,
): readonly SmallFirmDecl[] {
  const rng = prng(seed, 'smallBusiness');
  const out: SmallFirmDecl[] = [];
  const total = banks.reduce((t, b) => t + b.size, 0);
  if (total <= 0 || lines.length === 0 || population <= 0) return out;
  for (const bank of banks) {
    // Seed B4: its share of the sector is its share of the banking system, and that is a read.
    const here = Math.round((population * bank.size) / total / lines.length);
    if (here <= 0) continue;
    for (const line of lines) {
      // 0f.9: EVERY FIRM DRAWS ITS OWN SIZE. The dispersion A2.a needs is within a key, and it is
      // the draw that gives it: the lattice's size bands then cut one population into the cells
      // that differ, and a band with nobody in it is a cell that does not exist.
      for (let n = 0; n < here; n += 1) {
        out.push({ bank: bank.bank, line, size: drawSize(rng, SMALL_SIZE) });
      }
    }
  }
  return out;
}
