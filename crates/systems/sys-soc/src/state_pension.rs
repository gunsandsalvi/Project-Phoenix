//! The state pension at the opening: each adult who has reached the country's pension age for its sex draws the state
//! pension at its sex's coverage — the rule's flat amount, the replacement rate of the labour share's mean wage per
//! worker — paid monthly by the treasury, a person's row on the country's state pension line.

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
use phx_num::{Missing, Money, violation};
use phx_rand::{Subject, open_unit};

use crate::consts::{AGE_PARTS, MONTHS_PER_YEAR, PERCENT, SHARE_PARTS};

declare_stream! { pub PensionStream = "SOC.opening_pensions" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// The state pension: the treasury owes it to each pensioner, one member a pensioner.
pub const STATE_PENSION: LineKindDecl = LineKindDecl {
    name: "state pension",
    asset: SideDecl {
        holder_kinds: &[if_pop::HOUSEHOLD, phx_core::ESTATE_KIND.name],
        words: BALANCE,
        holder_list: false,
        holder_roles: &[if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name],
        exclusive: true,
    },
    liability: SideDecl {
        holder_kinds: &["treasury"],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
    },
    dated: true,
    transfer_requesters: &["SOC"],
};

const TREASURIES: &str = "CB.treasury";

/// The state pension line's kind in the books.
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "social protection declarations"
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
        books::of(opening).ledger.lines.declare_money(STATE_PENSION);
    }
}

/// Social protection's draw of the state pensions in payment.
#[derive(Debug)]
pub struct StatePension {
    pub age: Prim<Table1>,
    pub replacement: Prim<Table1>,
    pub coverage: Prim<Table1>,
}

/// One country's state pension as its pensioners draw it: by sex, the age it starts at, the share covered, and the
/// pension's line.
struct Country {
    year: i32,
    age: [f64; 2],
    coverage: [f64; 2],
    line: [LineSpec; 2],
}

fn by_sex(table: &Table1, parts: f64) -> [f64; 2] {
    [if_pop::FEMALE, if_pop::MALE].map(|sex| {
        let Ok(v) = table.at(i64::from(sex)) else { violation!(clause = "GEN.2", "no value for a sex", sex = sex) };
        phx_rand::float::from_i64(v) / parts
    })
}

impl AttachmentDraw for StatePension {
    #[clause("GEN.2", "SOC.3")]
    fn country(
        &self,
        books: &mut Books,
        register: &Register,
        (calendar, today): (&phx_core::Calendar, Day),
        c: &OpeningCountry,
    ) -> Box<dyn CountryAttachments> {
        let Some(&[(treasury, _)]) = books.drawn.get(&key(TREASURIES, c.id)).map(Vec::as_slice) else {
            violation!(clause = "GEN.3", "the state pension read before one treasury is drawn", country = c.id.get());
        };
        let adults = 1.0 - derived(c, "GEN.share_under_15") / PERCENT;
        let workers = phx_rand::float::from_u64(c.people) * adults * derived(c, "GEN.employment_rate") / PERCENT;
        let wage = derived(c, "GEN.labour_share") / PERCENT * c.gdp / workers / MONTHS_PER_YEAR;
        let ccy = currency(c.id);
        let date = calendar.date(today);
        let dates = monthly(date, c.id);
        let kind = books.ledger.lines.kind_index(STATE_PENSION.name);
        let replacement = by_sex(self.replacement.get(register, c.id), SHARE_PARTS);
        let line = replacement.map(|rate| {
            let amount = Money::new(whole(rate * wage), ccy);
            let terms: TermsId = books.ledger.terms.intern(plain_terms(
                ccy,
                vec![Leg::FixedAmount(amount)],
                Schedule { dates, count: Missing::Absent },
            ));
            LineSpec {
                kind,
                terms,
                counterparty: Missing::Present(treasury),
                first: Missing::Present((dates.nth(calendar, 1), 1)),
            }
        });
        Box::new(Country {
            year: date.year(),
            age: by_sex(self.age.get(register, c.id), AGE_PARTS),
            coverage: by_sex(self.coverage.get(register, c.id), SHARE_PARTS),
            line,
        })
    }
}

impl CountryAttachments for Country {
    #[clause("GEN.2", "SOC.3")]
    fn draw(
        &mut self,
        _: &mut Books,
        h: Drawing<'_>,
        (ctx, subject): (&OpeningCtx<'_>, Subject),
        rows: &mut Vec<DrawnRow>,
        _: &mut Vec<(&'static str, u32)>,
    ) {
        let mut d = ctx.draws(&PensionStream::DECL, subject);
        let adult_roles = [if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name];
        for (place, p) in h.household.persons.iter().enumerate().filter(|(_, p)| adult_roles.contains(&p.role)) {
            let Some(life) = p.values.iter().find(|(g, _)| if_pop::LIFE_GROUPS.contains(g)).map(|(_, v)| *v) else {
                violation!(clause = "REP.26", "an adult with no life value");
            };
            let covered = open_unit(&mut d);
            let sex = phx_core::component(if_pop::LIFE, life, if_pop::SEX_AT);
            let born = phx_core::component(if_pop::LIFE, life, if_pop::BIRTH_YEAR_AT);
            let Ok(born) = i32::try_from(born) else { violation!(clause = "REP.25", "a birth year beyond the calendar") };
            let age = phx_rand::float::from_i64(i64::from(self.year - if_pop::consts::FIRST_BIRTH_YEAR - born));
            let at = match sex {
                if_pop::FEMALE => 0,
                if_pop::MALE => 1,
                _ => violation!(clause = "REP.26", "a sex beyond the two", sex = sex),
            };
            let (Some(start), Some(share), Some(pension)) = (self.age.get(at), self.coverage.get(at), self.line.get(at))
            else {
                violation!(clause = "GEN.2", "a sex the state pension does not hold");
            };
            if age < *start || covered >= *share {
                continue;
            }
            rows.push(DrawnRow { line: *pension, side: Side::Asset, holder: Holder::Person(place), balance: Balance::None });
        }
    }

    fn pools(&self) -> Vec<(u32, i64)> {
        Vec::new()
    }

    fn counterparties(&self, _: &Books, _: &LineSpec) -> Vec<(PartyId, u64)> {
        Vec::new()
    }
}
