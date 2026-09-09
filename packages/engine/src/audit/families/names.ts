/**
 * Names resolve (Audit B6): every party referenced exists; every issuer of a held instrument exists
 * or has a successor; nothing is held by nobody.
 *
 * @spec Audit B6 Register A1.a Register A3 Register A4 Register F2 Currency A2 XI-15
 */
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

export function namesFamily(): Family {
  return {
    name: 'names',
    contributor: 'kernel',
    spec: 'Audit B6',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      const v = (spec: string, owner: string, message: string): void => {
        out.push({
          family: 'names',
          spec,
          owner,
          size: 1,
          unit: 'reference',
          period: view.period,
          message,
        });
      };
      for (const c of view.registry.currencies.values()) {
        if (!view.parties.has(c.centralBank))
          v(
            'Currency A2',
            c.code,
            `currency ${c.code} names issuer ${c.centralBank}, which does not exist`,
          );
        else if (view.registry.partyKind(view.parties.get(c.centralBank).kind).moneyIssuer === null)
          v('Currency A2', c.code, `issuer of ${c.code} is not a central bank`);
      }
      for (const p of view.parties.all()) {
        if (!view.parties.has(p.bank))
          v('Money B1', p.id, `${p.id} banks at ${p.bank}, which does not exist`);
        else if (!view.registry.issuesMoney(view.parties.get(p.bank).kind))
          v('Money A1.d', p.id, `${p.id} banks at ${p.bank}, which issues no money`);
        if (!p.status.alive && !view.parties.has(p.status.successor))
          v(
            'Register F2',
            p.id,
            `${p.id} ceased with successor ${p.status.successor}, which does not exist`,
          );
        if (p.representation === 'cell') {
          if (!view.parties.has(p.key.bank))
            v('XI-15', p.id, `cell ${p.id} keys on bank ${p.key.bank}, which does not exist`);
          view.registry.cohort(p.key.cohort);
        }
        view.registry.region(p.region);
      }
      for (const i of view.instruments.all()) {
        // A claim names the party that promised it; a physical thing names nobody (Goods A1).
        if (i.issuer.some) {
          const issuer = i.issuer.value;
          if (!view.parties.has(issuer))
            v('Register A4', i.id, `instrument ${i.id} has issuer ${issuer}, which does not exist`);
          else if (
            view.registry.instrumentKind(i.kind).pricing === 'money' &&
            !view.registry.issuesMoney(view.parties.get(issuer).kind)
          ) {
            v('Money A1.d', i.id, `${i.id} is money issued by a non-issuer`);
          }
        } else if (view.registry.instrumentKind(i.kind).physical !== true) {
          v('Register B3', i.id, `${i.id} is a claim on nobody`);
        }
        if (
          i.market.some &&
          !view.markets.some(
            (m) => m.id === i.market.valueOf() || (i.market.some && m.id === i.market.value),
          )
        ) {
          v(
            'Clearing D1',
            i.id,
            `instrument ${i.id} names market ${i.market.value}, which does not exist`,
          );
        }
      }
      for (const h of view.register.allHoldings()) {
        if (!view.parties.has(h.holder))
          v(
            'Register A3',
            h.holder,
            `holding of ${h.instrument} on ${h.holder}, which does not exist`,
          );
        else if (!view.parties.get(h.holder).status.alive)
          v('Register F2', h.holder, `${h.holder} has ceased but still holds ${h.instrument}`);
        if (!view.instruments.has(h.instrument))
          v('Register A4', h.instrument, `${h.holder} holds ${h.instrument}, which does not exist`);
        for (const lien of h.liens) {
          if (!view.parties.has(lien.beneficiary))
            v(
              'Register D5.b',
              h.holder,
              `lien on ${h.instrument} names ${lien.beneficiary}, which does not exist`,
            );
        }
      }
      return out;
    },
  };
}
