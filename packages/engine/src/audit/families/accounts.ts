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
import { issuedBy } from '../../register/instruments.js';
import { combineDust, sum, withinDust, type Sum } from '../../core/num.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { weightOf } from '../../parties/party.js';
import type { Period } from '../../calendar/calendar.js';
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
  readonly instruments: Pick<AuditView['instruments'], 'all' | 'get'>;
  readonly register: Pick<
    AuditView['register'],
    'holdingsOf' | 'holdersOf' | 'holding' | 'moneyWalk'
  >;
  readonly valuation: Pick<AuditView['valuation'], 'valueOfLots' | 'inMoney'>;
}

/** The two sides and the rounding the read carries, per member of the party (XI-15). */
export interface BalanceSheet {
  readonly assets: Sum;
  readonly liabilities: Sum;
  /** Law 7: the walk behind every money balance the two sides are read off, per member. */
  readonly walked: number;
  readonly ccy: CurrencyCode;
}

export function balanceSheet(view: BalanceReads, party: PartyId): BalanceSheet {
  const p = view.parties.get(party);
  const home = view.registry.region(p.region).ccy;
  const assetTerms: number[] = [];
  // Law 7: the rounding the READ carries, which is the walk behind every money balance it is read
  // off, not the rounding of adding them up today.
  let walked = 0;
  for (const h of view.register.holdingsOf(party)) {
    const inst = view.instruments.get(h.instrument);
    // Currency C4, C5, D2: A POSITION IN ANOTHER MONEY IS AN ASSET LIKE ANY OTHER, converted at the
    // rate in force — the same rate the same period settled at, so what a balance sheet says and
    // what a payment does cannot disagree.
    assetTerms.push(
      view.valuation.inMoney(
        view.valuation.valueOfLots(inst.id, h.lots, view.period),
        inst.ccy,
        home,
        view.period,
      ),
    );
    if (view.registry.instrumentKind(inst.kind).pricing === 'money') {
      walked += view.register.moneyWalk(party, inst.id).dust;
    }
  }
  const liabilityTerms: number[] = [];
  for (const inst of view.instruments.all()) {
    if (!issuedBy(inst, party) || !view.registry.instrumentKind(inst.kind).liabilityOfIssuer)
      continue;
    const isMoney = view.registry.instrumentKind(inst.kind).pricing === 'money';
    for (const holder of view.register.holdersOf(inst.id)) {
      const h = view.register.holding(holder, inst.id);
      if (!h.some) continue;
      const w = weightOf(view.parties.get(holder));
      liabilityTerms.push(
        view.valuation.inMoney(
          view.valuation.valueOfLots(inst.id, h.value.lots, view.period),
          inst.ccy,
          home,
          view.period,
        ) * w,
      );
      // What this party owes IS those balances, read from the other side (Register B3), so every
      // rounding they have taken since they were opened is a rounding in this number.
      if (isMoney) walked += view.register.moneyWalk(holder, inst.id).dust * w;
    }
  }
  // Per member of the party (XI-15): holdings are per member; liabilities are held by others in
  // total and are divided by the party's own weight.
  const w = weightOf(p);
  return {
    assets: sum(assetTerms),
    liabilities: sum(liabilityTerms.map((t) => t / w)),
    walked: walked / w,
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
            size: assets.value - liabilities.value,
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
        const stands = sum([equity.value, revaluation.value]);
        const read = sum([assets.value, -liabilities.value]);
        // Per member, like the two sides it belongs to (XI-15).
        const dust =
          combineDust(assets, liabilities, read) +
          equity.dust +
          revaluation.dust +
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
            message: `${p.id}: assets ${assets.value} - liabilities ${liabilities.value} != the ${revaluation.value === 0 ? `equity account ${equity.value}` : `accounts it stands on ${stands.value} (equity ${equity.value} and revaluation ${revaluation.value})`} (per member)`,
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
        const home = view.registry.region(p.region).ccy;
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
