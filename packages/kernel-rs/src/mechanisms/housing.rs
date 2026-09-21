//! HOUSING: a dwelling is a durable, immovable, indivisible asset owned by a named party in a named
//! location, and the price of one is what a bid and a reservation crossed at.
//!
//! @spec 40 A1 · 40 A1.a · 40 A2 · 40 A3 · 40 A4 · 40 A5 · 40 B1 · 40 B1.a · 40 B1.b · 40 B1.c ·
//! @spec 40 B2 · 40 B2.a · 40 B3 · 40 B4 · 40 B4.a · 40 B5 · 40 C1 · 40 C2 · 40 C3 · 40 C4 ·
//! @spec 40 C4.a · 40 C5 · 40 C5.a · 40 C5.b · 40 E1 · 40 E2 · 40 E3 · 40 E4 · XI-2 · Law 3, Law 5,
//! @spec Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId, RegionId};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{agreed, standing, Standard};

/// A durable, immovable, indivisible asset owned by a named party, in a named location.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Dwelling {
    pub owner: PartyId,
    /// Part of the identity, which is why there is no single housing market.
    pub at: RegionId,
    /// Who lives in it.
    pub occupier: PartyId,
    /// It depreciates and needs maintenance, which is a real cost to the owner, per period.
    pub upkeep: f64,
}

impl Dwelling {
    /// What the occupier pays the owner for the shelter it consumes.
    pub fn rent_flows(&self, rent: f64) -> Option<(PartyId, PartyId, f64)> {
        if self.owner == self.occupier {
            return None;
        }
        Some((self.occupier, self.owner, rent))
    }
}

/// A loan from a named lender secured on the house, held as a row like any other loan.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mortgage {
    pub lender: PartyId,
    pub borrower: PartyId,
    /// What is still owed.
    pub principal: f64,
    pub rate: f64,
    /// Fixed or floating.
    pub floats: bool,
    pub periods_left: u32,
}

impl Mortgage {
    /// Interest and principal, both.
    pub fn instalment(&self) -> f64 {
        assert!(self.periods_left > 0, "40 C2: a mortgage with no term left has no schedule");
        self.principal / (self.periods_left as f64) + self.principal * self.rate
    }
}

/// The loan-to-value is a read of the loan against the house's CURRENT price, and it moves when the
/// price moves without anybody doing anything.
pub fn loan_to_value(m: &Mortgage, price_now: Option<f64>) -> Option<f64> {
    let price = price_now?;
    if price <= 0.0 {
        return None;
    }
    Some(m.principal / price)
}

/// An owner whose tenure ends, with a reservation: what it must fetch to discharge its own mortgage,
/// and never below what it costs to build.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Offer {
    pub seller: PartyId,
    pub at: RegionId,
    pub reservation: f64,
}

impl Offer {
    /// The reservation, built from the seller's own position.
    pub fn reserving(seller: PartyId, at: RegionId, owed: f64, cost_to_build: f64) -> Offer {
        let reservation = if owed > cost_to_build { owed } else { cost_to_build };
        Offer { seller, at, reservation }
    }
}

/// What a buyer can borrow at the keenest quote available to it — which is why the mortgage rate and
/// the lending standard are the dominant inputs to the price.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bid {
    pub buyer: PartyId,
    pub at: RegionId,
    pub bidding: f64,
}

/// What this buyer can borrow, from the standard it faces and its own income and deposit — and
/// therefore what it can bid.
pub fn can_bid(income: f64, deposit: f64, standard: &Standard) -> f64 {
    let borrowable = income * standard.income_multiple;
    let by_deposit = deposit / standard.deposit_share;
    let funded = if borrowable < by_deposit { borrowable } else { by_deposit };
    funded + deposit
}


/// What a location's book produced.
#[derive(Clone, Debug)]
pub struct Cleared {
    pub trades: Vec<(PartyId, PartyId, f64)>,
    /// The price is what the marginal pair crossed at.
    pub print: Option<f64>,
    pub unsold: usize,
}

/// It clears between buyers and sellers, per location.
pub fn clearing(bids: &[Bid], offers: &[Offer], at: RegionId) -> Cleared {
    let mut buying: Vec<&Bid> = bids.iter().filter(|b| b.at == at).collect();
    let mut selling: Vec<&Offer> = offers.iter().filter(|o| o.at == at).collect();
    buying.sort_by(|a, b| b.bidding.total_cmp(&a.bidding));
    selling.sort_by(|a, b| a.reservation.total_cmp(&b.reservation));

    let mut trades = Vec::new();
    let mut print = None;
    let mut sold = 0usize;
    for (bid, offer) in buying.iter().zip(selling.iter()) {
        if bid.bidding < offer.reservation {
            break;
        }
        // The cross.
        trades.push((offer.seller, bid.buyer, offer.reservation));
        print = Some(offer.reservation);
        sold += 1;
    }
    Cleared { trades, print, unsold: selling.len() - sold }
}

/// A price index built only from transactions is measuring a changing sample, and that is a real
/// property of housing data rather than an error to correct away.
pub fn sample(cleared: &Cleared, offered: usize) -> Option<f64> {
    if offered == 0 {
        return None;
    }
    Some(cleared.trades.len() as f64 / offered as f64)
}

/// The rent and the price are linked but not equal — the yield is a READ of the two, and it competes
/// with other yields.
pub fn yield_on(rent_per_period: f64, price: f64) -> Option<f64> {
    if price <= 0.0 {
        return None;
    }
    Some(rent_per_period / price)
}

/// A foreclosure moves a dwelling — from the household to the lender — and the foreclosed supply
/// RETURNS TO THE MARKET.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Foreclosed {
    /// The dwelling as the register now holds it: the same house, the lender on it as owner.
    pub now: Dwelling,
    pub to: PartyId,
    /// And the offer that now stands in the location's book, at what the lender must fetch.
    pub returns_as: Offer,
}

pub fn foreclose(house: Dwelling, m: &Mortgage, cost_to_build: f64) -> Foreclosed {
    assert!(
        m.borrower == house.owner,
        "40 C4: a lender forecloses on its own borrower's house and on nobody else's"
    );
    Foreclosed {
        now: Dwelling { owner: m.lender, ..house },
        to: m.lender,
        returns_as: Offer::reserving(m.lender, house.at, m.principal, cost_to_build),
    }
}

/// Mortgage debt owed by households equals mortgage assets held by lenders and pools, exactly.
pub fn debts_match_assets(owed_by_households: f64, held_by_lenders: f64, terms: usize) -> bool {
    (owed_by_households - held_by_lenders).abs()
        <= crate::num::dust(terms, &[owed_by_households, held_by_lenders])
}


/// DWELLINGS ARE LET AND SOLD, AND BOTH PRICES CLEAR.
pub struct Housing {
    pub kind: u32,
    pub lets: u32,
    /// What a dwelling costs its owner to keep, per period.
    pub upkeep: &'static str,
    /// What share of its money a household will put towards a roof.
    pub will_spend: &'static str,
}

impl Mechanism for Housing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::stores::Standard;
        let will_spend = ctx.params().ratio(self.will_spend);
        let _upkeep = ctx.params().price_per_unit(self.upkeep);

        // The standard each lender is currently lending at.
        let mut keenest: Option<Standard> = None;
        for row in 0..ctx.standing().len() as u32 {
            let st = crate::stores::StandingId(row);
            if !ctx.standing().live(st) || ctx.standing().kind_of(st) != standing::LENDING_STANDARD {
                continue;
            }
            let terms = ctx.standing().terms(st);
            let here = Standard { income_multiple: terms[0], deposit_share: terms[1] };
            keenest = Some(match keenest {
                Some(best) if best.income_multiple >= here.income_multiple => best,
                _ => here,
            });
        }
        // A buyer with no lender cannot bid.
        let Some(standard) = keenest else { return };

        // The dwellings, where they are, and who lives in them.
        let mut offers: Vec<Offer> = Vec::new();
        let mut spare: Vec<(PartyId, crate::ids::RegionId)> = Vec::new();
        for row in 0..ctx.instruments().len() as u32 {
            let line = InstrumentId::at(row);
            if !crate::places::is_a_structure(ctx.registry(), line) {
                continue;
            }
            for &holding in ctx.register().of_instrument(line) {
                let holding = crate::ids::HoldingId(holding);
                let owner = ctx.register().holder_of(holding);
                if !ctx.parties().alive(owner) || ctx.register().free(holding) <= 0.0 {
                    continue;
                }
                let at = ctx.parties().region_of(owner);
                // It will not sell below what it owes or what a dwelling costs to build there,
                // whichever is more — and the build cost is higher where more already stands.
                let Some(print) = ctx.prints().latest(line, ctx.period()) else { continue };
                let owed: f64 = ctx
                    .schedules()
                    .of_payer(owner)
                    .iter()
                    .map(|r| crate::stores::DueId(*r))
                    .filter(|d| !ctx.schedules().paid(*d))
                    .map(|d| ctx.schedules().amount(d))
                    .sum();
                offers.push(Offer::reserving(owner, at, owed, print.price));
                // And an owner holding more than one has a roof to let.
                if ctx.register().quantity(holding) > 1.0 {
                    spare.push((owner, at));
                }
            }
        }
        if offers.is_empty() {
            return;
        }

        // What each household can fund.
        let mut bids: Vec<Bid> = Vec::new();
        let mut renting: Vec<(PartyId, crate::ids::RegionId, f64)> = Vec::new();
        for &household in ctx.parties().of_kind(kinds::HOUSEHOLD) {
            let who = PartyId(household);
            if !ctx.parties().alive(who) {
                continue;
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let deposit = ctx.register().quantity(ctx.register().row(who, money)) * will_spend;
            if deposit <= 0.0 {
                continue;
            }
            let at = ctx.parties().region_of(who);
            let bidding = match ctx.outlooks().of(who, crate::stores::about::WHAT_IT_KEEPS_EARNING) {
                Some(income) => can_bid(income, deposit, &standard),
                None => deposit,
            };
            bids.push(Bid { buyer: who, at, bidding });
            renting.push((who, at, deposit));
        }

        // Per LOCATION.
        let mut sold: Vec<(PartyId, PartyId, f64)> = Vec::new();
        let mut printed: Vec<(u32, f64)> = Vec::new();
        let mut places: Vec<u32> = offers.iter().map(|o| o.at.0).collect();
        places.sort_unstable();
        places.dedup();
        for place in places {
            let at = crate::ids::RegionId::at(place);
            let cleared = clearing(&bids, &offers, at);
            if let Some(print) = cleared.print {
                printed.push((place, print));
            }
            sold.extend(cleared.trades);
        }

        // And the letting session — a spare roof, and a household without one.
        let mut let_to: Vec<(PartyId, PartyId, f64)> = Vec::new();
        for (owner, at) in spare {
            let Some((tenant, _, can_pay)) = renting.iter().copied().find(|(_, where_it_is, _)| *where_it_is == at)
            else {
                continue;
            };
            if tenant == owner {
                continue;
            }
            // The rent is what this session crossed at — what the tenant will pay against a roof
            // that is standing empty, and an owner that will not let at it keeps it empty.
            let_to.push((owner, tenant, can_pay));
        }

        for (seller, buyer, price) in sold {
            // A loan from a NAMED lender, secured on the house.
            ctx.agrees(crate::module::Agrees {
                kind: agreed::MORTGAGE,
                one: seller,
                other: buyer,
                terms: crate::stores::AgreementTerms::Mortgage { purchase_price: price, deposit_share: standard.deposit_share },
                until: None,
            });
            ctx.say(self.kind, &[seller.0, buyer.0], &[(0, Value::Num(price))], true);
            // Both sides observed this house clear. Their histories remain separate even though
            // this transaction is one public fact.
            ctx.observe(seller, crate::stores::about::WHAT_A_HOUSE_IS_WORTH, price);
            ctx.observe(buyer, crate::stores::about::WHAT_A_HOUSE_IS_WORTH, price);
        }
        for (owner, tenant, rent) in let_to {
            // A tenancy is a relation, and the rent is its term.
            ctx.agrees(crate::module::Agrees {
                kind: agreed::TENANCY,
                one: owner,
                other: tenant,
                terms: crate::stores::AgreementTerms::Tenancy { rent },
                until: None,
            });
            ctx.say(self.lets, &[owner.0, tenant.0], &[(0, Value::Num(rent))], true);
        }
        for (place, print) in printed {
            ctx.say(self.kind, &[], &[(0, Value::Num(f64::from(place))), (1, Value::Num(print))], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn here() -> RegionId {
        RegionId::at(1)
    }

    fn there() -> RegionId {
        RegionId::at(2)
    }

    fn dwelling(owner: u32, occupier: u32) -> Dwelling {
        Dwelling { owner: party(owner), at: here(), occupier: party(occupier), upkeep: 12.0 }
    }

    fn mortgage(lender: u32, borrower: u32, principal: f64) -> Mortgage {
        Mortgage {
            lender: party(lender),
            borrower: party(borrower),
            principal,
            rate: 0.01,
            floats: true,
            periods_left: 100,
        }
    }

    #[test]
    fn a_landlord_is_the_owner_of_a_dwelling_somebody_lives_in() {
        // A rental stock must have dwellings behind it, not a firm producing an abstract service.
        let let_out = dwelling(10, 20);
        assert_eq!(let_out.rent_flows(30.0), Some((party(20), party(10), 30.0)));
        assert!(dwelling(10, 10).rent_flows(30.0).is_none());
    }

    #[test]
    fn the_loan_to_value_moves_when_the_price_moves_without_anybody_doing_anything() {
        // It is a READ of the loan against the house's current price.
        let m = mortgage(1, 20, 800.0);
        assert_eq!(loan_to_value(&m, Some(1_000.0)), Some(0.8));
        assert_eq!(loan_to_value(&m, Some(800.0)), Some(1.0));
        // And an unpriced house has no loan-to-value rather than a convenient one.
        assert!(loan_to_value(&m, None).is_none());
    }

    #[test]
    fn a_mortgage_payment_is_interest_and_principal() {
        // A payment that is only interest never amortises and the loan never ends.
        let m = mortgage(1, 20, 1_000.0);
        assert!(m.instalment() > m.principal * m.rate);
    }

    #[test]
    fn an_offer_no_bid_reaches_does_not_clear() {
        // A seller can refuse, and in a falling market transaction volumes collapse before prices
        // do.
        let offers = [
            Offer::reserving(party(10), here(), 900.0, 700.0),
            Offer::reserving(party(11), here(), 1_400.0, 700.0),
        ];
        let bids = [Bid { buyer: party(20), at: here(), bidding: 1_000.0 }];
        let cleared = clearing(&bids, &offers, here());
        assert_eq!(cleared.trades.len(), 1);
        assert_eq!(cleared.print, Some(900.0));
        assert_eq!(cleared.unsold, 1);
    }

    #[test]
    fn a_book_where_no_bid_reaches_any_reservation_prints_nothing() {
        // Volumes go first.
        let offers = [Offer::reserving(party(10), here(), 1_200.0, 700.0)];
        let bids = [Bid { buyer: party(20), at: here(), bidding: 800.0 }];
        let cleared = clearing(&bids, &offers, here());
        assert!(cleared.print.is_none());
        assert_eq!(cleared.unsold, 1);
    }

    #[test]
    fn location_is_part_of_the_identity_so_there_is_no_single_housing_market() {
        // A bid here does not reach an offer there.
        let offers = [Offer::reserving(party(10), there(), 900.0, 700.0)];
        let bids = [Bid { buyer: party(20), at: here(), bidding: 1_000.0 }];
        assert!(clearing(&bids, &offers, here()).trades.is_empty());
        assert!(clearing(&bids, &offers, there()).trades.is_empty());
    }

    #[test]
    fn a_reservation_is_never_below_what_it_costs_to_build() {
        // Both terms are facts about this seller.
        let cheap_debt = Offer::reserving(party(10), here(), 400.0, 700.0);
        let dear_debt = Offer::reserving(party(10), here(), 1_100.0, 700.0);
        assert_eq!(cheap_debt.reservation, 700.0);
        assert_eq!(dear_debt.reservation, 1_100.0);
    }

    #[test]
    fn a_price_index_from_transactions_is_measuring_a_changing_sample() {
        // A real property of housing data, not an error to correct away.
        let offers = [
            Offer::reserving(party(10), here(), 900.0, 700.0),
            Offer::reserving(party(11), here(), 1_400.0, 700.0),
        ];
        let bids = [Bid { buyer: party(20), at: here(), bidding: 1_000.0 }];
        let cleared = clearing(&bids, &offers, here());
        assert_eq!(sample(&cleared, 2), Some(0.5));
        assert!(sample(&cleared, 0).is_none());
    }

    #[test]
    fn a_foreclosure_moves_a_dwelling_and_the_supply_returns_to_the_market() {
        // The extra supply is what makes a falling price fall further.
        let house = dwelling(20, 20);
        let m = mortgage(1, 20, 950.0);
        let f = foreclose(house, &m, 700.0);
        assert_eq!(f.to, party(1));
        assert_eq!(f.now.owner, party(1));
        assert_eq!(f.now.at, house.at);
        assert_eq!(f.returns_as.seller, party(1));
        assert_eq!(f.returns_as.reservation, 950.0);
        assert_eq!(f.returns_as.at, here());
    }

    #[test]
    #[should_panic(expected = "on nobody else's")]
    fn a_lender_forecloses_on_its_own_borrowers_house() {
        foreclose(dwelling(21, 21), &mortgage(1, 20, 950.0), 700.0);
    }

    #[test]
    fn the_yield_is_a_read_of_the_rent_and_the_price_and_sets_neither() {
        // Linked but not equal, and it competes with other yields.
        assert_eq!(yield_on(50.0, 1_000.0), Some(0.05));
        assert!(yield_on(50.0, 0.0).is_none());
    }

    #[test]
    fn mortgage_debt_owed_equals_mortgage_assets_held() {
        // A VERIFY on derived dust.
        assert!(debts_match_assets(1_000_000.0, 1_000_000.0, 2));
        assert!(!debts_match_assets(1_000_000.0, 999_000.0, 2));
    }

    #[test]
    fn a_tighter_standard_lowers_what_a_buyer_can_bid_which_is_the_dominant_input_to_the_price() {
        // The buyer's demand is governed by what it can BORROW, not only what it wants. What the
        // lender is lending at is read, not computed here.
        let calm = Standard { income_multiple: 4.0, deposit_share: 0.10 };
        let worried = Standard { income_multiple: 2.5, deposit_share: 0.25 };
        assert!(can_bid(100.0, 200.0, &worried) < can_bid(100.0, 200.0, &calm));
    }
}
