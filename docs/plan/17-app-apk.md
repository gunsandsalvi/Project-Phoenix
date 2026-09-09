# Item 17 — The app and the APK

**Objective.** The product on the phone. Until here the app is the **inspector**: a development
surface that shows every store, every print, every family and every parameter, running the engine
in a worker with snapshots that are copies. This item takes the one decision the specification
reserves for the owner — inspector's full view or participant's partial view (Observer A4) — and
ships that product as an Android APK for the target device, tested on GitHub Pages first. App
detail was deliberately left to the end; this file is the whole of it.

**Read first.** §45 Observer Surface (all); Observer A4 (the reserved decision); Law 8 (period and
unit are part of the number), Law 9 (names), Observer F1–F4; Law 18 (performance); `docs/
ARCHITECTURE.md` (worker boundary, snapshots, the app package); `.github/workflows/{pages,android}.yml`.
Code: `packages/app/` (Vite, the worker bridge, the inspector), `packages/engine/src/observe/`.

**Clauses this item meets.** Observer A1, A1.a, A2, A3, A4 (as decided), A5, A5.a, B1–B5, C1–C4
(participant surface only), D1–D3, E1–E3, F1–F4 (all already MET as reads at the engine; this item
is where they are shown).

---

## Design

- **The decision** (Observer A4): asked of the owner when this item is opened, with the two products
  described: the **inspector** (every party's state visible; no actions; a measurement tool: what
  the development surface already is) or the **participant** (one named party's own state, public
  state and prints; actions that enter the mechanisms with the means checked; a different product
  and a different threat model: A4, C2.a). The answer is recorded and this file's steps are trimmed
  to the product chosen before the first step is ticked.
- **Engine API for the surface**: `observe/snapshot.ts` returns copies with **derivations** attached
  to every number (D3: the reads it came from), `Missing` shown as missing (F4), the one calendar's
  dates (F3), names from the one grammar (F1), price and derived spread for fixed income (F2), stale
  prints marked stale with their period (A1.a), events with time and subjects (B3) and developing
  stories (B5), published statistics with their lag and revision (A5, A5.a). A participant surface
  receives a `ParticipantView` snapshot only (A2, A4); an inspector receives the world view.
- **Actions** (C, participant product only): post a schedule, trade, lend, borrow, hedge, hold —
  each is the same participant declaration a module makes, evaluated in the next period's phases
  with the party's means checked by settlement (C1, C1.a, C2, C2.a: the notional leaves the account;
  C3, C4: the party is in every family). The surface never writes a store (E3): an action is a queued
  participant declaration the kernel evaluates at the right phase.
- **Worker and persistence**: the engine runs in a Web Worker (exists); runs are persisted as
  (seed, journal) and replayed deterministically on any device (D1: the history is a read of what
  happened); no network, no telemetry, no clock in the engine.
- **Performance** (Law 18): a profile run on the target device's class; a declared campaign if
  the per-period cost exceeds what the interaction needs, gated on behaviour (families hold; reads
  unchanged to dust); the engine's API has no layout commitments so the campaign is free.
- **The build**: Vite production build; Capacitor Android project generated in `android.yml`;
  signed APK as the workflow artifact; versioning from the record; installed and run on the target
  device; Pages deployment from `main` for browser testing on every push.

### Files

```
packages/app/src/{screens/*,worker.ts,snapshot.ts,actions.ts}
packages/engine/src/observe/{snapshot.ts,derivations.ts,stories.ts}
packages/app/e2e/*.spec.ts
.github/workflows/{pages,android}.yml
docs/RELEASE.md
```

---

## Steps

- [ ] The owner's decision recorded (inspector or participant); this file's steps trimmed to the product; `docs/ARCHITECTURE.md` updated
- [ ] Snapshot API with derivations on every number, Missing shown as missing, one calendar, one naming grammar, price and spread for fixed income, stale prints marked; tests (A1, A1.a, D3, F1–F4)
- [ ] Events with time and subjects, developing stories, published statistics with lag and revision; tests (A5, A5.a, B1–B5)
- [ ] Screens: markets and prints; balance sheets and statements per party (own state only on a participant surface); events and stories; families and VERIFY reads; parameters with kind, owner, setter and placeholder deaths; the progress and coverage figures
- [ ] Actions as queued participant declarations evaluated by the kernel with the means checked; no write path from the surface; tests (C1–C4, E3) — participant product only
- [ ] Persistence and replay: (seed, journal) saved and replayed deterministically across devices; tests
- [ ] Performance on the target device class: profile run; a Law 18 campaign only if needed, gated on behaviour
- [ ] Browser end-to-end on Pages: smoke, a season run, a scenario replay, a family report
- [ ] Android: Capacitor project generated in the workflow, signed APK artifact, versioned from the record; installed and run on the target device; offline
- [ ] Release checklist in `docs/RELEASE.md`: `npm run check` green, coverage figure, record entry, tag
- [ ] Coverage re-marked; record entry
- [ ] Delete this file; worklist row 17 → done; commit and push

## Exit criteria

The chosen product runs on the target device from a signed APK, shows nothing it cannot derive,
changes nothing by being looked at, and replays any run deterministically.

## Guard

Observer A4, C2.a, E1–E3, F4; Law 18 (no mechanism changes in a performance campaign).
