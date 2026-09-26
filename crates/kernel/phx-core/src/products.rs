//! The products the world makes, as the data declares them: what differs between a good and a service is these
//! declared fields, which a mechanism reads, never a kind it branches on.

use phx_macros::clause;
use phx_num::Missing;

/// A product as the register reads it: its name, the name of the unit it is counted in, its industry, whether it
/// can be stored, whether it is delivered as it is made, and the deposit resource it is extracted from, if any.
#[clause("TEC.1")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductEntry {
    pub name: String,
    pub unit: String,
    pub industry: String,
    pub storable: bool,
    pub delivered_at_once: bool,
    pub extracts: Missing<u16>,
}

impl ProductEntry {
    /// What is delivered as it is made is never held, so it cannot be stored.
    ///
    /// # Errors
    /// When a product is both.
    #[clause("TEC.1")]
    pub fn validate(&self) -> Result<(), String> {
        if self.storable && self.delivered_at_once {
            return Err(format!("product `{}` is stored and delivered as it is made", self.name));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_service_is_not_stored() {
        let mut p = ProductEntry {
            name: "care".to_owned(),
            unit: "care".to_owned(),
            industry: "health".to_owned(),
            storable: false,
            delivered_at_once: true,
            extracts: Missing::Absent,
        };
        assert!(p.validate().is_ok());
        p.storable = true;
        assert!(p.validate().is_err());
    }
}
