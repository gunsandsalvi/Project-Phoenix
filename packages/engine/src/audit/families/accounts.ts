/**
 * Accounts balance (Audit B5): assets minus liabilities, read from the register at marks, equals
 * the stated equity account, per party, per member.
 *
 * @spec Central Bank A2 Audit B5 Audit B5.a Audit B5.b Banks Funding F3 Central Bank A2.c Firm C3 Fund Shares A3 Households D3 Law 7 Money D2
 *
 * The two sides are independent records: the register and the price store on one side, the equity
 * account moved by named events on the other. Equality is the check.
 *
 * The two sides are not reached by the same arithmetic, and Law 7 says the tolerance is what the
 * arithmetic did: one side is a fresh sum over today's holdings, the other a balance that has been
 * moved once per event since the party was born. So the account brings its own walk (`equityWalk`)
 * and the read brings the sum's, and the dust is the two together — never a band anyone chose.
 *
 * AND THE READ IS NOT ONE ROUNDING OLD EITHER. A money balance is itself a walk: one lot moved once
 * per leg since the account was opened (Money D2). A bank's deposits and a central bank's reserves
 * are read off those balances, so what a party's assets and liabilities in money have accumulated
 * in rounding is every move of every one of those accounts — not the rounding of adding them up
 * today. An issuer of money whose customers are busy is the case that shows it: its liability is a
 * sum over accounts that moved hundreds of times, and a tolerance that saw only the sum reports a
 * violation the moment the world starts paying itself.
 */
import { sumCash, type CashSum } from '../../core/measure.js';
import { type Cash, heldAsMoney } from '../../core/measure.js';
import { negated } from '../../core/measure.js';
import { combineDust, sum, withinDust } from '../../core/num.js';
import { Impossible } from '../../core/errors.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { weightOf } from '../../parties/party.js';
import type { Period } from '../../calendar/calendar.js';
import type { Contract } from '../../registry/derivatives.js';
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

/**
 * Audit B5, Reporting A2, Law 4: WHAT A PARTY IS WORTH, read from the register and the marks.
 *
 * Two readers want this and there must be one of it. The `accounts` family compares it against the
 * equity account, which is the check; a public company's report PUBLISHES it as its balance sheet
 * at the fiscal close (Reporting A2), which is a read of the same thing. A report with its own
 * implementation of the balance sheet would be a second set of accounts able to disagree with the
 * one the audit checks — exactly what A2.a forbids — and the disagreement would surface as a
 * company whose published sheet balances and whose audited one does not.
 *
 * It is structural in its view so both callers can pass what they have: the family has an
 * `AuditView`, a module has a `MechanismContext`, and every read here is on both.
 */
export interface BalanceReads {
  readonly period: Period;
  readonly registry: AuditView['registry'];
  readonly parties: { get: AuditView['parties']['get'] };
  readonly instruments: Pick<AuditView['instruments'], 'all' | 'get' | 'issuedBy'>;
  readonly register: Pick<
    AuditView['register'],
    'holdingsOf' | 'holdersOf' | 'holding' | 'moneyWalk'
  >;
  readonly valuation: Pick<AuditView['valuation'], 'valueOfLots' | 'inMoney'>;
  /**
   * Derivative D1, X1: the rows this party is a side of, and what each is worth to it. A contract
   * is not a holding and is not in the register, and it IS on the balance sheet — an asset to one
   * side and a liability to the other, at every instant — so the sheet reads it from its own store.
   */
  readonly contracts: {
    openOf(party: PartyId): readonly Contract[];
    valueTo(contract: Contract, party: PartyId, at: Period): Cash;
  };
}

/** The two sides and the rounding the read carries, per member of the party (XI-15). */
export interface BalanceSheet {
  readonly assets: CashSum;
  readonly liabilities: CashSum;
  /** Law 7: the walk behind every money balance the two sides are read off, per member. */
  readonly walked: number;
  readonly ccy: CurrencyCode;
}

export function balanceSheet(view: BalanceReads, party: PartyId): BalanceSheet {
  return sheetOf(view, party, NOBODY);
}

/** Nothing is eliminated for a party on its own: it owes itself nothing (Law 5). */
const NOBODY: ReadonlySet<string> = new Set<string>();

/**
 * The read, with the set of parties whose claims on this one are INSIDE the same group and are
 * therefore not the group's assets or liabilities. For a party on its own that set is empty and
 * this is exactly the sheet it always was.
 */
function sheetOf(view: BalanceReads, party: PartyId, inside: ReadonlySet<string>): BalanceSheet {
  const p = view.parties.get(party);
  const home = view.registry.currencyOf(p.region);
  const assetTerms: Cash[] = [];
  // Law 7: the rounding the READ carries, which is the walk behind every money balance it is read
  // off, not the rounding of adding them up today.
  let walked = 0;
  for (const h of view.register.holdingsOf(party)) {
    const inst = view.instruments.get(h.instrument);
    // A4: a claim on somebody in the same group is not the group's asset. It is one member's
    // claim on another, and it is a liability of that other in the same consolidation.
    if (inst.issuer.some && inside.has(String(inst.issuer.value))) continue;
    // Currency C4, C5, D2: A POSITION IN ANOTHER MONEY IS AN ASSET LIKE ANY OTHER, converted at the
    // rate in force — the same rate the same period settled at, so what a balance sheet says and
    // what a payment does cannot disagree.
    // Currency C4, D2: a report in the party's home money — a translation at the rate in force,
    // never a conversion of what it holds (B3).
    assetTerms.push(
      view.valuation.inMoney(
        view.valuation.valueOfLots(inst.id, h.lots, view.period),
        home,
        view.period,
      ),
    );
    if (view.registry.instrumentKind(inst.kind).pricing === 'money') {
      walked += view.register.moneyWalk(party, inst.id).dust;
    }
  }
  const liabilityTerms: Cash[] = [];
  /**
   * D1: an open contract is an asset to one side and a liability to the other, at every instant.
   * It is kept apart from the liabilities the register holds because those are read from the OTHER
   * side — held by other parties in total, and divided by this one's weight below — while a
   * contract's value is already per member, the way the leg that opened it booked it (XI-15).
   */
  const contractLiabilities: Cash[] = [];
  for (const c of view.contracts.openOf(party)) {
    // D1: a contract is an asset to one side and a liability to the other, so one between two
    // members of the group is worth exactly nothing to the group (Law 4).
    if (inside.has(String(c.a === party ? c.b : c.a))) continue;
    const worth = view.valuation.inMoney(
      view.contracts.valueTo(c, party, view.period),
      home,
      view.period,
    );
    if (worth.pieces >= 0) assetTerms.push(worth);
    else contractLiabilities.push(negated(worth, 'what this contract owes'));
  }
  /**
   * Law 18: WHAT THIS PARTY ISSUED, asked of the index that already answers it. This used to walk
   * every instrument in the world for every party, which is parties times instruments once a period
   * — three thousand parties against five thousand lines is fifteen million visits to find each
   * party's handful of liabilities. `Instruments` has kept a `byIssuer` index since it was written;
   * this is the one read that was not using it.
   */
  for (const inst of view.instruments.issuedBy(party)) {
    if (!view.registry.instrumentKind(inst.kind).liabilityOfIssuer) continue;
    const isMoney = view.registry.instrumentKind(inst.kind).pricing === 'money';
    for (const holder of view.register.holdersOf(inst.id)) {
      // A4: what it owes a party in the same group is not the group's liability — the other half
      // of the elimination above, and both halves have to go or the sheet stops balancing.
      if (inside.has(String(holder))) continue;
      const h = view.register.holding(holder, inst.id);
      if (!h.some) continue;
      /**
       * Register B3 (13f): WHAT THIS PARTY OWES, and for almost everything that is the BALANCE
       * rather than what somebody would pay for it today. A borrower owes the whole of what it
       * promised whatever the paper trades at; the holder's mark is the holder's business, and the
       * two are read off the same row from opposite ends without being the same number.
       *
       * It used to read the holder's carrying value on both sides, which was consistent only
       * because the issuer's equity was being moved by the holder's re-mark — the same defect that
       * had a firm booking a profit as it walked towards default. With that gone, a balance sheet
       * that still valued its own debt at the market would be a balance sheet whose equity account
       * and whose liabilities disagreed by exactly the fiction that was removed.
       *
       * `'value'` is the fund share, whose issuer genuinely owes what the book is worth (A3).
       */
      const kind = view.registry.instrumentKind(inst.kind);
      const owed =
        kind.owes === 'value'
          ? view.valuation.valueOfLots(inst.id, h.value.lots, view.period)
          : heldAsMoney(sum(h.value.lots.map((l) => l.qty)).value, inst.ccy, 'the face it holds');
      // 0f.1: the holder's lots are its TOTAL, so what this party owes on them is read straight.
      liabilityTerms.push(view.valuation.inMoney(owed, home, view.period));
      // What this party owes IS those balances, read from the other side (Register B3), so every
      // rounding they have taken since they were opened is a rounding in this number.
      if (isMoney) walked += view.register.moneyWalk(holder, inst.id).dust;
    }
  }
  // 0f.1: THE SHEET IS A TOTAL on both sides. Holdings are the party's totals (the register holds
  // them so), liabilities are what others hold of its paper in total, and a contract's value is
  // what the contract is worth to the party. Per member is a READ of this, where anybody wants one.
  return {
    assets: sumCash(home, assetTerms, 'what it holds'),
    liabilities: sumCash(home, [...liabilityTerms, ...contractLiabilities], 'what it owes'),
    walked,
    ccy: home,
  };
}

/**
 * M&A A4, item 9: THE GROUP'S ACCOUNTS — the sheets of every party in it, added, with every claim
 * one member holds on another taken out of BOTH sides.
 *
 * A group's accounts used to be the parent's, because there was no group. What makes this a
 * consolidation and not an addition is the ELIMINATION: a parent's loan to its subsidiary is an
 * asset of the parent and a liability of the subsidiary, and a group cannot owe itself money.
 * Leaving it in would show a group that has lent to nobody outside as holding assets it has not
 * got, and the deeper the group the larger the fiction (Law 4: one representation per real thing).
 *
 * It is a READ and stores nothing (Law 19; Appendix B: no stored aggregate). It is here rather than
 * in a module because it is the SAME read as the sheet above, restricted to a set of parties: a
 * second implementation would be a second set of accounts able to disagree with the one the audit
 * proves (A2.a). Members' home currencies may differ; each sheet is already in its own party's
 * money and `inMoney` puts them in the root's at the rate in force (Currency C4, C5, D2).
 *
 * XI-15: each member's sheet is PER MEMBER, so a member that is a cell is multiplied by its weight
 * before it is added — a group holding a hundred identical subsidiaries holds a hundred of them.
 */
export function consolidated(view: BalanceReads, group: readonly PartyId[]): BalanceSheet {
  const root = group[0];
  if (root === undefined) {
    throw new Impossible('M&A A4', 'a group with nobody in it has no accounts to consolidate');
  }
  const home = view.registry.currencyOf(view.parties.get(root).region);
  const inside = new Set(group.map((p) => String(p)));
  const assets: Cash[] = [];
  const liabilities: Cash[] = [];
  let walked = 0;
  for (const member of group) {
    // 0f.1: a member's sheet is already its TOTAL; the group's is the sum of them.
    const sheet = sheetOf(view, member, inside);
    assets.push(view.valuation.inMoney(sheet.assets.value, home, view.period));
    liabilities.push(view.valuation.inMoney(sheet.liabilities.value, home, view.period));
    // Law 7: the dust the READS carried, added the way the numbers were, and never a band.
    walked += sheet.walked;
  }
  return {
    assets: sumCash(home, assets, 'the group holds'),
    liabilities: sumCash(home, liabilities, 'the group owes'),
    walked,
    ccy: home,
  };
}

export function accountsFamily(): Family {
  return {
    name: 'accounts',
    contributor: 'kernel',
    spec: 'Audit B5',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      for (const p of view.parties.alive()) {
        // Law 4: ONE READ OF THE BALANCE SHEET, the same one a public company publishes (Reporting
        // A2). A family with its own copy of it would be checking the equity account against a
        // number no reader outside the audit ever sees.
        const sheet = balanceSheet(view, p.id);
        const home = sheet.ccy;
        const assets = sheet.assets;
        const liabilities = sheet.liabilities;
        const walked = sheet.walked;
        if (!view.register.hasEquityAccount(p.id)) {
          out.push({
            family: 'accounts',
            spec: 'Audit B5',
            owner: p.id,
            size: assets.value.pieces - liabilities.value.pieces,
            unit: home,
            period: view.period,
            message: `${p.id} has no stated equity account`,
          });
          continue;
        }
        // Central Bank A2.c, Currency D2.a: what stands against the read is the party's equity AND,
        // for the central bank of its own money, its revaluation account — the one place a rate move
        // on foreign reserves goes, because those reserves are the other side of what it printed
        // rather than a position it took. For everybody else the account is zero and this is the
        // equity account, which is the one number they have.
        const equity = view.register.equityWalk(p.id);
        const revaluation = view.register.revaluationWalk(p.id);
        // 0f.1: the sheet is a TOTAL and the equity account is still PER MEMBER (settlement writes
        // it so until 0f.2), so what stands against the read is the account over the people. The
        // multiplication is by a count and adds no dust (Law 7).
        const people = weightOf(p);
        const stands = sum([equity.value * people, revaluation.value * people]);
        const read = sum([assets.value.pieces, -liabilities.value.pieces]);
        /**
         * Law 7 (21.104): THE DUST OF A PRODUCT IS SCALED BY THE MULTIPLIER. The two accounts are
         * kept PER MEMBER and the sheet is a TOTAL, so the comparison happens at the total's
         * magnitude — and the rounding a per-member walk carries arrives there multiplied by the
         * people, exactly as its value does. This added the per-member dust unscaled, so a cell of
         * fourteen thousand members was held to a tolerance fourteen thousand times tighter than
         * the arithmetic it was measuring: four household cells reported residuals of 0.18 to 1.0
         * pieces against assets of three hundred and forty BILLION — eight parts in ten trillion,
         * which is what a walk of that length at that magnitude leaves behind.
         *
         * It is not a widened band (Law 7 forbids one): it is the same multiplication applied to
         * the same number's dust, and a cell of one is unchanged by it.
         */
        const dust =
          combineDust(assets, liabilities, read) +
          equity.dust * people +
          revaluation.dust * people +
          stands.dust +
          walked;
        if (!withinDust(read.value, stands.value, dust)) {
          out.push({
            family: 'accounts',
            spec: 'Audit B5',
            owner: p.id,
            // A2: the size is the gap in the identity that was CHECKED, which is the read against
            // what stands — equity and, for a central bank of its own money, its revaluation
            // account. Reporting it against equity alone named a different number from the one the
            // comparison failed on, for exactly the party whose second account is the point.
            size: read.value - stands.value,
            unit: home,
            period: view.period,
            message: `${p.id}: assets ${assets.value.pieces} - liabilities ${liabilities.value.pieces} != the ${revaluation.value === 0 ? `equity account ${equity.value}` : `accounts it stands on ${stands.value} (equity ${equity.value} and revaluation ${revaluation.value})`} (per member)`,
          });
        }
      }
      return out;
    },
  };
}

/**
 * Reporting A2, A2.a, G2; Audit A1.a, Law 4: THE ITEMISATION AND THE BALANCE ARE TWO RECORDS OF ONE
 * THING, and this is where they are made to agree.
 *
 * A report says what a company earned by reading the entries that moved its equity account
 * (`equityEntries`). That is only worth reading if the entries are ALL of them — a report built on
 * an itemisation that has lost one is a second set of accounts, which is exactly what A2.a forbids.
 * So the check is the sum of every entry against the balance the walk carries, and the COUNT of
 * them against the number of moves the walk counted. The second is what catches an entry that is
 * missing and one that was invented to replace it: a sum can be made to agree by two errors, a
 * count cannot.
 *
 * It is never the other way round. The walk is the balance and stays the balance (Law 4); if the
 * entries were summed to PRODUCE it this check would be a tautology and the report would be
 * unfalsifiable.
 */
export function equityLedgerFamily(): Family {
  return {
    name: 'accounts',
    contributor: 'kernel.equityLedger',
    spec: 'Reporting A2 Reporting G2 Audit A1.a Law 4',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      for (const p of view.parties.alive()) {
        if (!view.register.hasEquityAccount(p.id)) continue;
        const walk = view.register.equityWalk(p.id);
        const entries = view.register.equityEntries(p.id, 0 as Period, view.period);
        const home = view.registry.currencyOf(p.region);
        // One entry for the opening statement, one for every move since (Register E2.a: nothing is
        // edited and nothing is reversed, so the count only ever grows).
        if (entries.length !== walk.moves + 1) {
          out.push({
            family: 'accounts',
            spec: 'Reporting A2',
            owner: String(p.id),
            size: entries.length - (walk.moves + 1),
            unit: 'entries',
            period: view.period,
            message: `${p.id}: the equity ledger has ${entries.length} entries and the account has been moved ${walk.moves} times from one opening`,
          });
          continue;
        }
        const itemised = sum(entries.map((e) => e.delta));
        if (!withinDust(itemised.value, walk.value, itemised.dust + walk.dust)) {
          out.push({
            family: 'accounts',
            spec: 'Reporting G2',
            owner: String(p.id),
            size: itemised.value - walk.value,
            unit: home,
            period: view.period,
            message: `${p.id}: the equity ledger sums to ${itemised.value} and the account stands at ${walk.value} (per member)`,
          });
        }
      }
      return out;
    },
  };
}
