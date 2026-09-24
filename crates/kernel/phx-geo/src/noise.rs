use core::f64::consts::TAU;

use libm::{cos, floor, sin};
use phx_macros::clause;
use phx_rand::{Draws, below_u64};

use crate::consts::{FADE_CUBIC, FADE_QUARTIC, FADE_QUINTIC};

/// One octave's lattice of unit gradients at its `cols × rows` corners, repeating beyond them, drawn once per map
/// attempt. Each corner keeps its gradient as one of a byte's worth of evenly spaced directions, so the finest
/// octaves, a million corners each, stay small.
#[derive(Clone, Debug, PartialEq)]
pub struct Lattice {
    cols: u32,
    rows: u32,
    directions: Vec<(f64, f64)>,
    corners: Vec<u8>,
}

/// The improved fade of gradient noise, `6t⁵ − 15t⁴ + 10t³`, whose first and second derivatives vanish at the
/// lattice.
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * FADE_QUINTIC - FADE_QUARTIC) + FADE_CUBIC)
}

/// The whole part of a coordinate, and what is left of it in the cell.
fn split(coord: f64) -> (i64, f64) {
    let whole = floor(coord);
    let Ok(w) = phx_num::Fixed::<0>::from_f64(whole, phx_num::Round::Floor).map(phx_num::Fixed::raw) else {
        phx_num::violation!(clause = "GEO.10", "a noise coordinate beyond a whole number's width");
    };
    (w, coord - whole)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

impl Lattice {
    /// A lattice with a gradient at each of its `cols × rows` corners, each at a uniformly drawn direction.
    #[must_use]
    pub fn draw(cols: u32, rows: u32, d: &mut Draws) -> Lattice {
        let count = u64::from(u8::MAX) + 1;
        let directions = (0..count)
            .map(|k| {
                let angle = TAU * phx_rand::float::from_u64(k) / phx_rand::float::from_u64(count);
                (cos(angle), sin(angle))
            })
            .collect();
        let corners = (0..u64::from(cols) * u64::from(rows))
            .map(|_| u8::try_from(below_u64(d, count)).unwrap_or(u8::MAX))
            .collect();
        Lattice { cols, rows, directions, corners }
    }

    /// The gradient at a corner, the lattice repeating beyond its square.
    fn grad(&self, cx: i64, cy: i64) -> (f64, f64) {
        let (x, y) = (cx.rem_euclid(i64::from(self.cols)), cy.rem_euclid(i64::from(self.rows)));
        let at = y * i64::from(self.cols) + x;
        let direction = usize::try_from(at).ok().and_then(|i| self.corners.get(i)).map(|k| usize::from(*k));
        direction.and_then(|k| self.directions.get(k)).copied().unwrap_or((0.0, 0.0))
    }

    /// The noise at a point, a coordinate of 1 spanning the lattice's square, which repeats beyond it: zero at every
    /// corner, smooth between.
    #[must_use]
    pub fn at(&self, across: f64, down: f64) -> f64 {
        let ((cx, tx), (cy, ty)) = (split(across * f64::from(self.cols)), split(down * f64::from(self.rows)));
        let dot = |gx: i64, gy: i64, dx: f64, dy: f64| {
            let (a, b) = self.grad(gx, gy);
            a * dx + b * dy
        };
        let n00 = dot(cx, cy, tx, ty);
        let n10 = dot(cx + 1, cy, tx - 1.0, ty);
        let n01 = dot(cx, cy + 1, tx, ty - 1.0);
        let n11 = dot(cx + 1, cy + 1, tx - 1.0, ty - 1.0);
        let (sx, sy) = (fade(tx), fade(ty));
        lerp(lerp(n00, n10, sx), lerp(n01, n11, sx), sy)
    }
}

/// The lattices of a fractal sum: octave `k` has `2ᵏ` times the base's cells.
#[must_use]
pub fn octaves(base_cols: u32, base_rows: u32, count: u8, d: &mut Draws) -> Vec<Lattice> {
    (0..count).map(|k| Lattice::draw(base_cols << k, base_rows << k, d)).collect()
}

/// The fractal sum of the octaves at `(u, v)`, each octave's weight `roughness` times the one before.
#[clause("GEO.10")]
#[must_use]
pub fn fractal(layers: &[Lattice], roughness: f64, u: f64, v: f64) -> f64 {
    let mut weight = 1.0;
    let mut sum = 0.0;
    for layer in layers {
        sum += weight * layer.at(u, v);
        weight *= roughness;
    }
    sum
}

#[cfg(test)]
mod tests {
    use phx_core::{Purpose, StreamDecl, Streams};
    use phx_id::Day;
    use phx_rand::{Draws, Seed, Subject, SubjectTag};

    use super::{Lattice, fractal, octaves};

    const MAP: StreamDecl = StreamDecl { name: "GEO.map", purpose: Purpose::Opening, keyed: false, clause: "GEO.10" };

    fn draws(seed: u64, attempt: u64) -> Draws {
        let streams = Streams::new(Seed::new(seed), &[MAP]).unwrap();
        streams.open(&MAP, Subject::new(SubjectTag::World, attempt), Day::new(0), 0)
    }

    #[test]
    fn noise_is_pure_and_seeded() {
        let a = octaves(4, 3, 3, &mut draws(1, 0));
        let b = octaves(4, 3, 3, &mut draws(1, 0));
        let other = octaves(4, 3, 3, &mut draws(1, 1));
        assert_eq!(a, b, "the same draws give the same lattices");
        assert_ne!(a, other, "another attempt draws other gradients");
        let at = |l: &[Lattice], u, v| fractal(l, 0.5, u, v);
        assert_eq!(at(&a, 0.3, 0.7).to_bits(), at(&b, 0.3, 0.7).to_bits());
        let one = Lattice::draw(4, 4, &mut draws(2, 0));
        assert!(one.at(0.25, 0.5).abs() < 1e-12, "zero at a corner");
        let (p, q) = (one.at(0.4, 0.4), one.at(0.4 + 1e-7, 0.4));
        assert!((p - q).abs() < 1e-5, "continuous");
    }
}
