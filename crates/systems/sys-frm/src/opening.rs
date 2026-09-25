use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::{
    Contribution, DECLARATIONS, Opening, OpeningCountry, OpeningPhase, PARTIES, PHYSICAL_STOCK, StreamDef,
    opening_subject,
};
use phx_id::{CountryId, PartyId};
use phx_ledger::algebra::Terms;
use phx_ledger::books::{self, Books};
use phx_ledger::instruction::{Effect, ReasonDecl, ReasonId};
use phx_ledger::instrument::{InstrumentFamily, NewInstrument};
use phx_ledger::opening::{currency, derived, hold, key};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::float::{floor_to_u64, from_u64};
use phx_rand::open_unit;

use crate::consts::{MILLION, PERCENT, PURPOSES, SITES, SIZES};
use crate::{CountPrim, FIRM, FixedPrim, OpeningStream};

const FIRMS: &str = "FRM.firms";
const PLANT: &str = "FRM.plant";

/// The primitives the firms' opening reads.
#[derive(Clone, Copy, Debug)]
pub struct Prims {
    pub firms_per_employed: FixedPrim,
    pub size_exponent: FixedPrim,
    pub rank: CountPrim,
    pub deposit_share: FixedPrim,
    pub depreciation: FixedPrim,
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
/// the promotion rank, drawn by size and sited in the country. Their debt, deposits and plant are drawn with the small
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
        let mut headcounts: Vec<(PartyId, u64)> = Vec::with_capacity(sizes.len());
        for (ordinal, size) in (0_u32..).zip(&sizes) {
            let mut at = opening.ctx.draws(&OpeningStream::DECL, subject(c.id, SITES, ordinal));
            let site = c.site(&mut at);
            let firm = books::of(opening).parties.begin(FIRM.name, site, day);
            headcounts.push((firm, *size));
        }
        let (b, report) = books::split(opening);
        b.drawn.insert(key(FIRMS, c.id), headcounts);
        report.distributions.push((
            key(FIRMS, c.id),
            format!(
                "the {rank} largest of {firms} firms (FRM.firms_per_employed over {employed:.0} employed) by a Pareto \
                 law of exponent {alpha} (FRM.size_exponent, Axtell 2001)"
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

/// What the opening's instructions are for: capital on both sides, since they open the books.
const REASON: ReasonDecl = ReasonDecl { name: "FRM opening", order: 0, paid: Effect::Equity, received: Effect::Equity };

fn reason(b: &Books) -> ReasonId {
    b.ledger.reasons.named(REASON.name)
}

/// The firms' declarations in the books: their opening's reason.
#[clause("FRM.1")]
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "firm declarations"
    }
    fn phase(&self) -> OpeningPhase {
        DECLARATIONS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
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
        let b = books::of(opening);
        let _ = b.ledger.reasons.declare(REASON);
    }
}

/// Each firm's plant: units of the country's plant, a physical class counted in its replacement value until the
/// classes of capital goods are declared, held as a lot at that value.
#[clause("GEN.5", "REG.9")]
#[derive(Debug)]
pub struct Plant;

impl Contribution for Plant {
    fn name(&self) -> &'static str {
        "plant"
    }
    fn phase(&self) -> OpeningPhase {
        PHYSICAL_STOCK
    }
    fn reads(&self) -> &'static [&'static str] {
        &[PLANT]
    }
    fn writes(&self) -> &'static [&'static str] {
        &["FRM.plant_held"]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &["FRM.plant_held"]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (countries, register, date) = (opening.countries, opening.register, opening.date);
        let Missing::Present(unit) = register.units().named("plant") else {
            violation!(clause = "NUM.3", "the world declares no unit of plant");
        };
        let Some(months) = Period::months(1) else { violation!(clause = "TIME.4", "a month that is no period") };
        let (b, report) = books::split(opening);
        let reason = reason(b);
        for c in countries {
            let ccy = currency(c.id);
            let dates = ScheduleDates {
                anchor: date,
                period: months,
                eom: EndOfMonth::Plain,
                convention: BusinessDayConvention::Following,
                country: c.id,
            };
            let terms = b.ledger.terms.intern(Terms::account(ccy, dates));
            let plant = b.ledger.instruments.issue(NewInstrument {
                family: InstrumentFamily::RealAsset,
                issuer: Missing::Absent,
                unit,
                ccy,
                terms,
            });
            let Some(held) = b.drawn.get(&key(PLANT, c.id)).cloned() else {
                violation!(clause = "GEN.3", "plant held by firms not drawn", country = c.id.get());
            };
            for (firm, value) in held {
                let Ok(v) = i64::try_from(value) else {
                    violation!(clause = "MON.16", "plant beyond whole units", firm = firm.get());
                };
                b.open(reason, vec![hold(firm, plant, v, unit, v, firm.get())], firm.get(), report);
            }
        }
    }
}
