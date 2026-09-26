//! The households' tenancies at the opening: a household owns its home at the country's home ownership, and rents it
//! otherwise, paying the country's median rent burden of its income — its persons' share of the country's labour
//! income, times its income as a multiple of the mean — on the nearest rent point. A tenancy is a row on the tenancy
//! line of its rent point; its landlords stand in the country's firms by their plant, apportioned once every
//! household is drawn, until the dwelling stock and its owners are drawn.

use std::collections::BTreeMap;

use phx_core::register::values::Table1;
use phx_core::{
    Contribution, DECLARATIONS, Opening, OpeningCountry, OpeningCtx, OpeningPhase, Prim, Register, StreamDef,
    declare_stream,
};
use phx_id::{Day, PartyId};
use phx_ledger::algebra::{Leg, Schedule, Side};
use phx_ledger::attachments::{AttachmentDraw, Balance, CountryAttachments, Drawing, DrawnRow, Holder, LineSpec};
use phx_ledger::books::{self, Books};
use phx_ledger::line::{LineKindDecl, SideDecl};
use phx_ledger::opening::{currency, derived, key, monthly, plain_terms, whole};
use phx_ledger::rows::BALANCE;
use phx_ledger::terms::TermsId;
use phx_macros::clause;
use phx_num::{Fixed, Missing, Money, violation};
use phx_rand::{Subject, open_unit};

use crate::consts::{MONTHS_PER_YEAR, PERCENT, RENT_BURDEN, SHARE_PARTS};

declare_stream! { pub TenancyStream = "HSG.opening_tenancies" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// A tenancy: the tenant owes the rent to the landlord, each household a member; many landlords and many tenants on
/// a line, so it records no pairing.
pub const TENANCY: LineKindDecl = LineKindDecl {
    name: "tenancy",
    asset: SideDecl {
        holder_kinds: &["firm", phx_core::ESTATE_KIND.name],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: true,
    },
    liability: SideDecl {
        holder_kinds: &[if_pop::HOUSEHOLD, phx_core::ESTATE_KIND.name],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: true,
        many: false,
    },
    dated: true,
    transfer_requesters: &["HSG"],
};

/// The large firms' structures, whose owners stand in for the landlords.
const PLANT: &str = "CAP.structures";

/// The tenancy line's kind in the books.
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "housing declarations"
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
        books::of(opening).ledger.lines.declare_money(TENANCY);
    }
}

/// Housing's draw of the households' tenancies.
#[derive(Debug)]
pub struct Tenancies {
    pub tenure: Prim<Table1>,
    pub ratio: Prim<Fixed<6>>,
}

/// One country's housing as the households draw it.
struct Country {
    owners: f64,
    burden: f64,
    income: f64,
    ratio: f64,
    kind: u16,
    schedule: Schedule,
    first: Missing<(Day, u32)>,
    ccy: phx_num::Ccy,
    terms: BTreeMap<i64, TermsId>,
    landlords: Vec<(PartyId, u64)>,
}

impl AttachmentDraw for Tenancies {
    #[clause("GEN.2", "HSG.2")]
    fn country(
        &self,
        books: &mut Books,
        register: &Register,
        (calendar, today): (&phx_core::Calendar, Day),
        c: &OpeningCountry,
    ) -> Box<dyn CountryAttachments> {
        let Ok(burden) = self.tenure.get(register, c.id).at(RENT_BURDEN) else {
            violation!(clause = "GEN.2", "no rent burden for the country", country = c.id.get());
        };
        let Some(landlords) = books.drawn.get(&key(PLANT, c.id)).cloned() else {
            violation!(clause = "GEN.3", "housing's opening reading firms not yet drawn", country = c.id.get());
        };
        let per_person = derived(c, "GEN.labour_share") / PERCENT * c.gdp / phx_rand::float::from_u64(c.people);
        let dates = monthly(calendar.date(today), c.id);
        Box::new(Country {
            owners: derived(c, "GEN.home_ownership"),
            burden: phx_rand::float::from_i64(burden) / SHARE_PARTS,
            income: per_person / MONTHS_PER_YEAR,
            ratio: self.ratio.shared(register).to_f64(),
            kind: books.ledger.lines.kind_index(TENANCY.name),
            schedule: Schedule { dates, count: Missing::Absent },
            first: Missing::Present((dates.nth(calendar, 1), 1)),
            ccy: currency(c.id),
            terms: BTreeMap::new(),
            landlords,
        })
    }
}

impl Country {
    /// The terms of a rent point: its amount paid on each monthly date.
    fn terms(&mut self, books: &mut Books, point: i64) -> TermsId {
        if let Some(t) = self.terms.get(&point) {
            return *t;
        }
        let amount = whole(libm::pow(self.ratio, phx_rand::float::from_i64(point)));
        let t = books.ledger.terms.intern(plain_terms(
            self.ccy,
            vec![Leg::FixedAmount(Money::new(amount, self.ccy))],
            self.schedule,
        ));
        self.terms.insert(point, t);
        t
    }
}

impl CountryAttachments for Country {
    #[clause("GEN.2", "HSG.2", "REP.34")]
    fn draw(
        &mut self,
        books: &mut Books,
        h: Drawing<'_>,
        (ctx, subject): (&OpeningCtx<'_>, Subject),
        rows: &mut Vec<DrawnRow>,
        _: &mut phx_ledger::attachments::Keys,
    ) {
        let mut d = ctx.draws(&TenancyStream::DECL, subject);
        if open_unit(&mut d) < self.owners {
            return;
        }
        let persons = phx_rand::float::len_u64(h.household.persons.len());
        let rent = self.burden * self.income * phx_rand::float::from_u64(persons) * h.income;
        let Some(point) = phx_rand::float::floor_to_i64(libm::rint(libm::log(rent) / libm::log(self.ratio))) else {
            violation!(clause = "REP.34", "a rent beyond the rent points");
        };
        let terms = self.terms(books, point);
        rows.push(DrawnRow {
            line: LineSpec { kind: self.kind, terms, counterparty: Missing::Absent, first: self.first },
            side: Side::Liability,
            holder: Holder::Household,
            balance: Balance::None,
        });
    }

    fn pools(&self) -> Vec<(u32, i64)> {
        Vec::new()
    }

    fn counterparties(&mut self, _: &Books, line: &LineSpec, _: &mut phx_rand::Draws) -> Vec<(PartyId, u64)> {
        if line.kind == self.kind { self.landlords.clone() } else { Vec::new() }
    }
}
