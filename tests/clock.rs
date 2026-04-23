//! Tests for `clock.rs` — monotonic Clock trait.

use blocktxt::clock::{Clock, FakeClock, RealClock};
use std::time::{Duration, Instant};

// ── RealClock ─────────────────────────────────────────────────────────────

/// Two successive `now()` calls on `RealClock` must be monotonically
/// non-decreasing.
#[test]
fn real_clock_advances_monotonically() {
    let clock = RealClock;
    let t1 = clock.now();
    let t2 = clock.now();
    assert!(t2 >= t1, "time went backwards: {t2:?} < {t1:?}");
}

// ── FakeClock ─────────────────────────────────────────────────────────────

/// `advance` on a `FakeClock` must shift `now()` by exactly the given
/// duration.
#[test]
fn fake_clock_advance_deterministic() {
    let start = Instant::now();
    let clock = FakeClock::new(start);

    assert_eq!(clock.now(), start);

    clock.advance(Duration::from_secs(1));
    assert_eq!(clock.now(), start + Duration::from_secs(1));

    clock.advance(Duration::from_millis(500));
    assert_eq!(clock.now(), start + Duration::from_millis(1500));
}

/// Advancing from a background thread must be visible on the main thread.
#[test]
fn fake_clock_shared_across_threads() {
    let start = Instant::now();
    let clock = FakeClock::new(start);
    let clock2 = clock.clone();

    let handle = std::thread::spawn(move || {
        clock2.advance(Duration::from_secs(42));
    });

    handle.join().expect("thread panicked");

    assert_eq!(
        clock.now(),
        start + Duration::from_secs(42),
        "advance from thread not visible on main thread"
    );
}
