use core::f64::consts::{SQRT_2, TAU};

use libm::{cos, exp, pow, sin, sqrt};
use phx_macros::clause;
use phx_rand::{Draws, normal, open_unit};

use crate::consts::HALF;
use crate::grid::Grid;
use crate::hydrology::{drainage, upstream};
use crate::noise::{Lattice, fractal, octaves};

/// What the relief is generated from, every value read from the register.
#[derive(Clone, Debug, PartialEq)]
pub struct ReliefParams {
    pub base_cells: u32,
    pub octaves: u8,
    pub roughness: f64,
    pub falloff: f64,
    pub plates: u32,
    pub belt: f64,
    pub plate_weight: f64,
    pub mountain_weight: f64,
    pub warp: f64,
    pub plate_warp: f64,
    pub erosion_passes: u32,
    pub erosion_rate: f64,
    pub area_exponent: f64,
}

/// A tectonic plate: where it lies, how high its crust stands, and how it moves.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Plate {
    u: f64,
    v: f64,
    base: f64,
    du: f64,
    dv: f64,
}

fn plates(count: u32, d: &mut Draws) -> Vec<Plate> {
    (0..count)
        .map(|_| {
            let (u, v) = (open_unit(d), open_unit(d));
            let base = normal(d);
            let (angle, speed) = (TAU * open_unit(d), open_unit(d));
            Plate { u, v, base, du: speed * cos(angle), dv: speed * sin(angle) }
        })
        .collect()
}

/// The two plates nearest a point, and the point's distance to the line halfway between them.
fn nearest_two(plates: &[Plate], u: f64, v: f64) -> Option<(Plate, Plate, f64)> {
    let dist2 = |p: &Plate| (p.u - u) * (p.u - u) + (p.v - v) * (p.v - v);
    let mut best: Option<(f64, Plate)> = None;
    let mut second: Option<(f64, Plate)> = None;
    for p in plates {
        let d = dist2(p);
        if best.is_none_or(|(b, _)| d < b) {
            second = best;
            best = Some((d, *p));
        } else if second.is_none_or(|(s, _)| d < s) {
            second = Some((d, *p));
        }
    }
    let ((d1, a), (d2, b)) = (best?, second?);
    let apart = sqrt((b.u - a.u) * (b.u - a.u) + (b.v - a.v) * (b.v - a.v));
    Some((a, b, (d2 - d1) / (2.0 * apart)))
}

/// How far a point of the plane lies toward the ocean around the map: nothing at the centre, growing as the square of
/// the distance from it there, and without end toward the edges, so no land reaches them. It is `1/q - 1` over four,
/// where `q`, the product of each axis's `4x(1 - x)`, is one at the centre and nothing at an edge.
fn ocean(east: f64, south: f64) -> f64 {
    let (du, dv) = (east - HALF, south - HALF);
    let q = (1.0 - 2.0 * 2.0 * du * du) * (1.0 - 2.0 * 2.0 * dv * dv);
    (1.0 / q - 1.0) * HALF * HALF
}

/// The noise fields one attempt draws, in a fixed order: the relief's octaves, the two warp fields, the ridges, the two
/// fields that bend the plates' sutures, and the plates.
struct Fields {
    relief: Vec<Lattice>,
    warp_u: Vec<Lattice>,
    warp_v: Vec<Lattice>,
    ridges: Vec<Lattice>,
    plate_u: Vec<Lattice>,
    plate_v: Vec<Lattice>,
    plates: Vec<Plate>,
}

/// The raw relief on the fine grid, before erosion: each plate's crust blended across its borders, mountain belts
/// raised where plates converge (ridged), the fractal relief over a warped plane, less the falloff that puts the sea
/// at the edges.
#[clause("GEO.10")]
#[must_use]
pub fn raw(p: &ReliefParams, fine: &Grid, draws: &mut Draws) -> Vec<f64> {
    let fields = Fields {
        relief: octaves(p.base_cells, p.base_cells, p.octaves, draws),
        warp_u: octaves(p.base_cells, p.base_cells, p.octaves, draws),
        warp_v: octaves(p.base_cells, p.base_cells, p.octaves, draws),
        ridges: octaves(p.base_cells, p.base_cells, p.octaves, draws),
        plate_u: octaves(p.base_cells, p.base_cells, p.octaves, draws),
        plate_v: octaves(p.base_cells, p.base_cells, p.octaves, draws),
        plates: plates(p.plates, draws),
    };
    let span = f64::from(fine.width);
    (0..fine.len())
        .map(|index| {
            let (col, row) = fine.xy(fine.tile(index));
            let (east, south) = ((f64::from(col) + HALF) / span, (f64::from(row) + HALF) / span);
            let (wu, wv) = (
                east + p.warp * fractal(&fields.warp_u, p.roughness, east, south),
                south + p.warp * fractal(&fields.warp_v, p.roughness, east, south),
            );
            let (pu, pv) = (
                wu + p.plate_warp * fractal(&fields.plate_u, p.roughness, wu, wv),
                wv + p.plate_warp * fractal(&fields.plate_v, p.roughness, wu, wv),
            );
            let (plate, uplift) = match nearest_two(&fields.plates, pu, pv) {
                Some((own, other, border)) => {
                    let near = exp(-border / p.belt);
                    let crust = own.base * (1.0 - near) + (own.base + other.base) * HALF * near;
                    let apart = sqrt((other.u - own.u) * (other.u - own.u) + (other.v - own.v) * (other.v - own.v));
                    let converging =
                        ((own.du - other.du) * (other.u - own.u) + (own.dv - other.dv) * (other.v - own.v)) / apart;
                    (crust, if converging > 0.0 { converging * near } else { 0.0 })
                }
                None => (0.0, 0.0),
            };
            let ridge = 1.0 - fractal(&fields.ridges, p.roughness, wu, wv).abs();
            p.plate_weight * plate
                + fractal(&fields.relief, p.roughness, wu, wv)
                + p.mountain_weight * uplift * (1.0 + ridge)
                - p.falloff * ocean(east, south)
        })
        .collect()
}

/// Rivers cut the relief: each pass routes the water to the outlets, then lowers every cell toward its receiver by
/// the stream-power law, solved implicitly from the outlets up so no pass overshoots.
#[clause("GEO.10")]
pub fn erode(p: &ReliefParams, fine: &Grid, height: &mut [f64], outlet: &[bool]) {
    for _ in 0..p.erosion_passes {
        let routes = drainage(fine, height, outlet);
        let area = upstream(&routes);
        for cell in &routes.order {
            let Some(r) = routes.receiver.get(*cell).copied().flatten() else { continue };
            let (Some(h_r), Some(a)) = (height.get(r).copied(), area.get(*cell).copied()) else { continue };
            let (cx, cy) = fine.xy(fine.tile(*cell));
            let (rx, ry) = fine.xy(fine.tile(r));
            let run = if cx != rx && cy != ry { SQRT_2 } else { 1.0 };
            let cut = p.erosion_rate * pow(f64::from(a), p.area_exponent) / run;
            if let Some(h) = height.get_mut(*cell) {
                *h = (*h + cut * h_r) / (1.0 + cut);
            }
        }
    }
}

/// A measured curve: points at parts per thousand of the cells, with the metres at each, read between its points.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    points: Vec<(f64, f64)>,
}

impl Curve {
    #[must_use]
    pub fn new(axis: &[i64], values: &[i64]) -> Curve {
        let points = axis
            .iter()
            .zip(values)
            .map(|(a, v)| (phx_rand::float::from_i64(*a), phx_rand::float::from_i64(*v)))
            .collect();
        Curve { points }
    }

    /// The curve's value at `per_mille`, flat beyond its ends.
    #[must_use]
    pub fn at(&self, per_mille: f64) -> f64 {
        let mut below = self.points.first().copied().unwrap_or((0.0, 0.0));
        for (x, y) in &self.points {
            if *x >= per_mille {
                let (x0, y0) = below;
                return if *x > x0 { y0 + (y - y0) * (per_mille - x0) / (x - x0) } else { *y };
            }
            below = (*x, *y);
        }
        below.1
    }
}

/// Heights mapped by rank to a measured curve: the cells of `mask`, lowest first (ties by place), each at the curve's
/// value for its share of the cells below it, so the relief keeps its shape and takes the Earth's distribution.
#[clause("GEO.10")]
pub fn to_curve(height: &mut [f64], mask: &[bool], curve: &Curve, per_mille_whole: f64) {
    let mut ranked: Vec<(f64, usize)> =
        height.iter().zip(mask).enumerate().filter(|(_, (_, m))| **m).map(|(i, (h, _))| (*h, i)).collect();
    ranked.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let n = phx_rand::float::from_u64(phx_rand::float::len_u64(ranked.len()));
    for (rank, (_, cell)) in ranked.iter().enumerate() {
        let share = (phx_rand::float::from_u64(phx_rand::float::len_u64(rank)) + HALF) / n * per_mille_whole;
        if let Some(h) = height.get_mut(*cell) {
            *h = curve.at(share);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Curve, to_curve};

    #[test]
    fn heights_take_the_measured_curve_by_rank() {
        let curve = Curve::new(&[0, 500, 1000], &[0, 100, 1000]);
        assert!((curve.at(250.0) - 50.0).abs() < 1e-9);
        assert!((curve.at(750.0) - 550.0).abs() < 1e-9);
        let mut h = vec![9.0, -3.0, 4.0, 7.0];
        let mask = [true, false, true, true];
        to_curve(&mut h, &mask, &curve, 1000.0);
        assert!(h[2] < h[3] && h[3] < h[0], "order kept");
        assert_eq!(h[1].to_bits(), (-3.0_f64).to_bits(), "cells outside the mask untouched");
        assert!((h[3] - 100.0).abs() < 1e-9, "the median cell at the curve's median");
    }
}
