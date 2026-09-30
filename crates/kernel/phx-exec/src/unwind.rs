//! A stopping run's panic, carried from the worker that raised it to the caller of its dispatch: the day never runs
//! through it but to stop.

/// A panic's payload, whatever it raised.
pub(crate) type Payload = Box<dyn std::any::Any + Send>;

/// A payload moved to the heap behind a thin pointer, as an atomic holds it until the caller takes it back.
pub(crate) fn held(payload: Payload) -> *mut Payload {
    Box::into_raw(Box::new(payload))
}
