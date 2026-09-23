use phx_core::Level;
use phx_macros::clause;
use phx_rand::{Draws, below_u64};
use serde::Deserialize;

use crate::consts::WHOLE;

/// A three-level choice's level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Degree {
    Low,
    Medium,
    High,
}

/// How much risk a country's people bear gladly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appetite {
    Cautious,
    Balanced,
    Bold,
}

const DEGREES: [Degree; 3] = [Degree::Low, Degree::Medium, Degree::High];
const APPETITES: [Appetite; 3] = [Appetite::Cautious, Appetite::Balanced, Appetite::Bold];
const LEVELS: [Level; 3] = [Level::Developed, Level::Emerging, Level::Developing];

impl Degree {
    /// Which third of its group's distribution the level's values fall in, from the bottom.
    #[must_use]
    pub fn third(self) -> u8 {
        match self {
            Degree::Low => 0,
            Degree::Medium => 1,
            Degree::High => 2,
        }
    }
}

/// A choice as the setup states it: a level, or drawn from the seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Choice<T> {
    Set(T),
    Drawn(Drawn),
}

/// The word that leaves a choice to the seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drawn {
    Drawn,
}

/// A country's name: generated from the seed, or a real country's, by its ISO 3166 alpha-3 code.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum NameChoice {
    Generated(GeneratedWord),
    Real { real: String },
}

/// The word that asks for a generated name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedWord {
    Generated,
}

/// One country's six choices and its name, as the setup states them.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountryChoices {
    pub development: Choice<Level>,
    pub public_debt: Choice<Degree>,
    pub private_debt: Choice<Degree>,
    pub risk_appetite: Choice<Appetite>,
    pub inequality: Choice<Degree>,
    pub openness: Choice<Degree>,
    pub name: NameChoice,
}

/// The population split in whole percent, one per country, or drawn.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Split {
    Percent(Vec<u64>),
    Drawn(Drawn),
}

/// A new game's setup: the population split and each country's choices.
#[clause("GEN.14")]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub split: Split,
    pub country: Vec<CountryChoices>,
}

/// What a setup must keep to: the countries' number, and each one's least and greatest share of the population in
/// whole percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guardrails {
    pub countries: u64,
    pub floor: u64,
    pub ceiling: u64,
}

/// A country's choices with nothing left to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountrySetup {
    pub development: Level,
    pub public_debt: Degree,
    pub private_debt: Degree,
    pub risk_appetite: Appetite,
    pub inequality: Degree,
    pub openness: Degree,
    pub name: NameChoice,
}

/// A setup with every choice made: the split in whole percent, and each country's levels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSetup {
    pub split: Vec<u64>,
    pub countries: Vec<CountrySetup>,
}

/// Reads a setup from its file.
///
/// # Errors
/// When the text is not a setup.
pub fn read(text: &str) -> Result<Setup, String> {
    toml::from_str(text).map_err(|e| format!("setup: {e}"))
}

/// The rules a setup breaks: a country count other than the world's, a share outside the guardrails, or shares that
/// do not make up the whole. A setup that breaks one is refused, never adjusted.
#[clause("GEN.14")]
#[must_use]
pub fn refusals(setup: &Setup, g: Guardrails) -> Vec<String> {
    let mut errors = Vec::new();
    let count = u64::try_from(setup.country.len()).unwrap_or(u64::MAX);
    if count != g.countries {
        errors.push(format!("the setup has {count} countries, and the world {}", g.countries));
    }
    if let Split::Percent(shares) = &setup.split {
        if u64::try_from(shares.len()).ok() != Some(g.countries) {
            errors.push(format!("the split has {} shares for {} countries", shares.len(), g.countries));
        }
        for (i, s) in shares.iter().enumerate() {
            if *s < g.floor || *s > g.ceiling {
                errors.push(format!("country {i}'s share of {s}% lies outside {}% to {}%", g.floor, g.ceiling));
            }
        }
        let total: u64 = shares.iter().sum();
        if total != WHOLE {
            errors.push(format!("the shares make {total}%, not the whole"));
        }
    }
    errors
}

/// Every split the guardrails allow, in whole percent, in a fixed order.
fn splits(g: Guardrails) -> Vec<Vec<u64>> {
    let mut out: Vec<Vec<u64>> = vec![Vec::new()];
    for _ in 0..g.countries {
        out = out
            .into_iter()
            .flat_map(|prefix| {
                (g.floor..=g.ceiling).map(move |s| {
                    let mut next = prefix.clone();
                    next.push(s);
                    next
                })
            })
            .collect();
    }
    out.retain(|s| s.iter().sum::<u64>() == WHOLE);
    out
}

fn pick<T: Copy>(d: &mut Draws, of: &[T]) -> T {
    let n = u64::try_from(of.len()).unwrap_or(u64::MAX);
    let Some(x) = usize::try_from(below_u64(d, n)).ok().and_then(|i| of.get(i)) else {
        phx_num::violation!(clause = "GEN.14", "a choice drawn from no levels");
    };
    *x
}

fn made<T: Copy>(c: Choice<T>, d: &mut Draws, levels: &[T]) -> T {
    match c {
        Choice::Set(level) => level,
        Choice::Drawn(_) => pick(d, levels),
    }
}

/// Makes every choice the setup leaves to the seed: the split uniformly among those the guardrails allow, and each
/// open level uniformly among its three. `split_draws` is the stream's draws for the world, `country_draws` for each
/// country in turn.
#[clause("GEN.14")]
pub fn resolve(setup: &Setup, g: Guardrails, split_draws: &mut Draws, country_draws: &mut [Draws]) -> ResolvedSetup {
    let split = match &setup.split {
        Split::Percent(shares) => shares.clone(),
        Split::Drawn(_) => {
            let all = splits(g);
            let n = u64::try_from(all.len()).unwrap_or(u64::MAX);
            let Some(s) = usize::try_from(below_u64(split_draws, n)).ok().and_then(|i| all.get(i)) else {
                phx_num::violation!(clause = "GEN.14", "guardrails no split can keep");
            };
            s.clone()
        }
    };
    let countries = setup
        .country
        .iter()
        .zip(country_draws.iter_mut())
        .map(|(c, d)| CountrySetup {
            development: made(c.development, d, &LEVELS),
            public_debt: made(c.public_debt, d, &DEGREES),
            private_debt: made(c.private_debt, d, &DEGREES),
            risk_appetite: made(c.risk_appetite, d, &APPETITES),
            inequality: made(c.inequality, d, &DEGREES),
            openness: made(c.openness, d, &DEGREES),
            name: c.name.clone(),
        })
        .collect();
    ResolvedSetup { split, countries }
}

#[cfg(test)]
mod tests {
    use phx_core::{Purpose, StreamDecl, Streams};
    use phx_id::Day;
    use phx_rand::{Draws, Seed, Subject, SubjectTag};

    use super::{Guardrails, Split, read, refusals, resolve, splits};

    const SETUP: StreamDecl =
        StreamDecl { name: "GEN.setup", purpose: Purpose::Opening, keyed: false, clause: "GEN.14" };

    fn draws(subject: u64) -> Draws {
        let streams = Streams::new(Seed::new(1), &[SETUP]).unwrap();
        streams.open(&SETUP, Subject::new(SubjectTag::Country, subject), Day::new(0), 0)
    }

    const G: Guardrails = Guardrails { countries: 3, floor: 10, ceiling: 70 };

    fn setup(split: &str) -> String {
        let country = "[[country]]\ndevelopment = \"developed\"\npublic_debt = \"drawn\"\nprivate_debt = \"low\"\n\
                       risk_appetite = \"bold\"\ninequality = \"medium\"\nopenness = \"high\"\nname = \"generated\"\n";
        format!("split = {split}\n{}", country.repeat(3))
    }

    #[test]
    fn shares_outside_guardrails_refused() {
        let ok = read(&setup("[50, 30, 20]")).unwrap();
        assert!(refusals(&ok, G).is_empty());
        assert_eq!(refusals(&read(&setup("[9, 71, 20]")).unwrap(), G).len(), 2, "9% and 71%");
        assert_eq!(refusals(&read(&setup("[50, 30, 30]")).unwrap(), G).len(), 1, "not the whole");
        assert_eq!(read(&setup("[50, 30, 20]")).unwrap().country.len(), 3);
        assert!(read(&setup("\"random\"")).is_err());
    }

    #[test]
    fn drawn_choices_keep_the_guardrails() {
        let all = splits(G);
        assert!(all.iter().all(|s| s.iter().sum::<u64>() == 100 && s.iter().all(|x| (10..=70).contains(x))));
        assert!(all.contains(&vec![10, 20, 70]) && !all.contains(&vec![5, 25, 70]));
        let s = read(&setup("\"drawn\"")).unwrap();
        assert_eq!(s.split, Split::Drawn(super::Drawn::Drawn));
        let mut world = draws(1);
        let mut countries = [draws(2), draws(3), draws(4)];
        let r = resolve(&s, G, &mut world, &mut countries);
        assert!(refusals(&super::Setup { split: Split::Percent(r.split.clone()), ..s.clone() }, G).is_empty());
        assert_eq!(r.countries.len(), 3);
    }
}
