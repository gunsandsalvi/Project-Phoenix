pub mod binomial;
pub mod consts;
pub mod draws;
pub mod float;
pub mod geometric;
pub mod hypergeometric;
pub mod key;
pub mod multinomial;
pub mod philox;
pub mod picks;
#[cfg(test)]
mod testing;
pub mod thin;
pub mod uniform;

pub use binomial::{binomial, binomial_at_least_one, binomials_joint_at_least_one};
pub use draws::Draws;
pub use geometric::geometric;
pub use hypergeometric::{hypergeometric, multivariate_hypergeometric};
pub use key::{Seed, StreamKey, Subject, SubjectTag, stream_key};
pub use multinomial::{AliasTable, multinomial, multinomial_alias};
pub use philox::{philox, philox_x4};
pub use picks::{Fenwick, pick_without_replacement};
pub use thin::accept;
pub use uniform::{below_u32, below_u64, open_unit};
