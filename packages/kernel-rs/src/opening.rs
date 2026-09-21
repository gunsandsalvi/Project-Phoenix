//! The validated description of week zero. It names opening facts but writes no kernel store.

use std::collections::{HashMap, HashSet};

use crate::calendar::Week;
use crate::instruments::Class;
use crate::ledger::{Cause, Delivery, Receipt};
use crate::params::Params;
use crate::parties::{LatticeKey, Representation};
use crate::stores::Owing;

#[derive(Clone, PartialEq, Debug)]
pub struct OpeningCurrency {
    pub id: String,
    pub issuer: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct OpeningUnit {
    pub id: String,
    pub pieces: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct OpeningParty {
    pub id: String,
    pub kind: u32,
    pub currency: String,
    pub bank: Option<String>,
    pub representation: Representation,
    pub key: LatticeKey,
}

#[derive(Clone, PartialEq, Debug)]
pub struct OpeningInstrument {
    pub id: String,
    pub issuer: String,
    pub currency: String,
    pub class: Class,
    pub unit: String,
    pub coupon: Option<f64>,
    pub matures: Option<Week>,
    pub issued: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct OpeningHolding {
    pub holder: String,
    pub instrument: String,
    pub units: f64,
    pub basis_per_unit: f64,
    pub carrying: crate::register::Carrying,
}

#[derive(Clone, PartialEq, Debug)]
pub enum OpeningOwed {
    Under { agreement: usize, to: String },
    On(String),
}

#[derive(Clone, PartialEq, Debug)]
pub struct OpeningObligation {
    pub owed_by: String,
    pub on: OpeningOwed,
    pub currency: String,
    pub amount: f64,
    pub from: Week,
    pub due: Week,
    pub of: Owing,
}

#[derive(Clone, PartialEq, Debug)]
pub struct OpeningAgreement {
    pub one: String,
    pub other: String,
    pub terms: OpeningAgreementTerms,
    pub from: Week,
    pub until: Option<Week>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum OpeningAgreementTerms {
    Engagement {
        wage_per_person: f64,
        hours_per_person: f64,
        heads: u32,
    },
    Mortgage {
        purchase_price: f64,
        deposit_share: f64,
    },
    Tenancy {
        rent: f64,
    },
    Mandate {
        minimum_grade: crate::stores::Grade,
    },
    FundSubscription {
        shares: f64,
        paid: f64,
    },
    PrivateCommitment {
        committed: f64,
    },
    PrimeBrokerage {
        lent: f64,
        limit: f64,
    },
    SecuritiesLoan {
        instrument: String,
        units: f64,
        fee: f64,
    },
    TradeCredit {
        amount: f64,
        due: Week,
    },
    PriceForward {
        underlying: String,
        struck_at: f64,
        notional: f64,
        years: f64,
        settlement: String,
    },
    CreditDefaultSwap {
        reference: String,
        spread: f64,
        tenor_years: f64,
        settlement: String,
    },
    FxForward {
        pays: String,
        receives: String,
        rate: f64,
        amount: f64,
        tenor_years: f64,
    },
    CentralBankFacility {
        principal: f64,
        rate: f64,
        settlement: String,
        collateral: Vec<(String, f64)>,
    },
}

impl OpeningAgreementTerms {
    fn validate(
        &self,
        at: &str,
        parties: &HashSet<&str>,
        currencies: &HashSet<&str>,
        instruments: &HashMap<&str, &OpeningInstrument>,
        faults: &mut Vec<OpeningFault>,
    ) {
        let numbers_valid = match self {
            Self::Engagement {
                wage_per_person,
                hours_per_person,
                heads,
            } => {
                wage_per_person.is_finite()
                    && *wage_per_person >= 0.0
                    && hours_per_person.is_finite()
                    && *hours_per_person > 0.0
                    && *heads > 0
            }
            Self::Mortgage {
                purchase_price,
                deposit_share,
            } => {
                purchase_price.is_finite()
                    && *purchase_price > 0.0
                    && deposit_share.is_finite()
                    && (0.0..=1.0).contains(deposit_share)
            }
            Self::Tenancy { rent } => rent.is_finite() && *rent >= 0.0,
            Self::Mandate { .. } => true,
            Self::FundSubscription { shares, paid } => {
                shares.is_finite() && *shares > 0.0 && paid.is_finite() && *paid > 0.0
            }
            Self::PrivateCommitment { committed } => committed.is_finite() && *committed > 0.0,
            Self::PrimeBrokerage { lent, limit } => {
                lent.is_finite() && *lent >= 0.0 && limit.is_finite() && *limit >= 0.0
            }
            Self::SecuritiesLoan {
                instrument,
                units,
                fee,
            } => {
                instruments.contains_key(instrument.as_str())
                    && units.is_finite()
                    && *units > 0.0
                    && fee.is_finite()
            }
            Self::TradeCredit { amount, .. } => amount.is_finite() && *amount > 0.0,
            Self::PriceForward {
                underlying,
                struck_at,
                notional,
                years,
                settlement,
            } => {
                instruments.contains_key(underlying.as_str())
                    && currencies.contains(settlement.as_str())
                    && struck_at.is_finite()
                    && notional.is_finite()
                    && *notional > 0.0
                    && years.is_finite()
                    && *years > 0.0
            }
            Self::CreditDefaultSwap {
                reference,
                spread,
                tenor_years,
                settlement,
            } => {
                parties.contains(reference.as_str())
                    && currencies.contains(settlement.as_str())
                    && spread.is_finite()
                    && *spread >= 0.0
                    && tenor_years.is_finite()
                    && *tenor_years > 0.0
            }
            Self::FxForward {
                pays,
                receives,
                rate,
                amount,
                tenor_years,
            } => {
                pays != receives
                    && currencies.contains(pays.as_str())
                    && currencies.contains(receives.as_str())
                    && rate.is_finite()
                    && *rate > 0.0
                    && amount.is_finite()
                    && *amount > 0.0
                    && tenor_years.is_finite()
                    && *tenor_years > 0.0
            }
            Self::CentralBankFacility {
                principal,
                rate,
                settlement,
                collateral,
            } => {
                principal.is_finite()
                    && *principal > 0.0
                    && rate.is_finite()
                    && currencies.contains(settlement.as_str())
                    && collateral.iter().all(|(line, units)| {
                        instruments.contains_key(line.as_str()) && units.is_finite() && *units > 0.0
                    })
            }
        };
        if !numbers_valid {
            fault(
                faults,
                at,
                "agreement terms are invalid or name an undeclared object",
            );
        }
    }

    fn resolve(&self, ids: &OpeningIds) -> (u32, crate::stores::AgreementTerms) {
        use crate::stores::{agreed, AgreementTerms as Terms};
        match self {
            Self::Engagement {
                wage_per_person,
                hours_per_person,
                heads,
            } => (
                agreed::ENGAGEMENT,
                Terms::Engagement {
                    wage_per_person: *wage_per_person,
                    hours_per_person: *hours_per_person,
                    heads: *heads,
                },
            ),
            Self::Mortgage {
                purchase_price,
                deposit_share,
            } => (
                agreed::MORTGAGE,
                Terms::Mortgage {
                    purchase_price: *purchase_price,
                    deposit_share: *deposit_share,
                },
            ),
            Self::Tenancy { rent } => (agreed::TENANCY, Terms::Tenancy { rent: *rent }),
            Self::Mandate { minimum_grade } => (
                agreed::MANDATE,
                Terms::Mandate {
                    minimum_grade: *minimum_grade,
                },
            ),
            Self::FundSubscription { shares, paid } => (
                agreed::SUBSCRIPTION,
                Terms::FundSubscription {
                    shares: *shares,
                    paid: *paid,
                },
            ),
            Self::PrivateCommitment { committed } => (
                agreed::PRIVATE_COMMITMENT,
                Terms::PrivateCommitment {
                    committed: *committed,
                },
            ),
            Self::PrimeBrokerage { lent, limit } => (
                agreed::PRIME_BROKERAGE,
                Terms::PrimeBrokerage {
                    lent: *lent,
                    limit: *limit,
                },
            ),
            Self::SecuritiesLoan {
                instrument,
                units,
                fee,
            } => (
                agreed::SECURITIES_LOAN,
                Terms::SecuritiesLoan {
                    instrument: ids.instruments[instrument],
                    units: *units,
                    fee: *fee,
                },
            ),
            Self::TradeCredit { amount, due } => (
                agreed::TRADE_CREDIT,
                Terms::TradeCredit {
                    amount: *amount,
                    due: *due,
                },
            ),
            Self::PriceForward {
                underlying,
                struck_at,
                notional,
                years,
                settlement,
            } => (
                agreed::DERIVATIVE,
                Terms::PriceForward {
                    underlying: ids.instruments[underlying],
                    struck_at: *struck_at,
                    notional: *notional,
                    years: *years,
                    settlement: ids.currencies[settlement],
                },
            ),
            Self::CreditDefaultSwap {
                reference,
                spread,
                tenor_years,
                settlement,
            } => (
                agreed::CDS,
                Terms::CreditDefaultSwap {
                    reference: ids.parties[reference],
                    spread: *spread,
                    tenor_years: *tenor_years,
                    settlement: ids.currencies[settlement],
                },
            ),
            Self::FxForward {
                pays,
                receives,
                rate,
                amount,
                tenor_years,
            } => (
                agreed::FX_FORWARD,
                Terms::FxForward {
                    pays: ids.currencies[pays],
                    receives: ids.currencies[receives],
                    rate: *rate,
                    amount: *amount,
                    tenor_years: *tenor_years,
                },
            ),
            Self::CentralBankFacility {
                principal,
                rate,
                settlement,
                collateral,
            } => (
                agreed::CENTRAL_BANK_FACILITY,
                Terms::CentralBankFacility {
                    principal: *principal,
                    rate: *rate,
                    settlement: ids.currencies[settlement],
                    collateral: collateral
                        .iter()
                        .map(|(line, units)| (ids.instruments[line], *units))
                        .collect(),
                },
            ),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum OpeningLeg {
    Money {
        from: String,
        to: String,
        instrument: String,
        amount: f64,
        receipt: Receipt,
    },
    Asset {
        from: String,
        to: String,
        instrument: String,
        units: f64,
    },
}

#[derive(Clone, PartialEq, Debug)]
pub struct OpeningInstruction {
    pub reason: String,
    pub cause: Cause,
    pub delivery: Delivery,
    pub legs: Vec<OpeningLeg>,
}

/// Everything a supported constructor must know before week one.
#[derive(Default, PartialEq, Debug)]
pub struct OpeningState {
    pub currencies: Vec<OpeningCurrency>,
    pub units: Vec<OpeningUnit>,
    pub parties: Vec<OpeningParty>,
    pub instruments: Vec<OpeningInstrument>,
    pub holdings: Vec<OpeningHolding>,
    pub obligations: Vec<OpeningObligation>,
    pub agreements: Vec<OpeningAgreement>,
    pub instructions: Vec<OpeningInstruction>,
    /// Parameters the opening-state constructor itself consumes.
    pub required_params: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OpeningFault {
    pub at: String,
    pub message: String,
}

impl OpeningState {
    /// Validate the complete description without mutating a world.
    pub fn validate(&self, params: &Params) -> Result<(), Vec<OpeningFault>> {
        let mut faults = Vec::new();
        let mut parties: HashSet<&str> = HashSet::new();
        for p in &self.parties {
            if p.id.is_empty() {
                fault(&mut faults, "party", "a party has no id");
            } else if !parties.insert(&p.id) {
                fault(&mut faults, &p.id, "party id is declared twice");
            }
            if p.currency.is_empty() {
                fault(&mut faults, &p.id, "party has no reporting currency");
            }
        }

        let mut currencies = HashSet::new();
        for currency in &self.currencies {
            if currency.id.is_empty() || !currencies.insert(currency.id.as_str()) {
                fault(
                    &mut faults,
                    &currency.id,
                    "currency id is empty or declared twice",
                );
            }
            if !parties.contains(currency.issuer.as_str()) {
                fault(
                    &mut faults,
                    &currency.id,
                    "currency issuer is not an opening party",
                );
            } else if self
                .parties
                .iter()
                .find(|party| party.id == currency.issuer)
                .is_some_and(|issuer| issuer.bank.is_some())
            {
                fault(
                    &mut faults,
                    &currency.id,
                    "currency issuer must bank nowhere",
                );
            }
        }
        let mut units = HashSet::new();
        for unit in &self.units {
            if unit.id.is_empty() || !units.insert(unit.id.as_str()) {
                fault(&mut faults, &unit.id, "unit id is empty or declared twice");
            }
            if unit.pieces == 0 {
                fault(&mut faults, &unit.id, "unit has no pieces");
            }
        }
        for party in &self.parties {
            if !currencies.contains(party.currency.as_str()) {
                fault(&mut faults, &party.id, "party currency is not declared");
            }
        }

        let mut instruments: HashMap<&str, &OpeningInstrument> = HashMap::new();
        for i in &self.instruments {
            if i.id.is_empty() {
                fault(&mut faults, "instrument", "an instrument has no id");
                continue;
            }
            if instruments.insert(&i.id, i).is_some() {
                fault(&mut faults, &i.id, "instrument id is declared twice");
            }
            if !parties.contains(i.issuer.as_str()) {
                fault(&mut faults, &i.id, "issuer is not an opening party");
            }
            if i.currency.is_empty() {
                fault(&mut faults, &i.id, "instrument has no currency");
            } else if !currencies.contains(i.currency.as_str()) {
                fault(&mut faults, &i.id, "instrument currency is not declared");
            }
            if !units.contains(i.unit.as_str()) {
                fault(&mut faults, &i.id, "instrument unit is not declared");
            }
            if !i.issued.is_finite() || i.issued < 0.0 {
                fault(
                    &mut faults,
                    &i.id,
                    "issued units are not finite and non-negative",
                );
            }
            if i.coupon
                .is_some_and(|coupon| !coupon.is_finite() || coupon < 0.0)
            {
                fault(&mut faults, &i.id, "coupon is not finite and non-negative");
            }
            if i.coupon.is_some() && i.class != Class::Claim {
                fault(&mut faults, &i.id, "only a claim may carry a coupon");
            }
        }

        let money_accounts: HashSet<(&str, &str)> = self
            .instruments
            .iter()
            .filter(|i| i.class == Class::Money)
            .map(|i| (i.issuer.as_str(), i.currency.as_str()))
            .collect();
        for p in &self.parties {
            if let Some(bank) = &p.bank {
                if !parties.contains(bank.as_str()) {
                    fault(&mut faults, &p.id, "bank is not an opening party");
                } else if !money_accounts.contains(&(bank.as_str(), p.currency.as_str())) {
                    fault(
                        &mut faults,
                        &p.id,
                        "bank issues no opening account in the party's currency",
                    );
                }
            }
        }

        let mut held: HashMap<&str, (f64, usize)> = HashMap::new();
        for h in &self.holdings {
            if !parties.contains(h.holder.as_str()) {
                fault(
                    &mut faults,
                    &h.holder,
                    "holding owner is not an opening party",
                );
            }
            if !h.units.is_finite() || h.units <= 0.0 {
                fault(
                    &mut faults,
                    &h.instrument,
                    "holding units are not finite and positive",
                );
            }
            if !h.basis_per_unit.is_finite() || h.basis_per_unit < 0.0 {
                fault(
                    &mut faults,
                    &h.instrument,
                    "holding basis is not finite and non-negative",
                );
            }
            if !instruments.contains_key(h.instrument.as_str()) {
                fault(
                    &mut faults,
                    &h.instrument,
                    "holding instrument is not declared",
                );
                continue;
            }
            let instrument = instruments[h.instrument.as_str()];
            if instrument.class == Class::Money {
                let holder = self.parties.iter().find(|party| party.id == h.holder);
                if holder.is_some_and(|party| {
                    party.id != instrument.issuer
                        && party.bank.as_deref() != Some(instrument.issuer.as_str())
                }) {
                    fault(
                        &mut faults,
                        &h.instrument,
                        "money is not the holder's bank account",
                    );
                }
            }
            let entry = held.entry(&h.instrument).or_insert((0.0, 0));
            entry.0 += h.units;
            entry.1 += 1;
        }
        for i in &self.instruments {
            let (total, terms) = held.get(i.id.as_str()).copied().unwrap_or((0.0, 0));
            let dust = crate::num::dust(terms + 2, &[total, i.issued]);
            if (total - i.issued).abs() > dust {
                fault(
                    &mut faults,
                    &i.id,
                    "opening holdings do not equal issued units",
                );
            }
        }

        for (n, o) in self.obligations.iter().enumerate() {
            let at = format!("obligation {n}");
            if !parties.contains(o.owed_by.as_str()) {
                fault(&mut faults, &at, "debtor is not an opening party");
            }
            if !o.amount.is_finite() || o.amount <= 0.0 {
                fault(&mut faults, &at, "amount is not finite and positive");
            }
            if o.from > o.due {
                fault(&mut faults, &at, "obligation begins after it falls due");
            }
            if o.currency.is_empty() {
                fault(&mut faults, &at, "obligation has no currency");
            } else if !currencies.contains(o.currency.as_str()) {
                fault(&mut faults, &at, "obligation currency is not declared");
            }
            match &o.on {
                OpeningOwed::Under { agreement, to } => match self.agreements.get(*agreement) {
                    None => fault(&mut faults, &at, "obligation agreement is not declared"),
                    Some(contract)
                        if !((contract.one == o.owed_by && contract.other == *to)
                            || (contract.other == o.owed_by && contract.one == *to)) =>
                    {
                        fault(
                            &mut faults,
                            &at,
                            "obligation parties differ from its agreement",
                        );
                    }
                    Some(_) if !parties.contains(to.as_str()) => {
                        fault(&mut faults, &at, "beneficiary is not an opening party");
                    }
                    Some(_) => {}
                },
                OpeningOwed::On(line) => match instruments.get(line.as_str()) {
                    None => fault(&mut faults, &at, "claim instrument is not declared"),
                    Some(i) if i.currency != o.currency => {
                        fault(
                            &mut faults,
                            &at,
                            "obligation currency differs from its instrument",
                        );
                    }
                    Some(_) => {}
                },
            }
        }

        for (n, agreement) in self.agreements.iter().enumerate() {
            let at = format!("agreement {n}");
            if !parties.contains(agreement.one.as_str())
                || !parties.contains(agreement.other.as_str())
            {
                fault(&mut faults, &at, "agreement names an unknown party");
            }
            if agreement.one == agreement.other {
                fault(&mut faults, &at, "a party cannot agree with itself");
            }
            agreement
                .terms
                .validate(&at, &parties, &currencies, &instruments, &mut faults);
            if agreement.until.is_some_and(|until| until < agreement.from) {
                fault(&mut faults, &at, "agreement ends before it begins");
            }
        }

        for (n, instruction) in self.instructions.iter().enumerate() {
            let at = format!("instruction {n}");
            if instruction.reason.is_empty() || instruction.legs.is_empty() {
                fault(
                    &mut faults,
                    &at,
                    "instruction needs a reason and at least one leg",
                );
            }
            for leg in &instruction.legs {
                let (from, to, line, amount, money) = match leg {
                    OpeningLeg::Money {
                        from,
                        to,
                        instrument,
                        amount,
                        receipt: _,
                    } => (from, to, instrument, *amount, true),
                    OpeningLeg::Asset {
                        from,
                        to,
                        instrument,
                        units,
                    } => (from, to, instrument, *units, false),
                };
                if !parties.contains(from.as_str()) || !parties.contains(to.as_str()) {
                    fault(&mut faults, &at, "instruction leg names an unknown party");
                }
                match instruments.get(line.as_str()) {
                    None => fault(
                        &mut faults,
                        &at,
                        "instruction leg names an unknown instrument",
                    ),
                    Some(i) if money != (i.class == Class::Money) => {
                        fault(
                            &mut faults,
                            &at,
                            "instruction leg uses the wrong instrument class",
                        );
                    }
                    Some(_) => {}
                }
                if !amount.is_finite() || amount <= 0.0 {
                    fault(
                        &mut faults,
                        &at,
                        "instruction amount is not finite and positive",
                    );
                }
            }
            let actual = opening_shape(&instruction.legs);
            if instruction.delivery != actual {
                fault(&mut faults, &at, "declared delivery differs from its legs");
            }
        }

        let mut required = HashSet::new();
        for id in &self.required_params {
            if !required.insert(id.as_str()) {
                fault(&mut faults, id, "opening parameter is required twice");
            } else if !params.declared(id) {
                fault(&mut faults, id, "opening parameter is not declared");
            }
        }

        if faults.is_empty() {
            Ok(())
        } else {
            Err(faults)
        }
    }
}

fn fault(out: &mut Vec<OpeningFault>, at: &str, message: &str) {
    out.push(OpeningFault {
        at: at.to_string(),
        message: message.to_string(),
    });
}

fn opening_shape(legs: &[OpeningLeg]) -> Delivery {
    let mut deliveries = Vec::new();
    let mut money = Vec::new();
    for leg in legs {
        match leg {
            OpeningLeg::Money { from, to, .. } if from != to => money.push((from, to)),
            OpeningLeg::Asset { from, to, .. } if from != to => deliveries.push((from, to)),
            _ => {}
        }
    }
    if deliveries.is_empty() {
        return Delivery::Nothing;
    }
    if money.iter().any(|(money_from, money_to)| {
        deliveries
            .iter()
            .any(|(asset_from, asset_to)| money_from == asset_to || money_to == asset_from)
    }) {
        Delivery::AgainstPayment
    } else {
        Delivery::Free
    }
}

fn settle_opening(
    world: &mut crate::assembly::World,
    instruction: &crate::ledger::Instruction<'_>,
) -> crate::ledger::Outcome {
    let mut stores = crate::ledger::Settling {
        register: &mut world.register,
        journal: &mut world.journal,
        parties: &world.parties,
        instruments: &mut world.instruments,
        calendar: &world.calendar,
        says: world.says,
    };
    world.wire.settle(instruction, 0, &mut stores)
}

/// The one deterministic draw stream available to an opening-state generator.
#[derive(Clone, Debug)]
pub struct OpeningDraw {
    state: u64,
}

impl OpeningDraw {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// One raw draw. The algorithm is fixed so platform and build mode cannot change the stream.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// A reproducible unit-interval draw with 53 bits of precision.
    pub fn unit_interval(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64))
    }
}

/// The stable name-to-row result of admitting an opening state.
#[derive(Default, Debug)]
pub struct OpeningIds {
    pub parties: HashMap<String, crate::ids::PartyId>,
    pub instruments: HashMap<String, crate::ids::InstrumentId>,
    pub currencies: HashMap<String, crate::ids::CurrencyCode>,
    pub units: HashMap<String, crate::ids::UnitId>,
}

impl OpeningState {
    /// Validate and populate a new world through the stores' owning admission, issuance, wire,
    /// schedule and agreement doors.
    pub fn build(
        &self,
        config: crate::assembly::RunConfig,
        declare: impl FnOnce(&mut Params),
    ) -> Result<(crate::assembly::World, OpeningIds), Vec<OpeningFault>> {
        let world = crate::assembly::World::with_parameters(config, declare);
        self.populate(world)
    }

    /// Generate and populate week zero from the run's one parameter register and RNG stream.
    pub fn generate(
        config: crate::assembly::RunConfig,
        declare: impl FnOnce(&mut Params),
        generate: impl FnOnce(&Params, &mut OpeningDraw) -> OpeningState,
    ) -> Result<(crate::assembly::World, OpeningIds, OpeningState), Vec<OpeningFault>> {
        let world = crate::assembly::World::with_parameters(config, declare);
        let mut draw = OpeningDraw::new(config.seed);
        let state = generate(&world.params, &mut draw);
        let (world, ids) = state.populate(world)?;
        Ok((world, ids, state))
    }

    fn populate(
        &self,
        mut world: crate::assembly::World,
    ) -> Result<(crate::assembly::World, OpeningIds), Vec<OpeningFault>> {
        use std::num::NonZeroU32;

        use crate::ids::{PartyId, RegionId};
        use crate::ledger::{Instruction, Leg, Outcome, Units};
        use crate::stores::{Owed, Payment};

        self.validate(&world.params)?;
        let mut ids = OpeningIds::default();
        let mut countries = HashMap::new();

        for unit in &self.units {
            let id = world
                .registry
                .unit(NonZeroU32::new(unit.pieces).expect("validated opening unit"));
            ids.units.insert(unit.id.clone(), id);
        }

        // A currency issuer is the bootstrap edge: as in the ordinary assembly path, its party row
        // exists immediately before that row becomes the issuer of a currency and its first region.
        for currency in &self.currencies {
            let party = self
                .parties
                .iter()
                .find(|party| party.id == currency.issuer)
                .expect("validated currency issuer");
            let predicted_region = RegionId::at(world.registry.regions() as u32);
            let issuer = world.parties.add(
                party.kind,
                predicted_region,
                PartyId::NONE,
                party.representation,
                party.key.clone(),
            );
            ids.parties.insert(party.id.clone(), issuer);
            let code = world.registry.currency(issuer);
            let country = world.registry.country(code);
            let region = world.registry.region(country);
            debug_assert_eq!(region, predicted_region);
            ids.currencies.insert(currency.id.clone(), code);
            countries.insert(currency.id.clone(), country);
        }

        let mut remaining_parties: HashSet<&str> = self
            .parties
            .iter()
            .filter(|party| !ids.parties.contains_key(&party.id))
            .map(|party| party.id.as_str())
            .collect();
        let mut remaining_instruments: HashSet<&str> = self
            .instruments
            .iter()
            .map(|line| line.id.as_str())
            .collect();

        loop {
            let mut progressed = false;
            for line in &self.instruments {
                if !remaining_instruments.contains(line.id.as_str()) {
                    continue;
                }
                let Some(&issuer) = ids.parties.get(&line.issuer) else {
                    continue;
                };
                let id = world.instruments.issue(
                    issuer,
                    ids.currencies[&line.currency],
                    line.class,
                    ids.units[&line.unit],
                    line.coupon,
                    line.matures,
                );
                ids.instruments.insert(line.id.clone(), id);
                remaining_instruments.remove(line.id.as_str());
                progressed = true;
            }
            for party in &self.parties {
                if !remaining_parties.contains(party.id.as_str()) {
                    continue;
                }
                let bank = match &party.bank {
                    None => PartyId::NONE,
                    Some(name) => match ids.parties.get(name) {
                        Some(&bank) if world.instruments.money_issued_by(bank).is_some() => bank,
                        _ => continue,
                    },
                };
                let region = world.registry.region(countries[&party.currency]);
                let id = world.admit(
                    party.kind,
                    region,
                    bank,
                    party.representation,
                    party.key.clone(),
                );
                ids.parties.insert(party.id.clone(), id);
                remaining_parties.remove(party.id.as_str());
                progressed = true;
            }
            if remaining_parties.is_empty() && remaining_instruments.is_empty() {
                break;
            }
            if !progressed {
                return Err(vec![OpeningFault {
                    at: "opening topology".to_string(),
                    message: "banking and issuance dependencies cannot be admitted".to_string(),
                }]);
            }
        }

        for line in &self.instruments {
            let instrument = ids.instruments[&line.id];
            let issuer = ids.parties[&line.issuer];
            if line.class == Class::Money && line.issued > 0.0 {
                let legs = [Leg::Mint {
                    issuer,
                    money: instrument,
                    amount: Units::new(line.issued).expect("validated issuance"),
                }];
                let instruction = Instruction::plain(&legs, Cause::Settlement);
                assert_eq!(settle_opening(&mut world, &instruction), Outcome::Settled);
            }
        }
        for holding in &self.holdings {
            let instrument = ids.instruments[&holding.instrument];
            let holder = ids.parties[&holding.holder];
            world.register.carry(holder, instrument, holding.carrying);
            let line = self
                .instruments
                .iter()
                .find(|line| line.id == holding.instrument)
                .expect("validated holding instrument");
            let units = Units::new(holding.units).expect("validated opening holding");
            let legs = if line.class == Class::Money {
                let issuer = ids.parties[&line.issuer];
                if holder == issuer {
                    continue;
                }
                vec![Leg::Money {
                    from: issuer,
                    to: holder,
                    instrument,
                    amount: units,
                    receipt: Receipt::Transfer,
                }]
            } else {
                vec![Leg::Create {
                    party: holder,
                    instrument,
                    qty: units,
                    cost_per_unit: holding.basis_per_unit,
                }]
            };
            let instruction = Instruction::plain(&legs, Cause::Settlement);
            assert_eq!(settle_opening(&mut world, &instruction), Outcome::Settled);
        }

        for opening in &self.instructions {
            let legs: Vec<Leg> = opening
                .legs
                .iter()
                .map(|leg| match leg {
                    OpeningLeg::Money {
                        from,
                        to,
                        instrument,
                        amount,
                        receipt,
                    } => Leg::Money {
                        from: ids.parties[from],
                        to: ids.parties[to],
                        instrument: ids.instruments[instrument],
                        amount: Units::new(*amount).expect("validated opening amount"),
                        receipt: *receipt,
                    },
                    OpeningLeg::Asset {
                        from,
                        to,
                        instrument,
                        units,
                    } => Leg::Asset {
                        from: ids.parties[from],
                        to: ids.parties[to],
                        instrument: ids.instruments[instrument],
                        qty: Units::new(*units).expect("validated opening units"),
                        price_per_unit: None,
                    },
                })
                .collect();
            let instruction = match opening.delivery {
                Delivery::AgainstPayment => Instruction::against_payment(&legs, opening.cause),
                Delivery::Free => Instruction::free_of_payment(&legs, opening.cause),
                Delivery::Nothing => Instruction::plain(&legs, opening.cause),
            };
            let outcome = settle_opening(&mut world, &instruction);
            if outcome != Outcome::Settled {
                return Err(vec![OpeningFault {
                    at: opening.reason.clone(),
                    message: format!("opening instruction did not settle: {outcome:?}"),
                }]);
            }
        }

        let mut agreements = Vec::with_capacity(self.agreements.len());
        for agreement in &self.agreements {
            let (kind, terms) = agreement.terms.resolve(&ids);
            agreements.push(world.agreements.strike(
                kind,
                ids.parties[&agreement.one],
                ids.parties[&agreement.other],
                terms,
                agreement.from,
                agreement.until,
            ));
        }
        for obligation in &self.obligations {
            let payment = Payment {
                from: obligation.from,
                due: obligation.due,
                amount: obligation.amount,
                of: obligation.of,
            };
            match &obligation.on {
                OpeningOwed::Under { agreement, to } => {
                    world.schedules.owes_under(
                        agreements[*agreement],
                        ids.parties[to],
                        ids.parties[&obligation.owed_by],
                        ids.currencies[&obligation.currency],
                        payment,
                    );
                }
                OpeningOwed::On(line) => {
                    world.schedules.owes(
                        Owed::On(ids.instruments[line]),
                        ids.parties[&obligation.owed_by],
                        ids.currencies[&obligation.currency],
                        payment,
                    );
                }
            }
        }
        let opening_equity = (0..world.parties.len())
            .map(|row| {
                let party = PartyId::at(row as u32);
                let equity = crate::instruments::booked_equity(
                    party,
                    &world.register,
                    &world.instruments,
                    &world.prints,
                    &world.claims,
                    world.week,
                );
                (party, equity)
            })
            .collect::<Vec<_>>();
        for (party, equity) in opening_equity {
            if let Some(equity) = equity {
                world.parties.records_opening_equity(party, equity);
            }
        }
        Ok((world, ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{Dimension, Kind, Owner, ParamDecl};

    fn valid() -> (OpeningState, Params) {
        let mut params = Params::new(100.0, 60.0);
        params.declare(ParamDecl {
            id: "opening.households".to_string(),
            value: 1.0,
            unit: "households".to_string(),
            dimension: Dimension::Count,
            kind: Kind::Resolution,
            owner: Owner::Model,
            why: "opening population resolution".to_string(),
        });
        let state = OpeningState {
            currencies: vec![OpeningCurrency {
                id: "USD".to_string(),
                issuer: "bank".to_string(),
            }],
            units: vec![OpeningUnit {
                id: "cent".to_string(),
                pieces: 100,
            }],
            parties: vec![
                OpeningParty {
                    id: "bank".to_string(),
                    kind: 0,
                    currency: "USD".to_string(),
                    bank: None,
                    representation: Representation::Named,
                    key: LatticeKey::Named(0),
                },
                OpeningParty {
                    id: "firm".to_string(),
                    kind: 1,
                    currency: "USD".to_string(),
                    bank: Some("bank".to_string()),
                    representation: Representation::Named,
                    key: LatticeKey::Named(0),
                },
            ],
            instruments: vec![OpeningInstrument {
                id: "bank.usd".to_string(),
                issuer: "bank".to_string(),
                currency: "USD".to_string(),
                class: Class::Money,
                unit: "cent".to_string(),
                coupon: None,
                matures: None,
                issued: 100.0,
            }],
            holdings: vec![OpeningHolding {
                holder: "firm".to_string(),
                instrument: "bank.usd".to_string(),
                units: 100.0,
                basis_per_unit: 1.0,
                carrying: crate::register::Carrying::Cost,
            }],
            obligations: Vec::new(),
            agreements: Vec::new(),
            instructions: Vec::new(),
            required_params: vec!["opening.households".to_string()],
        };
        (state, params)
    }

    fn declare_population(params: &mut Params, value: f64) {
        params.declare(ParamDecl {
            id: "opening.households".to_string(),
            value,
            unit: "households".to_string(),
            dimension: Dimension::Count,
            kind: Kind::Resolution,
            owner: Owner::Model,
            why: "opening population resolution".to_string(),
        });
    }

    fn generated(params: &Params, draw: &mut OpeningDraw) -> OpeningState {
        let (mut state, _) = valid();
        let units = params.count("opening.households") + (draw.next_u64() % 11) as f64;
        state.instruments[0].issued = units;
        state.holdings[0].units = units;
        state
    }

    fn audit_signature(world: &crate::assembly::World) -> String {
        let mut audit = crate::audit::Audit::over(vec![
            Box::<crate::audit::ATotalCarriesNoLots>::default(),
            Box::<crate::audit::NoCollateralCountedTwice>::default(),
            Box::<crate::audit::HoldersAgainstIssued>::default(),
            Box::<crate::audit::MoneyIsConserved>::default(),
            Box::<crate::audit::FlowsAreComplete>::default(),
            Box::<crate::audit::NamesResolve>::default(),
        ]);
        format!(
            "{:?}",
            audit.run(&crate::audit::Sources {
                wire: &world.wire,
                register: &world.register,
                instruments: &world.instruments,
                parties: &world.parties,
                week: 0,
                prints: Some(&world.prints),
                claims: Some(&world.claims),
                schedules: Some(&world.schedules),
                agreements: Some(&world.agreements),
                sessions: None,
            })
        )
    }

    #[test]
    fn a_complete_opening_description_validates_without_writing_a_world() {
        let (state, params) = valid();
        assert_eq!(state.validate(&params), Ok(()));
        assert!(
            params.consumed().is_empty(),
            "validation declares dependencies but consumes no parameter"
        );
    }

    #[test]
    fn references_currency_accounts_and_issuance_are_checked_together() {
        let (mut state, params) = valid();
        state.parties[1].bank = Some("missing-bank".to_string());
        state.holdings[0].units = 90.0;
        state.obligations.push(OpeningObligation {
            owed_by: "firm".to_string(),
            on: OpeningOwed::On("bank.usd".to_string()),
            currency: "EUR".to_string(),
            amount: 5.0,
            from: Week(0),
            due: Week(1),
            of: Owing::Principal,
        });
        state.agreements.push(OpeningAgreement {
            one: "firm".to_string(),
            other: "firm".to_string(),
            terms: OpeningAgreementTerms::Tenancy { rent: f64::NAN },
            from: Week(2),
            until: Some(Week(1)),
        });
        let faults = state.validate(&params).unwrap_err();
        assert!(faults.iter().any(|f| f.message.contains("bank is not")));
        assert!(faults
            .iter()
            .any(|f| f.message.contains("holdings do not equal")));
        assert!(faults
            .iter()
            .any(|f| f.message.contains("currency differs")));
        assert!(faults
            .iter()
            .any(|f| f.message.contains("agree with itself")));
        assert!(faults
            .iter()
            .any(|f| f.message.contains("agreement terms are invalid")));
        assert!(faults.iter().any(|f| f.message.contains("ends before")));
    }

    #[test]
    fn building_uses_issuance_wire_schedule_and_agreement_doors() {
        let (mut state, _) = valid();
        state.holdings[0].units = 90.0;
        state.holdings.push(OpeningHolding {
            holder: "bank".to_string(),
            instrument: "bank.usd".to_string(),
            units: 10.0,
            basis_per_unit: 1.0,
            carrying: crate::register::Carrying::Cost,
        });
        state.instruments.push(OpeningInstrument {
            id: "bank.share".to_string(),
            issuer: "bank".to_string(),
            currency: "USD".to_string(),
            class: Class::Share,
            unit: "cent".to_string(),
            coupon: None,
            matures: None,
            issued: 10.0,
        });
        state.holdings.push(OpeningHolding {
            holder: "firm".to_string(),
            instrument: "bank.share".to_string(),
            units: 10.0,
            basis_per_unit: 2.5,
            carrying: crate::register::Carrying::Market,
        });
        state.instructions.push(OpeningInstruction {
            reason: "opening purchase".to_string(),
            cause: Cause::Trade,
            delivery: Delivery::AgainstPayment,
            legs: vec![
                OpeningLeg::Asset {
                    from: "firm".to_string(),
                    to: "bank".to_string(),
                    instrument: "bank.share".to_string(),
                    units: 2.0,
                },
                OpeningLeg::Money {
                    from: "bank".to_string(),
                    to: "firm".to_string(),
                    instrument: "bank.usd".to_string(),
                    amount: 5.0,
                    receipt: Receipt::Sale,
                },
            ],
        });
        state.instructions.push(OpeningInstruction {
            reason: "opening free delivery".to_string(),
            cause: Cause::Settlement,
            delivery: Delivery::Free,
            legs: vec![OpeningLeg::Asset {
                from: "firm".to_string(),
                to: "bank".to_string(),
                instrument: "bank.share".to_string(),
                units: 1.0,
            }],
        });
        state.obligations.push(OpeningObligation {
            owed_by: "firm".to_string(),
            on: OpeningOwed::Under {
                agreement: 0,
                to: "bank".to_string(),
            },
            currency: "USD".to_string(),
            amount: 4.0,
            from: Week(0),
            due: Week(7),
            of: Owing::Rent,
        });
        state.agreements.push(OpeningAgreement {
            one: "bank".to_string(),
            other: "firm".to_string(),
            terms: OpeningAgreementTerms::Tenancy { rent: 4.0 },
            from: Week(0),
            until: None,
        });

        let (world, ids) = state
            .build(crate::assembly::RunConfig::default(), |params| {
                params.declare(ParamDecl {
                    id: "opening.households".to_string(),
                    value: 1.0,
                    unit: "households".to_string(),
                    dimension: Dimension::Count,
                    kind: Kind::Resolution,
                    owner: Owner::Model,
                    why: "opening population resolution".to_string(),
                });
            })
            .unwrap();

        let line = ids.instruments["bank.usd"];
        let firm = ids.parties["firm"];
        assert_eq!(world.instruments.issued_of(line), 100.0);
        assert_eq!(
            world.register.quantity(world.register.row(firm, line)),
            95.0
        );
        assert_eq!(
            world.wire.len(),
            5,
            "issuance, DvP and FoP all use the wire"
        );
        assert_eq!(world.schedules.len(), 1);
        assert_eq!(world.agreements.len(), 1);
        assert_eq!(
            world.parties.opening_equity_of(firm),
            crate::instruments::booked_equity(
                firm,
                &world.register,
                &world.instruments,
                &world.prints,
                &world.claims,
                world.week,
            )
        );
        assert_eq!(
            world.schedules.agreement(crate::stores::DueId(0)),
            Some(crate::stores::AgreementId(0))
        );
    }

    #[test]
    fn seed_parameters_events_and_audit_form_one_reproducibility_contract() {
        let config = crate::assembly::RunConfig {
            seed: 42,
            ..Default::default()
        };
        let run = || {
            OpeningState::generate(
                config,
                |params| declare_population(params, 100.0),
                generated,
            )
            .unwrap()
        };
        let (first, first_ids, first_state) = run();
        let (again, again_ids, again_state) = run();

        assert_eq!(first_state, again_state);
        assert_eq!(first_ids.parties, again_ids.parties);
        assert_eq!(first.params.consumed(), again.params.consumed());
        assert_eq!(first.wire.len(), again.wire.len());
        for n in 0..first.wire.len() {
            assert_eq!(first.wire.outcome_of(n), again.wire.outcome_of(n));
            assert_eq!(first.wire.cause_of(n), again.wire.cause_of(n));
            assert_eq!(first.wire.legs_of(n), again.wire.legs_of(n));
        }
        assert_eq!(audit_signature(&first), audit_signature(&again));

        let (_, different_seed_ids, different_seed) = OpeningState::generate(
            crate::assembly::RunConfig {
                seed: 43,
                ..Default::default()
            },
            |params| declare_population(params, 100.0),
            generated,
        )
        .unwrap();
        assert_eq!(first_ids.parties, different_seed_ids.parties);
        assert_eq!(first_state.parties, different_seed.parties);
        assert_ne!(first_state.holdings, different_seed.holdings);

        let (different_param, different_param_ids, different_param_state) = OpeningState::generate(
            config,
            |params| declare_population(params, 101.0),
            generated,
        )
        .unwrap();
        assert_eq!(first_ids.parties, different_param_ids.parties);
        assert_eq!(first_state.parties, different_param_state.parties);
        assert_ne!(first_state.holdings, different_param_state.holdings);
        assert_eq!(first.params.consumed()[0].id, "opening.households");
        assert_eq!(
            different_param.params.consumed()[0].id,
            "opening.households"
        );
        assert_ne!(
            first.params.consumed()[0].value,
            different_param.params.consumed()[0].value
        );
    }
}
