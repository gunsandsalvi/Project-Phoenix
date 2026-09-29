//! Injection: one family's discrepancy put into a world read back from a save, and the audit run over the state as it
//! stands without a day stepped, to show that family alone sees it. Each discrepancy is a unit put where only its own
//! family's invariant reads it.

use phx_core::flows::{Denom, Flow};
use phx_core::goods::{Bound, Cost};
use phx_id::{Day, PartyKey, Slot};
use phx_macros::clause;
use phx_store::edges::Row as _;

use crate::core::Core;
use crate::world::World;

impl World {
    /// One family's injection into this world, read back from a save, then the audit over the state as it stands at
    /// the save's next day, no day stepped: the families that found something, in the order they did. The world is
    /// discarded after; it is never run on.
    ///
    /// # Errors
    /// A world not read from a save, a family the audit does not run, or an injection the save cannot take.
    #[clause("N1")]
    pub fn inject(&mut self, family: &str) -> Result<Vec<&'static str>, String> {
        if !self.loaded {
            return Err("an injection goes into a save loaded apart, never into the world being run".to_owned());
        }
        let day = self.today.succ();
        let held = self.core.held();
        self.core.inject(family, day)?;
        self.core.audit_close(day, held);
        let mut lit: Vec<&'static str> = Vec::new();
        for f in self.core.found.drain(..) {
            if !lit.contains(&f.family) {
                lit.push(f.family);
            }
        }
        Ok(lit)
    }
}

impl Core {
    /// A live party of a kind, by its name: the first in slot order.
    fn first_of(&self, kind: &str) -> Option<PartyKey> {
        let place = self.names.iter().position(|n| *n == kind)?;
        let slot = self.kinds.get(place)?.parties.live_slots().next()?;
        Some(PartyKey::new(crate::core::kind_number(place), slot))
    }

    /// A family's discrepancy: a unit of money in a treasury's account no flow paid, a unit of a good a firm holds
    /// that nothing made, a contract naming a household that never was, a person the households are counted to hold
    /// that none does, a unit of tax arisen, of bills issued, on a creditor's loan book, in an equity account, of
    /// revenue recognised, and in a finite deposit, each moved by nothing.
    fn inject(&mut self, family: &str, day: Day) -> Result<(), String> {
        let refused = || format!("the save holds nothing to inject `{family}` into");
        match family {
            "money" => {
                let treasury = self.treasuries.iter().flatten().next().copied().ok_or_else(refused)?;
                let account = self
                    .kinds
                    .get_mut(usize::from(treasury.kind()))
                    .and_then(|k| k.accounts.as_mut())
                    .ok_or_else(refused)?;
                let balance = account.balance.get(treasury.slot()).ok_or_else(refused)?;
                account.balance.set(treasury.slot(), balance + 1);
            }
            "goods" => {
                let firm = self.first_of("firm").ok_or_else(refused)?;
                let unit = self.goods.stocks.holdings(firm).next().map(|h| h.unit).ok_or_else(refused)?;
                let made = Flow {
                    payer: phx_core::goods::NATURE,
                    payee: firm,
                    amount: 1,
                    source: 0,
                    denomination: Denom::units(unit),
                    reason: crate::consts::reason::MADE,
                    order: 0,
                };
                let _ = self.goods.stocks.apply(&made, Bound::Free, Cost::At(0), day);
            }
            "contracts" => {
                let place = self.names.iter().position(|n| *n == "household").ok_or_else(refused)?;
                let never = Slot::new(self.kinds.get(place).ok_or_else(refused)?.parties.high_water());
                let jobs = self
                    .families
                    .iter_mut()
                    .find(|f| f.name == crate::consts::families::EMPLOYMENT)
                    .ok_or_else(refused)?;
                let edge = jobs.store.edges.open_slots().next().ok_or_else(refused)?;
                let row = jobs
                    .store
                    .edges
                    .rows_mut()
                    .get_mut(usize::try_from(edge.get()).unwrap_or(usize::MAX))
                    .ok_or_else(refused)?;
                row.set_end(1, PartyKey::new(crate::core::kind_number(place), never));
            }
            "persons" => self.persons_opened += 1,
            "taxes" => *self.taxes.arisen.first_mut().ok_or_else(refused)? += 1,
            "debt" => self.bills.issued += 1,
            "loans" => self.loan_books.values_mut().next().ok_or_else(refused)?.book += 1,
            "accounts" => *self.accounts.opening.values_mut().next().ok_or_else(refused)? += 1,
            "revenue" => self.accounts.revenue += 1,
            "deposits" => *self.deposits.remaining.iter_mut().flatten().next().ok_or_else(refused)? += 1,
            _ => return Err(format!("the audit runs no family `{family}`")),
        }
        Ok(())
    }
}
