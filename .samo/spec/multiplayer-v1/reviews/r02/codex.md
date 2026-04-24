# Reviewer A — Codex

## summary

Primary blockers are the fake second factor in host admission, unnecessary exposure of RNG state, and missing public-edge abuse controls. The spec is tighter than before, but several controls are written as if they solve an attack class when they mostly relocate it.

## weak-implementation

- (major) §3 says the host sees a raw IP, but §4.4/§5.5 replace that with a keyed IP hash. A keyed hash is not human-verifiable, so the host still cannot distinguish the intended friend from the first attacker who guessed or brute-forced the code; the claimed 'second credential' is therefore mostly illusory. Add a join fingerprint/PIN that both humans can compare, or make invites single-use and host-generated.
- (major) §2.3/§4.3 define `seq` and `client_tick`, but the spec never defines duplicate handling, monotonicity rules, or max future skew. A malicious client can replay old inputs or send far-future ones to grow per-match queues, waste CPU on validation, or create ambiguous ordering semantics. Define a strict acceptance window and disconnect on violation.
- (minor) §4.5 allows server-originated text to keep `\n`. That blocks ANSI escapes but still lets a malicious or compromised server render multi-line overlays that mimic local prompts or status chrome inside the TUI. Collapse to a single escaped line or render server text in a fixed non-interactive container.

## unnecessary-scope

- (major) §4.3 sends `seed` in `MatchStart` even though the client architecture only uses authoritative snapshots plus a local echo ghost. That gives modified clients perfect foresight of future bags and sudden-death garbage gaps, creating a cheap bot/cheat vector for no stated gameplay need. Keep RNG state server-side and reveal only intentionally visible queue state.
- (minor) §4.3 allocates a `Pause-request` wire bit even though no multiplayer pause behavior is specified anywhere else. Shipping undefined control surface in protocol v1 adds parser/state complexity and compatibility burden without user value. Remove it from v1 or mark the bit reserved=0 until semantics exist.

## missing-risk

- (major) §5.5 hardens frame parsing but says nothing about WebSocket handshake policy. If browser-originated WS handshakes are accepted, any website can use visitors' browsers to consume the node's tiny unauthenticated session/join budget, which is an avoidable public-edge DoS vector for a CLI-only service. Reject unexpected `Origin` headers and document that only non-browser clients are supported.
- (major) §4.4/§5.5 depend almost entirely on per-IP buckets/counters even though a target user story is joining from behind a corporate proxy. That both weakens abuse resistance against distributed attackers and creates easy false positives for legitimate users behind shared egress. Add a non-IP admission signal (join token, proof-of-work, cookie, or edge filtering) or explicitly narrow the rollout assumptions.
- (minor) §5.5.3 retains only `match_id,event,ip_hash,ts`, and the salt state is memory-only and reset on restart. On the one production node, that leaves you with too little evidence to investigate join abuse, rate-limit evasion, or disconnect storms, and it breaks the stated cross-rotation correlation story after any restart. Add bounded-cardinality reason fields and persist rotation state securely if 7-day correlation is a real requirement.

## suggested-next-version

v0.2.1

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "weak-implementation",
      "text": "§3 says the host sees a raw IP, but §4.4/§5.5 replace that with a keyed IP hash. A keyed hash is not human-verifiable, so the host still cannot distinguish the intended friend from the first attacker who guessed or brute-forced the code; the claimed 'second credential' is therefore mostly illusory. Add a join fingerprint/PIN that both humans can compare, or make invites single-use and host-generated.",
      "severity": "major"
    },
    {
      "category": "unnecessary-scope",
      "text": "§4.3 sends `seed` in `MatchStart` even though the client architecture only uses authoritative snapshots plus a local echo ghost. That gives modified clients perfect foresight of future bags and sudden-death garbage gaps, creating a cheap bot/cheat vector for no stated gameplay need. Keep RNG state server-side and reveal only intentionally visible queue state.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "§2.3/§4.3 define `seq` and `client_tick`, but the spec never defines duplicate handling, monotonicity rules, or max future skew. A malicious client can replay old inputs or send far-future ones to grow per-match queues, waste CPU on validation, or create ambiguous ordering semantics. Define a strict acceptance window and disconnect on violation.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "§5.5 hardens frame parsing but says nothing about WebSocket handshake policy. If browser-originated WS handshakes are accepted, any website can use visitors' browsers to consume the node's tiny unauthenticated session/join budget, which is an avoidable public-edge DoS vector for a CLI-only service. Reject unexpected `Origin` headers and document that only non-browser clients are supported.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "§4.4/§5.5 depend almost entirely on per-IP buckets/counters even though a target user story is joining from behind a corporate proxy. That both weakens abuse resistance against distributed attackers and creates easy false positives for legitimate users behind shared egress. Add a non-IP admission signal (join token, proof-of-work, cookie, or edge filtering) or explicitly narrow the rollout assumptions.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "§4.5 allows server-originated text to keep `\\n`. That blocks ANSI escapes but still lets a malicious or compromised server render multi-line overlays that mimic local prompts or status chrome inside the TUI. Collapse to a single escaped line or render server text in a fixed non-interactive container.",
      "severity": "minor"
    },
    {
      "category": "missing-risk",
      "text": "§5.5.3 retains only `match_id,event,ip_hash,ts`, and the salt state is memory-only and reset on restart. On the one production node, that leaves you with too little evidence to investigate join abuse, rate-limit evasion, or disconnect storms, and it breaks the stated cross-rotation correlation story after any restart. Add bounded-cardinality reason fields and persist rotation state securely if 7-day correlation is a real requirement.",
      "severity": "minor"
    },
    {
      "category": "unnecessary-scope",
      "text": "§4.3 allocates a `Pause-request` wire bit even though no multiplayer pause behavior is specified anywhere else. Shipping undefined control surface in protocol v1 adds parser/state complexity and compatibility burden without user value. Remove it from v1 or mark the bit reserved=0 until semantics exist.",
      "severity": "minor"
    }
  ],
  "summary": "Primary blockers are the fake second factor in host admission, unnecessary exposure of RNG state, and missing public-edge abuse controls. The spec is tighter than before, but several controls are written as if they solve an attack class when they mostly relocate it.",
  "suggested_next_version": "v0.2.1",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
