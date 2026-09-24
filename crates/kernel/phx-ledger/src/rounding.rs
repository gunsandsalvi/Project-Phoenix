use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{Money, Round, capacity_exceeded, div_round, violation};

/// Who the residue of a rounded split lands on: the payer, who keeps it; the payee with the largest share; or a
/// party the governing contract or law names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Residue {
    Payer,
    Payee,
    Named(PartyId),
}

/// How the contract or law that governs an amount rounds it, and where its residue lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoundingLanding {
    pub convention: Round,
    pub residue_to: Residue,
}

/// A total split by weights: each party's whole-unit share, and what the payer pays, their sum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Split {
    pub shares: Vec<(PartyId, Money)>,
    pub paid: Money,
}

/// A total split among parties by their weights, each share rounded by the convention, the residue landing where the
/// convention says, so no fraction of a unit is paid and none is lost.
#[clause("MON.16")]
#[must_use]
pub fn split(total: Money, weights: &[(PartyId, i64)], landing: RoundingLanding) -> Split {
    let all: i128 = weights.iter().map(|(_, w)| i128::from(*w)).sum();
    if all <= 0 || weights.iter().any(|(_, w)| *w < 0) {
        violation!(clause = "MON.16", "a split by weights that are not all positive", total = total.amt());
    }
    let mut shares: Vec<(PartyId, Money)> = weights
        .iter()
        .map(|(p, w)| {
            let s = div_round(i128::from(total.amt()) * i128::from(*w), all, landing.convention);
            let Ok(s) = i64::try_from(s) else {
                capacity_exceeded!("a share of a split", i64::MAX, s);
            };
            (*p, Money::new(s, total.ccy()))
        })
        .collect();
    let rounded: i64 = shares.iter().map(|(_, m)| m.amt()).sum();
    let residue = total.amt() - rounded;
    let onto = match landing.residue_to {
        Residue::Payer => None,
        Residue::Payee => {
            let mut best = 0;
            for (i, (_, w)) in weights.iter().enumerate() {
                if weights.get(best).is_some_and(|(_, b)| w > b) {
                    best = i;
                }
            }
            Some(best)
        }
        Residue::Named(p) => Some(if let Some(i) = shares.iter().position(|(q, _)| *q == p) {
            i
        } else {
            shares.push((p, Money::new(0, total.ccy())));
            shares.len() - 1
        }),
    };
    if let Some(i) = onto
        && let Some((_, m)) = shares.get_mut(i)
    {
        *m = Money::new(m.amt() + residue, total.ccy());
    }
    let paid = Money::new(shares.iter().map(|(_, m)| m.amt()).sum(), total.ccy());
    Split { shares, paid }
}
