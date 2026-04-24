# decisions

- No review-loop decisions yet.

## Round 1 — 2026-04-24T04:15:20.730Z

- accepted missing-risk#1: Added §4.4 host-admission step (JoinRequest/HostDecision) so the 5-char code is no longer the only credential — host must explicitly approve each join.
- accepted missing-risk#2: §4.4 now pins HostRoom, concurrent sessions/rooms per IP, idle-room lifetime, invalid-frame budget, and per-node caps as enforced limits.
- accepted missing-risk#3: §2.5 and §4.4 add an explicit 256-match admission cap with ServerFull rejection and 1024-session TLS-accept cap so overload fails closed rather than degrading live matches.
- accepted weak-implementation#1: §4.4 defines a 32-snapshot ring, keyframe fallback, 60-snapshot disconnect threshold, outbound queue depth of 8, and coalesce-on-backpressure — removes the unbounded path for a lagging client.
- accepted missing-risk#4: §4.4 pins 16 KiB max WebSocket message, 64 KiB max decoded allocation, per-client frame rate, invalid-frame budget, and per-IP malformed-frame ban.
- accepted missing-risk#5: §1, §2.2, and §4.6 now make v0.2 an explicit single-node single-region beta; horizontal scale + registry/sticky routing are a v0.3 workstream with an explicit restart-loses-matches behaviour and a clear upgrade path.
- accepted missing-risk#6: §5.5.4 binds /metrics to 127.0.0.1:9090 and documents that self-hosters must supply their own proxy/auth for remote scraping.
- accepted missing-risk#7: §4.5 mandates client-side stripping to printable ASCII + column cap; Error.detail is declared ASCII-only on the wire and validated both sides.
- accepted unnecessary-scope#1: §5.6 removes `mp local` from the public CLI; the two-player single-process path now lives only behind the internal-harness cargo feature for CI use.
- accepted weak-implementation#2: §5.5.3 specifies blake3(salt ∥ ip)[..8] with a 256-bit salt rotated every 24 h, previous 7 salts retained in memory only, raw IPs purged on session close, 7-day log retention.
- accepted contradiction#1: §4 no longer contains the `(architecture not yet specified)` placeholder; the diagram and §4.1–4.6 are the only architecture content.
- accepted contradiction#2: §2.5 now states 150 ms is round-trip (≈75 ms one-way) and §2.3's perceptibility claim references RTT explicitly.
- accepted contradiction#3: Rewrote §2.3 and §4.4 to replace the self-restating rate-limit clause with a concrete bounded ack-window + keyframe + coalesce-on-backpressure policy.
- accepted ambiguity#1: §5.3.1 gives a complete integer-only garbage table (single through perfect clear, combos, T-spins), replaces B2B ×1.5 with flat +1 to eliminate rounding questions, pins a 40-row queue cap with drop-behaviour and an 8-row drain cap.
- accepted ambiguity#2: §5.3.2 defines sudden-death at tick 36000: drain cap raised, periodic drip garbage, height tiebreak, match_id parity final tiebreak.
- accepted weak-testing#1: §6.1 adds TDD cases for snapshot delta/keyframe round-trip, ack-window boundary, input-delay pipeline, heartbeat stall auto-award, and ANSI sanitization.
- accepted weak-testing#2: §6.2 adds a bandwidth-budget integration test (≤8 KiB/s mean, ≤32 KiB/s peak) and a nightly 200-match load-test job with explicit tick-budget and admission assertions.
- accepted ambiguity#3: §4.3 introduces a distinct Ping (0x0C) frame from the client and Pong (0x0D) from the server, matching RFC 6455 directional semantics.
- accepted ambiguity#4: §4.2 and §5.1 remove the `dt` parameter from `sim::Match::step`; each call advances exactly one tick with no other cadence permitted.
- accepted ambiguity#5: §5.3.3 pins garbage RNG derivation to ChaCha20Rng::from_seed(blake3(match_seed ∥ recipient_id ∥ tick)[..32]) and flags it as part of the wire contract; §6.1 adds a golden-vector cross-arch test.
- accepted ambiguity#6: §4.3 pins the u16 Input bit layout exhaustively; reserved bits must be zero and set reserved bits close the connection with Error(MalformedInput).
- accepted ambiguity#7: §4.3 removes `capabilities` from Hello for v1; forward compatibility is handled strictly by proto_version bumps, removing a downgrade-attack surface.
- accepted ambiguity#8: §5.6 now states that when both --server and BLOCKTXT_SERVER are set, --server wins, matching common CLI convention.
- accepted ambiguity#9: §5.3.4 pins the pending-row HUD colour-ramp thresholds (0, 1–3 DIM, 4–7 OVERLAY, 8+ NEW_BEST, capped flashing at 4 Hz).
- accepted weak-testing#3: §6.2 adds explicit rate-limit enforcement, admission-cap, and frame-size-enforcement integration tests, covering the previously-assertion-only §5.5 rules.
- accepted weak-testing#4: §6.3 adds macOS x86_64 to the determinism CI matrix so the digest check covers every platform QA signs off on.
- accepted weak-testing#5: §6.2 adds an automated SIGTERM graceful-shutdown test that spawns the binary, sends SIGTERM, and asserts Goodbye(UserQuit) plus zero exit within 2 s.
- accepted ambiguity#10: §2.3 now specifies a local input-echo ghost for the player's own active piece only; well, opponent, lock, clear, and garbage are rendered strictly from the server snapshot, with explicit snap-to-server on divergence.
- accepted missing-requirement#1: Acknowledged as informational — all nine mandatory baseline sections are present; no action required beyond noting it.
