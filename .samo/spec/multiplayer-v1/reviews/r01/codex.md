# Reviewer A — Codex

## summary

The gameplay direction is workable, but this spec is not yet safe to operationalize: it under-specifies adversarial clients, resource exhaustion, routing and rollout behavior, and it accidentally gives modified clients enough randomness to cheat. Tighten the protocol and deployment model before implementation starts.

## missing-risk

- (major) Sec. 4.2/5.1 leaks hidden server randomness to untrusted clients: `MatchStart` sends the match `seed`, and that seed drives both 7-bag generation and garbage-hole placement. A modified client can precompute future pieces and garbage gaps for both players, which is a serious competitive-integrity break. Keep hidden RNG server-only and send only the currently visible queue state.
- (major) Sec. 4.2/5.5 treats the 5-character room code as the only join secret, with only `JoinRoom` rate-limited per IP and no host approval step. That makes room hijack and room enumeration practical for distributed attackers or anyone who overhears a code. Add a second shared secret or host-confirm flow, plus global and per-room join throttles.
- (major) Sec. 5.5 rate-limits `JoinRoom` attempts but says nothing about `HostRoom`, idle pre-match rooms, concurrent sockets, handshake timeouts, or per-IP room quotas. An attacker can exhaust memory and file descriptors by opening rooms or holding connections without ever starting matches. The spec needs explicit admission control, room TTLs, and hard per-connection and per-IP caps.
- (major) Sec. 4.3 uses `bincode` on an untrusted public interface but does not specify hard decode bounds, websocket max-frame limits, or per-field size caps for strings and capability vectors. The inner `u16` length prefix is not enough unless the websocket layer is also capped before allocation. Without explicit bounds, malformed peers can turn decoding into an allocation or CPU DoS.
- (major) Sec. 1/4.3 enforces hard protocol-version refusal, but the spec gives no deployment strategy for the official service. A routine rolling deploy or client release can strand users on version mismatches unless you version endpoints or run blue-green capacity during rollout. For a public server, compatibility policy and drain behavior need to be part of the spec.
- (minor) Sec. 8 adds a Prometheus `/metrics` endpoint but never says where it binds or how it is protected. Exposing it on the public interface leaks operational state and creates another parser surface for attackers. Bind it to localhost or an admin network only, or put it behind explicit auth.

## weak-implementation

- (major) Sec. 2.3/4.3 has no backpressure policy for slow or malicious clients even though snapshots are pushed unconditionally at 60 Hz and delta state is tracked per client. If a client stops acking or reading, outbound buffers and diff history can grow until they become a server-side DoS. Specify bounded queues, snapshot coalescing/drop-old behavior, and a disconnect threshold for lagging peers.
- (major) Sec. 2.3 says the server applies input at `client_tick + INPUT_DELAY`, but the spec never defines how client tick is synchronized, bounded, or corrected for drift. As written, a hostile client can lie about `client_tick` to reduce effective delay, inject far-future inputs, or force oversized reorder windows. The server should assign authoritative input slots or at minimum enforce a narrow accept window around server time.
- (major) Sec. 2.2 claims the server is 'stateless between matches and horizontally scalable', but room codes, waiting rooms, and active matches are all long-lived in-memory state. Without a shared room directory and sticky routing, or a single authoritative ingress, horizontal scaling and failover are not actually defined. This is an ops hole, not just an implementation detail.
- (minor) Sec. 5.4 is internally inconsistent on liveness: the client is said to send `Pong(nonce)` every 500 ms and the server replies, but the protocol table only defines `Pong` and never defines `Ping`. This is likely to produce mismatched implementations and weak liveness semantics. Define a server-initiated `Ping`/client `Pong` pair or a clearly named client heartbeat.

## unnecessary-scope

- (minor) Sec. 5.6/8 keeps `mp local` in the v1 delivery scope even though it requires split-screen TTY UX, separate local input mapping, extra QA, and an offline-syscall guarantee that does not harden the internet-facing service. For a risky first public multiplayer release, this is scope competing directly with protocol hardening and abuse controls.
- (minor) Sec. 2.4 never states that websocket compression and nonessential extensions are disabled. For tiny binary frames in a CLI game, permessage-deflate is upside-down risk: more CPU and memory complexity, worse DoS characteristics, and no meaningful product benefit. Explicitly turn compression and optional extensions off in v1.

## suggested-next-version

v0.1.1

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "missing-risk",
      "text": "Sec. 4.2/5.1 leaks hidden server randomness to untrusted clients: `MatchStart` sends the match `seed`, and that seed drives both 7-bag generation and garbage-hole placement. A modified client can precompute future pieces and garbage gaps for both players, which is a serious competitive-integrity break. Keep hidden RNG server-only and send only the currently visible queue state.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Sec. 4.2/5.5 treats the 5-character room code as the only join secret, with only `JoinRoom` rate-limited per IP and no host approval step. That makes room hijack and room enumeration practical for distributed attackers or anyone who overhears a code. Add a second shared secret or host-confirm flow, plus global and per-room join throttles.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Sec. 5.5 rate-limits `JoinRoom` attempts but says nothing about `HostRoom`, idle pre-match rooms, concurrent sockets, handshake timeouts, or per-IP room quotas. An attacker can exhaust memory and file descriptors by opening rooms or holding connections without ever starting matches. The spec needs explicit admission control, room TTLs, and hard per-connection and per-IP caps.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Sec. 2.3/4.3 has no backpressure policy for slow or malicious clients even though snapshots are pushed unconditionally at 60 Hz and delta state is tracked per client. If a client stops acking or reading, outbound buffers and diff history can grow until they become a server-side DoS. Specify bounded queues, snapshot coalescing/drop-old behavior, and a disconnect threshold for lagging peers.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Sec. 2.3 says the server applies input at `client_tick + INPUT_DELAY`, but the spec never defines how client tick is synchronized, bounded, or corrected for drift. As written, a hostile client can lie about `client_tick` to reduce effective delay, inject far-future inputs, or force oversized reorder windows. The server should assign authoritative input slots or at minimum enforce a narrow accept window around server time.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Sec. 4.3 uses `bincode` on an untrusted public interface but does not specify hard decode bounds, websocket max-frame limits, or per-field size caps for strings and capability vectors. The inner `u16` length prefix is not enough unless the websocket layer is also capped before allocation. Without explicit bounds, malformed peers can turn decoding into an allocation or CPU DoS.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Sec. 2.2 claims the server is 'stateless between matches and horizontally scalable', but room codes, waiting rooms, and active matches are all long-lived in-memory state. Without a shared room directory and sticky routing, or a single authoritative ingress, horizontal scaling and failover are not actually defined. This is an ops hole, not just an implementation detail.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Sec. 1/4.3 enforces hard protocol-version refusal, but the spec gives no deployment strategy for the official service. A routine rolling deploy or client release can strand users on version mismatches unless you version endpoints or run blue-green capacity during rollout. For a public server, compatibility policy and drain behavior need to be part of the spec.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Sec. 5.4 is internally inconsistent on liveness: the client is said to send `Pong(nonce)` every 500 ms and the server replies, but the protocol table only defines `Pong` and never defines `Ping`. This is likely to produce mismatched implementations and weak liveness semantics. Define a server-initiated `Ping`/client `Pong` pair or a clearly named client heartbeat.",
      "severity": "minor"
    },
    {
      "category": "missing-risk",
      "text": "Sec. 8 adds a Prometheus `/metrics` endpoint but never says where it binds or how it is protected. Exposing it on the public interface leaks operational state and creates another parser surface for attackers. Bind it to localhost or an admin network only, or put it behind explicit auth.",
      "severity": "minor"
    },
    {
      "category": "unnecessary-scope",
      "text": "Sec. 5.6/8 keeps `mp local` in the v1 delivery scope even though it requires split-screen TTY UX, separate local input mapping, extra QA, and an offline-syscall guarantee that does not harden the internet-facing service. For a risky first public multiplayer release, this is scope competing directly with protocol hardening and abuse controls.",
      "severity": "minor"
    },
    {
      "category": "unnecessary-scope",
      "text": "Sec. 2.4 never states that websocket compression and nonessential extensions are disabled. For tiny binary frames in a CLI game, permessage-deflate is upside-down risk: more CPU and memory complexity, worse DoS characteristics, and no meaningful product benefit. Explicitly turn compression and optional extensions off in v1.",
      "severity": "minor"
    }
  ],
  "summary": "The gameplay direction is workable, but this spec is not yet safe to operationalize: it under-specifies adversarial clients, resource exhaustion, routing and rollout behavior, and it accidentally gives modified clients enough randomness to cheat. Tighten the protocol and deployment model before implementation starts.",
  "suggested_next_version": "v0.1.1",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
