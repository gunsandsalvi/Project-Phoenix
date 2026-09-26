//! The ways each opening firm knows: its industry's public ways in its country, a large firm's as a fact of its
//! row, a small firm's as its agent's attribute. Its own ways come later, by discovery, licence or imitation.

use if_base::IndustryId;
use if_firm::known::{Industry, KNOWN, Known as KnownFact};
use phx_core::{Contribution, FactDef, Opening, OpeningCountry, OpeningPhase, PHYSICAL_STOCK};
use phx_id::PartyId;
use phx_ledger::books::Books;
use phx_ledger::opening::key;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::population::Population;
use phx_store::SystemBacking;

use crate::prims::TecPrims;
use crate::technology::Technology;

/// The firms' draws the ways are given to: the large firms, and the small firms' agents.
const FIRMS: &str = "FRM.firms";
const SMALL_FIRMS: &str = "FRM.small_firms";
const SMALL_FIRM: &str = "small_firm";

/// Gives every opening firm its industry's public ways.
#[clause("TEC.4")]
#[derive(Debug)]
pub struct Known {
    pub prims: TecPrims,
}

fn drawn(b: &Books, name: &str, c: &OpeningCountry) -> Vec<(PartyId, u64)> {
    let Some(list) = b.drawn.get(&key(name, c.id)) else {
        violation!(clause = "GEN.3", "firms read before they are drawn", country = c.id.get());
    };
    list.clone()
}

/// The public set of an industry in a country, as the set's identity a firm keeps.
fn public(tech: &Technology, c: &OpeningCountry, industry: i64) -> u32 {
    let set = u16::try_from(industry).ok().and_then(|i| tech.public(c.id, IndustryId::new(i)));
    let Some(set) = set else {
        violation!(clause = "TEC.4", "a firm of an industry the products do not declare", industry = industry);
    };
    set.index()
}

/// Firms counted in their industry.
fn tally(by: &mut [u64], industry: i64, firms: u64) {
    if let Some(x) = usize::try_from(industry).ok().and_then(|i| by.get_mut(i)) {
        *x += firms;
    }
}

impl Contribution for Known {
    fn name(&self) -> &'static str {
        "known ways"
    }
    fn phase(&self) -> OpeningPhase {
        PHYSICAL_STOCK
    }
    fn reads(&self) -> &'static [&'static str] {
        &[FIRMS, SMALL_FIRMS]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let Opening { register, countries, books, population, report, .. } = opening;
        let (Some(books), Some(population)) = (books.downcast_mut::<Books>(), population.downcast_mut::<Population>())
        else {
            violation!(clause = "GEN.3", "an opening handed something other than the world's books and population");
        };
        let Ok(tech) = Technology::compile(&self.prims, register, countries.len()) else {
            violation!(clause = "TEC.2", "a technology the assembly let through that does not compile");
        };
        let Some(at) = population.kinds.iter().position(|k| k.decl.kind == SMALL_FIRM) else {
            violation!(clause = "FRM.23", "a world that keeps no small firm kind");
        };
        let Some(kd) = population.kinds.get(at) else { violation!(clause = "FRM.23", "a kind beyond the world's") };
        let (Some(industry_at), Some(known_at)) =
            (kd.decl.attr(if_firm::known::INDUSTRY.name), kd.decl.attr(KNOWN.name))
        else {
            violation!(clause = "REP.41", "a small firm kind without its industry or its known ways");
        };
        let (industry, known) = (<Industry as FactDef>::ITEM.name, <KnownFact as FactDef>::ITEM.name);
        let industries = tech.products.industries.len();
        for c in *countries {
            let (mut large, mut small) = (vec![0_u64; industries], vec![0_u64; industries]);
            for (firm, _) in drawn(books, FIRMS, c) {
                let Missing::Present(i) = books.parties.fact(firm, industry) else {
                    violation!(clause = "TEC.4", "a firm with no industry", firm = firm.get());
                };
                books.parties.open_fact(firm, known, i64::from(public(&tech, c, i)));
                tally(&mut large, i, 1);
            }
            let first = books.parties.first_cell_place();
            for (firm, _) in drawn(books, SMALL_FIRMS, c) {
                let (place, slot) = books.parties.row(firm);
                let Some(table) = place.checked_sub(first).map(usize::from) else {
                    violation!(clause = "FRM.23", "a small firm that is no agent", firm = firm.get());
                };
                let (tables, _, _) = books.parties.cells_mut();
                let agents = Population::table_mut::<SystemBacking>(tables, table);
                let i = i64::from(agents.attr(slot, industry_at));
                agents.set_attr(slot, known_at, public(&tech, c, i));
                tally(&mut small, i, u64::from(agents.multiplicity(slot).get()));
            }
            report.distributions.push((
                key(<KnownFact as FactDef>::ITEM.name, c.id),
                format!(
                    "country {}: each firm knows its industry's public ways; firms by industry, in the products' \
                     order of industries, large {large:?}, small {small:?}",
                    c.id.get()
                ),
            ));
        }
    }
}
