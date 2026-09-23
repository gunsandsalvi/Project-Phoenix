use crate::consts::MAX_VIOLATION_KEYS;

/// An impossible state, carried by the panic that stops the run; the application's hook adds the site.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Violation {
    pub clause: &'static str,
    pub message: &'static str,
    pub keys: [(&'static str, i128); MAX_VIOLATION_KEYS],
    pub n_keys: u8,
}

impl Violation {
    /// The key list is sized at compile time, so a ninth key does not compile rather than being dropped.
    pub fn new<const N: usize>(clause: &'static str, message: &'static str, given: [(&'static str, i128); N]) -> Self {
        const { assert!(N <= MAX_VIOLATION_KEYS, "a violation carries at most eight keys") };
        let mut keys = [("", 0); MAX_VIOLATION_KEYS];
        for (slot, key) in keys.iter_mut().zip(given) {
            *slot = key;
        }
        let n_keys = u8::try_from(N).unwrap_or(u8::MAX);
        Violation { clause, message, keys, n_keys }
    }

    #[must_use]
    pub fn keys(&self) -> &[(&'static str, i128)] {
        self.keys.get(..usize::from(self.n_keys)).unwrap_or(&self.keys)
    }
}

/// An engineering limit reached: a reservation, a field width or an index size, naming the declaration to enlarge.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapacityExceeded {
    pub what: &'static str,
    pub declared: i128,
    pub needed: i128,
}

/// A violation key, read as `i128` so every integer and every counted type fits without loss.
pub trait Key {
    fn key(self) -> i128;
}

macro_rules! lossless_keys {
    ($($t:ty),*) => { $(impl Key for $t { fn key(self) -> i128 { i128::from(self) } })* };
}
lossless_keys!(i8, i16, i32, i64, i128, u8, u16, u32, u64, bool);

impl Key for usize {
    /// Every supported target's pointer is at most 64 bits wide, so the fallback is never taken.
    fn key(self) -> i128 {
        u64::try_from(self).map_or(i128::MAX, i128::from)
    }
}

/// The one way the run stops: a panic whose payload the application's hook writes out before aborting.
#[expect(clippy::panic, reason = "a contract violation stops the run by design; nothing catches it")]
fn stop<P: std::any::Any + Send + 'static>(payload: P) -> ! {
    std::panic::panic_any(payload)
}

pub fn raise(v: Violation) -> ! {
    stop(v)
}

pub fn raise_capacity(c: CapacityExceeded) -> ! {
    stop(c)
}

/// Stops the run on an impossible state: `violation!(clause = "<clause>", "what is impossible", key = value, …)`.
#[macro_export]
macro_rules! violation {
    (clause = $clause:literal, $message:literal $(, $key:ident = $value:expr)* $(,)?) => {
        $crate::violation::raise($crate::violation::Violation::new(
            $clause,
            $message,
            [$((stringify!($key), $crate::violation::Key::key($value))),*],
        ))
    };
}

/// Stops the run on an engineering limit: `capacity_exceeded!("what", declared, needed)`.
#[macro_export]
macro_rules! capacity_exceeded {
    ($what:literal, $declared:expr, $needed:expr $(,)?) => {
        $crate::violation::raise_capacity($crate::violation::CapacityExceeded {
            what: $what,
            declared: $crate::violation::Key::key($declared),
            needed: $crate::violation::Key::key($needed),
        })
    };
}

#[cfg(test)]
pub mod testing {
    use super::Violation;

    /// Runs `f`, which must violate, and returns the clause its payload names.
    ///
    /// # Panics
    /// When `f` returns, or stops with a payload that is not a `Violation`.
    pub fn violated_clause<R>(f: impl FnOnce() -> R + std::panic::UnwindSafe) -> &'static str {
        let Err(payload) = std::panic::catch_unwind(f) else {
            panic!("expected a violation");
        };
        payload.downcast_ref::<Violation>().expect("the payload is a Violation").clause
    }
}

#[cfg(test)]
mod tests {
    use super::testing::violated_clause;

    #[test]
    fn violation_carries_clause_message_and_keys() {
        let Err(payload) = std::panic::catch_unwind(|| violation!(clause = "NUM.5", "mixed", a = 1_i64, b = 7_u8));
        let v = payload.downcast_ref::<super::Violation>().expect("payload");
        assert_eq!((v.clause, v.message, v.keys()), ("NUM.5", "mixed", &[("a", 1), ("b", 7)][..]));
        assert_eq!(violated_clause(|| violation!(clause = "Law 7", "overflow")), "Law 7");
    }

    #[test]
    fn capacity_exceeded_carries_its_own_payload() {
        let Err(caught) = std::panic::catch_unwind(|| capacity_exceeded!("slots", 4_u32, 9_u32));
        let c = caught.downcast_ref::<super::CapacityExceeded>().expect("payload");
        assert_eq!((c.what, c.declared, c.needed), ("slots", 4, 9));
    }
}
