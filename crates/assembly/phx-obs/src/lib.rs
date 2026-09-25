//! The observer: what the world shows, read from it and never written to it. The macro reads are series the
//! declarations name, read each day from the run's records; views are built at a turn's close from the reads and the
//! state, with fixed-bin histograms, and swapped in whole.

pub mod histogram;
pub mod liveness;
pub mod reads;
pub mod tracers;
pub mod view;

pub use histogram::Histogram;
pub use liveness::{Drift, drift, rises_throughout, still_from};
pub use reads::{Definitions, HistogramDecl, Measure, ReadDecl, Recorder, Series};
pub use tracers::{Tracer, Tracers, Watch};
pub use view::{View, Views};
