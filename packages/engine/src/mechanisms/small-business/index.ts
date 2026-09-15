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
import { paramId, partyId } from '../../core/ids.js';
import type { Option } from '../../core/option.js';
import { banksAwayFromTrouble } from '../../registry/switching.js';
import type { ParticipantView, SeedContext } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';
import type { ParamDecl } from '../../registry/params.js';
import { SMALL_FIRM } from '../../registry/profiles.js';
import type { SystemModule } from '../../world/module.js';
import { smallParam, type SmallFirmDecl } from './data.js';
import type { PartyId } from '../../core/ids.js';
import type { LatticeDecl, LatticeReads } from '../../registry/lattice.js';
import { none, some } from '../../core/option.js';

export * from './data.js';

/**
 * Banks Funding A1.b, A1.d, E1: WHAT IT COSTS ONE SMALL FIRM TO MOVE ITS ACCOUNT, once.
 *
 * A5 says this tier is bank-dependent, and A1.b says why that account is operational money: it is
 * where the takings land and the wages go out. So a small firm banks the way a named firm does —
 * it does not chase a board, and what moves it is the whole uninsured balance against the cost of
 * moving — and the decision is the one every depositor in this world takes
 * (`registry/switching.ts`). What is this sector's own is the COST, and it is the smallest of the
 * five: a firm with one account and a dozen counterparties has less to redirect than a named one.
 */
export const SMALL_FIRM_SWITCHING_COST = paramId('smallBusiness.switchingCost');

export const smallFirmChoosesBank = (view: ParticipantView): Option<BankChoice> =>
  banksAwayFromTrouble(view, SMALL_FIRM_SWITCHING_COST, 'moneyMarket.window');

/**
 * A3, XI-14: what each cell is like, declared under its own name so the register prints the
 * distribution rather than hiding it inside a draw.
 *
 * It is a PREFERENCE in the register's sense — a fact about this firm that nothing else decides —
 * and it is drawn per cell rather than stated per cell, which is what A2.a is about.
 */
function paramsOf(rows: readonly SmallFirmDecl[]): ParamDecl[] {
  return [
    {
      id: paramId('smallBusiness.lattice.size.1'),
      value: 5,
      unit: "members' cash, in money",
      dimension: 'amount',
      denominated: 'money',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the small-firm lattice on size. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('smallBusiness.lattice.size.2'),
      value: 50,
      unit: "members' cash, in money",
      dimension: 'amount',
      denominated: 'money',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the small-firm lattice on size. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('smallBusiness.lattice.leverage.1'),
      value: 0.5,
      unit: 'ratio of debt to what it holds',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the small-firm lattice on leverage. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('smallBusiness.lattice.leverage.2'),
      value: 1,
      unit: 'ratio of debt to what it holds',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the small-firm lattice on leverage. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: SMALL_FIRM_SWITCHING_COST,
      value: 60,
      denominated: 'money' as const,
      unit: 'of the money the account is in, per move',
      dimension: 'amount' as const,
      kind: 'preference' as const,
      owner: 'model' as const,
      why: 'Banks Funding A1.b, A1.d, E1: what it costs one small firm to move the account its takings land in. Smaller than a named firm\u2019s (250) because there is less to redirect and fewer counterparties to tell, and far smaller than a fund\u2019s, which moves more money in one go than this tier holds. It is weighed against the money it would lose if its bank failed \u2014 nothing insures a business balance (E4), so that is the whole of it.',
    },
    ...rows.map((r) => ({
      id: smallParam(r.cell, 'size'),
      value: r.size,
      unit: 'multiple of the smallest firm in its line',
      dimension: 'ratio' as const,
      kind: 'preference' as const,
      owner: 'model' as const,
      why: r.why,
    })),
  ];
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

/** 0f.3: the edges a small-firm lattice bands on — RESOLUTION, tested by invariance (0f.10). */
export const SMALL_FIRM_EDGES = {
  size: [paramId('smallBusiness.lattice.size.1'), paramId('smallBusiness.lattice.size.2')],
  leverage: [paramId('smallBusiness.lattice.leverage.1'), paramId('smallBusiness.lattice.leverage.2')],
} as const;

/**
 * A6.a, XI-15, 0f.3: THE LATTICE A POPULATION OF SMALL FIRMS LIVES ON. Where it is, where it
 * banks, what it does, and how old it is (Hopenhayn 1992: selection is by age); banded on its
 * size and its leverage (Stiglitz–Weiss 1981, Petersen–Rajan 1994: a lender rations by both).
 */
export const SMALL_FIRM_LATTICE: LatticeDecl = {
  kind: SMALL_FIRM,
  categorical: [
    { dim: 'region', movedBy: 'entry', why: 'a firm is somewhere, and its bank books there' },
    { dim: 'bank', movedBy: 'bank.choice', why: 'a small firm is bank-dependent (A5); its bank is who it borrows from' },
    { dim: 'line', movedBy: 'entry', why: 'a mill and a haulier face different prices for different things; a cell that mixed them would decide at an average' },
    {
      dim: 'age',
      movedBy: 'entry',
      opening: (): string => 'opening',
      why: 'Hopenhayn 1992, Axtell 2001: exit is by age and size together; a firm placed at the opening has no age yet, which is a real state',
    },
  ],
  banded: [
    {
      dim: 'size',
      quantity: (reads: LatticeReads, cell: PartyId): Option<number> =>
        some(reads.cashPerMember(cell, reads.homeCurrency(cell))),
      edges: SMALL_FIRM_EDGES.size,
      why: 'Melitz 2003, Axtell 2001: the size distribution is the state, and a bank rations by it',
    },
    {
      dim: 'leverage',
      quantity: (): Option<number> => none<number>(),
      edges: SMALL_FIRM_EDGES.leverage,
      why: 'Stiglitz–Weiss 1981: rationing is by leverage; the read is a debt over what it holds and is unread until it has borrowed (0f.8)',
    },
  ],
};

export function smallBusiness(rows: readonly SmallFirmDecl[]): SystemModule {
  return {
    id: 'small-business',
    spec: 'Small-Business Pools',
    /**
     * A1, A4, A5: it sells to and buys from named firms, it banks and it borrows, so the banks and
     * the firms this sector is connected to have to be there before it.
     *
     * NOT `seed.foundation`, and the reason is the one `land()` is declared after the seed for:
     * assembly sorts by `requires`, so a module declared in the middle of the list that needs the
     * seed DRAGS THE SORT and takes every module after it along. Requiring it here pulled `freight`
     * behind the foundation, the foundation's hull block ran before the carrier parties existed,
     * and this world opened with no merchant fleet at all — the second time that has happened.
     * The seed below adds nothing, so nothing here needs it; when item 0b turns that seed back on,
     * this module is DECLARED AFTER the seed rather than declaring it a requirement in place.
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
        /**
         * A6.a, XI-15: WHERE IT IS, WHERE IT BANKS AND WHAT IT DOES — and no cohort, because a firm
         * has no age at which it retires. The LINE is what makes a population of them a sector: a
         * mill and a haulier face different prices for different things, and a cell that mixed them
         * would be a decision taken at an average (Appendix B).
         */
        lattice: SMALL_FIRM_LATTICE,
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
     * Banks Funding E1: A MODULE THAT DECLARES A DEPOSITOR SAYS HOW IT LEAVES, or the world does
     * not open — which is what stopped it (item 0, stop 2). A small firm's account is operational
     * money and it goes where a named firm's goes, for the same reason and on its own cost.
     */
    bankChoices: [{ partyKind: SMALL_FIRM, chooses: smallFirmChoosesBank }],
    /**
     * Item 11.5–11.10: it has none YET, and the module says so rather than declaring an empty one
     * that reads as done. What it sells, what it employs, what it borrows and what it defaults on
     * are four steps, and each arrives with the phase that runs it.
     */
    phases: [],
    participants: [],
    families: [],
    /**
     * A6, A6.a, XI-15, Seed B1.a: THE SECTOR OPENS, one cell per (region, bank, line).
     *
     * It was off between item 0 and item 0b, and the reason was the kernel's: one list of key
     * dimensions for the whole world meant this world keyed on `cohort`, which a small firm has
     * not got, and a small firm keys on its `line`, which the world did not list — so every party
     * this seed added was refused both ways at once (item 0, stop 6). The key belongs to the KIND
     * now, and a population of firms and a population of people are cut as each of them is.
     */
    seed(ctx: SeedContext): void {
      for (const r of rows) {
        const bank = ctx.parties.get(partyId(r.bank));
        ctx.parties.add({
          id: partyId(r.cell),
          kind: SMALL_FIRM,
          representation: 'cell',
          // Law 4, Law 19: a cell lives where its bank books, read off the bank rather than drawn
          // beside it — the same sentence the households seed makes about where the people are.
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
