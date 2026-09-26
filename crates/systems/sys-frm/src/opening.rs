use phx_core::{Contribution, FactDef, Opening, OpeningCountry, OpeningPhase, PARTIES, StreamDef, opening_subject};
use phx_id::{CountryId, PartyId};
use phx_ledger::books;
use phx_ledger::opening::{derived, key};
use phx_macros::clause;
use phx_num::violation;
use phx_rand::float::{floor_to_u64, from_u64};
use phx_rand::open_unit;

use crate::consts::{INDUSTRIES, MILLION, PERCENT, PURPOSES, SITES, SIZES};
use crate::industry::Industries;
use crate::{CountPrim, FIRM, FixedPrim, OpeningStream, TablePrim};

const FIRMS: &str = "FRM.firms";
/// Each large firm's region, as the jobs' opening places its employees.
pub const REGIONS: &str = "FRM.firm_regions";
/// Each large firm's industry, as the jobs' opening weighs the occupations it employs.
pub const INDUSTRIES_DRAWN: &str = "FRM.firm_industries";

/// The primitives the firms' opening reads.
#[derive(Clone, Copy, Debug)]
pub struct Prims {
    pub firms_per_employed: FixedPrim,
    pub size_exponent: FixedPrim,
    pub rank: CountPrim,
    pub deposit_share: FixedPrim,
    pub industries: TablePrim,
}

/// The country's firms by industry and size, as its data declares them.
pub(crate) fn industries(table: TablePrim, register: &phx_core::Register, c: &OpeningCountry) -> Industries {
    match Industries::of(table.get(register, c.id)) {
        Ok(i) => i,
        Err(_) => {
            violation!(clause = "GEN.2", "a country's firms by industry that are no spread", country = c.id.get())
        }
    }
}

fn subject(country: CountryId, purpose: u32, ordinal: u32) -> phx_rand::Subject {
    opening_subject(u32::from(country.get()) * PURPOSES + purpose, ordinal)
}

/// The sizes, in persons employed, of the `rank` largest of `firms` firms whose sizes follow a Pareto law of exponent
/// `alpha` from one person up, largest first: the top order statistics drawn one after another, each the one before
/// times a uniform to the power of one over the firms left, in logs so the largest keep their precision.
#[clause("GEN.2")]
#[must_use]
pub fn largest(firms: u64, rank: u64, alpha: f64, draws: &mut phx_rand::Draws) -> Vec<u64> {
    if rank > firms {
        violation!(clause = "REP.2", "a promotion rank beyond the country's firms", rank = rank, firms = firms);
    }
    let mut log_cdf = 0.0_f64;
    (0..rank)
        .map(|k| {
            log_cdf += open_unit(draws).ln() / from_u64(firms - k);
            let size = (-log_cdf.exp_m1()).powf(-1.0 / alpha);
            let Some(n) = floor_to_u64(size) else {
                violation!(clause = "GEN.2", "a firm's size beyond counting", rank = k);
            };
            n
        })
        .collect()
}

/// Each country's largest firms: the persons employed, the firms the law's scale gives them, and the largest down to
/// the promotion rank, drawn by size, sited in the country and each in an industry drawn by its size. Their debt, deposits and plant are drawn with the small
/// firms', over every firm.
#[clause("GEN.2", "GEN.3", "REP.2", "PTY.9")]
#[derive(Debug)]
pub struct Parties {
    pub prims: Prims,
}

impl Parties {
    fn open_country(&self, opening: &mut Opening<'_>, c: &OpeningCountry) {
        let (register, day) = (opening.register, opening.day);
        let p = self.prims;
        let adults = 1.0 - derived(c, "GEN.share_under_15") / PERCENT;
        let employed = from_u64(c.people) * adults * derived(c, "GEN.employment_rate") / PERCENT;
        let Some(firms) = floor_to_u64(employed * p.firms_per_employed.get(register, c.id).to_f64()) else {
            violation!(clause = "GEN.2", "a country's firms beyond counting", country = c.id.get());
        };
        let rank = p.rank.shared(register).get() * c.people / MILLION;
        let alpha = p.size_exponent.shared(register).to_f64();
        let mut draws = opening.ctx.draws(&OpeningStream::DECL, subject(c.id, SIZES, 0));
        let sizes = largest(firms, rank, alpha, &mut draws);
        let by_size = industries(p.industries, register, c);
        let industry = <if_firm::known::Industry as FactDef>::ITEM.name;
        let mut headcounts: Vec<(PartyId, u64)> = Vec::with_capacity(sizes.len());
        let mut regions: Vec<(PartyId, u64)> = Vec::with_capacity(sizes.len());
        let mut industries: Vec<(PartyId, u64)> = Vec::with_capacity(sizes.len());
        for (ordinal, size) in (0_u32..).zip(&sizes) {
            let mut at = opening.ctx.draws(&OpeningStream::DECL, subject(c.id, SITES, ordinal));
            let site = c.site(&mut at);
            let mut lot = opening.ctx.draws(&OpeningStream::DECL, subject(c.id, INDUSTRIES, ordinal));
            let trade = by_size.draw(*size, &mut lot);
            let parties = &mut books::of(opening).parties;
            let firm = parties.begin(FIRM.name, site, day);
            parties.open_fact(firm, industry, i64::from(trade));
            headcounts.push((firm, *size));
            let Some(region) = c.regions.iter().find(|(_, tiles)| tiles.contains(&site)).map(|(r, _)| *r) else {
                violation!(clause = "PTY.5", "a firm sited in no region of its country", country = c.id.get());
            };
            regions.push((firm, u64::from(region)));
            industries.push((firm, u64::from(trade)));
        }
        let (b, report) = books::split(opening);
        b.drawn.insert(key(FIRMS, c.id), headcounts);
        b.drawn.insert(key(REGIONS, c.id), regions);
        b.drawn.insert(key(INDUSTRIES_DRAWN, c.id), industries);
        report.distributions.push((
            key(FIRMS, c.id),
            format!(
                "the {rank} largest of {firms} firms (FRM.firms_per_employed over {employed:.0} employed) by a Pareto \
                 law of exponent {alpha} (FRM.size_exponent, Axtell 2001), each in an industry drawn by its size \
                 (FRM.industry_by_size)"
            ),
        ));
    }
}

impl Contribution for Parties {
    fn name(&self) -> &'static str {
        "large firms"
    }
    fn phase(&self) -> OpeningPhase {
        PARTIES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[FIRMS]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[FIRMS]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let countries = opening.countries;
        for c in countries {
            self.open_country(opening, c);
        }
    }
}
