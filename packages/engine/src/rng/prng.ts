/**
 * The engine's only source of randomness: a seeded sfc32 generator.
 *
 * @spec Seed A5 Audit D3
 *
 * Streams are derived by label so that adding a mechanism's draws never reshuffles another's: the
 * same seed and the same label give the same sequence whatever else ran.
 */
import { Impossible } from '../core/errors.js';

export interface Prng {
  /** Uniform in [0, 1). */
  next(): number;
  /** Uniform integer in [0, n). */
  int(n: number): number;
  /** An independent stream keyed by label, deterministic in (seed, label). */
  derive(label: string): Prng;
  readonly label: string;
}

function hash32(s: string): number {
  // FNV-1a, 32-bit.
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i += 1) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

function splitmix32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x9e3779b9) | 0;
    let t = a ^ (a >>> 16);
    t = Math.imul(t, 0x21f0aaad);
    t = t ^ (t >>> 15);
    t = Math.imul(t, 0x735a2d97);
    return (t ^ (t >>> 15)) >>> 0;
  };
}

class Sfc32 implements Prng {
  private a: number;
  private b: number;
  private c: number;
  private d: number;
  readonly label: string;
  private readonly seedString: string;

  constructor(seedString: string, label: string) {
    this.seedString = seedString;
    this.label = label;
    const mix = splitmix32(hash32(`${seedString} ${label}`));
    this.a = mix();
    this.b = mix();
    this.c = mix();
    this.d = mix();
    // Discard the first outputs so poor seeds do not correlate.
    for (let i = 0; i < 12; i += 1) this.next();
  }

  next(): number {
    const t = (((this.a + this.b) | 0) + this.d) | 0;
    this.d = (this.d + 1) | 0;
    this.a = this.b ^ (this.b >>> 9);
    this.b = (this.c + (this.c << 3)) | 0;
    this.c = (this.c << 21) | (this.c >>> 11);
    this.c = (this.c + t) | 0;
    return (t >>> 0) / 4294967296;
  }

  int(n: number): number {
    if (!Number.isInteger(n) || n <= 0) {
      throw new Impossible('Seed A5', `int(${n}) needs a positive integer`);
    }
    return Math.floor(this.next() * n);
  }

  derive(label: string): Prng {
    return new Sfc32(this.seedString, `${this.label}/${label}`);
  }
}

export function prng(seed: string | number, label = 'root'): Prng {
  return new Sfc32(String(seed), label);
}
