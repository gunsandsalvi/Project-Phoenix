/**
 * The inspector surface (Observer A4, inspector scope): reads snapshots from the engine worker and
 * renders them. Nothing here can reach the world.
 */
import type { EventKind } from '@phoenix/engine';
import type { Request, Response } from './worker.js';
import { render } from './ui/render.js';
import './ui/style.css';

/**
 * Observer B1, D1: how many recent events the inspector asks for. It is a DISPLAY DEPTH and nothing
 * reads it back (Observer E3).
 */
const JOURNAL_TAIL = 40;

/**
 * The kinds this page shows a party saying once a period (Sovereign C1.a, C4). They are asked for BY
 * KIND rather than picked out of the feed above, because how far back a feed of everything reaches
 * shrinks every time the world finds more to say: the quotes every bank gives every borrower
 * arrived with the cost of capital (worklist 10), and a period stopped fitting in forty events.
 */
const FOLLOWED: readonly EventKind[] = ['treasury.programme', 'auction.result'];

const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
const root = document.getElementById('app');
if (root === null) throw new Error('no #app root');

let busy = false;
let seed = new URLSearchParams(window.location.search).get('seed') ?? 'phoenix-1';

function send(req: Request): void {
  worker.postMessage(req);
}

function refresh(): void {
  send({
    type: 'snapshot',
    scope: { kind: 'inspector' },
    journalTail: JOURNAL_TAIL,
    follow: FOLLOWED,
  });
}

const actions = {
  step(periods: number): void {
    if (busy) return;
    busy = true;
    send({ type: 'step', periods });
  },
  reseed(next: string): void {
    seed = next;
    busy = true;
    send({ type: 'init', seed });
  },
};

worker.onmessage = (ev: MessageEvent<Response>): void => {
  const msg = ev.data;
  switch (msg.type) {
    case 'ready':
      busy = false;
      refresh();
      return;
    case 'stepped':
      busy = false;
      refresh();
      return;
    case 'snapshot':
      render(root, msg.snapshot, actions, { busy, seed });
      return;
    case 'error':
      busy = false;
      render(root, null, actions, { busy, seed, error: `${msg.message}\n${msg.stack ?? ''}` });
      return;
  }
};

render(root, null, actions, { busy: true, seed });
send({ type: 'init', seed });
