use phx_id::MarketId;
use phx_macros::clause;
use phx_num::Missing;

/// How a market meets: one of the six forms, the call also over a network of zones and lines, or of lenders and
/// borrowers.
#[clause("MKT.1", "MKT.3", "MKT.4", "MKT.5", "MKT.6", "MKT.7", "MKT.8")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Form {
    Call,
    CoupledCall,
    LinkedCall,
    Book,
    Dealer,
    Posted,
    Bilateral,
    Administered,
}

/// One rule of the sequence a call's operator declares for choosing among prices that clear equally well.
#[clause("MKT.21")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieRule {
    MaxVolume,
    MinImbalance,
    /// Skipped where the market has no print yet.
    NearestLastPrint,
    LowerPrice,
}

/// How quantity at the clearing price is shared among the orders there: in proportion to what each asked, or in the
/// order of a declared priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ration {
    ProRata,
    Priority,
}

/// What a market is over, keyed as that thing is keyed: the kind of thing (an instrument, a good at a place, labour
/// of an occupation in a region) and its identity within that kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarketKey {
    pub kind: &'static str,
    pub subject: u64,
}

/// A market as its operator declares it: what it is over, its form, the schedule of its meeting days, the business
/// days its trades take to settle, the rule handle that says who may take part, its tick, the tie sequence and
/// rationing of a call, the stream its lots are drawn from, and the rule handles for an administered facility's
/// quantity response and for admission.
#[clause("MKT.1", "MKT.21")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarketDecl {
    pub id: MarketId,
    pub name: &'static str,
    pub key: MarketKey,
    pub form: Form,
    pub operator: &'static str,
    pub meeting_days: &'static str,
    pub settle_days: u16,
    pub participants: &'static str,
    pub tick: i64,
    pub ties: &'static [TieRule],
    pub ration: Ration,
    pub stream: &'static str,
    pub quantity_response: Missing<&'static str>,
    pub admission: Missing<&'static str>,
}

/// Whether a form meets as a call, where the tie sequence decides among clearing prices.
fn calls(form: Form) -> bool {
    matches!(form, Form::Call | Form::CoupledCall | Form::LinkedCall | Form::Book)
}

/// What assembly refuses in a market's declaration: a tick of no size; a call whose tie sequence can leave a tie,
/// since only the lower price chooses one price from any set; an administered price with no quantity response.
#[clause("MKT.8", "MKT.21")]
#[must_use]
pub fn refusals(decl: &MarketDecl) -> Vec<String> {
    let mut out = Vec::new();
    if decl.tick <= 0 {
        out.push(format!("market `{}` declares a tick of no size", decl.name));
    }
    if calls(decl.form) && decl.ties.last() != Some(&TieRule::LowerPrice) {
        out.push(format!("market `{}`'s tie sequence does not end in a rule that leaves no tie", decl.name));
    }
    if decl.form == Form::Administered && matches!(decl.quantity_response, Missing::Absent) {
        out.push(format!("market `{}` administers a price with no quantity response", decl.name));
    }
    out
}

#[cfg(test)]
mod tests {
    use phx_id::MarketId;
    use phx_num::Missing;

    use super::{Form, MarketDecl, MarketKey, Ration, TieRule, refusals};

    const DECL: MarketDecl = MarketDecl {
        id: MarketId::new(0),
        name: "a session",
        key: MarketKey { kind: "instrument", subject: 7 },
        form: Form::Call,
        operator: "exchange",
        meeting_days: "business days",
        settle_days: 2,
        participants: "members",
        tick: 1,
        ties: &[TieRule::MaxVolume, TieRule::LowerPrice],
        ration: Ration::ProRata,
        stream: "MKT.session",
        quantity_response: Missing::Absent,
        admission: Missing::Absent,
    };

    #[test]
    fn declarations_refused() {
        assert!(refusals(&DECL).is_empty());
        assert_eq!(refusals(&MarketDecl { ties: &[TieRule::MaxVolume], ..DECL }).len(), 1, "a sequence that can tie");
        assert_eq!(refusals(&MarketDecl { tick: 0, ..DECL }).len(), 1);
        let facility = MarketDecl { form: Form::Administered, ties: &[], ..DECL };
        assert_eq!(refusals(&facility).len(), 1, "an administered price needs its quantity response");
        assert!(refusals(&MarketDecl { quantity_response: Missing::Present("lend"), ..facility }).is_empty());
    }
}
