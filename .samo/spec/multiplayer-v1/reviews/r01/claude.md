# Reviewer B — Claude

## summary

Strong first draft: all nine mandatory sections are present, the non-goals are unusually disciplined, and the TDD-first test list exists. The main weaknesses are (a) a broken architecture placeholder that contradicts the real content, (b) a confused RTT-vs-one-way budget in §2.5 that undermines the sync-model justification, (c) underspecified garbage/B2B rules and undefined sudden-death behavior at the 10-minute cap, (d) missing automated coverage for the snapshot-delta pipeline, input-delay behavior, heartbeat stall-timeout, bandwidth, and 200-match concurrency claims, and (e) a set of smaller wire-protocol ambiguities (Ping vs Pong, Input bit layout, capabilities negotiation, delta encoding, garbage-RNG derivation) that must be pinned before a versioned v1 protocol can claim cross-implementation compatibility.

## contradiction

- (major) §4 contains an architecture placeholder block that literally reads `(architecture not yet specified)` between the `<!-- architecture:begin -->` and `<!-- architecture:end -->` markers, directly contradicting the detailed content in §§4.1–4.3. Either the placeholder must be removed or the real diagram moved inside the block; as written, an automated doc-generator reading only the marked region will emit `architecture not yet specified` for a spec that does in fact specify architecture.
- (major) §2.5 table row reads: `| RTT budget | ≤ 150 ms one-way latency for "feels responsive" |`. RTT (round-trip time) and one-way latency are different quantities; the budget can be one or the other but not both. This is load-bearing for §2.3's claim that RTT + 2-tick input delay is imperceptible — if the real target is 150 ms one-way (i.e. 300 ms RTT) the perceptibility claim is wrong.
- (major) §4.3 states snapshots `are sent unconditionally every tick at 60 Hz; the server rate-limits them to a minimum tick gap of 1 even when acks stall.` A minimum gap of 1 tick at 60 Hz IS every tick, so the 'rate-limits' clause either restates the previous clause or silently means something else (e.g. gap of ≥1 tick under backpressure). The actual backpressure policy is undefined.

## ambiguity

- (major) Garbage-exchange rules are underspecified. §5.3 gives `garbage_out = lines_cleared - 1` and a specific `B2B quad → 5` value; §6.1 adds a test for `B2B × 1.5 rounding`. Rounding mode (floor/ceil/banker's) is not stated; B2B doubles and triples are not specified; interaction with the `up to 8 rows` drain cap in §5.3 (does excess stay queued? drop?) is not stated; and §6.1's garbage-table test does not cover the cap or the B2B-double/triple cases.
- (major) §2.5 specifies a 10-minute hard cap with `otherwise sudden-death`, but sudden-death mechanics are never defined anywhere in the spec — how the winner is determined (by score? by well height? coin flip?), whether garbage rate changes, or whether the cap terminates the match outright. A release-blocking rule with no defined behavior.
- (minor) §5.4 says `client sends Pong(nonce) every 500 ms; server replies`, but §4.3's frame catalogue lists only `Pong (0x09)` with no `Ping`. Either the naming is inverted (clients should send `Ping`, server responds with `Pong`) or both sides emit `Pong`, which makes correlation ambiguous and inconsistent with RFC 6455 semantics.
- (minor) §4.2 defines `sim::Match::step(inputs_a, inputs_b, dt) -> …` while §5.1 says `time advances only via step(dt) where dt is always exactly one tick on the server`. Exposing `dt` as a parameter while requiring it to be a constant invites determinism drift — if the client mirror ever passes a different `dt` (e.g. for catch-up), determinism breaks silently. Spec should either remove the parameter or state the permitted values and what happens otherwise.
- (minor) §5.3's garbage RNG is described as `seeded from match seed + tick + recipient id, so deterministic`, but the mixing function is not defined. §5.1 separately says the simulation uses a single `ChaCha20Rng` seeded from the match seed. If the garbage RNG is a sub-stream of the sim RNG the call-order must be pinned; if it is an independent RNG the derivation function (e.g. `ChaCha20Rng::from_seed(hash(match_seed ∥ tick ∥ recipient))`) must be specified. Either way, two reasonable implementations can diverge.
- (minor) §4.3's `Input` frame is described as `bitset of Inputs`, but the bit layout (which input maps to which bit) is not specified. For a versioned wire protocol where `Input` is replay-safe and cross-client, the bit assignment is load-bearing and should be pinned in the spec.
- (minor) §4.3's `Hello` frame carries `capabilities`, but the set of legal capability tags and the negotiation algorithm (intersection? fail on unknown? required vs optional caps?) are not defined. Given §1's `Version mismatch → clean refusal` non-goal, the role of `capabilities` in v1 is unclear — is it forward-compatibility scaffolding, or does it gate behavior in v1?
- (minor) §5.6 defines both `--server URL` and `BLOCKTXT_SERVER` env var, but precedence when both are set is not specified. Convention varies (CLI usually wins); a spec that names both should say which.
- (minor) §5.3 HUD garbage indicator reuses existing `DIM → OVERLAY → NEW_BEST` colour ramp, but the pending-row thresholds at which each step triggers are not defined. Without thresholds, two implementations render different HUDs for identical state.
- (minor) §2.3 promises `No client-side prediction in v1` with `server-authoritative fixed-tick`, and §4.2 describes `SnapshotMirror` as `One tick behind at worst`. The visible rendering policy is under-described: is the local player's own piece rendered from the server snapshot (perceived lag = RTT + 2 ticks) or from a local input-echo ghost? §5.2 state diagram doesn't resolve this, and the choice materially affects feel on the stated ≤150 ms budget.

## weak-testing

- (major) The tests plan has no coverage for the snapshot pipeline itself: no test for delta-snapshot encoding/decoding correctness, no test for recovery when `ack_seq` falls behind beyond a one-tick window, no test for the `INPUT_DELAY = 2 ticks` pipeline (e.g. that an input pressed at client tick T is applied at server tick T+2 under nominal RTT), and no test for the 3-second heartbeat-stall → auto-award rule in §5.4. These are the core correctness properties of the chosen sync model.
- (major) §2.5 commits to concrete performance budgets: `≤ 8 KiB/s` steady-state client bandwidth and `200 concurrent matches per 1-vCPU / 512 MiB` server. §6.1's only performance test is a criterion bench of `server step for 2 active matches < 1 ms`. There is no bandwidth measurement and no load test at or near the 200-match target, so both headline numbers are unverified claims.
- (minor) Security-posture rules in §5.5 have thin automated coverage. `≤ 10 JoinRoom attempts per IP per minute` rate-limit, `silently dropped` invalid-input rule, and `7-day log retention` are all asserted but have no test (automated or manual) in §6. The fuzz target on `proto::decode` partially covers crash-resistance but not validation/rate limiting.
- (minor) §6.1 cross-arch determinism lists macOS arm64 and Linux x86_64, but §8 Sprint 4 QA matrix adds macOS x86_64, and §6.3 CI additions do not include it. The determinism SHA-256 digest check as specified does not cover the platform QA is expected to sign off on.
- (minor) `Ctrl-C / SIGTERM sends Goodbye(UserQuit) before closing the socket` (§5.4) is only exercised by a manual checklist item in §6.4. A signal-driven graceful-shutdown path is easy to regress silently; an automated test spawning the binary and sending SIGTERM would be cheap and is missing.

## missing-requirement

- (minor) Mandatory baseline check: all nine sections are present in the document (version header `SPEC v0.1`; §1 goal/why; §3 five user stories with persona+action+outcome; §4 architecture — though see the `architecture not yet specified` contradiction above; §5 implementation details; §6 tests plan with red/green TDD call-out in §6.1; §7 team with counts and skill labels totalling 4 FT + 1 split; §8 4-sprint plan with explicit `→`/`∥` parallelization; §10 changelog). No missing-section finding is raised; this entry is informational.

## suggested-next-version

v0.2

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "contradiction",
      "text": "§4 contains an architecture placeholder block that literally reads `(architecture not yet specified)` between the `<!-- architecture:begin -->` and `<!-- architecture:end -->` markers, directly contradicting the detailed content in §§4.1–4.3. Either the placeholder must be removed or the real diagram moved inside the block; as written, an automated doc-generator reading only the marked region will emit `architecture not yet specified` for a spec that does in fact specify architecture.",
      "severity": "major"
    },
    {
      "category": "contradiction",
      "text": "§2.5 table row reads: `| RTT budget | ≤ 150 ms one-way latency for \"feels responsive\" |`. RTT (round-trip time) and one-way latency are different quantities; the budget can be one or the other but not both. This is load-bearing for §2.3's claim that RTT + 2-tick input delay is imperceptible — if the real target is 150 ms one-way (i.e. 300 ms RTT) the perceptibility claim is wrong.",
      "severity": "major"
    },
    {
      "category": "contradiction",
      "text": "§4.3 states snapshots `are sent unconditionally every tick at 60 Hz; the server rate-limits them to a minimum tick gap of 1 even when acks stall.` A minimum gap of 1 tick at 60 Hz IS every tick, so the 'rate-limits' clause either restates the previous clause or silently means something else (e.g. gap of ≥1 tick under backpressure). The actual backpressure policy is undefined.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "Garbage-exchange rules are underspecified. §5.3 gives `garbage_out = lines_cleared - 1` and a specific `B2B quad → 5` value; §6.1 adds a test for `B2B × 1.5 rounding`. Rounding mode (floor/ceil/banker's) is not stated; B2B doubles and triples are not specified; interaction with the `up to 8 rows` drain cap in §5.3 (does excess stay queued? drop?) is not stated; and §6.1's garbage-table test does not cover the cap or the B2B-double/triple cases.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§2.5 specifies a 10-minute hard cap with `otherwise sudden-death`, but sudden-death mechanics are never defined anywhere in the spec — how the winner is determined (by score? by well height? coin flip?), whether garbage rate changes, or whether the cap terminates the match outright. A release-blocking rule with no defined behavior.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "The tests plan has no coverage for the snapshot pipeline itself: no test for delta-snapshot encoding/decoding correctness, no test for recovery when `ack_seq` falls behind beyond a one-tick window, no test for the `INPUT_DELAY = 2 ticks` pipeline (e.g. that an input pressed at client tick T is applied at server tick T+2 under nominal RTT), and no test for the 3-second heartbeat-stall → auto-award rule in §5.4. These are the core correctness properties of the chosen sync model.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "§2.5 commits to concrete performance budgets: `≤ 8 KiB/s` steady-state client bandwidth and `200 concurrent matches per 1-vCPU / 512 MiB` server. §6.1's only performance test is a criterion bench of `server step for 2 active matches < 1 ms`. There is no bandwidth measurement and no load test at or near the 200-match target, so both headline numbers are unverified claims.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.4 says `client sends Pong(nonce) every 500 ms; server replies`, but §4.3's frame catalogue lists only `Pong (0x09)` with no `Ping`. Either the naming is inverted (clients should send `Ping`, server responds with `Pong`) or both sides emit `Pong`, which makes correlation ambiguous and inconsistent with RFC 6455 semantics.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§4.2 defines `sim::Match::step(inputs_a, inputs_b, dt) -> …` while §5.1 says `time advances only via step(dt) where dt is always exactly one tick on the server`. Exposing `dt` as a parameter while requiring it to be a constant invites determinism drift — if the client mirror ever passes a different `dt` (e.g. for catch-up), determinism breaks silently. Spec should either remove the parameter or state the permitted values and what happens otherwise.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.3's garbage RNG is described as `seeded from match seed + tick + recipient id, so deterministic`, but the mixing function is not defined. §5.1 separately says the simulation uses a single `ChaCha20Rng` seeded from the match seed. If the garbage RNG is a sub-stream of the sim RNG the call-order must be pinned; if it is an independent RNG the derivation function (e.g. `ChaCha20Rng::from_seed(hash(match_seed ∥ tick ∥ recipient))`) must be specified. Either way, two reasonable implementations can diverge.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§4.3's `Input` frame is described as `bitset of Inputs`, but the bit layout (which input maps to which bit) is not specified. For a versioned wire protocol where `Input` is replay-safe and cross-client, the bit assignment is load-bearing and should be pinned in the spec.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§4.3's `Hello` frame carries `capabilities`, but the set of legal capability tags and the negotiation algorithm (intersection? fail on unknown? required vs optional caps?) are not defined. Given §1's `Version mismatch → clean refusal` non-goal, the role of `capabilities` in v1 is unclear — is it forward-compatibility scaffolding, or does it gate behavior in v1?",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.6 defines both `--server URL` and `BLOCKTXT_SERVER` env var, but precedence when both are set is not specified. Convention varies (CLI usually wins); a spec that names both should say which.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.3 HUD garbage indicator reuses existing `DIM → OVERLAY → NEW_BEST` colour ramp, but the pending-row thresholds at which each step triggers are not defined. Without thresholds, two implementations render different HUDs for identical state.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "Security-posture rules in §5.5 have thin automated coverage. `≤ 10 JoinRoom attempts per IP per minute` rate-limit, `silently dropped` invalid-input rule, and `7-day log retention` are all asserted but have no test (automated or manual) in §6. The fuzz target on `proto::decode` partially covers crash-resistance but not validation/rate limiting.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§6.1 cross-arch determinism lists macOS arm64 and Linux x86_64, but §8 Sprint 4 QA matrix adds macOS x86_64, and §6.3 CI additions do not include it. The determinism SHA-256 digest check as specified does not cover the platform QA is expected to sign off on.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "`Ctrl-C / SIGTERM sends Goodbye(UserQuit) before closing the socket` (§5.4) is only exercised by a manual checklist item in §6.4. A signal-driven graceful-shutdown path is easy to regress silently; an automated test spawning the binary and sending SIGTERM would be cheap and is missing.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§2.3 promises `No client-side prediction in v1` with `server-authoritative fixed-tick`, and §4.2 describes `SnapshotMirror` as `One tick behind at worst`. The visible rendering policy is under-described: is the local player's own piece rendered from the server snapshot (perceived lag = RTT + 2 ticks) or from a local input-echo ghost? §5.2 state diagram doesn't resolve this, and the choice materially affects feel on the stated ≤150 ms budget.",
      "severity": "minor"
    },
    {
      "category": "missing-requirement",
      "text": "Mandatory baseline check: all nine sections are present in the document (version header `SPEC v0.1`; §1 goal/why; §3 five user stories with persona+action+outcome; §4 architecture — though see the `architecture not yet specified` contradiction above; §5 implementation details; §6 tests plan with red/green TDD call-out in §6.1; §7 team with counts and skill labels totalling 4 FT + 1 split; §8 4-sprint plan with explicit `→`/`∥` parallelization; §10 changelog). No missing-section finding is raised; this entry is informational.",
      "severity": "minor"
    }
  ],
  "summary": "Strong first draft: all nine mandatory sections are present, the non-goals are unusually disciplined, and the TDD-first test list exists. The main weaknesses are (a) a broken architecture placeholder that contradicts the real content, (b) a confused RTT-vs-one-way budget in §2.5 that undermines the sync-model justification, (c) underspecified garbage/B2B rules and undefined sudden-death behavior at the 10-minute cap, (d) missing automated coverage for the snapshot-delta pipeline, input-delay behavior, heartbeat stall-timeout, bandwidth, and 200-match concurrency claims, and (e) a set of smaller wire-protocol ambiguities (Ping vs Pong, Input bit layout, capabilities negotiation, delta encoding, garbage-RNG derivation) that must be pinned before a versioned v1 protocol can claim cross-implementation compatibility.",
  "suggested_next_version": "v0.2",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
