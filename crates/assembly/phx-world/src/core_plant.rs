//! Plant on the core. A firm's plant is capital units in its holdings, each kind held in condition classes of age at
//! its region, opened from its output a year and its way's plant of each kind a unit, spread over the classes a stock
//! grown at its country's growth holds. A firm makes no more a day than its scarcest kind's efficient units allow.
//! Every review period the kinds wear along their chains, the value they lose charged to income as depreciation. A
//! capital good bought as investment is a project of its buyer at what it paid, named with its producer, until its
//! kind's lead has passed, when it enters service as new plant.

use phx_core::OpeningCountry;
use phx_core::flows::{Denom, Flow};
use phx_core::goods::{Bound, Cost, Held, NATURE};
use phx_core::units::{Chain, Class};
use phx_id::{CountryId, Day, PartyKey};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::float::{floor_to_i64, from_i64};

use crate::consts::reason::{BUILT, WORN};
use crate::consts::{DAYS_A_YEAR, PERCENT};
use crate::core::Core;
use crate::core_accounts::Line;

/// A capital good bought and not yet in service: its buyer and producer, its kind, its units and what they cost, the
/// day it was delivered and the day it enters service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Project {
    pub holder: PartyKey,
    pub producer: PartyKey,
    pub kind: u16,
    pub units: i64,
    pub cost: i64,
    pub delivered: Day,
    pub ready: Day,
}

/// What the plant did on a day: the money paid into projects, the units that entered service, those worn from one
/// class to the next and those retired from the last, the makings plant bound and those beyond what it allowed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct PlantDay {
    pub day: u32,
    pub invested: i64,
    pub completed: i64,
    pub worn: i64,
    pub retired: i64,
    pub bound: u64,
    pub beyond: u64,
}

/// The plant: each kind's chain of classes, each way's plant of each kind a unit of output a year by country and
/// product, the product each kind is bought as and its days from delivery to service, the days between reviews and
/// the day the plant opened, the projects under way and the days' records.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Plant {
    pub chains: Vec<Chain>,
    needs: Vec<Vec<(usize, f64)>>,
    products: usize,
    bought_as: Vec<u16>,
    lead: Vec<u32>,
    review_days: u32,
    began: u32,
    pub projects: Vec<Project>,
    pub days: Vec<PlantDay>,
    today: PlantDay,
}

impl Core {
    /// Each firm's plant at the opening: of each kind its way needs, its output a year times the way's plant of the
    /// kind a unit in efficient units, spread over the classes as a stock grown at its country's growth since its
    /// oldest units were new, each class's units at its value's share of the kind's product's opening price new.
    ///
    /// # Errors
    /// A kind with no lead, a country's growth or prices unread.
    #[clause("CAP.1", "GEN.2", "GEN.5", "REP.24")]
    pub(crate) fn open_plant(
        &mut self,
        (cap, register): (&sys_cap::CapOwn, &phx_core::Register),
        (countries, regions): (&[OpeningCountry], &[CountryId]),
        today: Day,
    ) -> Result<(), String> {
        let classes = cap.kinds.classes;
        let chains: Vec<Chain> = cap
            .kinds
            .kinds
            .iter()
            .map(|k| Chain {
                leaving_per_year: sys_cap::rules::wear::leaving_rate(classes, k.life),
                efficiency: k.efficiencies(classes),
                value: k.values(classes),
            })
            .collect();
        let lead = cap
            .lead
            .iter()
            .enumerate()
            .map(|(k, l)| match l {
                Missing::Present(d) => Ok(*d),
                Missing::Absent => Err(format!("plant of kind {k} with no days from order to service")),
            })
            .collect::<Result<Vec<u32>, String>>()?;
        let products = register.products("TEC.products")?.len();
        self.plant = Plant {
            chains,
            needs: cap.needs.clone(),
            products,
            bought_as: cap.bought_as.clone(),
            lead,
            review_days: u32::try_from(floor_to_i64(cap.review_days).unwrap_or(0)).map_err(|e| e.to_string())?,
            began: today.get(),
            ..Plant::default()
        };
        let mut prices = Vec::new();
        let mut growth = Vec::new();
        for c in countries {
            prices.push(crate::core_firms::snapshot(register, c)?.price);
            growth.push(c.derived("GEN.growth").ok_or("no drawn `GEN.growth`")? / PERCENT);
        }
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return Ok(()) };
        let conditions = u8::try_from(classes).map_err(|e| e.to_string())?;
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(regions, firm, slot) else { continue };
            let needs =
                self.plant.needs.get(f.country * products + usize::from(f.product)).cloned().unwrap_or_default();
            for (kind, per) in needs.into_iter().filter(|(_, per)| *per > 0.0) {
                let (Some(chain), Some(k)) = (self.plant.chains.get(kind).cloned(), cap.kinds.kinds.get(kind)) else {
                    continue;
                };
                let product = self.plant.bought_as.get(kind).copied().ok_or("a kind bought as no product")?;
                let price = prices.get(f.country).and_then(|p| p.get(usize::from(product))).copied().unwrap_or(0.0);
                let g = growth.get(f.country).copied().unwrap_or(0.0);
                let weights = sys_cap::rules::wear::steady_weights(classes, k.life, g);
                let per_efficient: f64 = weights.iter().zip(&chain.efficiency).map(|(w, e)| w * e).sum();
                let need = per * f.output;
                let kind16 = u16::try_from(kind).map_err(|e| e.to_string())?;
                let newest = Class { kind: kind16, band: 0, condition: 0, zone: f.region };
                let units = phx_core::units::issue_chain(&mut self.goods.units, newest, conditions);
                for ((unit, w), value) in units.iter().zip(&weights).zip(&chain.value) {
                    let n = floor_to_i64((need * w / per_efficient).round()).unwrap_or(0);
                    if n <= 0 {
                        continue;
                    }
                    let cost = floor_to_i64((from_i64(n) * value * price).round()).unwrap_or(0);
                    let opened = Flow {
                        payer: NATURE,
                        payee: f.key,
                        amount: n,
                        source: u32::from(kind16),
                        denomination: Denom::units(*unit),
                        reason: BUILT,
                        order: 0,
                    };
                    if self.goods.stocks.apply(&opened, Bound::Free, Cost::At(cost), today).is_err() {
                        violation!(clause = "CAP.1", "plant opened that could not be held");
                    }
                }
            }
        }
        Ok(())
    }

    /// The output a day a firm's plant allows its way: its scarcest kind's efficient units over the way's plant of
    /// the kind a unit of output a year; without end where its way needs no plant.
    #[clause("CAP.9", "CAP.1")]
    pub(crate) fn plant_capacity(&self, key: PartyKey, (country, product): (usize, u16)) -> f64 {
        let Some(needs) = self.plant.needs.get(country * self.plant.products + usize::from(product)) else {
            return f64::INFINITY;
        };
        let efficient: Vec<f64> = (0..self.plant.chains.len())
            .map(|k| {
                let (Some(chain), Ok(kind)) = (self.plant.chains.get(k), u16::try_from(k)) else { return 0.0 };
                phx_core::units::capacity(&self.goods.stocks, &self.goods.units, key, (kind, None), chain)
            })
            .collect();
        match sys_cap::rules::capacity::capacity(&efficient, needs) {
            Missing::Present(a_year) => a_year / DAYS_A_YEAR,
            Missing::Absent => f64::INFINITY,
        }
    }

    /// A making held to what the maker's plant allows: counted where plant bound it, and where it went beyond.
    #[clause("CAP.9")]
    pub(crate) fn count_plant(&mut self, made: i64, allowed: f64) {
        let made = from_i64(made);
        if made > allowed {
            self.plant.today.beyond += 1;
        } else if made + 1.0 > allowed {
            self.plant.today.bound += 1;
        }
    }

    /// On each review day, every holder's plant worn along its kinds' chains over the review period; the value its
    /// units lost charged to its income as depreciation.
    #[clause("CAP.6", "CAP.8", "REP.24")]
    pub(crate) fn wear_plant(&mut self, day: Day, moved: &mut Vec<Flow>) {
        let period = self.plant.review_days;
        if period == 0 || !(day.get() - self.plant.began).is_multiple_of(period) {
            return;
        }
        let mut holders: Vec<PartyKey> = Vec::new();
        for (k, store) in self.kinds.iter().enumerate() {
            let Ok(kind) = u8::try_from(k) else { continue };
            holders.extend(store.parties.live_slots().map(|s| PartyKey::new(kind, s)));
        }
        let chains = self.plant.chains.clone();
        for holder in holders {
            let before = self.capital_cost(holder);
            let mut flows = Vec::new();
            phx_core::units::wear(
                (&mut self.goods.stocks, &self.goods.units),
                holder,
                |k| chains.get(usize::from(k)),
                (i64::from(period), phx_core::consts::DAYS_365),
                WORN,
                &mut flows,
            );
            if flows.is_empty() {
                continue;
            }
            for f in flows.iter().filter(|f| f.payee == NATURE) {
                let next = f.payer == holder && self.next_condition_held(f.denomination.unit());
                if next {
                    self.plant.today.worn += f.amount;
                } else {
                    self.plant.today.retired += f.amount;
                }
            }
            let lost = before - self.capital_cost(holder);
            self.recognise(holder, Line::Depreciation, i64::try_from(lost).unwrap_or(i64::MAX));
            moved.extend(flows);
        }
    }

    /// Whether a capital unit's class has a next condition in its chain.
    fn next_condition_held(&self, unit: u16) -> bool {
        match self.goods.units.held(unit) {
            Some(Held::Capital(c)) => self
                .plant
                .chains
                .get(usize::from(c.kind))
                .is_some_and(|ch| usize::from(c.condition) + 1 < ch.value.len()),
            _ => false,
        }
    }

    /// What a holder's capital units cost.
    fn capital_cost(&self, holder: PartyKey) -> i128 {
        self.goods
            .stocks
            .holdings(holder)
            .filter(|h| matches!(self.goods.units.held(h.unit), Some(Held::Capital(_))))
            .map(|h| i128::from(h.cost))
            .sum()
    }

    /// The products plant is bought as.
    pub(crate) fn plant_products(&self) -> Vec<u16> {
        self.plant.bought_as.clone()
    }

    /// The kind of plant a unit's product is bought as, if any.
    pub(crate) fn plant_kind_of_unit(&self, unit: u16) -> Option<u16> {
        match self.goods.units.held(unit) {
            Some(Held::Good(g)) => self.plant_kind(g.product),
            _ => None,
        }
    }

    /// The kind of plant a product is bought as, if any.
    pub(crate) fn plant_kind(&self, product: u16) -> Option<u16> {
        self.plant.bought_as.iter().position(|p| *p == product).and_then(|k| u16::try_from(k).ok())
    }

    /// A capital good delivered to its buyer as investment: a project at what it paid, until its kind's lead passes.
    #[clause("CAP.2", "CAP.5")]
    pub(crate) fn start_project(
        &mut self,
        (holder, producer): (PartyKey, PartyKey),
        (kind, units, cost): (u16, i64, i64),
        day: Day,
    ) {
        let Some(lead) = self.plant.lead.get(usize::from(kind)).copied() else {
            violation!(clause = "CAP.5", "a project of a kind with no lead", kind = kind);
        };
        let ready = (0..lead).fold(day, |d, _| d.succ());
        self.plant.projects.push(Project { holder, producer, kind, units, cost, delivered: day, ready });
        self.plant.today.invested += cost;
    }

    /// Each project whose lead has passed entered service as new plant of its kind at its holder's region, at what
    /// it cost.
    #[clause("CAP.5", "CAP.8")]
    pub(crate) fn complete_projects(&mut self, day: Day, moved: &mut Vec<Flow>) {
        let (ready, waiting): (Vec<Project>, Vec<Project>) =
            std::mem::take(&mut self.plant.projects).into_iter().partition(|p| p.ready <= day);
        self.plant.projects = waiting;
        let conditions = self.plant.chains.first().map_or(0, |c| c.value.len());
        let Ok(conditions) = u8::try_from(conditions) else { return };
        for p in ready {
            let Some(region) = self.region_of(p.holder) else {
                violation!(clause = "CAP.5", "a project whose holder stands in no region", holder = p.holder.word());
            };
            let newest = Class { kind: p.kind, band: 0, condition: 0, zone: region };
            let units = phx_core::units::issue_chain(&mut self.goods.units, newest, conditions);
            let Some(unit) = units.first().copied() else { continue };
            let built = Flow {
                payer: NATURE,
                payee: p.holder,
                amount: p.units,
                source: u32::from(p.kind),
                denomination: Denom::units(unit),
                reason: BUILT,
                order: 0,
            };
            let _ = self.move_goods(built, Cost::At(p.cost), day, moved);
            self.plant.today.completed += p.units;
        }
    }

    /// The region a firm or estate stands in, by its record.
    fn region_of(&self, party: PartyKey) -> Option<u32> {
        let store = self.kinds.get(usize::from(party.kind()))?;
        match store.record(party.slot()).get(crate::consts::firm::REGION)?.get() {
            Missing::Present(r) => u32::try_from(r).ok(),
            Missing::Absent => None,
        }
    }

    /// What a party's projects cost, an asset of it until they enter service.
    #[must_use]
    pub fn projects_of(&self, party: PartyKey) -> i128 {
        self.plant.projects.iter().filter(|p| p.holder == party).map(|p| i128::from(p.cost)).sum()
    }

    /// A holder's projects passed to its successor, as its goods pass.
    pub(crate) fn pass_projects(&mut self, from: PartyKey, to: PartyKey) {
        for p in self.plant.projects.iter_mut().filter(|p| p.holder == from) {
            p.holder = to;
        }
    }

    /// The day's plant record kept, and the next begun.
    pub(crate) fn close_plant_day(&mut self, day: Day) {
        let mut record = std::mem::take(&mut self.plant.today);
        record.day = day.get();
        self.plant.days.push(record);
    }
}
