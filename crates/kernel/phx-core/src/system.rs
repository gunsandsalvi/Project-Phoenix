use phx_id::SystemCode;
use phx_num::violation;

use crate::contribution::Contribution;
use crate::decisions::DecisionPointDecl;
use crate::events::EventKindDecl;
use crate::facts::Claim;
use crate::family::{AuditFamily, FamilyDecl};
use crate::hazards::HazardDecl;
use crate::kind_tables::FacetDecl;
use crate::kinds::KindDecl;
use crate::kinks::{KinkDecl, KinkRegistry};
use crate::messages::MessageKindDecl;
use crate::pop::{PopEntry, PopKindBuilder};
use crate::records::RecordKindDecl;
use crate::register::values::PrimType;
use crate::register::{Prim, PrimDecl, RegisterBuilder};
use crate::rules::{RuleSig, RuleTable};
use crate::streams::StreamDecl;

/// A system: a zero-sized type that declares what it owns.
pub trait System: Send + Sync + 'static {
    const CODE: &'static str;
    fn declare(d: &mut Declarations);
}

/// A decision point as the assembly sees it, whatever its input and output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionMeta {
    pub name: &'static str,
    pub system: &'static str,
    pub valid: Result<(), &'static str>,
}

/// Everything the systems declare, each entry with the system that declared it.
#[derive(Default)]
pub struct Declarations {
    system: &'static str,
    pub prims: RegisterBuilder,
    pub kinds: Vec<(&'static str, KindDecl)>,
    pub claims: Vec<(&'static str, &'static str)>,
    pub facets: Vec<(&'static str, FacetDecl)>,
    pub streams: Vec<(&'static str, StreamDecl)>,
    pub hazards: Vec<(&'static str, HazardDecl)>,
    pub messages: Vec<(&'static str, MessageKindDecl)>,
    pub decisions: Vec<DecisionMeta>,
    pub rules: RuleTable,
    pub records: Vec<(&'static str, RecordKindDecl)>,
    pub events: Vec<(&'static str, EventKindDecl)>,
    pub kinks: KinkRegistry,
    pub families: Vec<(&'static str, Box<dyn AuditFamily>)>,
    pub contributions: Vec<(&'static str, Box<dyn Contribution>)>,
    /// Each line-owning system's draw of the households' lines, which the household's formation calls; opaque here,
    /// since the kernel crate that knows lines lies above this one.
    pub attachments: Vec<(&'static str, Box<dyn core::any::Any + Send + Sync>)>,
    /// Each market kind a system declares, the template of its instances; opaque here, since the kernel crate that
    /// knows markets lies above this one.
    pub markets: Vec<(&'static str, Box<dyn core::any::Any + Send + Sync>)>,
    pub pop: Vec<PopEntry>,
    pub pop_processes: Vec<(&'static str, Box<dyn crate::pop_process::PopProcess>)>,
    /// Each decision taken on the rows of a kind as they come due.
    pub visits: Vec<(&'static str, crate::visit::VisitDecl)>,
    /// Each kind's insolvency law: the grace after which a party in arrears defaults and ends into an estate.
    pub insolvency: Vec<(&'static str, crate::insolvency::InsolvencyDecl)>,
    /// The wear of each system's chains of classes, realised at a visit.
    pub wear: Vec<(&'static str, crate::wear::WearDecl)>,
    /// The spoilage of each system's goods in stock, realised at a visit.
    pub spoilage: Vec<(&'static str, crate::spoilage::SpoilageDecl)>,
    pub setup_values: Vec<(&'static str, SetupValue)>,
    /// Each system's state compiled from the register at assembly, which its handlers and its family read.
    pub compiled: Vec<(&'static str, Compile)>,
    kink_errors: Vec<String>,
}

/// A system's state compiled from the register and the number of countries: built once at assembly, and again
/// from the same register when a save is loaded, so nothing it holds is saved.
pub type Compile =
    Box<dyn FnOnce(&crate::Register, usize) -> Result<Box<dyn core::any::Any + Send + Sync>, String> + Send>;

/// A country primitive a new game sets from one of the country's derived values: the system's mapping of a derived
/// value into the data its processes read, written with the country's data when the game is instantiated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetupValue {
    pub prim: &'static PrimDecl,
    pub derived: &'static str,
}

impl std::fmt::Debug for Declarations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Declarations").field("system", &self.system).finish_non_exhaustive()
    }
}

impl Declarations {
    #[must_use]
    pub fn new() -> Declarations {
        Declarations::default()
    }

    pub fn prim<T: PrimType>(&mut self, decl: &PrimDecl) -> Prim<T> {
        self.prims.declare(decl)
    }

    pub fn kind(&mut self, decl: KindDecl) {
        self.kinds.push((self.system, decl));
    }

    /// The system claims an interface item it writes or answers.
    pub fn claim(&mut self, item: &'static str) {
        self.claims.push((self.system, item));
    }

    pub fn facet(&mut self, decl: FacetDecl) {
        self.facets.push((self.system, decl));
    }

    /// A country primitive the new game sets from a derived value.
    pub fn setup_value(&mut self, value: SetupValue) {
        self.setup_values.push((self.system, value));
    }

    pub fn stream(&mut self, decl: StreamDecl) {
        self.streams.push((self.system, decl));
    }

    pub fn hazard(&mut self, decl: HazardDecl) {
        self.hazards.push((self.system, decl));
    }

    pub fn message(&mut self, decl: MessageKindDecl) {
        self.messages.push((self.system, decl));
    }

    pub fn decision<I, O>(&mut self, decl: &DecisionPointDecl<I, O>) {
        let valid = decl.validate().map_err(|_| "no schedule and no wake");
        self.decisions.push(DecisionMeta { name: decl.name, system: decl.system, valid });
    }

    pub fn implement<I: 'static, O: 'static>(&mut self, sig: RuleSig<I, O>, f: fn(&I) -> O) {
        self.rules.implement(self.system, sig, f);
    }

    pub fn record(&mut self, decl: RecordKindDecl) {
        self.records.push((self.system, decl));
    }

    pub fn event(&mut self, decl: EventKindDecl) {
        self.events.push((self.system, decl));
    }

    pub fn kink(&mut self, decl: KinkDecl) {
        if let Err(e) = self.kinks.register(decl) {
            self.kink_errors.push(e);
        }
    }

    pub fn family(&mut self, family: Box<dyn AuditFamily>) {
        self.families.push((self.system, family));
    }

    pub fn contribution(&mut self, contribution: Box<dyn Contribution>) {
        self.contributions.push((self.system, contribution));
    }

    /// The state the system compiles from the register at assembly.
    pub fn compile(&mut self, compile: Compile) {
        self.compiled.push((self.system, compile));
    }

    /// A draw of the households' lines of the system's kinds, made with each household as the opening forms it.
    pub fn attachment(&mut self, draw: Box<dyn core::any::Any + Send + Sync>) {
        self.attachments.push((self.system, draw));
    }

    /// A kind of market the system runs, whose instances its orders name.
    pub fn market(&mut self, kind: Box<dyn core::any::Any + Send + Sync>) {
        self.markets.push((self.system, kind));
    }

    /// A process on a population kind's members, whose outcome the declaring system writes.
    pub fn pop_process(&mut self, process: Box<dyn crate::pop_process::PopProcess>) {
        self.pop_processes.push((self.system, process));
    }

    /// The insolvency law a kind's parties are under.
    pub fn insolvency(&mut self, decl: crate::insolvency::InsolvencyDecl) {
        self.insolvency.push((self.system, decl));
    }

    /// The wear of a system's chains of classes, realised on the rows of one of its visits.
    pub fn wear(&mut self, decl: crate::wear::WearDecl) {
        self.wear.push((self.system, decl));
    }

    /// The spoilage of goods in stock, realised on the rows of one of the system's visits.
    pub fn spoilage(&mut self, decl: crate::spoilage::SpoilageDecl) {
        self.spoilage.push((self.system, decl));
    }

    /// A decision taken on a kind's rows as they come due, by its schedule or its reviews.
    pub fn visit(&mut self, decl: crate::visit::VisitDecl) {
        self.visits.push((self.system, decl));
    }

    /// Adds items to a population kind: its roles, key attributes, positions, standing rates, profile groups,
    /// review kinds and pins, each the declaring system's to write.
    pub fn pop_kind(&mut self, kind: &'static str) -> PopKindBuilder<'_> {
        PopKindBuilder::new(self, kind)
    }

    /// The system whose declarations are being read.
    pub(crate) fn system(&self) -> &'static str {
        self.system
    }

    /// The claims as the item check reads them.
    ///
    /// # Errors
    /// A claim by a system whose code is malformed.
    pub fn claims(&self) -> Result<Vec<Claim>, String> {
        self.claims
            .iter()
            .map(|(system, item)| {
                SystemCode::new(system)
                    .map(|system| Claim { system, item })
                    .ok_or_else(|| format!("`{system}` is no system code"))
            })
            .collect()
    }

    /// The refusals the declarations alone decide: a stream twice, a hazard incomplete or drawing from an undeclared
    /// stream, a message reaching an unanswered kind, a decision point with no schedule or wake, a family twice, a
    /// kink twice.
    #[must_use]
    pub fn refusals(&self) -> Vec<String> {
        let mut errors = self.kink_errors.clone();
        let mut names: Vec<&str> = self.streams.iter().map(|(_, s)| s.name).collect();
        names.sort_unstable();
        for pair in names.windows(2) {
            if let [a, b] = pair
                && a == b
            {
                errors.push(format!("stream `{a}` declared twice"));
            }
        }
        for (_, h) in &self.hazards {
            if let Err(e) = h.validate() {
                errors.push(e);
            }
            if !self.streams.iter().any(|(_, s)| s.name == h.stream) {
                errors.push(format!("hazard `{}` draws from `{}`, which no system declares", h.name, h.stream));
            }
        }
        errors.extend(self.messages.iter().filter_map(|(_, m)| m.validate().err()));
        for d in &self.decisions {
            if let Err(why) = d.valid {
                errors.push(format!("decision point `{}`: {why}", d.name));
            }
        }
        let family_names: Vec<FamilyDecl> = self.families.iter().map(|(_, f)| f.decl()).collect();
        for (i, f) in family_names.iter().enumerate() {
            if family_names.iter().skip(i + 1).any(|g| g.name == f.name) {
                errors.push(format!("audit family `{}` declared twice", f.name));
            }
        }
        errors
    }
}

/// Calls a system's declarations, each entry carrying its code; a system is a zero-sized type.
pub fn declare_system<S: System>(d: &mut Declarations) {
    declare_entry(&SystemEntry::of::<S>(), d);
}

/// A system as the assembly lists it: its code and its declarations.
#[derive(Clone, Copy, Debug)]
pub struct SystemEntry {
    pub code: &'static str,
    pub declare: fn(&mut Declarations),
}

impl SystemEntry {
    #[must_use]
    pub fn of<S: System>() -> SystemEntry {
        if size_of::<S>() != 0 {
            violation!(clause = "Law 4", "a system that is not zero-sized");
        }
        SystemEntry { code: S::CODE, declare: S::declare }
    }
}

/// Calls a listed system's declarations, each entry carrying its code.
pub fn declare_entry(system: &SystemEntry, d: &mut Declarations) {
    if SystemCode::new(system.code).is_none() {
        violation!(clause = "Law 4", "a system whose code is not two to four capital letters");
    }
    d.system = system.code;
    (system.declare)(d);
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::{Declarations, System, declare_system};
    use crate::decisions::DecisionPointDecl;
    use crate::hazards::{ActsOn, DrawScheme, HazardDecl, RateFn};
    use crate::streams::{Purpose, StreamDecl};

    fn noop(_: &[i64; 2]) -> i64 {
        0
    }

    const LOOK: DecisionPointDecl<[i64; 2], i64> = DecisionPointDecl {
        name: "DEM.look",
        system: "DEM",
        rule: noop,
        schedule: Missing::Absent,
        wakes: &[],
        runs_on_non_business: false,
        clause: "DEM.1",
    };

    struct Dem;
    impl System for Dem {
        const CODE: &'static str = "DEM";
        fn declare(d: &mut Declarations) {
            d.stream(StreamDecl { name: "DEM.mortality", purpose: Purpose::Mortality, keyed: false, clause: "CHN.3" });
            d.hazard(HazardDecl {
                name: "DEM.death",
                acts_on: ActsOn::Role { kind: "household", role: "person" },
                rate: RateFn { table: "DEM.mortality_table", axes: &["DEM.age"], changes: &[] },
                outcome: "DEM.dies",
                scheme: DrawScheme::Daily,
                stream: "DEM.illness",
                clause: "DEM.2",
                source: "life tables",
            });
            d.decision(&LOOK);
        }
    }

    #[test]
    fn declarations_carry_their_system_and_are_refused_when_incomplete() {
        let mut d = Declarations::new();
        declare_system::<Dem>(&mut d);
        assert_eq!(d.streams[0].0, "DEM");
        let refusals = d.refusals();
        assert!(refusals.iter().any(|r| r.contains("DEM.illness")), "a hazard drawing from an undeclared stream");
        assert!(refusals.iter().any(|r| r.contains("DEM.look")), "a decision point with no schedule or wake");
    }
}
