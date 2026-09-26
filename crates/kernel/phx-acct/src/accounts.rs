use std::collections::BTreeMap;

use phx_core::{CarryingBasis, HeldFor, Permitted};
use phx_id::PartyId;
use phx_ledger::algebra::Side;
use phx_ledger::apply::DayBook;
use phx_ledger::books::Books;
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};
use phx_store::Backing;

use crate::accrual::Claims;
use crate::basis::{at_opening, held_for};
use crate::equity::{EquityAccounts, EquityEvent, EquityKind};
use crate::position::{Held, carrying};

/// A party's income and capital recognised in the current period, tallied from the day's records apart from the
/// equity account, and its account's balance when the period opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Tally {
    pub opened: i64,
    pub income: i128,
    pub capital: i128,
}

/// A period closed: a party's account at its opening and closing, and the income and capital its records show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Closed {
    pub party: PartyId,
    pub opened: i64,
    pub closed: i64,
    pub income: i128,
    pub capital: i128,
}

/// The parties' accounts: their equity accounts, their recognised claims, the period's tallies, and what the
/// standard permits and each kind's legal form, which the reads need.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Accounts {
    pub equity: EquityAccounts,
    pub claims: Claims,
    tallies: BTreeMap<PartyId, Tally>,
    period: u32,
    closed: Vec<Closed>,
    posted: (usize, usize),
    /// The day's income posted from dues and earnings, summed over every party: nothing while each revenue is an
    /// outlay of another. The day's alone, so never saved.
    income: i128,
    permitted: Vec<Permitted>,
    forms: BTreeMap<&'static str, String>,
}

impl Accounts {
    /// The accounts opened over the opening's books: every party of a kind whose legal form has owners keeps an
    /// equity account, opened at its equity as the opening wrote it, in the period the world opens in.
    #[clause("ACC.4", "GEN.4")]
    pub fn open<B: Backing>(
        permitted: Vec<Permitted>,
        forms: BTreeMap<&'static str, String>,
        owned: &dyn Fn(&str) -> bool,
        books: &Books<B>,
        ccy_of: &dyn Fn(PartyId) -> Ccy,
        period: u32,
    ) -> Accounts {
        let mut accounts = Accounts { period, permitted, forms, ..Accounts::default() };
        for kind in books.parties.kinds() {
            let Some(form) = accounts.forms.get(kind) else {
                violation!(clause = "PTY.4", "a kind with no legal form in the accounts");
            };
            if !owned(form) {
                continue;
            }
            for party in books.parties.of_kind(kind) {
                let Ok(opening) = i64::try_from(books.equity(party)) else {
                    phx_num::capacity_exceeded!("an opening equity", i64::MAX, 0);
                };
                accounts.equity.open(party, ccy_of(party), opening);
                accounts.tallies.insert(party, Tally { opened: opening, income: 0, capital: 0 });
            }
        }
        accounts
    }

    /// One party's events added to its period's tally, if it keeps one.
    fn tally_all(&mut self, run: &[EquityEvent]) {
        let Some(t) = run.first().and_then(|e| self.tallies.get_mut(&e.party())) else { return };
        for event in run {
            match event.kind() {
                EquityKind::Income => t.income += i128::from(event.amount()),
                EquityKind::Capital | EquityKind::Distribution | EquityKind::Revaluation => {
                    t.capital += i128::from(event.amount());
                }
            }
        }
    }

    /// A day's book posted when the accounts are read, after the day's money has settled: each due's interest accrued and, where paid, cleared, and each settled
    /// leg's declared effect; each event moves its party's equity account and, separately, its period's tally. A
    /// new period closes the last: each account's opening and closing kept with its tallies for the audit.
    #[clause("ACC.1", "ACC.4", "ACC.9", "ACC.11")]
    pub fn post_day(&mut self, book: &DayBook, period: u32) {
        if period != self.period {
            for (party, t) in &mut self.tallies {
                let Missing::Present(account) = self.equity.of(*party) else { continue };
                self.closed.push(Closed {
                    party: *party,
                    opened: t.opened,
                    closed: account.balance(),
                    income: t.income,
                    capital: t.capital,
                });
                *t = Tally { opened: account.balance(), income: 0, capital: 0 };
            }
            self.period = period;
        }
        let mut events = Vec::with_capacity(book.effects.len());
        for due in &book.dues {
            for event in self.claims.post(due) {
                self.income += i128::from(event.amount());
                events.push(event);
            }
        }
        for (party, amount) in book.earned.sorted() {
            let Ok(amount) = i64::try_from(*amount) else {
                phx_num::capacity_exceeded!("a party's income of one day", i64::MAX, 0);
            };
            self.income += i128::from(amount);
            events.push(EquityEvent::earned(party, amount));
        }
        events.extend(book.effects.iter().filter_map(|e| match EquityEvent::from_effect(e) {
            Missing::Present(event) => Some(event),
            Missing::Absent => None,
        }));
        // Each party's events posted together and in their order, so its account and its tally are each found once.
        events.sort_by_key(EquityEvent::party);
        for run in events.chunk_by(|a, b| a.party() == b.party()) {
            self.equity.post_all(run);
            self.tally_all(run);
        }
        self.posted = (book.dues.len() + book.earned.len(), book.effects.len());
    }

    /// The day's close: every due and effect of the day's book was posted when the accounts were read, since money
    /// settles before then; a record after it would move a balance no equity account follows.
    #[clause("ACC.4", "ACC.10")]
    pub fn close_day(&self, book: &DayBook) {
        if (book.dues.len() + book.earned.len(), book.effects.len()) != self.posted {
            violation!(
                clause = "ACC.4",
                "a due or effect recorded after the day's accounts were posted",
                dues = book.dues.len()
            );
        }
    }

    /// A party's accounts closed as it ends into an estate, which succeeds to all it held: its assets less its
    /// liabilities are its estate's from then, and nothing is left for an account of its own to follow.
    #[clause("PTY.9", "ACC.4")]
    pub fn close(&mut self, party: PartyId) {
        self.equity.close(party);
        let _ = self.tallies.remove(&party);
    }

    /// The day's audit done: the periods it closed are checked, and the next day posts afresh.
    pub fn end_day(&mut self) {
        self.closed.clear();
        self.posted = (0, 0);
        self.income = 0;
    }

    /// The day's income posted from dues and earnings over every party: what one earned less what another spent.
    #[must_use]
    pub fn day_income(&self) -> i128 {
        self.income
    }

    /// An income posted with no one's outlay against it, for the audit's injection alone.
    pub(crate) fn income_alone(&mut self, amount: i128) {
        self.income += amount;
    }

    /// The accounts into the world's hash: each equity account, each unpaid claim, and the period's tallies.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        h.u64(u64::from(self.period));
        for party in self.equity.parties() {
            let Missing::Present(a) = self.equity.of(party) else { continue };
            h.u64(party.get());
            h.u64(u64::from(a.ccy().index()));
            h.u64(a.balance().cast_unsigned());
        }
        self.claims.hash_into(h);
        for (party, t) in &self.tallies {
            h.u64(party.get());
            h.u64(t.opened.cast_unsigned());
            h.bytes(&t.income.to_le_bytes());
            h.bytes(&t.capital.to_le_bytes());
        }
    }

    /// The accounts for a save, taken at a day's close: the equity accounts, the claims, the period and its tallies.
    /// What the standard permits and each kind's form are the build's, and a close leaves no period to check.
    #[clause("SET.12")]
    pub fn save_to(&self, w: &mut phx_store::Writer<'_>) {
        use phx_store::Saved as _;
        if !self.closed.is_empty() || self.posted != (0, 0) || self.income != 0 {
            violation!(clause = "SET.13", "accounts saved before the day's audit ended");
        }
        self.equity.save(w);
        self.claims.save(w);
        self.tallies.save(w);
        self.period.save(w);
    }

    /// The accounts read back, over what the standard permits and each kind's form, as the build declares them.
    ///
    /// # Errors
    /// When the store is damaged.
    pub fn load_from(
        r: &mut phx_store::Reader<'_>,
        permitted: Vec<Permitted>,
        forms: BTreeMap<&'static str, String>,
    ) -> Result<Accounts, phx_store::LoadError> {
        use phx_store::Saved as _;
        Ok(Accounts {
            equity: EquityAccounts::load(r)?,
            claims: Claims::load(r)?,
            tallies: BTreeMap::load(r)?,
            period: u32::load(r)?,
            closed: Vec::new(),
            posted: (0, 0),
            income: 0,
            permitted,
            forms,
        })
    }

    /// The carrying bases the standard permits, as the accounts were opened with them.
    #[must_use]
    pub fn permitted(&self) -> &[Permitted] {
        &self.permitted
    }

    /// Each kind's legal form, as the accounts were opened with them.
    #[must_use]
    pub fn forms(&self) -> &BTreeMap<&'static str, String> {
        &self.forms
    }

    /// A closed period put among today's for the audit's injection alone.
    pub(crate) fn close_period(&mut self, closed: Closed) {
        self.closed.push(closed);
    }

    /// The periods that closed at today's posting.
    #[must_use]
    pub fn closed(&self) -> &[Closed] {
        &self.closed
    }

    /// A party's period to date, as its statement reads it.
    pub fn tally_of(&self, party: PartyId) -> Missing<Tally> {
        match self.tallies.get(&party) {
            Some(t) => Missing::Present(*t),
            None => Missing::Absent,
        }
    }

    /// A kind's legal form.
    pub fn form_of(&self, kind: &str) -> Missing<&str> {
        match self.forms.get(kind) {
            Some(f) => Missing::Present(f.as_str()),
            None => Missing::Absent,
        }
    }

    /// A party's assets less its liabilities, read from the register and the contracts at their carrying values,
    /// with its recognised receivables and payables: each contract row it holds on its basis — a claim held to
    /// collect on the basis its form was permitted, a debt owed at amortised cost — and each holding on the basis its
    /// form was permitted for what it is held for.
    ///
    /// # Errors
    /// Unreadable, with why, where the form is permitted no basis or a basis reads what is absent.
    #[clause("ACC.2", "ACC.10", "NUM.8")]
    pub fn net_assets<B: Backing>(&self, books: &Books<B>, party: PartyId) -> Result<i128, String> {
        let (place, slot) = books.parties.row(party);
        let table = books.parties.table(place);
        let Missing::Present(form) = self.form_of(table.kind()) else {
            return Err(format!("party {} of a kind with no legal form", party.get()));
        };
        let mut total = 0_i128;
        for row in phx_ledger::rows::rows(table, slot) {
            let Missing::Present(balance) = row.optional.balance else { continue };
            let basis = match row.side() {
                Side::Asset => at_opening(&self.permitted, form, HeldFor::Collect),
                Side::Liability => Missing::Present(CarryingBasis::AmortisedCost),
            };
            let Missing::Present(basis) = basis else {
                return Err(format!("party {}: its form is permitted no basis for a claim", party.get()));
            };
            let held = Held { cost: balance, amortised: Missing::Present(balance), depreciation: 0, written_down: 0 };
            let Missing::Present(value) = carrying(basis, &held, Missing::Absent) else {
                return Err(format!(
                    "party {}: a claim on line {} with no mark to carry it at",
                    party.get(),
                    row.row.line.get()
                ));
            };
            total += i128::from(value);
        }
        for (instrument, cost) in phx_ledger::holding::bases(table, slot) {
            let family = books.ledger.instruments.get(instrument).family;
            let Missing::Present(basis) = at_opening(&self.permitted, form, held_for(family)) else {
                return Err(format!(
                    "party {}: its form is permitted no basis for instrument {}",
                    party.get(),
                    instrument.get()
                ));
            };
            let held = Held { cost, amortised: Missing::Absent, depreciation: 0, written_down: 0 };
            let Missing::Present(value) = carrying(basis, &held, Missing::Absent) else {
                return Err(format!(
                    "party {}: instrument {} with no mark or valuation to carry it at",
                    party.get(),
                    instrument.get()
                ));
            };
            total += i128::from(value);
        }
        let (receivable, payable) = self.claims.of(party);
        Ok(total + receivable - payable)
    }
}
