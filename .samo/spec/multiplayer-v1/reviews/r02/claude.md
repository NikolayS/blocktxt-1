# Reviewer B — Claude

## summary

Nine mandatory baseline sections are all present and the spec is notably mature after r1. The strongest remaining issues are (1) a genuine contradiction — §4's 'architecture not yet specified' placeholder still sits above the real architecture despite the changelog claiming this was fixed; (2) several simulation rules that are part of the wire contract but under-specified in a determinism-breaking way (WELL_WIDTH undefined, garbage-RNG modulo, aggregate-well-height tiebreak, combo formula's N, sudden-death 'every 2 s' vs. ticks, DAS-charge semantics on a per-tick bitset); and (3) a weak-testing gap where ~half of §4.4's admission/backpressure limits have no named test despite §4.4 explicitly claiming they are 'enforced in code, not just documentation'. No idea-contradiction findings: the original idea contained no explicit disclaimers to re-check against the spec.

## contradiction

- (major) §4 still contains an architecture placeholder block (`<!-- architecture:begin -->` … `(architecture not yet specified)` … `<!-- architecture:end -->`) sitting directly above §4.1's actual ASCII diagram. This is exactly the defect the v0.2 changelog claims to have fixed ('fixed architecture placeholder contradiction'), yet the placeholder text is still in the file. Either the placeholder must be removed, or the machine-readable fence must wrap the real §4.1 diagram.

## ambiguity

- (major) §5.3.2 defines the sudden-death tiebreak as 'the player with the lower aggregate well height', but 'aggregate well height' is never defined. It could mean sum of per-column stack heights, max column height, total non-empty cells, or something else; each yields different winners in realistic same-tick double top-outs. Determinism of the tiebreak is claimed but un-specified at the mechanic level, and §6.1 lists no test that pins which definition wins.
- (major) §5.3.1 row 'Combo (N consecutive clears ≥ 2)  floor((N-1)/2)' is under-specified. It is unclear (a) whether 'N' is the current chain length including this clear or the previous chain length, (b) whether 'clears ≥ 2' means each clear must be at least a double, or the combo chain must be ≥ 2 long, and (c) how N resets (on non-clear drop? on a single-line clear?). Different readings differ by ±1 row sent on most combos, so the §6.1 'every row of §5.3.1' test cannot be written deterministically against this table.
- (major) §5.3.3 pins the garbage RNG derivation as 'first u8 % WELL_WIDTH output is the gap column', but WELL_WIDTH is not defined anywhere in the spec (no mention in §4, §5, or §6), and a naive `u8 % W` introduces modulo bias unless W divides 256 evenly. The spec declares this derivation part of the wire contract and requires a cross-platform golden vector, yet two conforming implementations can produce different gap columns just by choosing different rejection-sampling strategies.
- (major) §4.3 defines `Input` bit 4 as 'DAS-charge', but DAS (delayed auto-shift) is a continuous/stateful client concept — a player holds a direction for N ms and then auto-repeats. A per-tick bitset cannot naturally encode 'charge state'. The spec does not say whether the bit means 'DAS is currently charged', 'consume DAS charge this tick', or 'begin charging'. §5.5.1 claims the server validates every Input as a 'named move', but DAS-charge is not a move. This is both an ambiguity and a contradiction with the §4.3/§5.5.1 validation story, and no §6.1 test nails down the semantics.
- (minor) §5.3.1 B2B bonus is '+1 flat, (consecutive quad/T-spin)' but it is not stated whether the +1 applies once per B2B chain start, or per subsequent qualifying clear in the chain. The manual test §6.4 asserts 'B2B quad sends 5 rows (4 + 1)', implying per-clear, but the table text is compatible with either reading. §6.1 should explicitly name the per-clear semantics in a test case.
- (minor) §2.5 claims 'the 150 ms budget is the figure §2.3 uses when it calls RTT + 2-tick input delay imperceptible', but §2.3 nowhere names 150 ms, RTT, or 'imperceptible' — it only says the input delay 'absorbs typical jitter without visible lag'. The self-referential cross-reference is broken and a reviewer cannot verify the RTT budget rationale from §2.3 alone.
- (minor) §5.3.1 'drain cap: up to 8 rows inserted per piece-spawn' does not define 'piece-spawn'. Candidate meanings: every time a new tetromino becomes the active piece (post-lock), every 'next' advance, or every lock-and-respawn including after a clear. Each yields different garbage-delivery cadence under heavy pressure; §6.1 asserts 'drain cap of 8 … leaves the correct residue' but 'correct' is undefined until 'piece-spawn' is.
- (minor) §5.3.2 says 'an extra garbage row is inserted into each well every 2 s' during sudden-death. 2 s at 60 Hz is 120 ticks, but the spec elsewhere forbids wall-clock time in the simulation (§5.1) and the spec does not state 'every 120 ticks starting at tick 36000'. Leaving this as a wall-clock-flavored rule contradicts the determinism contract.
- (minor) §5.2 state-transition diagram omits the `AwaitingAdmit` → (host declines or 15 s timeout) branch, even though §4.4 and user story 2 rely on it. A reviewer following only the diagram cannot determine which state the host returns to after declining, nor what the guest sees.
- (minor) §5.5.3 rotates the IP-hash salt every 24 h, but §4.3 `JoinRequest.peer_ip_hash` is shown to the host for admission (§4.4). If the host is in an extended session across a rotation, the same joiner IP will hash differently on either side of rotation, defeating host-side correlation ('is this the same guy who just tried to join?'). The spec does not state whether JoinRequest uses the currently-active salt, a session-pinned salt, or a host-scoped secondary salt.

## weak-testing

- (major) §4.4 enumerates seven admission limits (HostRoom/min, JoinRoom/min, WS sessions per IP = 8, rooms per IP as host = 2, total sessions per node = 1024, server outbound queue depth per client = 8, invalid-frame budget 4 / 10 s), but §6 only tests JoinRoom, HostRoom, and the 256-match cap explicitly. The per-IP session cap, per-IP host-room cap, per-node session cap, outbound queue overflow → close, and invalid-frame-budget → close all lack any named test. A reviewer cannot verify §4.4 is 'enforced in code, not just documentation' as the §4.4 preamble claims.
- (minor) §6.2 bandwidth test asserts '≤ 8 KiB/s mean and ≤ 32 KiB/s peak' but does not define the averaging window for 'peak' (1 s? 1 tick? 100 ms?). A flaky peak is almost guaranteed without this; the test is underspecified and will either rubber-stamp or randomly fail.
- (minor) §6.1 'Input-delay pipeline' asserts 'late inputs are dropped and logged as InputTooLate', but 'late' is not defined in §2.3 or §5.5.1 — is an input late if its client_tick is strictly less than the server's current apply tick, or strictly less than server_tick + INPUT_DELAY, or something else? The test references a term the spec does not pin down.
- (minor) §5.3.4 defines four HUD colour-ramp thresholds for pending-garbage rows (hidden / DIM / OVERLAY / NEW_BEST / flashing-at-capped), and §5.3.3 pins deterministic thresholds, but §6.1/§6.2 list no test that verifies thresholds flip at exactly 1, 4, 8, and cap. Given the spec explicitly calls these 'deterministic thresholds', the test plan should too.
- (minor) §4.4 'Server inbound frame rate per client 240 frames/s sustained' has no test. A nominal session sends ~60 Inputs/s + ~2 Pings/s + ~60 Acks/s ≈ 122 frames/s, so the 240 threshold's headroom is unverified; the spec also doesn't document what 'sustained' means (windowed over what interval).

## suggested-next-version

v0.3

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "contradiction",
      "text": "§4 still contains an architecture placeholder block (`<!-- architecture:begin -->` … `(architecture not yet specified)` … `<!-- architecture:end -->`) sitting directly above §4.1's actual ASCII diagram. This is exactly the defect the v0.2 changelog claims to have fixed ('fixed architecture placeholder contradiction'), yet the placeholder text is still in the file. Either the placeholder must be removed, or the machine-readable fence must wrap the real §4.1 diagram.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.3.2 defines the sudden-death tiebreak as 'the player with the lower aggregate well height', but 'aggregate well height' is never defined. It could mean sum of per-column stack heights, max column height, total non-empty cells, or something else; each yields different winners in realistic same-tick double top-outs. Determinism of the tiebreak is claimed but un-specified at the mechanic level, and §6.1 lists no test that pins which definition wins.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.3.1 row 'Combo (N consecutive clears ≥ 2)  floor((N-1)/2)' is under-specified. It is unclear (a) whether 'N' is the current chain length including this clear or the previous chain length, (b) whether 'clears ≥ 2' means each clear must be at least a double, or the combo chain must be ≥ 2 long, and (c) how N resets (on non-clear drop? on a single-line clear?). Different readings differ by ±1 row sent on most combos, so the §6.1 'every row of §5.3.1' test cannot be written deterministically against this table.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.3.3 pins the garbage RNG derivation as 'first u8 % WELL_WIDTH output is the gap column', but WELL_WIDTH is not defined anywhere in the spec (no mention in §4, §5, or §6), and a naive `u8 % W` introduces modulo bias unless W divides 256 evenly. The spec declares this derivation part of the wire contract and requires a cross-platform golden vector, yet two conforming implementations can produce different gap columns just by choosing different rejection-sampling strategies.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§4.3 defines `Input` bit 4 as 'DAS-charge', but DAS (delayed auto-shift) is a continuous/stateful client concept — a player holds a direction for N ms and then auto-repeats. A per-tick bitset cannot naturally encode 'charge state'. The spec does not say whether the bit means 'DAS is currently charged', 'consume DAS charge this tick', or 'begin charging'. §5.5.1 claims the server validates every Input as a 'named move', but DAS-charge is not a move. This is both an ambiguity and a contradiction with the §4.3/§5.5.1 validation story, and no §6.1 test nails down the semantics.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§5.3.1 B2B bonus is '+1 flat, (consecutive quad/T-spin)' but it is not stated whether the +1 applies once per B2B chain start, or per subsequent qualifying clear in the chain. The manual test §6.4 asserts 'B2B quad sends 5 rows (4 + 1)', implying per-clear, but the table text is compatible with either reading. §6.1 should explicitly name the per-clear semantics in a test case.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§2.5 claims 'the 150 ms budget is the figure §2.3 uses when it calls RTT + 2-tick input delay imperceptible', but §2.3 nowhere names 150 ms, RTT, or 'imperceptible' — it only says the input delay 'absorbs typical jitter without visible lag'. The self-referential cross-reference is broken and a reviewer cannot verify the RTT budget rationale from §2.3 alone.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.3.1 'drain cap: up to 8 rows inserted per piece-spawn' does not define 'piece-spawn'. Candidate meanings: every time a new tetromino becomes the active piece (post-lock), every 'next' advance, or every lock-and-respawn including after a clear. Each yields different garbage-delivery cadence under heavy pressure; §6.1 asserts 'drain cap of 8 … leaves the correct residue' but 'correct' is undefined until 'piece-spawn' is.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.3.2 says 'an extra garbage row is inserted into each well every 2 s' during sudden-death. 2 s at 60 Hz is 120 ticks, but the spec elsewhere forbids wall-clock time in the simulation (§5.1) and the spec does not state 'every 120 ticks starting at tick 36000'. Leaving this as a wall-clock-flavored rule contradicts the determinism contract.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.2 state-transition diagram omits the `AwaitingAdmit` → (host declines or 15 s timeout) branch, even though §4.4 and user story 2 rely on it. A reviewer following only the diagram cannot determine which state the host returns to after declining, nor what the guest sees.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§4.4 enumerates seven admission limits (HostRoom/min, JoinRoom/min, WS sessions per IP = 8, rooms per IP as host = 2, total sessions per node = 1024, server outbound queue depth per client = 8, invalid-frame budget 4 / 10 s), but §6 only tests JoinRoom, HostRoom, and the 256-match cap explicitly. The per-IP session cap, per-IP host-room cap, per-node session cap, outbound queue overflow → close, and invalid-frame-budget → close all lack any named test. A reviewer cannot verify §4.4 is 'enforced in code, not just documentation' as the §4.4 preamble claims.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "§6.2 bandwidth test asserts '≤ 8 KiB/s mean and ≤ 32 KiB/s peak' but does not define the averaging window for 'peak' (1 s? 1 tick? 100 ms?). A flaky peak is almost guaranteed without this; the test is underspecified and will either rubber-stamp or randomly fail.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§6.1 'Input-delay pipeline' asserts 'late inputs are dropped and logged as InputTooLate', but 'late' is not defined in §2.3 or §5.5.1 — is an input late if its client_tick is strictly less than the server's current apply tick, or strictly less than server_tick + INPUT_DELAY, or something else? The test references a term the spec does not pin down.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§5.3.4 defines four HUD colour-ramp thresholds for pending-garbage rows (hidden / DIM / OVERLAY / NEW_BEST / flashing-at-capped), and §5.3.3 pins deterministic thresholds, but §6.1/§6.2 list no test that verifies thresholds flip at exactly 1, 4, 8, and cap. Given the spec explicitly calls these 'deterministic thresholds', the test plan should too.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§4.4 'Server inbound frame rate per client 240 frames/s sustained' has no test. A nominal session sends ~60 Inputs/s + ~2 Pings/s + ~60 Acks/s ≈ 122 frames/s, so the 240 threshold's headroom is unverified; the spec also doesn't document what 'sustained' means (windowed over what interval).",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§5.5.3 rotates the IP-hash salt every 24 h, but §4.3 `JoinRequest.peer_ip_hash` is shown to the host for admission (§4.4). If the host is in an extended session across a rotation, the same joiner IP will hash differently on either side of rotation, defeating host-side correlation ('is this the same guy who just tried to join?'). The spec does not state whether JoinRequest uses the currently-active salt, a session-pinned salt, or a host-scoped secondary salt.",
      "severity": "minor"
    }
  ],
  "summary": "Nine mandatory baseline sections are all present and the spec is notably mature after r1. The strongest remaining issues are (1) a genuine contradiction — §4's 'architecture not yet specified' placeholder still sits above the real architecture despite the changelog claiming this was fixed; (2) several simulation rules that are part of the wire contract but under-specified in a determinism-breaking way (WELL_WIDTH undefined, garbage-RNG modulo, aggregate-well-height tiebreak, combo formula's N, sudden-death 'every 2 s' vs. ticks, DAS-charge semantics on a per-tick bitset); and (3) a weak-testing gap where ~half of §4.4's admission/backpressure limits have no named test despite §4.4 explicitly claiming they are 'enforced in code, not just documentation'. No idea-contradiction findings: the original idea contained no explicit disclaimers to re-check against the spec.",
  "suggested_next_version": "v0.3",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
