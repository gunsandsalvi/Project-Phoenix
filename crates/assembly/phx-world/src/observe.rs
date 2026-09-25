//! The observer's hook into the day: its reading at each day's end. It is outside the world, so a world run with an
//! observer and one run without are the same world.

use crate::Inspector;

/// What looks at the world once each day has ended.
pub trait Observer {
    fn day_closed(&mut self, w: Inspector<'_>);
}
