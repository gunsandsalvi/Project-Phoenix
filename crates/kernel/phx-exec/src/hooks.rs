//! The hooks a sweep takes: a base kept inside another base's sweep — lines, levies, book folds — is reached through
//! the sweep's hook parameter, so the crate that runs the sweep never names the crate that keeps the base.

use crate::apply::Item;

/// A base's part in a sweep: told when a range opens, each item the sweep applies in it, and when it closes, holding
/// its own mutable view of that range.
pub trait RangeHook: Send {
    fn open_range(&mut self, _range: usize) {}
    fn item(&mut self, _item: &Item) {}
    fn close_range(&mut self) {}
}

/// No hook.
impl RangeHook for () {}

/// Hooks composed as a tuple, each told in the tuple's order: monomorphised, so no call goes through a table.
macro_rules! tuple_hooks {
    ($($h:ident $i:tt),+) => {
        impl<$($h: RangeHook),+> RangeHook for ($($h,)+) {
            fn open_range(&mut self, range: usize) {
                $(self.$i.open_range(range);)+
            }
            fn item(&mut self, item: &Item) {
                $(self.$i.item(item);)+
            }
            fn close_range(&mut self) {
                $(self.$i.close_range();)+
            }
        }
    };
}

tuple_hooks!(A 0);
tuple_hooks!(A 0, B 1);
tuple_hooks!(A 0, B 1, C 2);
tuple_hooks!(A 0, B 1, C 2, D 3);

/// The hooks a sweep takes: one hook or a tuple of them.
pub trait SweepHooks: RangeHook {}

impl<T: RangeHook> SweepHooks for T {}
