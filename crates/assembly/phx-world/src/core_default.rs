//! Firms in default on the core. A contract's arrears keep the day they began; a firm whose arrears have outlasted its
//! country's grace is in default of payment and ends into an estate: its money opens the estate at its bank, its goods
//! pass to it, and every contract it was party to closes into a claim on it — its employees' wages owed and the
//! severance their years earn first, then what it owed its lenders — its employees searching again. The estate pays
//! its claims by rank on its country's next business day, each rank in proportion to what it is owed, and the rest
//! where the law sends what no one claims.

use std::collections::BTreeMap;

use phx_core::flows::{Denom, Flow};
use phx_core::goods::{Bound, Cost};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_macros::clause;

use crate::consts::firm::{PRODUCT, REGION};
use crate::core::Core;
use crate::core_labour::LabourCtx;

/// A creditor's claim on an estate: whom it is owed to, how much, the reason its payment is made for, and its rank,
/// the first paid first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claim {
    pub creditor: PartyKey,
    pub amount: i64,
    pub reason: u8,
    pub rank: u8,
}

/// The ranks of claims on a firm's estate: its employees', then every other creditor's.
pub const EMPLOYEES: u8 = 0;
pub const CREDITORS: u8 = 1;

/// A firm's ending: the day, its product and region, and whether it defaulted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ending {
    pub day: u32,
    pub product: u16,
    pub region: u32,
    pub defaulted: bool,
}

/// The insolvency law on the core: each country's grace for firms, the day each contract in arrears began to be, each
/// estate's claims, and the firms ended.
#[derive(Clone, Debug, Default)]
pub struct Insolvency {
    pub grace: Vec<Option<u32>>,
    pub since: BTreeMap<(usize, u32), Day>,
    pub claims: BTreeMap<PartyKey, Vec<Claim>>,
    pub endings: Vec<Ending>,
}

impl Core {
    /// Each country's grace for firms from its insolvency law.
    ///
    /// # Errors
    /// A grace the register does not hold, or one beyond a day count.
    #[clause("FRM.15")]
    pub fn open_insolvency(&mut self, register: &phx_core::Register) -> Result<(), String> {
        let days = register.counts_per_country("FRM.insolvency_grace_days")?;
        self.insolvency.grace = days
            .into_iter()
            .map(|d| u32::try_from(d).map(Some).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        Ok(())
    }

    /// Each contract a failed flow put in arrears remembered from the day its arrears began, and every one forgotten
    /// once paid or closed.
    pub(crate) fn note_arrears(&mut self, day: Day, failed: &[Flow]) {
        for f in failed {
            let Some(i) = self
                .families
                .iter()
                .position(|x| x.reason == f.reason && x.store.kinds.first() == Some(&f.payer.kind()))
            else {
                continue;
            };
            let held = self.families.get(i).is_some_and(|x| {
                let slot = Slot::new(f.source);
                x.store.edges.is_open(slot) && x.store.edges.row(slot).is_some_and(|r| r.arrears > 0)
            });
            if held {
                self.insolvency.since.entry((i, f.source)).or_insert(day);
            }
        }
        let families = &self.families;
        self.insolvency.since.retain(|(i, edge), _| {
            families.get(*i).is_some_and(|f| {
                let slot = Slot::new(*edge);
                f.store.edges.is_open(slot) && f.store.edges.row(slot).is_some_and(|r| r.arrears > 0)
            })
        });
    }

    /// The firms whose arrears have outlasted their country's grace today, each ended into an estate.
    #[clause("FRM.15", "L3", "PTY.9")]
    pub(crate) fn end_defaulted(&mut self, ctx: &LabourCtx<'_>, day: Day) -> u64 {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return 0 };
        let firm_kind = crate::core::kind_number(firm);
        let mut due: Vec<(PartyKey, u8)> = Vec::new();
        for ((i, edge), since) in &self.insolvency.since {
            let Some(row) = self.families.get(*i).and_then(|f| f.store.edges.row(Slot::new(*edge))) else { continue };
            let payer = row.ends[0];
            if payer.kind() != firm_kind {
                continue;
            }
            let Some(region) = self.record_of(firm, payer.slot(), REGION) else { continue };
            let country = ctx.country_of(u32::try_from(region).unwrap_or(u32::MAX));
            let Some(Some(grace)) = self.insolvency.grace.get(usize::from(country)).copied() else {
                phx_num::violation!(clause = "FRM.15", "a firm's country with no insolvency grace", country = country);
            };
            if ctx.calendar.days_between(*since, day).is_some_and(|d| d > grace) {
                due.push((payer, country));
            }
        }
        due.sort_unstable();
        due.dedup();
        for &(key, country) in &due {
            self.end_firm(ctx, (key, CountryId::new(country)), day, true);
        }
        phx_rand::float::len_u64(due.len())
    }

    /// Whether a solvent firm's owner winds it down on its production schedule: continuing — the margin a year it
    /// expects on the sales it expects, at the price it expects over its cost of making a unit, held at the return
    /// its management requires — against what winding down returns: its money and goods, its own product at the price
    /// it expects and the rest at their cost, less what it would owe on ending, its staff's severance with it. A firm
    /// whose management requires no return, or that does not know its cost, goes on.
    #[clause("FRM.11", "FRM.15")]
    pub(crate) fn winds_down(&self, ctx: &LabourCtx<'_>, (firm, slot): (usize, Slot), day: Day) -> bool {
        let key = PartyKey::new(crate::core::kind_number(firm), slot);
        let closing = self.bind(&sys_frm::points::CLOSE);
        let (_, prefs) = self.decider(closing, key);
        let (Some(product), Some(region), Some(expected), phx_num::Missing::Present(rate)) = (
            self.record_of(firm, slot, PRODUCT),
            self.record_of(firm, slot, REGION),
            self.record_of(firm, slot, crate::consts::firm::EXPECTED),
            prefs.required_return,
        ) else {
            return false;
        };
        if rate <= 0.0 {
            return false;
        }
        let Ok(product) = u16::try_from(product) else { return false };
        let Some(cost) = self.unit_cost_of(ctx.regions, firm, slot) else { return false };
        let lot = sys_frm::FilingPrims::lot(ctx.register, product);
        let price = self.price_expected(firm, slot, lot, &prefs);
        let expected = phx_rand::float::from_i64(expected) / crate::consts::firm::PART_ONE;
        let continuing = (price - cost) * expected * crate::consts::DAYS_A_YEAR / rate;
        let money = self
            .kinds
            .get(firm)
            .and_then(|k| k.accounts.as_ref())
            .map_or(0, |a| a.balance.get(slot).unwrap_or(0) + a.pending.get(slot).unwrap_or(0));
        let goods: f64 = self
            .goods
            .stocks
            .holdings(key)
            .map(|h| match self.goods.units.held(h.unit) {
                Some(phx_core::goods::Held::Good(g)) if g.product == product => {
                    phx_rand::float::from_i64(h.units) * price
                }
                _ => phx_rand::float::from_i64(h.cost),
            })
            .sum();
        let country = ctx.country_of(u32::try_from(region).unwrap_or(u32::MAX));
        let (claims, _) = self.claims_of(ctx, (key, country), day);
        let owed: i64 = claims.iter().map(|c| c.amount).sum();
        self.decide(closing, key, |_| sys_frm::rules::review::CloseIn {
            continuing,
            held: phx_rand::float::from_i64(money) + goods,
            owed: phx_rand::float::from_i64(owed),
        })
    }

    fn record_of(&self, kind: usize, slot: Slot, at: usize) -> Option<i64> {
        match self.kinds.get(kind)?.record(slot).get(at).map(|w| w.get()) {
            Some(phx_num::Missing::Present(v)) => Some(v),
            _ => None,
        }
    }

    /// A firm ended into an estate: its money and goods pass to it, each contract it was party to closes into a claim
    /// on it, its employees search again, and its vacancies close.
    #[clause("PTY.9", "LAB.12", "L3")]
    pub(crate) fn end_firm(
        &mut self,
        ctx: &LabourCtx<'_>,
        (key, country): (PartyKey, CountryId),
        day: Day,
        defaulted: bool,
    ) {
        let place = usize::from(key.kind());
        let (Some(product), Some(region)) =
            (self.record_of(place, key.slot(), PRODUCT), self.record_of(place, key.slot(), REGION))
        else {
            return;
        };
        let Some((bank, money)) = self
            .kinds
            .get(place)
            .and_then(|k| k.accounts.as_ref())
            .and_then(|a| Some((a.bank.get(key.slot())?, a.balance.get(key.slot())? + a.pending.get(key.slot())?)))
        else {
            return;
        };
        let estate = self.open_estate((bank, money), (country, day));
        if let Some(a) = self.kinds.get_mut(place).and_then(|k| k.accounts.as_mut()) {
            a.balance.set(key.slot(), 0);
            a.pending.set(key.slot(), 0);
        }
        self.pass_goods(key, estate, day);
        let claims = self.close_contracts(ctx, (key, country.get()), day);
        self.insolvency.claims.insert(estate, claims);
        for v in self.labour.vacancies.iter_mut().filter(|v| v.employer == key) {
            v.open = 0;
        }
        self.labour.fills.retain(|(employer, _), _| *employer != key);
        self.labour.reviews.remove(&key);
        self.goods.outlooks.awaiting.remove(&key.slot().get());
        self.goods.stocks.end(key);
        if let Some(k) = self.kinds.get_mut(place)
            && let Some(r) = k.parties.at(key.slot())
        {
            k.parties.end(r);
        }
        self.insolvency.endings.push(Ending {
            day: day.get(),
            product: u16::try_from(product).unwrap_or(u16::MAX),
            region: u32::try_from(region).unwrap_or(u32::MAX),
            defaulted,
        });
    }

    /// Every good a firm holds passed to its estate at its cost.
    fn pass_goods(&mut self, from: PartyKey, to: PartyKey, day: Day) {
        let held: Vec<(u16, i64)> = self.goods.stocks.holdings(from).map(|h| (h.unit, h.units)).collect();
        for (unit, units) in held.into_iter().filter(|(_, u)| *u > 0) {
            let flow = Flow {
                payer: from,
                payee: to,
                amount: units,
                source: from.slot().get(),
                denomination: Denom::units(unit),
                reason: crate::consts::reason::ESTATE,
                order: 0,
            };
            if self.goods.stocks.apply(&flow, Bound::Free, Cost::Carried, day).is_err() {
                phx_num::violation!(clause = "PTY.9", "an estate's succession to goods not held free", unit = unit);
            }
        }
    }

    /// What a firm owes on its ending, as claims on its estate — an employee's wages owed and severance for its years
    /// served at the employees' rank, what else it owes at the creditors' — and its employees, each with its wage.
    pub(crate) fn claims_of(
        &self,
        ctx: &LabourCtx<'_>,
        (key, country): (PartyKey, u8),
        day: Day,
    ) -> (Vec<Claim>, Vec<(PartyKey, u64, i64)>) {
        let (mut claims, mut staff) = (Vec::new(), Vec::new());
        let year = ctx.calendar.date(day).year();
        let law = self.labour.laws.get(usize::from(country));
        for family in &self.families {
            if family.store.kinds.first() != Some(&key.kind()) {
                continue;
            }
            if family.name != "LAB.employment" {
                continue;
            }
            for edge in family.store.of(0, key.slot()) {
                let Some(row) = family.store.edges.row(edge) else { continue };
                let at = usize::try_from(row.schedule).unwrap_or(usize::MAX);
                {
                    let band = family.classes.get(at).and_then(|k| k.get(2)).copied();
                    let years = band.map_or(0, |b| i64::from(year) - i64::from(b));
                    let severance = law.map_or(0, |l| {
                        phx_ledger::opening::whole((ctx.kind.owed)(
                            l,
                            phx_rand::float::from_i64(row.amount),
                            l.severance_days_a_year,
                            years,
                            1,
                        ))
                    });
                    claims.push(Claim {
                        creditor: row.ends[1],
                        amount: row.arrears,
                        reason: family.reason,
                        rank: EMPLOYEES,
                    });
                    claims.push(Claim {
                        creditor: row.ends[1],
                        amount: severance,
                        reason: crate::consts::reason::SEVERANCE,
                        rank: EMPLOYEES,
                    });
                    staff.push((row.ends[1], row.person, row.amount));
                }
            }
        }
        claims.extend(self.debts_of(key));
        claims.retain(|c| c.amount > 0);
        (claims, staff)
    }

    /// What a party owes its creditors beside its employees, as claims on its estate: on each contract it pays other
    /// than a job, its arrears, and, on one reckoned from its terms, the balance it still has to repay.
    pub(crate) fn debts_of(&self, key: PartyKey) -> Vec<Claim> {
        let mut claims = Vec::new();
        for family in self.families.iter().filter(|f| f.name != "LAB.employment") {
            if family.store.kinds.first() != Some(&key.kind()) {
                continue;
            }
            for edge in family.store.of(0, key.slot()) {
                let Some(row) = family.store.edges.row(edge) else { continue };
                let at = usize::try_from(row.schedule).unwrap_or(usize::MAX);
                let balance = if family.terms.get(at).and_then(Option::as_ref).is_some() { row.amount } else { 0 };
                claims.push(Claim {
                    creditor: row.ends[1],
                    amount: row.arrears + balance,
                    reason: family.reason,
                    rank: CREDITORS,
                });
            }
        }
        claims.retain(|c| c.amount > 0);
        claims
    }

    /// Every contract a firm was party to closed into its claims on the estate, and each of its employees searching
    /// again, claiming its benefit where it pays.
    fn close_contracts(&mut self, ctx: &LabourCtx<'_>, (key, country): (PartyKey, u8), day: Day) -> Vec<Claim> {
        let (claims, staff) = self.claims_of(ctx, (key, country), day);
        for i in 0..self.families.len() {
            let Some(family) = self.families.get_mut(i) else { continue };
            let Some(side) = family.store.kinds.iter().position(|k| *k == key.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, key.slot()).collect();
            for edge in mine {
                family.close_contract(edge);
                self.insolvency.since.remove(&(i, edge.get()));
                self.labour.noticed.remove(&edge.get());
            }
        }
        if let Some(law) = self.labour.laws.get(usize::from(country)).cloned() {
            for (household, person, wage) in staff {
                self.searches_again(ctx, (household, person), wage, &law);
                self.claim_benefit(ctx, day, (household, person), (wage, country), &law);
            }
        }
        claims
    }
}

/// What an estate pays of its claims out of what it holds: rank by rank, each claim its amount while the money covers
/// the rank, else its share of what is left in proportion to its amount, rounded down; each payee with its amount and
/// reason.
#[must_use]
pub fn shares(claims: &[Claim], held: i64) -> Vec<(PartyKey, i64, u8)> {
    let mut out = Vec::new();
    let mut left = held;
    let mut ranks: Vec<u8> = claims.iter().map(|c| c.rank).collect();
    ranks.sort_unstable();
    ranks.dedup();
    for rank in ranks {
        if left <= 0 {
            break;
        }
        let these: Vec<&Claim> = claims.iter().filter(|c| c.rank == rank && c.amount > 0).collect();
        let owed: i128 = these.iter().map(|c| i128::from(c.amount)).sum();
        let pool = i128::from(left);
        for c in these {
            let paid = if owed <= pool { i128::from(c.amount) } else { i128::from(c.amount) * pool / owed };
            let paid = i64::try_from(paid).unwrap_or(0);
            if paid > 0 {
                out.push((c.creditor, paid, c.reason));
                left -= paid;
            }
        }
    }
    out
}

#[path = "core_default_tests.rs"]
#[cfg(test)]
mod tests;
