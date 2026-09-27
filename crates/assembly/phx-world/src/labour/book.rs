//! The labour book: what the rounds carry from one day to the next, saved with the world.

use std::collections::{BTreeMap, BTreeSet};

use if_labour::kind::LabourKind;
use if_labour::law::Law;
use phx_id::{Day, LineId, PartyId};
use phx_ledger::terms::TermsId;

/// An employer's posted offer of jobs of a class at a wage point: jobs open counted in contracts, the day it was
/// first posted and the day its offer was last set.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Vacancy {
    pub id: u32,
    pub employer: PartyId,
    pub country: u8,
    pub region: u32,
    pub occupation: u32,
    pub skill: u32,
    pub hours: u32,
    pub point: i64,
    pub open: u32,
    pub first: Day,
    pub set: Day,
}

/// A searcher's application to a vacancy: the applicant agent and its person, its skill and experience, and the day it
/// was sent.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Application {
    pub vacancy: u32,
    pub applicant: PartyId,
    pub person: u32,
    pub skill: u32,
    pub experience: u32,
    pub sent: Day,
}

/// A job offered to an applicant, held from the vacancy until answered.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Offer {
    pub vacancy: u32,
    pub applicant: PartyId,
    pub person: u32,
    pub made: Day,
}

/// A hire accepted, to join its line at the next day's start of work: the employer, the employee and its person, the
/// country, the job's class and wage point, and the days its vacancy stood.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Hire {
    pub employer: PartyId,
    pub employee: PartyId,
    pub person: u32,
    pub country: u8,
    pub class: Vec<u32>,
    pub point: i64,
    pub stood: u32,
}

/// Jobs an employer laid off, leaving their line on the first business day, in the employer's country, their notice
/// has run by.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Separation {
    pub employer: PartyId,
    pub country: u8,
    pub line: LineId,
    pub count: u32,
    pub effective: Day,
}

/// The wage point an employer last filled a vacancy of an occupation at, and the days that took.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Fill {
    pub point: i64,
    pub days: u32,
}

/// An employer's offer at its review of one of its lines, applied at the next start of work: the members it covers,
/// the point offered and the most the work pays for, the point a counter may reach.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Review {
    pub employer: PartyId,
    pub country: u8,
    pub line: LineId,
    pub count: u32,
    pub offer: i64,
    pub most: i64,
}

/// Severance an employer owes a household's members, each member the same whole amount, paid at 7c.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Owed {
    pub employer: PartyId,
    pub worker: PartyId,
    pub each: i64,
    pub members: u32,
    pub ccy: phx_num::Ccy,
}

/// What the rounds carry across days.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct LabourBook {
    pub vacancies: Vec<Vacancy>,
    pub applications: Vec<Application>,
    pub offers: Vec<Offer>,
    pub hires: Vec<Hire>,
    pub separations: Vec<Separation>,
    pub fills: BTreeMap<(PartyId, u32), Fill>,
    pub retiring: Vec<PartyId>,
    pub reviews: BTreeMap<PartyId, Day>,
    pub reviewing: Vec<Review>,
    pub next: u32,
    /// Each employer's vacancies by identity, rebuilt from them rather than saved.
    #[saved(skip)]
    pub by_employer: BTreeMap<PartyId, Vec<u32>>,
}

/// The day's tally of the rounds, for the counters and the checks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LabourDay {
    pub searching_groups: u64,
    pub vacancies_visible: u64,
    pub applications: u64,
    pub offers: u64,
    pub acceptances: u64,
    pub matches: u64,
    /// Persons hired, laid off and posted for, so the three compare.
    pub hires: u64,
    pub layoffs: u64,
    pub separated: u64,
    pub retired: u64,
    pub posted: u64,
    pub match_days: u64,
    pub reviewed: u64,
    pub raised: u64,
    pub cut: u64,
    pub quits: u64,
    pub severance_unpaid: u64,
}

/// Labour as the world keeps it: its kind and laws, its book, and what is rebuilt from the world rather than saved —
/// each employment line by its terms, the agents whose persons search, and the employers whose schedule came due
/// today.
#[derive(Debug, Default)]
pub(crate) struct Labour {
    pub kind: Option<LabourKind>,
    pub laws: Vec<Law>,
    pub book: LabourBook,
    pub day: LabourDay,
    pub lines: BTreeMap<TermsId, LineId>,
    pub searchers: BTreeSet<PartyId>,
    pub employers_due: Vec<(crate::goods::Rows, phx_id::Slot)>,
    pub owed: Vec<Owed>,
}

impl Labour {
    pub fn none() -> Labour {
        Labour::default()
    }

    pub fn of(kind: &LabourKind, laws: Vec<Law>) -> Labour {
        Labour { kind: Some(*kind), laws, ..Labour::default() }
    }
}

impl LabourBook {
    /// A vacancy by its identity: the book keeps them in the order they were posted, so by their identities.
    pub(crate) fn vacancy(&self, id: u32) -> Option<&Vacancy> {
        self.vacancies.binary_search_by_key(&id, |v| v.id).ok().and_then(|i| self.vacancies.get(i))
    }

    pub(crate) fn vacancy_mut(&mut self, id: u32) -> Option<&mut Vacancy> {
        self.vacancies.binary_search_by_key(&id, |v| v.id).ok().and_then(|i| self.vacancies.get_mut(i))
    }

    /// Each employer's vacancies indexed again from the book.
    pub(crate) fn index_employers(&mut self) {
        self.by_employer.clear();
        for v in &self.vacancies {
            self.by_employer.entry(v.employer).or_default().push(v.id);
        }
    }

    /// A vacancy posted, the latest, indexed under its employer.
    pub(crate) fn post(&mut self, v: Vacancy) {
        self.by_employer.entry(v.employer).or_default().push(v.id);
        self.vacancies.push(v);
    }

    /// The vacancies kept that `keep` keeps, the index with them.
    pub(crate) fn keep(&mut self, keep: impl FnMut(&Vacancy) -> bool) {
        self.vacancies.retain(keep);
        self.index_employers();
    }

    /// An employer's vacancies' identities, oldest first.
    pub(crate) fn of_employer(&self, employer: PartyId) -> &[u32] {
        self.by_employer.get(&employer).map_or(&[], Vec::as_slice)
    }
}
