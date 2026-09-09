/**
 * The engine host. The world lives here and nowhere else; the page only ever receives snapshots
 * (Observer E3: no surface that changes the model).
 */
import { foundationWorld, snapshot, type Scope, type Snapshot, type World } from '@phoenix/engine';

export type Request =
  | { readonly type: 'init'; readonly seed: string }
  | { readonly type: 'step'; readonly periods: number }
  | { readonly type: 'snapshot'; readonly scope: Scope; readonly journalTail: number };

export type Response =
  | { readonly type: 'ready'; readonly seed: string }
  | { readonly type: 'stepped'; readonly period: number }
  | { readonly type: 'snapshot'; readonly snapshot: Snapshot }
  | { readonly type: 'error'; readonly message: string; readonly stack: string | undefined };

let world: World | undefined;

function require(): World {
  if (world === undefined) throw new Error('the world has not been seeded');
  return world;
}

function post(r: Response): void {
  self.postMessage(r);
}

self.onmessage = (ev: MessageEvent<Request>): void => {
  const req = ev.data;
  try {
    switch (req.type) {
      case 'init': {
        world = foundationWorld(req.seed);
        world.auditNow();
        post({ type: 'ready', seed: req.seed });
        return;
      }
      case 'step': {
        const w = require();
        for (let i = 0; i < req.periods; i += 1) w.step();
        post({ type: 'stepped', period: w.period });
        return;
      }
      case 'snapshot': {
        post({ type: 'snapshot', snapshot: snapshot(require(), req.scope, req.journalTail) });
        return;
      }
    }
  } catch (e: unknown) {
    // A contract violation stops the run at its site (docs/ARCHITECTURE.md 5); the page shows it.
    const err = e instanceof Error ? e : new Error(String(e));
    post({ type: 'error', message: err.message, stack: err.stack });
  }
};
