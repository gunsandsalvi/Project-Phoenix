//! The start of work at 4a: the hires accepted the day before join their lines, the layoffs whose notice has run and
//! the retired persons' jobs leave theirs, and each separation's severance is owed, paid at 7c. Also the employment
//! lines and the searching agents, read from the world when it opens or loads.

use if_labour::class;
use if_labour::kind::LabourKind;
use phx_core::SubStep;
use phx_id::{CountryId, Day, LineId, PartyId};
use phx_ledger::algebra::{Leg, Schedule, Side};
use phx_ledger::apply::ApplyAt;
use phx_ledger::instruction::Instruction;
use phx_macros::clause;
use phx_num::{Missing, Money, capacity_exceeded, violation};
use phx_pop::person::{Attachment, Holder};
use phx_pop::population::Population;
use phx_rand::{Subject, SubjectTag};
use phx_store::SystemBacking;

use super::book::{Hire, Owed, Separation};
use crate::world::World;

/// A person's attributes as their words hold them.
type Sets<'a> = &'a [(&'static str, u32)];

impl World {
    /// Whether a party is still the live party it was.
    pub(crate) fn live(&self, party: PartyId) -> bool {
        matches!(self.books.parties.directory().resolve(party), phx_core::Resolved::Live(p, _) if p == party)
    }

    /// The population kind whose persons hold the labour state.
    pub(crate) fn labour_kind_place(&self, kind: &LabourKind) -> Option<usize> {
        self.population.kinds.iter().position(|k| k.decl.person_attrs.iter().any(|f| f.decl.name == kind.state))
    }

    /// Each employment line by its terms, and the agents whose persons search, read from the world as it opens or
    /// loads.
    #[clause("LAB.1", "LAB.5")]
    pub(crate) fn labour_rebuild(&mut self) {
        let Some(kind) = self.labour.kind else { return };
        let k = self.books.ledger.lines.kind_index(kind.line);
        let lines = &self.books.ledger.lines;
        self.labour.lines = (0..lines.len())
            .filter_map(|i| u32::try_from(i).ok().map(LineId::new))
            .filter(|l| lines.kind_of(*l) == k)
            .map(|l| (lines.terms(l), l))
            .collect();
        let Some(place) = self.labour_kind_place(&kind) else { return };
        let Some(decl) = self.population.kinds.get(place).map(|k| k.decl.clone()) else { return };
        let table = Population::table::<SystemBacking>(self.books.parties.cells(), place);
        self.labour.searchers = table
            .slots()
            .filter(|s| {
                table
                    .persons(*s)
                    .iter()
                    .any(|w| phx_pop::person::unpack(&decl, *w).attr(kind.state) == Some(class::SEARCHING))
            })
            .map(|s| table.party(s))
            .collect();
    }

    /// An agent's person's attributes set, and the agent marked for its hazards to be drawn again.
    pub(crate) fn set_person(&mut self, party: PartyId, person: u32, sets: Sets<'_>) {
        let Some(kind) = self.labour.kind else { return };
        let Some(place) = self.labour_kind_place(&kind) else { return };
        let Some(decl) = self.population.kinds.get(place).map(|k| k.decl.clone()) else { return };
        let (_, slot) = self.books.parties.row(party);
        let table = Population::table_mut::<SystemBacking>(self.books.parties.cells_mut().0, place);
        let mut words = table.persons(slot).to_vec();
        let Some(w) = usize::try_from(person).ok().and_then(|i| words.get_mut(i)) else {
            violation!(clause = "REP.26", "a person set that its household does not hold", party = party.get());
        };
        let mut p = phx_pop::person::unpack(&decl, *w);
        for (name, v) in sets {
            p.set_attr(name, *v);
        }
        *w = phx_pop::person::pack(&decl, &p);
        table.set_persons(slot, &words);
        table.mark_changed(slot);
    }

    /// The employment line of a country's class and wage point, opened the first time a hire needs it: its wage paid
    /// monthly on the opening's dates.
    #[clause("LAB.1", "REP.3", "REP.34")]
    pub(crate) fn employment_line(&mut self, country: u8, class: &[u32], point: i64) -> LineId {
        let Some(kind) = self.labour.kind else {
            violation!(clause = "LAB.1", "an employment line in a world with no labour");
        };
        let law = super::law_of(&self.labour.laws, phx_id::CountryId::new(country));
        let amount = phx_ledger::opening::whole((kind.wage_at)(law, point));
        let ccy = phx_ledger::opening::currency(phx_id::CountryId::new(country));
        let dates = phx_ledger::opening::monthly(self.calendar.date(self.day_zero), phx_id::CountryId::new(country));
        let mut terms = phx_ledger::opening::plain_terms(
            ccy,
            vec![Leg::FixedAmount(Money::new(amount, ccy))],
            Schedule { dates, count: Missing::Absent },
        );
        terms.class = class.to_vec();
        let id = self.books.ledger.terms.intern(terms);
        if let Some(line) = self.labour.lines.get(&id) {
            return *line;
        }
        let mut n = 1_u32;
        while dates.nth(&self.calendar, n) <= self.today {
            n += 1;
        }
        let k = self.books.ledger.lines.kind_index(kind.line);
        let line = self.books.ledger.lines.open(k, id, Missing::Present((dates.nth(&self.calendar, n), n)));
        self.labour.lines.insert(id, line);
        line
    }

    /// 4a: the hires join their lines, the layoffs whose notice has run leave theirs with their severance owed, the
    /// retired persons' jobs leave theirs, and the reviews offered yesterday are answered.
    #[clause("LAB.4", "LAB.6", "LAB.8", "LAB.12")]
    pub(crate) fn labour_start(&mut self, day: Day) {
        if self.labour.kind.is_none() {
            return;
        }
        for h in std::mem::take(&mut self.labour.book.hires) {
            self.hire(day, &h);
        }
        let (due, later): (Vec<Separation>, Vec<Separation>) =
            std::mem::take(&mut self.labour.book.separations).into_iter().partition(|s| s.effective <= day);
        self.labour.book.separations = later;
        for s in due {
            self.separate(day, &s);
        }
        for party in std::mem::take(&mut self.labour.book.retiring) {
            self.retire_jobs(day, party);
        }
        for r in std::mem::take(&mut self.labour.book.reviewing) {
            self.apply_review(day, &r);
        }
    }

    /// A hire joining its line: the employee's members and as many of the employer's, the person attached and no
    /// longer searching, its occupation and wage point recorded.
    #[clause("LAB.13", "LAB.1")]
    fn hire(&mut self, day: Day, h: &Hire) {
        let Some(kind) = self.labour.kind else { return };
        if !self.live(h.employer) || !self.live(h.employee) {
            return;
        }
        let line = self.employment_line(h.country, &h.class, h.point);
        let Missing::Present(reason) = self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(kind.hired))
        else {
            violation!(clause = "LAB.1", "hires under a reason never declared");
        };
        let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S4a));
        let joined = self.books.members_join(
            (h.employee, line, Side::Asset),
            h.employer,
            h.unit,
            (reason, m),
            self.audit.stream(),
        );
        if joined.is_err() {
            violation!(clause = "LAB.1", "a hire that did not join its line", party = h.employee.get());
        }
        self.attach(h.employee, h.person, line);
        let Some(occupation) = h.class.get(class::OCCUPATION).copied() else {
            violation!(clause = "LAB.1", "a hire whose class names no occupation", party = h.employee.get());
        };
        let Some(point) = u32::try_from(h.point).ok().filter(|p| *p < class::WAGE_POINTS) else {
            capacity_exceeded!("a wage point a person can record", class::WAGE_POINTS, h.point);
        };
        self.set_person(
            h.employee,
            h.person,
            &[(kind.state, class::NOT_SEARCHING), (kind.occupation, occupation), (kind.last_point, point)],
        );
        self.labour.day.hires += 1;
    }

    /// A person of an agent attached to an employment line it works on.
    pub(crate) fn attach(&mut self, party: PartyId, person: u32, line: LineId) {
        let Ok(person) = usize::try_from(person) else {
            violation!(clause = "REP.26", "a person beyond a household's", party = party.get());
        };
        let (place, slot) = self.books.parties.row(party);
        let first = self.books.parties.first_cell_place();
        if let Some(k) = place.checked_sub(first).map(usize::from) {
            let table = Population::table_mut::<SystemBacking>(self.books.parties.cells_mut().0, k);
            let mut words = table.attachments(slot).to_vec();
            words.push(Attachment { holder: Holder::Person(person), line, side: Side::Asset }.pack());
            table.set_attachments(slot, &words);
        }
    }

    /// A layoff taking effect: the employer's members leave its line with as many employees drawn from the other
    /// side, each drawn person searching again, and each drawn household owed its severance for the years since the
    /// line's band began.
    fn separate(&mut self, day: Day, s: &Separation) {
        let Some(kind) = self.labour.kind else { return };
        if !self.live(s.employer) {
            return;
        }
        let (place, slot) = self.books.parties.row(s.employer);
        let found = phx_ledger::rows::find(self.books.parties.holder(place), slot, s.line, Side::Liability);
        let Some(held) = found.map(|v| v.row.count) else {
            return;
        };
        // Members an estate or a death took since the notice was given are no longer the employer's to lay off.
        let count = if s.count < held { s.count } else { held };
        let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S4a));
        let stream = self.streams.named(kind.layoff_stream);
        let Some(stream) = stream else { violation!(clause = "CHN.1", "layoffs drawn from a stream never declared") };
        let mut d = self.streams.open(
            &stream,
            Subject::new(SubjectTag::Line, u64::from(s.line.get())),
            day,
            SubStep::S4a.ordinal(),
        );
        let taken = match self.books.members_leave(
            (s.employer, s.line, Side::Liability),
            count,
            m,
            &mut d,
            self.audit.stream(),
        ) {
            Ok(t) => t,
            Err(f) => violation!(clause = "LAB.12", "a layoff that did not leave its line", party = f.party.get()),
        };
        let left: Vec<(PartyId, LineId, Side, u32)> =
            taken.iter().map(|(p, k)| (*p, s.line, Side::Asset, *k)).collect();
        let persons = self.detach(&left, &mut d);
        let point = self.line_point(s.line);
        for (party, person) in persons {
            self.set_person(party, person, &[(kind.state, class::SEARCHING), (kind.last_point, point)]);
            self.labour.searchers.insert(party);
        }
        for (party, k) in taken {
            self.owe_severance(day, (s.employer, party), (s.line, s.country), k);
        }
        self.labour.day.separated += u64::from(count);
    }

    /// A failed firm's estate releasing its staff before it settles: each employment line it holds left at once, as
    /// the firm can no longer give notice, through the separation path, its severance owed to be paid ahead of the
    /// estate's other debts.
    #[clause("LAB.12")]
    pub(crate) fn release_staff(&mut self, day: Day, estate: PartyId) {
        let Some(kind) = self.labour.kind else { return };
        let k = self.books.ledger.lines.kind_index(kind.line);
        let (place, slot) = self.books.parties.row(estate);
        let held: Vec<(LineId, u32)> = phx_ledger::rows::rows(self.books.parties.holder(place), slot)
            .into_iter()
            .filter(|r| r.side() == Side::Liability && self.books.ledger.lines.kind_of(r.row.line) == k)
            .map(|r| (r.row.line, r.row.count))
            .collect();
        if held.is_empty() {
            return;
        }
        let Missing::Present(country) = crate::world::geo_in(&self.own).country_of(self.books.parties.site(estate))
        else {
            violation!(clause = "PTY.5", "an estate sited in no country", estate = estate.get());
        };
        for (line, count) in held {
            self.separate(day, &Separation { employer: estate, country: country.get(), line, count, effective: day });
        }
    }

    /// The wage point of an employment line, from its monthly wage.
    pub(crate) fn line_point(&self, line: LineId) -> u32 {
        let terms = self.books.ledger.terms.get(self.books.ledger.lines.terms(line));
        // Every country's points share the trade's one ratio.
        let (Some(Leg::FixedAmount(m)), Some(law)) = (terms.legs.first(), self.labour.laws.first()) else {
            violation!(clause = "LAB.1", "an employment line that pays no fixed wage", line = line.get());
        };
        let point = self.labour.kind.and_then(|k| (k.point_near)(law, phx_rand::float::from_i64(m.amt())));
        match point.and_then(|p| u32::try_from(p).ok()) {
            Some(p) if p < class::WAGE_POINTS => p,
            _ => capacity_exceeded!("a wage point a person can record", class::WAGE_POINTS, line.get()),
        }
    }

    /// The severance a laid-off household's members are owed: the days a year of service the line's class names, for
    /// each whole year since its band began, of the line's daily wage, paid by the employer at 7c.
    fn owe_severance(
        &mut self,
        day: Day,
        (employer, worker): (PartyId, PartyId),
        (line, country): (LineId, u8),
        members: u32,
    ) {
        let Some(kind) = self.labour.kind else { return };
        let terms = self.books.ledger.terms.get(self.books.ledger.lines.terms(line)).clone();
        let (Some(Leg::FixedAmount(wage)), Some(days), Some(band)) =
            (terms.legs.first(), terms.class.get(class::SEVERANCE), terms.class.get(class::BAND))
        else {
            return;
        };
        let years = i64::from(self.calendar.date(day).year()) - i64::from(*band);
        let law = super::law_of(&self.labour.laws, CountryId::new(country));
        // Each member is owed the same whole amount, a whole share for each of the payer's twins, so every twin on
        // either side pays or receives an equal share.
        let payer = i64::from(self.books.parties.unit(employer));
        let raw = phx_ledger::opening::whole((kind.owed)(law, phx_rand::float::from_i64(wage.amt()), *days, years, 1));
        let each = raw - raw.rem_euclid(payer);
        if each <= 0 {
            return;
        }
        // A party that holds no money in the wage's currency can neither pay nor be paid it, as with any due.
        if !self.books.holds_money(employer, wage.ccy()) || !self.books.holds_money(worker, wage.ccy()) {
            self.labour.day.severance_unpaid += 1;
            return;
        }
        self.labour.owed.push(Owed { employer, worker, each, members, ccy: wage.ccy() });
    }

    /// A retired agent's retired persons leave their jobs: each one's attachments on employment lines taken off it,
    /// and its members leaving each line with as many of its employers'.
    fn retire_jobs(&mut self, day: Day, party: PartyId) {
        let Some(kind) = self.labour.kind else { return };
        if !self.live(party) {
            return;
        }
        let Some(place) = self.labour_kind_place(&kind) else { return };
        let Some(decl) = self.population.kinds.get(place).map(|k| k.decl.clone()) else { return };
        let k = self.books.ledger.lines.kind_index(kind.line);
        let (_, slot) = self.books.parties.row(party);
        let table = Population::table_mut::<SystemBacking>(self.books.parties.cells_mut().0, place);
        let twins = table.multiplicity(slot).get();
        let retired: Vec<usize> = (0..table.persons(slot).len())
            .filter(|i| {
                table.persons(slot).get(*i).map(|w| phx_pop::person::unpack(&decl, *w).attr(kind.state))
                    == Some(Some(class::RETIRED))
            })
            .collect();
        let lines = &self.books.ledger.lines;
        let (leaving, kept): (Vec<u64>, Vec<u64>) = table.attachments(slot).iter().partition(|w| {
            let a = Attachment::unpack(**w);
            matches!(a.holder, Holder::Person(i) if retired.contains(&i)) && lines.kind_of(a.line) == k
        });
        if leaving.is_empty() {
            return;
        }
        table.set_attachments(slot, &kept);
        let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S4a));
        let stream = self.streams.named(kind.layoff_stream);
        let Some(stream) = stream else {
            violation!(clause = "CHN.1", "retirement drawn from a stream never declared")
        };
        let mut d =
            self.streams.open(&stream, Subject::new(SubjectTag::Party, party.get()), day, SubStep::S4a.ordinal());
        for w in leaving {
            let a = Attachment::unpack(w);
            if let Err(f) =
                self.books.members_leave((party, a.line, Side::Asset), twins, m, &mut d, self.audit.stream())
            {
                violation!(clause = "LAB.6", "a retirement that did not leave its line", party = f.party.get());
            }
            self.labour.day.retired += 1;
        }
    }

    /// 7c: the day's severance paid, one instruction for each employer: all it owes where its money covers it, else
    /// each member the same share of what its money holds, as a whole share for each twin; what is not paid is
    /// counted unpaid. Layoffs take effect on business days, so what 4a owes 7c pays the same day.
    #[clause("LAB.12", "SET.1")]
    pub(crate) fn labour_settle(&mut self, day: Day, step: SubStep) {
        let Some(kind) = self.labour.kind else { return };
        let owed = std::mem::take(&mut self.labour.owed);
        if owed.is_empty() {
            return;
        }
        let Missing::Present(reason) =
            self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(kind.severance))
        else {
            violation!(clause = "LAB.12", "severance under a reason never declared");
        };
        // An employer pays its wages in its country's currency, so its severance is owed in that one.
        let mut by_employer: std::collections::BTreeMap<PartyId, Vec<Owed>> = std::collections::BTreeMap::new();
        for o in owed {
            by_employer.entry(o.employer).or_default().push(o);
        }
        let mut due = Vec::new();
        for (employer, list) in by_employer {
            let Some(ccy) = list.first().map(|o| o.ccy) else { continue };
            let total: i128 = list.iter().map(|o| i128::from(o.each) * i128::from(o.members)).sum();
            let held = match self.books.money_held(employer, ccy) {
                Missing::Present(h) if h > 0 => i128::from(h),
                _ => 0,
            };
            let payer = i128::from(self.books.parties.unit(employer));
            let mut legs = Vec::new();
            for o in &list {
                let each = if held >= total {
                    i128::from(o.each)
                } else {
                    let share = i128::from(o.each) * held / total;
                    share - share.rem_euclid(payer)
                };
                let Ok(amount) = i64::try_from(each * i128::from(o.members)) else {
                    phx_num::capacity_exceeded!("a separation's severance", i64::MAX, o.each);
                };
                if each < i128::from(o.each) {
                    self.labour.day.severance_unpaid += 1;
                }
                if amount > 0 {
                    self.books.pay_into(employer, o.worker, (amount, ccy), &mut legs);
                }
            }
            if legs.is_empty() {
                continue;
            }
            let id = self.books.ledger.next_id(day);
            due.push(Instruction {
                id,
                reason,
                trade_day: day,
                settle_day: day,
                legs,
                pays: Missing::Absent,
                covers: Vec::new(),
            });
        }
        let books = &mut self.books;
        let _ = books.ledger.settle(&mut books.parties, ApplyAt::Day(step), due, self.audit.stream());
    }
}
