use phx_core::Level;
use phx_macros::clause;
use phx_rand::{Draws, below_u64};
use serde::Deserialize;

use crate::opening::setup::{Appetite, Choice, CountryChoices, Degree, NameChoice};

/// What a generated name is made of.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedTable {
    pub first: Vec<String>,
    pub middle: Vec<String>,
    pub last: Vec<String>,
    pub currency: Vec<String>,
}

/// How institutions are labelled from a country's name.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelTemplates {
    pub central_bank: String,
    pub treasury: String,
    pub statistics: String,
    pub parliament: String,
    pub exchange: String,
    pub generated_currency: String,
}

/// A real country as a new game may name one: its name, currency and the levels its own data fall in.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealCountry {
    pub iso3: String,
    pub name: String,
    pub currency: String,
    pub development: Level,
    pub public_debt: Degree,
    pub private_debt: Degree,
    pub inequality: Degree,
    pub openness: Degree,
}

/// The real countries a new game may name.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RealTable {
    pub country: Vec<RealCountry>,
}

/// A country's name and the labels of its institutions and currency. A real name brings nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountryName {
    pub name: String,
    pub real: Option<String>,
    pub currency: String,
    pub central_bank: String,
    pub treasury: String,
    pub statistics: String,
    pub parliament: String,
    pub exchange: String,
}

fn pick<'a>(d: &mut Draws, of: &'a [String]) -> Result<&'a str, String> {
    let n = u64::try_from(of.len()).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("a name table with an empty list".to_owned());
    }
    usize::try_from(below_u64(d, n))
        .ok()
        .and_then(|i| of.get(i))
        .map(String::as_str)
        .ok_or_else(|| "no pick".to_owned())
}

fn fill(template: &str, name: &str, currency: &str) -> String {
    template.replace("{name}", name).replace("{currency}", currency)
}

fn labelled(name: String, real: Option<String>, currency: String, t: &LabelTemplates) -> CountryName {
    CountryName {
        central_bank: fill(&t.central_bank, &name, &currency),
        treasury: fill(&t.treasury, &name, &currency),
        statistics: fill(&t.statistics, &name, &currency),
        parliament: fill(&t.parliament, &name, &currency),
        exchange: fill(&t.exchange, &name, &currency),
        name,
        real,
        currency,
    }
}

/// A country's name: a real country's from the table, or one generated from the syllables by `d`, the stream
/// `GEN.names` for the country.
///
/// # Errors
/// When a real name is not in the table, or a list to draw from is empty.
#[clause("GEN.14", "GEN.1")]
pub fn name(
    choice: &NameChoice,
    real: &RealTable,
    generated: &GeneratedTable,
    labels: &LabelTemplates,
    d: &mut Draws,
) -> Result<CountryName, String> {
    match choice {
        NameChoice::Real { real: iso3 } => {
            let r = real.country.iter().find(|c| c.iso3 == *iso3).ok_or_else(|| format!("no real country `{iso3}`"))?;
            Ok(labelled(r.name.clone(), Some(r.iso3.clone()), r.currency.clone(), labels))
        }
        NameChoice::Generated(_) => {
            let text = [pick(d, &generated.first)?, pick(d, &generated.middle)?, pick(d, &generated.last)?].concat();
            let noun = pick(d, &generated.currency)?;
            let currency = fill(&labels.generated_currency, &text, noun);
            Ok(labelled(text, None, currency, labels))
        }
    }
}

/// The choices a real name pre-fills: its own levels, and the middle level of risk appetite, which it has no data
/// for. The player may change any of them; the economy is derived from whatever the setup then says.
#[must_use]
pub fn prefill(r: &RealCountry) -> CountryChoices {
    CountryChoices {
        development: Choice::Set(r.development),
        public_debt: Choice::Set(r.public_debt),
        private_debt: Choice::Set(r.private_debt),
        risk_appetite: Choice::Set(Appetite::Balanced),
        inequality: Choice::Set(r.inequality),
        openness: Choice::Set(r.openness),
        name: NameChoice::Real { real: r.iso3.clone() },
    }
}

#[cfg(test)]
mod tests {
    use phx_core::Level;

    use super::{RealCountry, RealTable, prefill};
    use crate::opening::setup::{Appetite, Choice, Degree, NameChoice};

    #[test]
    fn real_name_prefills_levels() {
        let text = "[[country]]\niso3 = \"KEN\"\nname = \"Kenya\"\ncurrency = \"Kenyan shilling\"\n\
                    development = \"developing\"\npublic_debt = \"high\"\nprivate_debt = \"low\"\n\
                    inequality = \"medium\"\nopenness = \"low\"\n";
        let table: RealTable = toml::from_str(text).unwrap();
        let kenya: &RealCountry = &table.country[0];
        let c = prefill(kenya);
        assert_eq!(c.development, Choice::Set(Level::Developing));
        assert_eq!((c.public_debt, c.private_debt), (Choice::Set(Degree::High), Choice::Set(Degree::Low)));
        assert_eq!((c.inequality, c.openness), (Choice::Set(Degree::Medium), Choice::Set(Degree::Low)));
        assert_eq!(c.risk_appetite, Choice::Set(Appetite::Balanced));
        assert_eq!(c.name, NameChoice::Real { real: "KEN".to_owned() });
        assert_eq!(kenya.currency, "Kenyan shilling", "the table itself is untouched");
    }
}
