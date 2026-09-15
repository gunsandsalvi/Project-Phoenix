/**
 * Small-business pools: the tier below the named firms, and it is firms with a weight.
 *
 * @spec Small-Business Pools A1 Small-Business Pools A2 Small-Business Pools A2.a Small-Business Pools A3 Small-Business Pools A5 Small-Business Pools A6 Small-Business Pools A6.a Small-Business Pools A6.b Small-Business Pools E5 Seed A3 Seed B1.a XI-15 Law 2 Law 15
 *
 * WHY IT IS HERE. §42 was 2 of 28 MET and both were the generic cell kernel — nothing in this
 * engine was a small firm, and the one mention of the sector in sixty thousand lines was a comment.
 * What that cost is not its own 28 clauses: **A5.a says small firms are the sector where a credit
 * tightening bites first and hardest**, and without them a tightening has nowhere to bite, because
 * every borrower in this world is large enough to reach a bond market. Trade Credit §36 A4 says this
 * is *"the tier that lives on"* trade credit, so eleven of its clauses are missing partly because
 * the tier below them does not exist.
 *
 * A SMALL FIRM IS A FIRM (A1): it sells, employs, borrows and can fail. What makes it a different
 * KIND is not what it does — it is that it is represented as a cell, and a kind states how it is
 * represented (`PartyKindProfile.representation`). A6.b is the point: *"a weight of one is a named
 * firm, so the boundary between this sector and Corporate Credit's is not a modelling line but a
 * SIZE"*, and A6.c promotes a cell across it. Two kinds that can name each other is what a promotion
 * needs; one kind with two representations is not a thing a registry can say.
 *
 * SO `registry/profiles.ts` CARRIES THE NAME AND THE LIST. `PRODUCING_KINDS` is what a module asks
 * when it wants this world's businesses, and every mechanism that reaches the sector reaches it by
 * reading that row rather than by branching on a kind id (Law 15).
 *
 * WHAT THIS STEP BUILDS AND WHAT IT DOES NOT. The sector EXISTS: cells with weights, drawn sizes,
 * a bank, a region and a line of business, and a parameter register that prints what each of them is
 * like. **It does nothing yet**, and that is the item's own order: it sells and buys on trade credit
 * (11.5), it employs (11.6), it borrows from a named lender on its own row (11.7), it defaults from
 * its own cash flow (11.8), and it is promoted when it outgrows A5 (11.10). Each of those is a step
 * and each names what it turns on.
 */
import type { RegionId } from '../../core/ids.js';
import { partyId } from '../../core/ids.js';
import type { ParamDecl } from '../../registry/params.js';
import { SMALL_FIRM } from '../../registry/profiles.js';
import type { SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { smallParam, type SmallFirmDecl } from './data.js';

export * from './data.js';

/**
 * A3, XI-14: what each cell is like, declared under its own name so the register prints the
 * distribution rather than hiding it inside a draw.
 *
 * It is a PREFERENCE in the register's sense — a fact about this firm that nothing else decides —
 * and it is drawn per cell rather than stated per cell, which is what A2.a is about.
 */
function paramsOf(rows: readonly SmallFirmDecl[]): ParamDecl[] {
  return rows.map((r) => ({
    id: smallParam(r.cell, 'size'),
    value: r.size,
    unit: 'multiple of the smallest firm in its line',
    dimension: 'ratio',
    kind: 'preference' as const,
    owner: 'model' as const,
    why: r.why,
  }));
}

/**
 * A6.a: the key IS what the cell is — where it is, who it banks with, what it does.
 *
 * The region comes from the BANK and never from the row: a cell lives where its bank books, which
 * is the same sentence the households seed makes about where this world's people are, and a second
 * statement of it beside the draw is how the two come to disagree (Law 4, Law 19).
 */
export const cellKeyOf = (r: SmallFirmDecl, region: RegionId): Readonly<Record<string, string>> => ({
  region: String(region),
  bank: r.bank,
  line: r.line,
});

export function smallBusiness(rows: readonly SmallFirmDecl[]): SystemModule {
  return {
    id: 'small-business',
    spec: 'Small-Business Pools',
    /**
     * A1, A4, A5: it sells to and buys from named firms, it banks and it borrows. The seed makes
     * the parties, so it runs after the banks and the firms this sector is connected to exist.
     */
    requires: ['firms', 'banks'],
    instrumentKinds: [],
    partyKinds: [
      {
        /**
         * Item 15, A1: what a small firm is FOR, and it is the same thing a named firm is for —
         * what is left after everybody else has been paid. That is A1 stated as an objective: these
         * are firms, and the only thing different about them is how many of them one party is.
         */
        objective: 'theResidual',
        id: SMALL_FIRM,
        representation: 'cell',
        moneyIssuer: null,
        /**
         * A1, XI-3: *"they can fail"*, and on both — it runs out of money (B3's threshold is its
         * own cash flow) or it owes more than it holds. Nothing in this world is immortal, and a
         * sector that could not fail is the credit content of §42 removed.
         */
        fails: ['cash', 'solvency'],
        /** A5: bank-dependent. It borrows, and it is too small for the bond market (B1, 11.7). */
        borrows: true,
        /**
         * Banks Funding A1: many, small and operationally sticky — a small firm's account is where
         * its takings land and where its wages go out, which is a different deposit from a saver's.
         */
        depositClass: 'operational',
      },
    ],
    curveFamilies: [],
    units: [],
    params: paramsOf(rows),
    /**
     * Item 11.5–11.10: it has none YET, and the module says so rather than declaring an empty one
     * that reads as done. What it sells, what it employs, what it borrows and what it defaults on
     * are four steps, and each arrives with the phase that runs it.
     */
    phases: [],
    participants: [],
    families: [],
    seed(ctx: SeedContext): void {
      for (const r of rows) {
        const bank = ctx.parties.get(partyId(r.bank));
        ctx.parties.add({
          id: partyId(r.cell),
          kind: SMALL_FIRM,
          representation: 'cell',
          region: bank.region,
          // Law 9: a market names a small firm by what it does and where, because that is all
          // anybody outside it knows about one — and a cell of them is that, with a count.
          name: `${String(r.weight)} ${r.line} firms at ${r.bank}`,
          bank: bank.id,
          // XI-15: how many real firms this party IS. A count, and the five events are the only
          // things that may move it (E5).
          weight: r.weight,
          key: cellKeyOf(r, bank.region),
          status: { alive: true, standing: 'good' },
        });
      }
    },
  };
}
