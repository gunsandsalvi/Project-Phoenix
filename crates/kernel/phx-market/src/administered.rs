use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, violation};

use crate::order::Side;
use crate::print::{Buyer, Match};

/// A facility a named institution stands behind at a rate or price it declares: which way it deals with those who
/// come, the rate, and the most it meets in a meeting, where its law bounds it.
#[clause("MKT.8")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facility {
    pub institution: PartyId,
    pub side: Side,
    pub rate: PriceRaw,
    pub limit: Missing<i64>,
}

/// A request to a facility: who asks and how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Want {
    pub party: PartyId,
    pub qty: i64,
}

/// The administered form: the institution meets the quantity that comes to it at its declared rate, each request
/// as far as its quantity response grants — the rule handle its system registers, which reads the request against
/// eligibility and collateral — and within the facility's limit, the requests taken in the order of their parties.
/// It is the one price not formed by those who trade, and exists only with that response.
#[clause("MKT.8")]
pub fn administer(facility: &Facility, wants: &[Want], response: &dyn Fn(&Want) -> i64) -> Vec<Match> {
    let mut order: Vec<&Want> = wants.iter().collect();
    order.sort_by_key(|w| w.party);
    let mut left = facility.limit;
    let mut out = Vec::new();
    for w in order {
        let granted = response(w);
        if granted < 0 || granted > w.qty {
            violation!(clause = "MKT.8", "a quantity response beyond what was asked", party = w.party.get());
        }
        let qty = match left {
            Missing::Present(l) if l < granted => l,
            _ => granted,
        };
        if let Missing::Present(l) = &mut left {
            *l -= qty;
        }
        if qty == 0 {
            continue;
        }
        let (buyer, seller) = match facility.side {
            Side::Sell => (w.party, facility.institution),
            Side::Buy => (facility.institution, w.party),
        };
        out.push(Match { buyer: Buyer::Party(buyer), seller, qty, price: facility.rate, draws: Missing::Absent });
    }
    out
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_num::{Missing, PriceRaw};

    use super::{Facility, Want, administer};
    use crate::order::Side;

    #[test]
    fn a_facility_meets_what_its_response_grants_within_its_limit() {
        let cb = PartyId::new(1);
        let facility =
            Facility { institution: cb, side: Side::Sell, rate: PriceRaw::from_raw(450), limit: Missing::Present(150) };
        let wants = [Want { party: PartyId::new(3), qty: 100 }, Want { party: PartyId::new(2), qty: 80 }];
        // Party 3 has collateral for 60 only.
        let response = |w: &Want| if w.party == PartyId::new(3) { 60 } else { w.qty };
        let met = administer(&facility, &wants, &response);
        let got: Vec<(u64, i64)> = met.iter().map(|m| (m.seller.get(), m.qty)).collect();
        assert_eq!(got, vec![(1, 80), (1, 60)], "in the parties' order, within the response and the limit");
        let tight = Facility { limit: Missing::Present(100), ..facility };
        assert_eq!(administer(&tight, &wants, &response).iter().map(|m| m.qty).collect::<Vec<_>>(), vec![80, 20]);
    }
}
