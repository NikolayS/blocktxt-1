# Reviewer B — Claude

## summary

The spec covers all nine mandatory baseline sections and is strong on scope discipline, topology rationale, and sprint parallelisation. The most serious gaps are: (1) the §4 architecture placeholder contradicts the populated §4.1–4.3; (2) the Sprint-1 feature-flag rule contradicts Sprint 2 shipping `mp local`; (3) garbage-exchange prose (§5.3) does not match the B2B ×1.5 table referenced by its test (§6.1); (4) RTT-vs-one-way conflated in §2.5; (5) heartbeat uses `Pong` with no `Ping` frame defined. Multiple enums (Goodbye/Error reasons), the fixed-point arithmetic scheme, sudden-death rules, and garbage queue-overflow behaviour are referenced but never specified. Testing is weak on the load-bearing claims: bandwidth budget, 200-match scale target, adversarial input validation, rate limiting, and silent-stall heartbeat timeout all lack corresponding tests. No idea-contradiction findings — the original idea contains no explicit `NOT X` disclaimers to violate.

## contradiction

- (major) §4 Architecture contains a placeholder block between `architecture:begin` and `architecture:end` that reads "(architecture not yet specified)" yet §4.1–4.3 provide a detailed architecture (component diagram, key abstractions, wire protocol). A mechanical reader (or template tool) honouring the marker block would conclude the architecture section is empty/unfilled. Either populate the marker block with the diagram from §4.1 or delete the placeholder; as written the two statements directly contradict each other.
- (major) §8 Sprint 1 states the `multiplayer` feature flag is "defaulted *off* until Sprint 3 so main stays shippable", yet Sprint 2 ships `blocktxt mp local` as a user-facing dev aid. `mp local` lives inside the multiplayer subcommand tree and cannot ship in Sprint 2 if the flag only turns on in Sprint 3. Resolve by either flipping the flag in Sprint 2 or gating `mp local` behind a separate flag.
- (major) §2.5 scale table labels the 150 ms budget as both RTT and one-way latency in the same row: `RTT budget | ≤ 150 ms one-way latency for "feels responsive"`. RTT (round-trip) and one-way are factor-of-2 different; the protocol's 2-tick input-delay argument in §2.3 only checks out under one interpretation. Pick one definition and use it consistently (the downstream heartbeat, input-delay, and manual-test thresholds all depend on it).
- (major) §5.3 specifies garbage via formula `garbage_out = lines_cleared - 1` plus a single carve-out "B2B quads send 5", but §6.1 requires a test that `lines_cleared → garbage_out` matches "the specified table exactly, including B2B × 1.5 rounding". No such table and no ×1.5 rule appear anywhere in the spec. The test cannot be written against the current text; either add the full table (including B2B behaviour for triples/doubles and the ×1.5 rounding rule) or rewrite §6.1 to match the simple formula.

## ambiguity

- (major) §5.4 says "client sends `Pong(nonce)` every 500 ms; server replies" and §4.3 lists only `Pong` (0x09) — there is no `Ping` frame. In standard usage the initiator sends Ping and the responder sends Pong; here the client is sending Pong unsolicited. It is also unclear what the server replies *with* (another Pong? An Ack?) and which side's 3 s timeout the "no pong for 3 s" rule refers to. Add a `Ping` frame or rename, and state explicitly who measures what timeout.
- (major) §5.3 states the server drains "up to 8 rows of garbage" before the next piece spawn, but does not say what happens to rows in excess of 8 in the queue. Do they persist to the next spawn, get coalesced, or get dropped? This is directly observable to players (it changes pressure dynamics after a quad combo) and must be pinned down.
- (major) §2.5 caps match duration at 10 minutes "otherwise sudden-death", but sudden-death is never defined anywhere else in the spec. Which rules change (gravity speed-up, forced garbage, simultaneous top-out resolution, draw handling)? Without a definition neither the server sim nor the manual test plan can verify it.
- (major) §5.1 and §9 rely on "fixed-point only" math to guarantee cross-arch determinism, but no section specifies which fields convert from float to fixed-point, what the scale/precision is, or how this interacts with the existing v0.1 simulation (which presumably uses floats for gravity, DAS, etc.). The determinism claim and the cross-arch CI test in §6.2 both rest on an unspecified conversion.
- (major) Error/Goodbye enum values are referenced but not enumerated. §5.4 names `Goodbye(PeerDisconnected)` and `Goodbye(UserQuit)`; §6.1 names `Error(VersionMismatch)`; user story 2 implies discriminated errors for DNS/TLS/handshake/version-mismatch/room-not-found. §4.3 merely describes the frames as "reason (enum)" and "code, human-readable string" without listing the enum members. Add an authoritative enum table — this is wire-breaking once v1 ships.
- (minor) §5.6 defines both `--server URL` and the `BLOCKTXT_SERVER` env var but does not specify precedence when both are set, nor what happens when the value is malformed (crash vs fall back to compiled-in default). CLI precedence rules should be unambiguous before shipping.
- (minor) §5.6 shows `blocktxt mp join ABCDE` (uppercase) but §4.2 defines codes as Crockford-base32, which is conventionally case-insensitive on input. The spec does not say whether `abcde` is accepted, normalised, or rejected. Given user story 2 explicitly emphasises actionable errors for room-not-found, case handling must be pinned down.
- (major) §5.3 seeds garbage-gap RNG from "match seed + tick + recipient id", but recipient id is not defined anywhere (is it host=0/guest=1? a UUID? the room code?). Two clients computing different recipient ids will produce divergent garbage and desync the determinism test in §6.2. Pin this down.
- (minor) §2.3 states "No client-side prediction in v1" but §4.2 defines a `SnapshotMirror` that the render thread reads "each frame" with the net thread "one tick behind at worst". If the render frame rate is 60 fps and the network is lossy/jittery, the render loop will either block (stutter) or display a stale tick. The spec does not say which, nor what the mirror does when no new snapshot has arrived by frame time. This is visible to players.

## weak-testing

- (major) §6.1 tick-budget invariant tests only 2 active matches, but §2.5 targets 200 concurrent matches per 1 vCPU. A 2-match microbench tells you almost nothing about whether the 200-match scale target holds (scheduler overhead, allocator pressure, and cache behaviour all change non-linearly). Add a soak or load test that exercises a realistic match count, or explicitly lower the advertised scale target.
- (major) The §2.5 bandwidth budget (≤ 8 KiB/s steady state per client, delta-compressed snapshots) has no corresponding test. Given delta-compression is claimed as the mechanism that makes the budget achievable, this is load-bearing and should be asserted in an integration test that measures bytes-on-wire over a scripted match.
- (major) §5.5 promises the server "validates every client `Input` against the simulation" and §5.5 also promises rate limiting on `JoinRoom` (10/min/IP), but §6 contains no tests for either: no adversarial-input test (malformed bitsets, inputs from the wrong player slot, inputs for a match that ended), and no rate-limit test. For a publicly-exposed service these are the tests you actually need.
- (major) §6.2 "Loopback match: … script a 30-second match" does not say what the scripted inputs are or what invariants the assertion checks beyond "`MatchResult` event is emitted and both clients exit cleanly". Without a deterministic input script and explicit output expectations (final scores, winner identity, garbage totals) the test is unlikely to catch regressions.
- (major) Heartbeat/stall detection (§5.4: "No pong for 3 s → server declares the session stalled and awards the match to the peer") is one of the most subtle pieces of the protocol and has no unit test. §6.2 has a "kill one client's socket" test, but TCP-reset is not the same code path as a silent stall (no RST, just packet loss). Add a test that simulates dropped packets/zero bandwidth without closing the socket.
- (minor) §6.1 version-mismatch test only exercises the client-newer-than-server direction (`proto_version = 2` against v1). The reverse (old client against new server) is the common rollout failure and is not tested. Add a symmetric test.
- (minor) Client-side snapshot-mirror correctness (SPSC slot, "one tick behind at worst") is called out as a key abstraction in §4.2 but has no test in §6. If the mirror tears or goes stale the render layer silently shows wrong state; this is exactly the kind of concurrency code that needs a targeted unit test with a stressing harness.

## missing-requirement

- (major) §9 risk table mentions "Fixed-point only" as the desync mitigation, but §5 Implementation details contains no subsection specifying the fixed-point representation, arithmetic rules, or conversion strategy from the existing v0.1 float-based simulation. Either add a §5.x "Deterministic arithmetic" subsection or drop the fixed-point commitment.

## suggested-next-version

v0.2

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "contradiction",
      "text": "§4 Architecture contains a placeholder block between `architecture:begin` and `architecture:end` that reads \"(architecture not yet specified)\" yet §4.1–4.3 provide a detailed architecture (component diagram, key abstractions, wire protocol). A mechanical reader (or template tool) honouring the marker block would conclude the architecture section is empty/unfilled. Either populate the marker block with the diagram from §4.1 or delete the placeholder; as written the two statements directly contradict each other.",
      "severity": "major"
    },
    {
      "category": "contradiction",
      "text": "§8 Sprint 1 states the `multiplayer` feature flag is \"defaulted *off* until Sprint 3 so main stays shippable\", yet Sprint 2 ships `blocktxt mp local` as a user-facing dev aid. `mp local` lives inside the multiplayer subcommand tree and cannot ship in Sprint 2 if the flag only turns on in Sprint 3. Resolve by either flipping the flag in Sprint 2 or gating `mp local` behind a separate flag.",
      "severity": "major"
    },
    {
      "category": "contradiction",
      "text": "§2.5 scale table labels the 150 ms budget as both RTT and one-way latency in the same row: `RTT budget | ≤ 150 ms one-way latency for \"feels responsive\"`. RTT (round-trip) and one-way are factor-of-2 different; the protocol's 2-tick input-delay argument in §2.3 only checks out under one interpretation. Pick one definition and use it consistently (the downstream heartbeat, input-delay, and manual-test thresholds all depend on it).",
      "severity": "major"
    },
    {
      "category": "contradiction",
      "text": "§5.3 specifies garbage via formula `garbage_out = lines_cleared - 1` plus a single carve-out \"B2B quads send 5\", but §6.1 requires a test that `lines_cleared → garbage_out` matches \"the specified table exactly, including B2B × 1.5 rounding\". No such table and no ×1.5 rule appear anywhere in the spec. The test cannot be written against the current text; either add the full table (including B2B behaviour for triples/doubles and the ×1.5 rounding rule) or rewrite §6.1 to match the simple formula.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.4 says \"client sends `Pong(nonce)` every 500 ms; server replies\" and §4.3 lists only `Pong` (0x09) — there is no `Ping` frame. In standard usage the initiator sends Ping and the responder sends Pong; here the client is sending Pong unsolicited. It is also unclear what the server replies *with* (another Pong? An Ack?) and which side's 3 s timeout the \"no pong for 3 s\" rule refers to. Add a `Ping` frame or rename, and state explicitly who measures what timeout.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.3 states the server drains \"up to 8 rows of garbage\" before the next piece spawn, but does not say what happens to rows in excess of 8 in the queue. Do they persist to the next spawn, get coalesced, or get dropped? This is directly observable to players (it changes pressure dynamics after a quad combo) and must be pinned down.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§2.5 caps match duration at 10 minutes \"otherwise sudden-death\", but sudden-death is never defined anywhere else in the spec. Which rules change (gravity speed-up, forced garbage, simultaneous top-out resolution, draw handling)? Without a definition neither the server sim nor the manual test plan can verify it.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.1 and §9 rely on \"fixed-point only\" math to guarantee cross-arch determinism, but no section specifies which fields convert from float to fixed-point, what the scale/precision is, or how this interacts with the existing v0.1 simulation (which presumably uses floats for gravity, DAS, etc.). The determinism claim and the cross-arch CI test in §6.2 both rest on an unspecified conversion.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "Error/Goodbye enum values are referenced but not enumerated. §5.4 names `Goodbye(PeerDisconnected)` and `Goodbye(UserQuit)`; §6.1 names `Error(VersionMismatch)`; user story 2 implies discriminated errors for DNS/TLS/handshake/version-mismatch/room-not-found. §4.3 merely describes the frames as \"reason (enum)\" and \"code, human-readable string\" without listing the enum members. Add an authoritative enum table — this is wire-breaking once v1 ships.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.6 defines both `--server URL` and the `BLOCKTXT_SERVER` env var but does not specify precedence when both are set, nor what happens when the value is malformed (crash vs fall back to compiled-in default). CLI precedence rules should be unambiguous before shipping.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.6 shows `blocktxt mp join ABCDE` (uppercase) but §4.2 defines codes as Crockford-base32, which is conventionally case-insensitive on input. The spec does not say whether `abcde` is accepted, normalised, or rejected. Given user story 2 explicitly emphasises actionable errors for room-not-found, case handling must be pinned down.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§6.1 tick-budget invariant tests only 2 active matches, but §2.5 targets 200 concurrent matches per 1 vCPU. A 2-match microbench tells you almost nothing about whether the 200-match scale target holds (scheduler overhead, allocator pressure, and cache behaviour all change non-linearly). Add a soak or load test that exercises a realistic match count, or explicitly lower the advertised scale target.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "The §2.5 bandwidth budget (≤ 8 KiB/s steady state per client, delta-compressed snapshots) has no corresponding test. Given delta-compression is claimed as the mechanism that makes the budget achievable, this is load-bearing and should be asserted in an integration test that measures bytes-on-wire over a scripted match.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "§5.5 promises the server \"validates every client `Input` against the simulation\" and §5.5 also promises rate limiting on `JoinRoom` (10/min/IP), but §6 contains no tests for either: no adversarial-input test (malformed bitsets, inputs from the wrong player slot, inputs for a match that ended), and no rate-limit test. For a publicly-exposed service these are the tests you actually need.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "§6.2 \"Loopback match: … script a 30-second match\" does not say what the scripted inputs are or what invariants the assertion checks beyond \"`MatchResult` event is emitted and both clients exit cleanly\". Without a deterministic input script and explicit output expectations (final scores, winner identity, garbage totals) the test is unlikely to catch regressions.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "Heartbeat/stall detection (§5.4: \"No pong for 3 s → server declares the session stalled and awards the match to the peer\") is one of the most subtle pieces of the protocol and has no unit test. §6.2 has a \"kill one client's socket\" test, but TCP-reset is not the same code path as a silent stall (no RST, just packet loss). Add a test that simulates dropped packets/zero bandwidth without closing the socket.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "§6.1 version-mismatch test only exercises the client-newer-than-server direction (`proto_version = 2` against v1). The reverse (old client against new server) is the common rollout failure and is not tested. Add a symmetric test.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "Client-side snapshot-mirror correctness (SPSC slot, \"one tick behind at worst\") is called out as a key abstraction in §4.2 but has no test in §6. If the mirror tears or goes stale the render layer silently shows wrong state; this is exactly the kind of concurrency code that needs a targeted unit test with a stressing harness.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.3 seeds garbage-gap RNG from \"match seed + tick + recipient id\", but recipient id is not defined anywhere (is it host=0/guest=1? a UUID? the room code?). Two clients computing different recipient ids will produce divergent garbage and desync the determinism test in §6.2. Pin this down.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§2.3 states \"No client-side prediction in v1\" but §4.2 defines a `SnapshotMirror` that the render thread reads \"each frame\" with the net thread \"one tick behind at worst\". If the render frame rate is 60 fps and the network is lossy/jittery, the render loop will either block (stutter) or display a stale tick. The spec does not say which, nor what the mirror does when no new snapshot has arrived by frame time. This is visible to players.",
      "severity": "minor"
    },
    {
      "category": "missing-requirement",
      "text": "§9 risk table mentions \"Fixed-point only\" as the desync mitigation, but §5 Implementation details contains no subsection specifying the fixed-point representation, arithmetic rules, or conversion strategy from the existing v0.1 float-based simulation. Either add a §5.x \"Deterministic arithmetic\" subsection or drop the fixed-point commitment.",
      "severity": "major"
    }
  ],
  "summary": "The spec covers all nine mandatory baseline sections and is strong on scope discipline, topology rationale, and sprint parallelisation. The most serious gaps are: (1) the §4 architecture placeholder contradicts the populated §4.1–4.3; (2) the Sprint-1 feature-flag rule contradicts Sprint 2 shipping `mp local`; (3) garbage-exchange prose (§5.3) does not match the B2B ×1.5 table referenced by its test (§6.1); (4) RTT-vs-one-way conflated in §2.5; (5) heartbeat uses `Pong` with no `Ping` frame defined. Multiple enums (Goodbye/Error reasons), the fixed-point arithmetic scheme, sudden-death rules, and garbage queue-overflow behaviour are referenced but never specified. Testing is weak on the load-bearing claims: bandwidth budget, 200-match scale target, adversarial input validation, rate limiting, and silent-stall heartbeat timeout all lack corresponding tests. No idea-contradiction findings — the original idea contains no explicit `NOT X` disclaimers to violate.",
  "suggested_next_version": "v0.2",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
