/**
 * A participant says which markets it is in, and the session asks only those.
 *
 * @spec Clearing B2 Clearing A2 Law 4 Law 18 Law 19
 *
 * Law 18 is the whole of this: mechanisms, economics and boundaries never change in performance
 * work, and the gate is BEHAVIOUR rather than bits. A session used to ask every party of a kind
 * whether it had an order in it, which at the real scale is 261 markets against 3,169 parties —
 * 827,000 questions a period whose answer is almost always no. The kernel cannot guess which of
 * them could answer yes, because which books a party is in is its own business, so the module that
 * owns the party says (`ParticipantDecl.markets`).
 *
 * WHAT COULD GO WRONG IS SILENT, which is why this file exists. A `markets` narrower than the
 * party's own `orders` loses real schedules out of a real book and nothing anywhere throws: the
 * print is simply made against fewer orders than the world had. So the check is the expensive path
 * run once, in a test, against the cheap one: ask every participant for its orders in every market
 * there is, and assert it never posts in a book it did not name.
 */
import { describe, expect, it } from 'vitest';
import { assemble, type SystemModule, type World } from '../src/index.js';
import { rigSpec } from './rig.js';

/** The participants a module declared, with the module's own id for the failure message. */
function participantsOf(
  modules: readonly SystemModule[],
  id: string,
): SystemModule['participants'] {
  const m = modules.find((x) => x.id === id);
  expect(m, `${id} is not in this world`).toBeDefined();
  return m!.participants;
}

describe('the markets a participant says it is in (Law 18, Law 19)', () => {
  it('never leaves out a book the same party would have posted in', () => {
    const spec = rigSpec('fan-out', 4, 24);
    const w: World = assemble(spec);
    // Far enough in that plans exist, goods are being bought and ladders are posted: a world at
    // period one has decided almost nothing and would pass this without testing anything.
    for (let i = 0; i < 6; i += 1) w.step();
    let asked = 0;
    let posted = 0;
    for (const id of ['firms', 'households']) {
      for (const decl of participantsOf(spec.modules, id)) {
        const naming = decl.markets;
        expect(naming, `${id} declares no markets, so there is nothing to check`).toBeDefined();
        for (const party of w.parties.ofKind(decl.partyKind)) {
          if (!party.status.alive) continue;
          const view = w.participantView(party.id);
          const named = new Set(naming!(view).map(String));
          asked += named.size;
          for (const m of w.markets) {
            if ((decl.in ?? 'asset') !== (m.kind ?? 'asset')) continue;
            const orders = decl.orders(view, m);
            if (orders.length === 0) continue;
            posted += 1;
            expect(
              named.has(String(m.id)),
              `${party.id} posts ${orders.length} orders in ${m.id} and ${id} did not name it`,
            ).toBe(true);
          }
        }
      }
    }
    // And the door is doing something: a world where every party named every market would pass the
    // assertion above and save nothing.
    expect(posted).toBeGreaterThan(0);
    expect(asked).toBeGreaterThan(0);
    expect(asked).toBeLessThan(w.markets.length * w.parties.all().length);
  });

  it('gives the same world instruction for instruction, twice from the same seed', () => {
    // A traversal change is safe when the order it traverses in is stable. The index is built by
    // walking the parties of a kind in the registry's own order and appending, so two worlds from
    // one seed post the same orders in the same sequence — and if they ever did not, a market's
    // rationing would break ties differently and the two ledgers would part.
    const ledger = (w: World): string[] => {
      const out: string[] = [];
      for (let p = 1; p <= 6; p += 1) {
        for (const r of w.ledger.inPeriod(p as never)) {
          out.push(
            `${r.instruction.id}|${r.outcome}|${r.instruction.cause}|${r.instruction.reason}`,
          );
        }
      }
      return out;
    };
    const a = assemble(rigSpec('fan-out-twice', 3, 12));
    const b = assemble(rigSpec('fan-out-twice', 3, 12));
    for (let i = 0; i < 6; i += 1) {
      a.step();
      b.step();
    }
    expect(ledger(a)).toEqual(ledger(b));
    expect(ledger(a).length).toBeGreaterThan(0);
  });
});
