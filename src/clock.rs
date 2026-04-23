//! Monotonic clock abstraction (SPEC §3).
//!
//! `RealClock` wraps `Instant::now()` for production use.
//! `FakeClock` provides deterministic, thread-safe time control for tests.
//!
//! **Never use `SystemTime`** for game timing — `Instant` is monotonic;
//! `SystemTime` can go backwards.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

// ── trait ─────────────────────────────────────────────────────────────────

/// Monotonic clock used by the game loop.
///
/// `Send + Sync` so it can be shared across threads (e.g. input thread
/// and render thread both reading the same clock).
pub trait Clock: Send + Sync {
    /// Returns the current monotonic instant.
    fn now(&self) -> Instant;
}

// ── real clock ────────────────────────────────────────────────────────────

/// Production clock — delegates straight to `Instant::now()`.
pub struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

// ── fake clock ────────────────────────────────────────────────────────────

/// Deterministic clock for testing.
///
/// All clones / Arc-wrapped copies share the same underlying time, so
/// advancing from one handle is immediately visible on all others.
#[derive(Clone)]
pub struct FakeClock {
    t: Arc<Mutex<Instant>>,
}

impl FakeClock {
    /// Creates a new `FakeClock` starting at `start`.
    pub fn new(start: Instant) -> Self {
        Self {
            t: Arc::new(Mutex::new(start)),
        }
    }

    /// Advances the fake clock by `d`.
    pub fn advance(&self, d: Duration) {
        let mut guard = self.t.lock().expect("FakeClock mutex poisoned");
        *guard += d;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        *self.t.lock().expect("FakeClock mutex poisoned")
    }
}
