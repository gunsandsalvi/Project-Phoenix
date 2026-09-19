//! WHAT STANDS ON A PLACE, AND WHAT THAT DOES TO BUILDING THERE.
//!
//! @spec 33 A4 · 33 B1 · 40 A1.a · 40 B1.a · Law 2, Law 6, Law 8, Law 15, Law 19 · Appendix B
//!
//! The world had regions and nothing distinguished them. A firm built a mill for the same draw
//! whether the place was empty or packed, so there was no reason to build anywhere in particular, no
//! city, and nothing for rent to be about.
//!
//! It is congestion, not scarcity. Nothing here is fixed, nothing runs out and nothing is
//! refused: a place fills up and building there simply draws more. Law 6 is satisfied the strong way
//! — there is no cap, and what stops a firm building in a packed place is that it stops being worth
//! it, which is 33 B1 doing real work rather than a rule somebody wrote. A FIXED stock of ground
//! would have refused, and it would also have been an interpolation: the specification names no land,
//! no ground and no hectare anywhere in its 5,428 lines, and 33 A4's *limited by the scarcest* is
//! already met by `recipe::decide`, which walks capacity, each input and labour and names the one
//! that bound.
//!
//! How built-up a place is, is an AREA. It cannot be a count of units: units of dwellings added
//! to units of mills is adding numbers in different units, which is Law 8's own defect. Each line
//! that is a STRUCTURE declares what one unit of it stands on (`Registry::stands_on`), and that
//! footprint is what makes a warehouse and a flat comparable at all.
//!
//! Commercial, residential and industrial are one mechanism. Which lines are structures is
//! registry DATA, so nothing here branches on a class or a party kind: an office block, a
//! dwelling and a works differ by a number in a table and by nothing else.
//!
//! Nothing stores the density. It is a walk over the register every period (Appendix B: no stored
//! aggregate), bucketed by the region its holder is in — because a structure is where its owner is.

use crate::ids::{InstrumentId, RegionId};
use crate::parties::Parties;
use crate::register::Register;
use crate::registry::Registry;

/// How much ground is covered in each region, as a read over the holdings — one pass, bucketed
/// by region, never a number anybody keeps.
///
/// It is the register's own rows, not a tally maintained beside them. The result is indexed
/// by region row, and a region with nothing standing in it is `0.0` — which here really is zero
/// ground covered rather than a missing answer, because the walk visited every holding.
pub fn built_up(parties: &Parties, register: &Register, registry: &Registry) -> Vec<f64> {
    let mut by_region = vec![0.0; registry.regions()];
    for row in register.all() {
        let line = register.instrument_of(row);
        // A line with no footprint is not a structure — it is flour. That is an answer,
        // not a missing number.
        let Some(footprint) = registry.footprint_of(line) else { continue };
        let holder = register.holder_of(row);
        if !parties.alive(holder) {
            continue;
        }
        // 40 A1.a: a structure is somewhere, and where it is, is where its owner is. A dwelling's
        // location is part of its identity and never a free choice of the reader.
        let at = parties.region_of(holder).0 as usize;
        if at < by_region.len() {
            by_region[at] += register.quantity(row) * footprint;
        }
    }
    by_region
}

/// WHAT A BUILD DRAWS WHERE THIS MUCH ALREADY STANDS, as a multiple of what the same build draws
/// on empty ground.
///
/// `crowds_at` is the standing area at which a build draws TWICE — one TECHNOLOGY primitive
/// about building, with a real unit, and the only declared number in this mechanism. Everything else
/// is an OUTCOME: the density is a read, and what the extra draw COSTS is whatever its inputs cleared
/// at in their own books. No money number is set anywhere here, which is the whole reason
/// this is a draw and not a price.
///
/// Unbounded above and below one. It never refuses a build and it never caps a cost — it is
/// deeper foundations, longer haulage and more hours on a tight site, and a firm that finds the draw
/// not worth it does not build, which is a decision rather than a limit.
pub fn crowding(standing: f64, crowds_at: f64) -> f64 {
    assert!(
        crowds_at > 0.0,
        "21i: a world that is fully built at nothing standing is not a world"
    );
    assert!(standing >= 0.0, "Law 8: {standing} km² of ground covered is not an area");
    1.0 + standing / crowds_at
}

/// Whether this line is a structure at all — what a builder asks before asking where. A line that
/// stands on nothing is built the same everywhere, which is what being flour means.
pub fn is_a_structure(registry: &Registry, line: InstrumentId) -> bool {
    registry.footprint_of(line).is_some()
}

/// The standing area in one region, out of a `built_up` walk. A region the walk did not reach has
/// nothing standing in it.
pub fn standing_in(built: &[f64], at: RegionId) -> f64 {
    match built.get(at.0 as usize) {
        Some(km2) => *km2,
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{PartyId, UnitId};
    use crate::instruments::{Class, Instruments};
    use crate::parties::Representation;

    #[test]
    fn a_place_fills_up_in_area_and_not_in_units() {
        // A dwelling and a mill are counted in different things, so what they have in common
        // is the ground they cover. Two mills at 3 km² and ten flats at 0.1 km² is 7 km² — a number
        // a count could not have produced.
        let mut parties = Parties::new();
        let mut instruments = Instruments::new();
        let mut registry = Registry::new();
        let mut register = Register::new();

        let cb = parties.add(0, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let usd = registry.currency(cb);
        let us = registry.country(usd);
        let here = registry.region(us);
        let elsewhere = registry.region(us);

        let mill = instruments.issue(cb, usd, Class::Plant, UnitId::at(0), None, None);
        let flat = instruments.issue(cb, usd, Class::Plant, UnitId::at(0), None, None);
        let flour = instruments.issue(cb, usd, Class::Good, UnitId::at(1), None, None);
        registry.stands_on(mill, 3.0);
        registry.stands_on(flat, 0.1);

        let firm = parties.add(1, here, cb, Representation::Named, 1, 0);
        let away = parties.add(1, elsewhere, cb, Representation::Named, 1, 0);
        register.credit(firm, mill, 2.0, 100.0, 0);
        register.credit(firm, flat, 10.0, 20.0, 0);
        // Flour covers no ground, however much of it there is.
        register.credit(firm, flour, 50_000.0, 1.0, 0);
        register.credit(away, mill, 1.0, 100.0, 0);

        let built = built_up(&parties, &register, &registry);
        assert_eq!(standing_in(&built, here), 7.0);
        assert_eq!(standing_in(&built, elsewhere), 3.0);
    }

    #[test]
    fn building_where_this_much_stands_draws_twice() {
        // The declared number IS the doubling point, which is what gives it a meaning a reader can
        // check. Law 6: it keeps rising past it — nothing is capped and nothing is refused.
        assert_eq!(crowding(0.0, 40.0), 1.0);
        assert_eq!(crowding(40.0, 40.0), 2.0);
        assert_eq!(crowding(400.0, 40.0), 11.0);
    }

    #[test]
    fn a_structure_is_a_line_with_a_footprint_and_not_a_class() {
        // What makes a line a structure is a number in the registry, so an office block, a
        // dwelling and a works go through one mechanism and nothing branches on which.
        let mut parties = Parties::new();
        let mut instruments = Instruments::new();
        let mut registry = Registry::new();
        let cb = parties.add(0, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let usd = registry.currency(cb);
        // A dwelling is a GOOD here and a works is PLANT, and both are structures.
        let dwelling = instruments.issue(cb, usd, Class::Good, UnitId::at(0), None, None);
        let works = instruments.issue(cb, usd, Class::Plant, UnitId::at(0), None, None);
        let wheat = instruments.issue(cb, usd, Class::Good, UnitId::at(1), None, None);
        registry.stands_on(dwelling, 0.05);
        registry.stands_on(works, 12.0);
        assert!(is_a_structure(&registry, dwelling));
        assert!(is_a_structure(&registry, works));
        assert!(!is_a_structure(&registry, wheat));
    }

    #[test]
    #[should_panic(expected = "stands on nothing")]
    fn a_structure_that_occupies_nowhere_is_not_one() {
        let mut parties = Parties::new();
        let mut instruments = Instruments::new();
        let mut registry = Registry::new();
        let cb = parties.add(0, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let usd = registry.currency(cb);
        let line = instruments.issue(cb, usd, Class::Plant, UnitId::at(0), None, None);
        registry.stands_on(line, 0.0);
    }
}
