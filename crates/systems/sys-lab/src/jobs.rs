//! The households' jobs at the opening: each adult is employed at the country's employment rate, and an employee at
//! its sex's share of employees among the employed; its wage is the mean wage the labour share gives, times its
//! household's income as a multiple of the mean, on the nearest wage point. A job is a row on the employment line of
//! its wage point, whose employers are the country's firms, large and small, apportioned by their headcounts once
//! every household is drawn.

use std::collections::BTreeMap;

use phx_core::register::values::Table2;
use phx_core::{
    Contribution, DECLARATIONS, Opening, OpeningCountry, OpeningCtx, OpeningPhase, Prim, Register, StreamDef,
    declare_stream,
};
use phx_id::{Day, PartyId};
use phx_ledger::algebra::{Leg, Schedule, Side};
use phx_ledger::attachments::{AttachmentDraw, Balance, CountryAttachments, Drawing, DrawnRow, Holder, LineSpec};
use phx_ledger::books::{self, Books};
use phx_ledger::line::{LineKindDecl, SideDecl};
use phx_ledger::opening::{currency, derived, key, whole};
use phx_ledger::rows::BALANCE;
use phx_ledger::terms::TermsId;
use phx_macros::clause;
use phx_num::{Fixed, Missing, Money, violation};
use phx_rand::{Subject, open_unit};

use crate::consts::{EMPLOYEES, MONTHS_PER_YEAR, PERCENT, SHARE_PARTS};

declare_stream! { pub JobsStream = "LAB.opening_jobs" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// Employment: the employer owes the wage to the employee, each job a member; many employers and many employees on
/// a line, so it records no pairing.
pub const EMPLOYMENT: LineKindDecl = LineKindDecl {
    name: "employment",
    asset: SideDecl {
        holder_kinds: &[if_pop::HOUSEHOLD, phx_core::ESTATE_KIND.name],
        words: BALANCE,
        holder_list: false,
        holder_roles: &[if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name],
        exclusive: true,
        many: false,
    },
    liability: SideDecl {
        holder_kinds: &["firm", "small_firm"],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: true,
    },
    dated: true,
    transfer_requesters: &["LAB"],
};

const FIRMS: &str = "FRM.firms";
const SMALL_FIRMS: &str = "FRM.small_firms";

/// The employment line's kind in the books.
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "labour declarations"
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
        books::of(opening).ledger.lines.declare_money(EMPLOYMENT);
    }
}

/// Labour's draw of the households' jobs.
#[derive(Debug)]
pub struct Jobs {
    pub status: Prim<Table2>,
    pub ratio: Prim<Fixed<6>>,
}

/// One country's labour as the households draw it.
struct Country {
    employed: f64,
    employees: [f64; 2],
    wage: f64,
    ratio: f64,
    kind: u16,
    schedule: Schedule,
    first: Missing<(Day, u32)>,
    ccy: phx_num::Ccy,
    terms: BTreeMap<i64, TermsId>,
    firms: Vec<(PartyId, u64)>,
}

impl AttachmentDraw for Jobs {
    #[clause("GEN.2", "LAB.1")]
    fn country(
        &self,
        books: &mut Books,
        register: &Register,
        (calendar, today): (&phx_core::Calendar, Day),
        c: &OpeningCountry,
    ) -> Box<dyn CountryAttachments> {
        let table = self.status.get(register, c.id);
        let employees = [if_pop::FEMALE, if_pop::MALE].map(|sex| {
            let Ok(v) = table.at(EMPLOYEES, i64::from(sex)) else {
                violation!(clause = "GEN.2", "no share of employees for a sex", sex = sex);
            };
            phx_rand::float::from_i64(v) / SHARE_PARTS
        });
        let adults = 1.0 - derived(c, "GEN.share_under_15") / PERCENT;
        let employed = derived(c, "GEN.employment_rate") / PERCENT;
        let workers = phx_rand::float::from_u64(c.people) * adults * employed;
        let wage = derived(c, "GEN.labour_share") / PERCENT * c.gdp / workers / MONTHS_PER_YEAR;
        let (Some(large), Some(small)) = (books.drawn.get(&key(FIRMS, c.id)), books.drawn.get(&key(SMALL_FIRMS, c.id)))
        else {
            violation!(clause = "GEN.3", "labour's opening reading firms not yet drawn", country = c.id.get());
        };
        let firms: Vec<_> = large.iter().chain(small).copied().collect();
        let date = calendar.date(today);
        let dates = phx_ledger::opening::monthly(date, c.id);
        let first = Missing::Present((dates.nth(calendar, 1), 1));
        Box::new(Country {
            employed,
            employees,
            wage,
            ratio: self.ratio.shared(register).to_f64(),
            kind: books.ledger.lines.kind_index(EMPLOYMENT.name),
            schedule: Schedule { dates, count: Missing::Absent },
            first,
            ccy: currency(c.id),
            terms: BTreeMap::new(),
            firms,
        })
    }
}

impl Country {
    /// The terms of a wage point: its amount paid on each monthly date.
    fn terms(&mut self, books: &mut Books, point: i64) -> TermsId {
        if let Some(t) = self.terms.get(&point) {
            return *t;
        }
        let amount = whole(libm::pow(self.ratio, phx_rand::float::from_i64(point)));
        let t = books.ledger.terms.intern(phx_ledger::opening::plain_terms(
            self.ccy,
            vec![Leg::FixedAmount(Money::new(amount, self.ccy))],
            self.schedule,
        ));
        self.terms.insert(point, t);
        t
    }
}

impl CountryAttachments for Country {
    #[clause("GEN.2", "LAB.1", "REP.34")]
    fn draw(
        &mut self,
        books: &mut Books,
        h: Drawing<'_>,
        (ctx, subject): (&OpeningCtx<'_>, Subject),
        rows: &mut Vec<DrawnRow>,
        _: &mut Vec<(&'static str, u32)>,
    ) {
        let mut d = ctx.draws(&JobsStream::DECL, subject);
        let adult_roles = [if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name];
        for (place, p) in h.household.persons.iter().enumerate().filter(|(_, p)| adult_roles.contains(&p.role)) {
            let Some(life) = p.values.iter().find(|(g, _)| if_pop::LIFE_GROUPS.contains(g)).map(|(_, v)| *v) else {
                violation!(clause = "REP.26", "an adult with no life value");
            };
            let sex = phx_core::component(if_pop::LIFE, life, if_pop::SEX_AT);
            let [female, male] = self.employees;
            let employee = match sex {
                if_pop::FEMALE => female,
                if_pop::MALE => male,
                _ => violation!(clause = "REP.26", "a sex beyond the two", sex = sex),
            };
            let (works, as_employee) = (open_unit(&mut d), open_unit(&mut d));
            if works >= self.employed || as_employee >= employee {
                continue;
            }
            let wage = self.wage * h.income;
            let point = phx_rand::float::floor_to_i64(libm::rint(libm::log(wage) / libm::log(self.ratio)));
            let Some(point) = point else { violation!(clause = "REP.34", "a wage beyond the wage points") };
            let terms = self.terms(books, point);
            rows.push(DrawnRow {
                line: LineSpec { kind: self.kind, terms, counterparty: Missing::Absent, first: self.first },
                side: Side::Asset,
                holder: Holder::Person(place),
                balance: Balance::None,
            });
        }
    }

    fn pools(&self) -> Vec<(u32, i64)> {
        Vec::new()
    }

    fn counterparties(&self, _: &Books, line: &LineSpec) -> Vec<(PartyId, u64)> {
        if line.kind == self.kind { self.firms.clone() } else { Vec::new() }
    }
}
