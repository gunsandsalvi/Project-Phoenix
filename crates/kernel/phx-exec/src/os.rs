#![expect(unsafe_code, reason = "affinity and thread ids are system calls")]

use crate::consts::MAX_CPUS;

/// The cores this process may run on, from its affinity mask, or none when the system will not say.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[must_use]
pub fn allowed_cores() -> Option<Vec<usize>> {
    // SAFETY: an all-zero cpu_set_t is a valid empty set.
    let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    // SAFETY: the set is a valid, writable cpu_set_t of the size passed; pid 0 is this thread.
    let status = unsafe { libc::sched_getaffinity(0, size_of::<libc::cpu_set_t>(), &raw mut set) };
    // SAFETY: every index is below the set's capacity, `MAX_CPUS`.
    (status == 0).then(|| (0..MAX_CPUS).filter(|c| unsafe { libc::CPU_ISSET(*c, &set) }).collect())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[must_use]
pub fn allowed_cores() -> Option<Vec<usize>> {
    None
}

/// Pins the calling thread to one core; false when the system refuses, which leaves the thread where it was.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[must_use]
pub fn pin_current(core: usize) -> bool {
    if core >= MAX_CPUS {
        return false;
    }
    // SAFETY: as above.
    let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    // SAFETY: the core is below the set's capacity, checked above.
    unsafe { libc::CPU_SET(core, &mut set) };
    // SAFETY: the set is a valid cpu_set_t of the size passed; pid 0 is this thread.
    unsafe { libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &raw const set) == 0 }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[must_use]
pub fn pin_current(_core: usize) -> bool {
    false
}

/// The calling thread's kernel id, which performance-hint sessions name threads by.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[must_use]
pub fn current_tid() -> i32 {
    // SAFETY: gettid has no preconditions and cannot fail.
    unsafe { libc::gettid() }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[must_use]
pub fn current_tid() -> i32 {
    0
}

/// Asks the core to start loading a line it will read soon; a hint with no effect on any value.
#[inline]
pub fn prefetch<T>(ptr: *const T) {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: a prefetch only hints the cache and never faults, whatever the address.
    unsafe {
        std::arch::asm!("prfm pldl1keep, [{0}]", in(reg) ptr, options(nostack, readonly, preserves_flags));
    }
    #[cfg(target_arch = "x86_64")]
    // SAFETY: as above; SSE is part of the x86-64 baseline.
    unsafe {
        std::arch::x86_64::_mm_prefetch::<{ std::arch::x86_64::_MM_HINT_T0 }>(ptr.cast());
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let _ = ptr;
}

/// The system's page size, which the probe reports beside its gathers, or none when the system will not say.
#[cfg(unix)]
#[must_use]
pub fn page_size() -> Option<u64> {
    // SAFETY: sysconf reads a constant of the system and has no preconditions.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    u64::try_from(page).ok()
}

#[cfg(not(unix))]
#[must_use]
pub fn page_size() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_os = "linux")]
    fn this_process_may_run_somewhere() {
        let cores = super::allowed_cores().expect("Linux reports the affinity mask");
        assert!(!cores.is_empty());
        assert!(super::current_tid() > 0);
    }
}
