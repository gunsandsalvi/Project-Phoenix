/**
 * 0f.10, 0g.1: THE SAME WORLD AT A FINER GRAIN — every lattice band cut in two, a midpoint edge
 * beside every declared one. A resolution is what the answer must not move with; this is how a
 * test asks.
 */
import type { ParamDecl } from '../src/index.js';
import type { rigSpec } from './rig.js';

/** The same world with every lattice band cut in two: a midpoint edge beside every declared one. */
export function refined(spec: ReturnType<typeof rigSpec>): ReturnType<typeof rigSpec> {
  return {
    ...spec,
    modules: spec.modules.map((m) => {
      const extra: ParamDecl[] = [];
      const kinds = m.partyKinds.map((k) => {
        if (k.lattice === undefined) return k;
        return {
          ...k,
          lattice: {
            ...k.lattice,
            banded: k.lattice.banded.map((b) => {
              const edges: ParamDecl['id'][] = [];
              let below = 0;
              for (const e of b.edges) {
                const decl = m.params.find((p) => p.id === e);
                if (decl === undefined) throw new Error(`${String(e)} is not declared by ${m.id}`);
                const mid = { ...decl, id: `${String(e)}.half` as ParamDecl['id'], value: (below + decl.value) / 2 };
                extra.push(mid);
                edges.push(mid.id, e);
                below = decl.value;
              }
              return { ...b, edges };
            }),
          },
        };
      });
      return { ...m, partyKinds: kinds, params: [...m.params, ...extra] };
    }),
  };
}
