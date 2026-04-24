# Reviewer A — Codex

## summary

The gameplay direction is plausible, but the spec is not yet safe to treat as a public-server rollout. The main gaps are room-level authentication, explicit abuse/resource controls, bounded snapshot/backpressure behavior, and deployment/routing details, while v1 scope still spends effort on mp local instead of hardening those basics.

## missing-risk

- (major) Section 4.2/5.5 make the 5-character room code the only join credential. There is no host-side admit/confirm step and no second secret, so anyone who overhears or guesses the code can steal the guest slot and grief the match; the current JoinRoom IP rate limit only slows brute force, it does not protect an individual room.
- (major) Section 5.5 rate-limits only JoinRoom. The spec omits limits for HostRoom creation, idle pre-match room lifetime, concurrent TLS/WebSocket sessions per IP, invalid-frame budgets, and total rooms per node, which leaves a public unauthenticated service on port 443 trivially exhaustible with cheap idle sockets or room spam.
- (major) Section 2.5 asserts 200 concurrent matches per 1-vCPU node, but the spec never defines admission control or overload behavior. Without explicit caps that reject new rooms/sessions before tick budget is breached, one spike or attack will degrade every active match instead of failing closed.
- (major) Section 4.3 length-prefixing and fuzzing are not enough for a public parser. The spec does not set a maximum WebSocket message size, maximum decoded allocation, per-connection outbound queue depth, or malformed-frame CPU/ban thresholds, so a single peer can force large allocations or sustained decode work.
- (major) Section 2.2 says the server is stateless between matches and horizontally scalable, but active room-code matchmaking requires either shared room state or sticky routing from host/join through match end. Without that routing/registry design, the first multi-instance deployment will break rendezvous or force a late rewrite.
- (minor) Section 8 adds a Prometheus /metrics endpoint but never says it is bound to localhost/private networking or otherwise authenticated. Exposing live concurrency and tick-budget data on the public game service gives attackers free capacity reconnaissance and unnecessary extra attack surface.
- (major) Section 4.3/3 expect the client to print server-provided human-readable Error strings as actionable terminal output. The spec does not require control-character stripping or length limits, so a malicious or self-hosted server can inject ANSI escapes, spoof prompts, or leave the terminal in a bad state.

## weak-implementation

- (major) Section 2.3/4.3 delta-compress snapshots against the last acked snapshot and still send every tick when acks stall. That leaves the server with no bounded recovery path for slow or malicious clients; it needs a finite ack window plus periodic keyframes/backoff/disconnect rules, or memory/CPU can grow around a single lagging socket.
- (minor) Section 5.5 logging as match_id plus truncated IP hash is under-specified. Without a keyed rotating salt, IPv4 hashes are brute-forceable; with too much truncation they stop being useful for abuse correlation, so the current text gives neither a strong privacy guarantee nor a reliable incident-response handle.

## unnecessary-scope

- (minor) Section 5.6/8 make mp local a first-class public feature in the same release. For v1 this is unnecessary scope: it adds split-screen TTY/input-multiplexing and extra regression surface in the offline-default binary, but it does not reduce the security or operational risk of the public server. Keep it as an internal harness or defer it.

## suggested-next-version

Narrow the next revision to online 1v1 on a single-region beta and make the server model explicit: add host admission or a second join secret, idle-room/session/frame-size/concurrency limits, overload/load-shedding rules, a bounded ack window plus periodic keyframes, private-only metrics, sanitized terminal strings, and a keyed rotating IP-hash policy; defer mp local unless it is re-scoped as an internal test harness.

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "missing-risk",
      "text": "Section 4.2/5.5 make the 5-character room code the only join credential. There is no host-side admit/confirm step and no second secret, so anyone who overhears or guesses the code can steal the guest slot and grief the match; the current JoinRoom IP rate limit only slows brute force, it does not protect an individual room.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Section 5.5 rate-limits only JoinRoom. The spec omits limits for HostRoom creation, idle pre-match room lifetime, concurrent TLS/WebSocket sessions per IP, invalid-frame budgets, and total rooms per node, which leaves a public unauthenticated service on port 443 trivially exhaustible with cheap idle sockets or room spam.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Section 2.5 asserts 200 concurrent matches per 1-vCPU node, but the spec never defines admission control or overload behavior. Without explicit caps that reject new rooms/sessions before tick budget is breached, one spike or attack will degrade every active match instead of failing closed.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Section 2.3/4.3 delta-compress snapshots against the last acked snapshot and still send every tick when acks stall. That leaves the server with no bounded recovery path for slow or malicious clients; it needs a finite ack window plus periodic keyframes/backoff/disconnect rules, or memory/CPU can grow around a single lagging socket.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Section 4.3 length-prefixing and fuzzing are not enough for a public parser. The spec does not set a maximum WebSocket message size, maximum decoded allocation, per-connection outbound queue depth, or malformed-frame CPU/ban thresholds, so a single peer can force large allocations or sustained decode work.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Section 2.2 says the server is stateless between matches and horizontally scalable, but active room-code matchmaking requires either shared room state or sticky routing from host/join through match end. Without that routing/registry design, the first multi-instance deployment will break rendezvous or force a late rewrite.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Section 8 adds a Prometheus /metrics endpoint but never says it is bound to localhost/private networking or otherwise authenticated. Exposing live concurrency and tick-budget data on the public game service gives attackers free capacity reconnaissance and unnecessary extra attack surface.",
      "severity": "minor"
    },
    {
      "category": "missing-risk",
      "text": "Section 4.3/3 expect the client to print server-provided human-readable Error strings as actionable terminal output. The spec does not require control-character stripping or length limits, so a malicious or self-hosted server can inject ANSI escapes, spoof prompts, or leave the terminal in a bad state.",
      "severity": "major"
    },
    {
      "category": "unnecessary-scope",
      "text": "Section 5.6/8 make mp local a first-class public feature in the same release. For v1 this is unnecessary scope: it adds split-screen TTY/input-multiplexing and extra regression surface in the offline-default binary, but it does not reduce the security or operational risk of the public server. Keep it as an internal harness or defer it.",
      "severity": "minor"
    },
    {
      "category": "weak-implementation",
      "text": "Section 5.5 logging as match_id plus truncated IP hash is under-specified. Without a keyed rotating salt, IPv4 hashes are brute-forceable; with too much truncation they stop being useful for abuse correlation, so the current text gives neither a strong privacy guarantee nor a reliable incident-response handle.",
      "severity": "minor"
    }
  ],
  "summary": "The gameplay direction is plausible, but the spec is not yet safe to treat as a public-server rollout. The main gaps are room-level authentication, explicit abuse/resource controls, bounded snapshot/backpressure behavior, and deployment/routing details, while v1 scope still spends effort on mp local instead of hardening those basics.",
  "suggested_next_version": "Narrow the next revision to online 1v1 on a single-region beta and make the server model explicit: add host admission or a second join secret, idle-room/session/frame-size/concurrency limits, overload/load-shedding rules, a bounded ack window plus periodic keyframes, private-only metrics, sanitized terminal strings, and a keyed rotating IP-hash policy; defer mp local unless it is re-scoped as an internal test harness.",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
