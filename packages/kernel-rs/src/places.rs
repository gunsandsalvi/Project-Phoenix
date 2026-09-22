//! WHAT STANDS ON A PLACE, AND WHAT THAT DOES TO BUILDING THERE.
//!
//! @spec 33 A4 · 33 B1 · 40 A1.a · 40 B1.a · Law 2, Law 6, Law 8, Law 15, Law 19 · Appendix B

use crate::geography::Geography;
use crate::ids::{InstrumentId, RegionId};
use crate::parties::Parties;
use crate::register::Register;
use crate::registry::Registry;

/// How much ground is covered in each region, as a read over the holdings — one pass, bucketed by
/// region, never a number anybody keeps.
pub fn built_up(
    parties: &Parties,
    register: &Register,
    registry: &Registry,
    geography: &Geography,
) -> Vec<f64> {
    let mut by_region = vec![0.0; geography.regions()];
    for row in register.all() {
        let line = register.instrument_of(row);
        // A line with no footprint is not a structure — it is flour.
        let Some(footprint) = registry.footprint_of(line) else {
            continue;
        };
        let holder = register.holder_of(row);
        if !parties.alive(holder) {
            continue;
        }
        // A structure is somewhere, and where it is, is where its owner is.
        let at = parties.region_of(holder).0 as usize;
        if at < by_region.len() {
            by_region[at] += register.quantity(row) * footprint.km2();
        }
    }
    by_region
}

/// WHAT A BUILD DRAWS WHERE THIS MUCH ALREADY STANDS, as a multiple of what the same build draws on
/// empty ground.
pub fn crowding(standing: f64, crowds_at: f64) -> f64 {
    assert!(
        crowds_at > 0.0,
        "21i: a world that is fully built at nothing standing is not a world"
    );
    assert!(
        standing >= 0.0,
        "Law 8: {standing} km² of ground covered is not an area"
    );
    1.0 + standing / crowds_at
}

/// Whether this line is a structure at all — what a builder asks before asking where.
pub fn is_a_structure(registry: &Registry, line: InstrumentId) -> bool {
    registry.footprint_of(line).is_some()
}

/// The standing area in one region, out of a `built_up` walk.
pub fn standing_in(built: &[f64], at: RegionId) -> f64 {
    match built.get(at.0 as usize) {
        Some(km2) => *km2,
        None => 0.0,
    }
}

// What a place holds is a read over the register, the parties, the registry and the ground itself,
// so `built_up` and `standing_in` are answered against the real world rather than a fixture.
//
// Two refusals the TYPE now makes unconstructible: a structure that stands on nothing (`Footprint`
// has no way to be zero) and a line declared a structure twice (the second `stands_on` panics at
// the site). What is a structure is a number in the registry, so an office block, a dwelling and a
// works go through one mechanism and nothing branches on which.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Footprint;

    #[test]
    fn a_footprint_of_nothing_is_not_a_footprint() {
        assert!(Footprint::new(0.0).is_none());
        assert!(Footprint::new(-1.0).is_none());
        assert!(Footprint::new(f64::INFINITY).is_none());
        assert_eq!(Footprint::new(3.0).map(Footprint::km2), Some(3.0));
    }

    #[test]
    fn building_where_this_much_stands_draws_twice() {
        // The declared number IS the doubling point, which is what gives it a meaning a reader can
        // check.
        assert_eq!(crowding(0.0, 40.0), 1.0);
        assert_eq!(crowding(40.0, 40.0), 2.0);
        assert_eq!(crowding(400.0, 40.0), 11.0);
    }

    #[test]
    fn the_standing_area_of_a_region_nobody_built_in_is_nothing() {
        assert_eq!(standing_in(&[7.0, 3.0], RegionId::at(1)), 3.0);
        assert_eq!(standing_in(&[7.0, 3.0], RegionId::at(9)), 0.0);
    }
}
