# blocktxt multiplayer — SPEC v0.2

> **Scope note.** This document specifies the two-player competitive
> multiplayer mode added to blocktxt in game version **v0.2**. The project
> slug `multiplayer-v1` refers to the *first* version of the multiplayer
> subsystem, not to a rewrite of v0.1 gameplay. Single-player v0.1
> behaviour is unchanged and must continue to work offline.

## 1. Goal & why it's needed

**Goal.** Ship a head-to-head competitive mode where two human players,
each in their own terminal, play simultaneous falling-block games against
each other. Line clears by one player send garbage rows to the other
player's well. First player to top-out (block-out or lock-out) loses.

**Why this exists.** blocktxt v0.1 is a polished but strictly solitary
experience. Competitive multiplayer is the single largest replayability
multiplier we can add: it turns a 5-minute coffee-break toy into a
social artifact you send to a friend. It also exercises a class of
engineering — deterministic simulation, network protocols, matchmaking
— that the repo has deliberately avoided so far, so v0.2 is the right
moment to pay that cost once, cleanly, rather than bolting it on later.

**v0.2 public rollout is scoped to a single-region beta.** We ship a
single production server node behind a single DNS name; horizontal scale
and multi-region are deferred to v0.3 (see §4.6).

**Non-goals for this version (enforced strictly).**

- **NOT a spectator / streaming feature.** No watch-only clients in v1.
- **NOT a leaderboard / rating service.** Matches are ad-hoc; no Elo,
  no persistent win/loss records beyond a local session counter.
- **NOT a lobby / chat / social product.** Two players connect via a
  short room code plus a host admission step. No friends list, no chat
  UI, no profiles.
- **NOT cross-version compatible.** v0.2 clients only talk to v0.2
  servers and v0.2 peers. Version mismatch → clean refusal.
- **NOT a replacement for single-player.** Single-player v0.1 remains
  the default launch mode; multiplayer is opt-in via a subcommand.
- **NOT >2 players.** No free-for-all, no teams. Exactly 2.
- **NOT a public offline-multiplayer feature.** `mp local` is deferred
  out of the v0.2 public surface (see §5.6); the same two-player engine
  is exercised only as an internal test harness.
- **NOT a horizontally scaled deployment in v1.** Exactly one server
  instance in production; registry/sticky-routing design deferred.

## 2. Design decisions (resolved from interview)

The interview left five questions as "decide for me". The chosen answers
and their rationale are recorded below so future reviewers don't have to
reverse-engineer them.

### 2.1 Language — Rust (client + server)

The client is already Rust; sharing the simulation crate between client
and server is the single highest-leverage decision we can make. It
makes bit-exact determinism trivial (same code, same RNG, same fixed-
point math) and halves the test surface. Server uses `tokio` for async
I/O; client keeps its existing blocking main loop and speaks to the
network from a dedicated thread.

### 2.2 Topology — authoritative central server ("relay + referee")

The server runs the canonical simulation for *both* players at 60 Hz.
Clients send inputs; server returns authoritative state diffs. Chosen
over:

- **Peer-to-peer with a host**: needs NAT traversal (STUN/TURN) and one
  peer becomes de-facto authoritative anyway — same cheating surface,
  worse UX.
- **Pure lockstep**: elegant for identical-input games but a falling-block
  game has independent boards; we'd still need a referee for garbage
  exchange, at which point we're 80 % of the way to server-authoritative.
- **Rollback netcode**: premium experience but a full sprint of work
  we can defer until there's demand.

For v0.2 the server is **deployed as a single node** behind a single
DNS name. Match state (boards, RNG, input queues) lives in-process;
room-code matchmaking is an in-memory registry on that node. The
server is stateless *across restarts* (we do not persist matches) but
it is emphatically **not** horizontally shardable yet — see §4.6 for
what changes in v0.3.

### 2.3 Sync model — server-authoritative fixed-tick, client input-forward with input delay

- Fixed simulation tick: **60 Hz** (16.67 ms), matching the existing
  single-player frame cadence.
- Clients forward inputs with a monotonically increasing sequence
  number plus the client tick they were pressed on.
- Server applies inputs at **`client_tick + INPUT_DELAY`** where
  `INPUT_DELAY = 2 ticks ≈ 33 ms`. This absorbs typical jitter without
  visible lag and avoids the complexity of rollback.
- Server emits a **state snapshot** every tick (two boards, scores,
  incoming-garbage queues, active pieces, phase). Snapshots are
  delta-encoded against the most recent snapshot the client has
  acked; see §4.4 for the bounded ack window, keyframe policy, and
  backpressure rules.
- **Local rendering policy.** The player's *own* active piece is drawn
  from a local input-echo ghost (the client applies its own input
  optimistically to the mirrored snapshot for that piece only) so that
  horizontal moves and soft-drops feel immediate. The well, the
  opponent's board, score, incoming-garbage queue, and all lock events
  are drawn strictly from the server snapshot. If the server
  snapshot's piece state diverges from the local ghost (e.g. server
  rejected an input), the client snaps to the server state on the next
  frame — no interpolation, no rollback.
- No speculative execution past the active piece: lock, clear, and
  garbage are server-authoritative and rendered only when snapshotted.
  Prediction of lock/clear is an explicit v2 lever if telemetry shows
  we need it.

### 2.4 Transport — TLS-terminated WebSocket, binary frames

- WebSocket over TLS (`wss://`) on port **443**. Chosen for
  firewall-friendliness (corporate / school networks), standard Rust
  tooling (`tokio-tungstenite`), and because matchmaking and game
  traffic share one connection.
- Binary frames carry a compact framed protocol (see §4.3); no JSON on
  the hot path.
- TLS is mandatory: room codes travel over the wire, and we refuse to
  teach users to type plaintext credentials into a terminal.

### 2.5 Scale target — 2 players, 60 Hz, 150 ms RTT budget

| Dimension                    | v0.2 target                                                 |
|------------------------------|-------------------------------------------------------------|
| Players per match            | exactly 2                                                   |
| Server tick rate             | 60 Hz                                                       |
| Client frame rate            | 60 fps (unchanged)                                          |
| RTT budget                   | ≤ 150 ms round-trip (≈ 75 ms one-way) for "feels responsive"|
| Bandwidth / client           | ≤ 8 KiB/s steady state (delta snapshots); hard cap 32 KiB/s |
| Target concurrent matches    | 200 per 1-vCPU / 512 MiB server (load-tested in §6.2)       |
| Admission-control hard cap   | 256 matches / node; new rooms rejected with `ServerFull`    |
| Match duration cap           | 10 minutes (hard cap; resolves via sudden-death, §5.3.2)    |

The 150 ms budget is **round-trip**; it is the figure §2.3 uses when
it calls RTT + 2-tick input delay imperceptible. Any reviewer-facing
claim elsewhere in this doc that uses "latency" refers to one-way
latency (≈ RTT/2).

## 3. User stories

1. **Casey the commuter** (persona: single-player regular, 30 min/day
   on trains). *Action:* runs `blocktxt mp host`, reads aloud the
   5-character room code to a friend on a call, then presses `y` when
   the HUD shows `join request from 203.0.113.4 — accept? [y/N]`.
   *Outcome:* within 15 seconds both are playing head-to-head; no
   signup, no account, no browser, and a stranger who guesses the code
   is refused because the host never approved them.

2. **Riley the reviewer** (persona: rust-curious developer trying
   blocktxt for the first time). *Action:* runs `blocktxt mp join
   ABCDE` from a fresh install behind a corporate HTTP proxy.
   *Outcome:* connection succeeds over port 443; if it fails, the CLI
   prints an actionable error naming *which* step failed (DNS, TLS,
   handshake, version-mismatch, room-not-found, host-declined,
   rate-limited, server-full), with all server-provided error text
   sanitized of control characters (§4.5).

3. **Sam the skeptic** (persona: privacy-conscious, offline-first).
   *Action:* launches `blocktxt` with no arguments. *Outcome:* gets
   exactly the v0.1 single-player experience, no network calls, no
   background threads touching the internet, binary size unchanged
   beyond the added multiplayer code path.

4. **Taylor the twitch player** (persona: competitive falling-block
   player used to Puyo Puyo and TETR.IO). *Action:* clears a quad
   (4-line) while their opponent has a full well. *Outcome:* opponent
   receives 4 garbage rows within one server tick of the flash-phase
   completing, and the garbage-incoming indicator in Taylor's HUD
   shows pending rows they're about to send, with colour ramp steps
   defined at deterministic thresholds (§5.3.3).

5. **Jordan the judge** (persona: QA / release-blocking role).
   *Action:* runs the manual multiplayer test plan against a release
   candidate on macOS arm64, macOS x86_64, and Linux x86_64
   simultaneously. *Outcome:* can verify in under 20 minutes that
   matchmaking, gameplay, disconnection, sudden-death, and clean
   shutdown all behave; any failure maps to a specific checklist item.

## 4. Architecture

<!-- architecture:begin -->

```text
(architecture not yet specified)
```

<!-- architecture:end -->

### 4.1 Components & boundaries

```
┌──────────────────── blocktxt client (Rust binary) ────────────────────┐
│                                                                       │
│  cli  ─▶  mode dispatch  ─▶  single-player loop (unchanged v0.1)      │
│                      │                                                │
│                      └───▶   mp::client  ──────────────┐              │
│                                 │                       │             │
│                                 ▼                       ▼             │
│                          net thread (tokio)     render/input (main)   │
│                                 │                       │             │
│                                 └── shared state ◀──────┘             │
│                                     (sim::SnapshotMirror + echo ghost)│
└─────────────────────────────────┬─────────────────────────────────────┘
                                  │ wss://  (TLS 1.3, binary frames)
                                  ▼
┌──────────────────── blocktxt-server (new binary) ─────────────────────┐
│                                                                       │
│  ws accept ─▶ session ─▶ admission ─▶ matchmaker (room registry)      │
│  (TLS, limits)             (caps)       │                             │
│                                          │                            │
│                                          ▼                            │
│                                      match ─▶ sim (60 Hz, auth.)      │
│                                         │       │                     │
│                                         │       └─▶ snapshot diff     │
│                                         └─────────▶ broadcast (2)     │
│                                                                       │
│  metrics (127.0.0.1:9090, auth)     access log (match_id + keyed-hash)│
└───────────────────────────────────────────────────────────────────────┘
```

New crates / modules in the workspace:

- `blocktxt-sim` (library, extracted from existing `src/game/`): pure,
  deterministic simulation. No `Instant::now()`, no thread-local RNG,
  no I/O. Shared verbatim between client and server.
- `blocktxt-proto` (library): wire-format types, framing, version
  constants, `Encode`/`Decode` traits, explicit size caps.
- `blocktxt` (existing binary): gains `mp host` / `mp join`
  subcommands. `mp local` is present but gated behind the
  `--internal-harness` feature flag and not documented in the public
  CLI help (see §5.6).
- `blocktxt-server` (new binary): WebSocket listener, admission
  controller, matchmaker, match runner.

### 4.2 Key abstractions

- **`sim::Match`** — holds two `GameState` instances plus a shared
  garbage exchange queue. Pure function:
  `step(inputs_a, inputs_b) -> (events_a, events_b, garbage_delta)`.
  There is no `dt` parameter; one call advances exactly one tick. The
  client mirror calls `step` zero or one times per rendered frame; it
  never calls it with any other cadence.
- **`sim::DetRng`** — seeded ChaCha20 RNG used for piece generation
  (7-bag). Both players' bags are derived from the match seed via the
  pinned derivation in §5.3.1 so sequences are reproducible for
  replays / desync audits.
- **`proto::Frame`** — tagged union of `Hello`, `HostRoom`,
  `JoinRoom`, `JoinRequest`, `HostDecision`, `RoomReady`,
  `MatchStart`, `Input`, `Snapshot`, `Keyframe`, `Ack`, `Ping`,
  `Pong`, `Goodbye`, `Error`. Versioned by the `Hello` handshake.
- **`client::SnapshotMirror`** — lock-free SPSC slot that the net
  thread writes and the render thread reads each frame. Holds the
  latest fully-applied snapshot plus the running keyframe baseline.
- **`client::EchoGhost`** — render-thread-only shadow of the local
  active piece's `(x, y, rot)`, updated from keyboard input and reset
  from the snapshot every time the server advances the piece
  (lock/spawn/rotate-reject). Never writes into the simulation.
- **`server::RoomCode`** — 5-character `Crockford-base32` code (no
  `ILOU`); 32⁵ ≈ 33 M codes. Matchmaker rejects codes already live.
- **`server::Admission`** — per-IP and global token buckets plus
  concurrency counters; see §4.4.

### 4.3 Wire protocol (v1)

All frames are length-prefixed (u16 big-endian) binary payloads inside
WebSocket binary messages. Fields are little-endian `bincode` v2 with
the `varint` codec. `proto_version = 1` is carried in `Hello` and any
mismatch is fatal.

| Tag  | Name          | Fields (summary)                                  |
|-----:|---------------|---------------------------------------------------|
| 0x01 | Hello         | proto_version (u16), client_build (str ≤ 64)      |
| 0x02 | HostRoom      | — (server assigns code)                           |
| 0x03 | JoinRoom      | code ([u8; 5])                                    |
| 0x04 | JoinRequest   | code, peer_build (str ≤ 64), peer_ip_hash ([u8;8])|
| 0x05 | HostDecision  | accept (bool), reason (enum)                      |
| 0x06 | RoomReady     | code, role (host/guest), peer_build               |
| 0x07 | MatchStart    | match_id (u128), seed (u64), start_tick (u32)     |
| 0x08 | Input         | client_tick (u32), seq (u32), inputs (u16 bitset) |
| 0x09 | Snapshot      | server_tick, ack_seq, baseline_tick, delta bytes  |
| 0x0A | Keyframe      | server_tick, full state blob                      |
| 0x0B | Ack           | ack_tick (u32)                                    |
| 0x0C | Ping          | nonce (u64), client_send_tick (u32)               |
| 0x0D | Pong          | nonce (u64), server_tick (u32)                    |
| 0x0E | Goodbye       | reason (enum)                                     |
| 0x0F | Error         | code (enum), detail (ASCII-only str ≤ 256)        |

**`capabilities` is intentionally not present in v1.** Forward
compatibility is handled by strict `proto_version` mismatch refusal
per §1; any future optional behaviour will be gated on a bumped
version, not on in-band negotiation. This keeps v1 parsing paths
shorter and removes a class of downgrade attacks.

**`Input` bit layout (u16):**

| Bit | Name          | Bit | Name            |
|----:|---------------|----:|-----------------|
|  0  | MoveLeft      |  6  | RotateCCW       |
|  1  | MoveRight     |  7  | RotateCW        |
|  2  | SoftDrop      |  8  | Rotate180       |
|  3  | HardDrop      |  9  | Hold            |
|  4  | DAS-charge    | 10  | Pause-request   |
|  5  | (reserved=0)  | 11–15 | (reserved=0)  |

Reserved bits MUST be zero on the wire; the server closes the
connection with `Error(MalformedInput)` on any reserved bit set.

**Ping/Pong.** Clients emit `Ping(nonce)` every 500 ms; the server
replies `Pong(nonce, server_tick)`. This matches RFC 6455 directional
semantics. The WebSocket-level control frames are not used for game
timing; we carry our own so the server `tick` is directly observable.

### 4.4 Reliability, backpressure & admission (new in v0.2)

The following limits are authoritative and enforced in code, not just
documentation.

**Frame & connection limits**

| Limit                                   | Value                       |
|-----------------------------------------|-----------------------------|
| Max WebSocket message size              | 16 KiB                      |
| Max decoded allocation per frame        | 64 KiB (hard, pre-alloc)    |
| Server outbound queue depth per client  | 8 frames; overflow → close  |
| Server inbound frame rate per client    | 240 frames/s sustained      |
| Invalid-frame budget per client         | 4 per 10 s → close          |
| Malformed-frame ban (per IP)            | 10 invalid closes / 5 min   |
| Idle pre-match room lifetime            | 60 s → room destroyed       |
| Match lifetime                          | 10 min → sudden-death (§5.3.2)|

**Matchmaking & session limits**

| Limit                                   | Value                       |
|-----------------------------------------|-----------------------------|
| `HostRoom` per IP                       | 3 per minute                |
| `JoinRoom` per IP                       | 10 per minute               |
| Concurrent WS sessions per IP           | 8                           |
| Concurrent rooms per IP (as host)       | 2                           |
| Total rooms per node (admission cap)    | 256 → `Error(ServerFull)`   |
| Total sessions per node (admission cap) | 1024 → TLS accept refused   |

All buckets are token buckets with per-minute refill; admission checks
run before room registry mutation.

**Snapshot/ack window.**

- The server maintains a ring buffer of the last **32 snapshots**
  (533 ms at 60 Hz) per client as delta baselines.
- Clients send `Ack(ack_tick)` on every snapshot received; the server
  advances the baseline to the oldest unacked snapshot.
- If the unacked window reaches 32, the server sends a full
  `Keyframe` and resets the baseline to it.
- If the unacked window exceeds 60 snapshots (1 s), the server closes
  the session with `Goodbye(ClientStalled)`; the peer receives
  `Goodbye(PeerDisconnected)` and wins by default.
- Snapshots are emitted at most **once per tick**; under outbound
  backpressure the server coalesces pending snapshots into the next
  `Keyframe` rather than queueing unbounded.

**Host admission.** A `JoinRoom` does not connect the two clients
directly. The server routes a `JoinRequest` to the host with a
truncated, keyed hash of the joiner's IP (§5.5.3). The host replies
`HostDecision(accept=true|false)` within 15 s or the join is
auto-rejected. Only on `accept=true` does the match start. This
removes the "guessable 5-character code is the only credential" class
of hijack.

### 4.5 Terminal-safe error rendering

Any server-provided string surfaced to the user terminal (`Error.detail`,
`Goodbye.reason` human text, room codes) is sanitized by the client:

- Strip everything outside printable ASCII `0x20–0x7E` plus `\n`.
- Cap at 200 display columns.
- Render through the existing TUI overlay, never via raw `print!`.

The protocol `Error.detail` field is declared ASCII-only on the wire
(validated server-side on emit and client-side on decode); a
self-hosted malicious server cannot inject ANSI escapes because the
client drops them before paint.

### 4.6 Deployment model (v0.2 beta)

- One production node behind one DNS name; no load balancer, no
  sticky routing. Room registry is an in-process `DashMap`.
- If the node restarts, in-flight matches are abandoned with
  `Goodbye(ServerRestart)`; clients show a retriable error.
- Horizontal scale and a shared room-routing tier (likely
  Redis-backed matchmaker + sticky WS routing by room code) are
  explicitly a **v0.3** workstream and are out of scope here. The
  protocol does not yet encode cross-node room handoff.

## 5. Implementation details

### 5.1 Deterministic simulation

- Extract `src/game/` into `blocktxt-sim`. Swap `StdRng` for
  `ChaCha20Rng` seeded from the match seed.
- Remove all `Instant::now()` from the simulation; time advances
  strictly via tick-accurate `step()` calls with no `dt` parameter.
- Add a property test: given the same seed and the same input stream,
  two independent runs produce byte-identical `Match` state after N
  ticks (N = 10 000).
- The *client* continues to use a real clock for animations (juice,
  spawn-fade, score rollup) — those are rendered from the mirrored
  snapshot and never feed back into simulation.
- Cross-platform digest check runs on macOS arm64, macOS x86_64, and
  Linux x86_64 in CI (§6.3).

### 5.2 State transitions (client)

```
            ┌────────────┐  mp host / mp join   ┌────────────────┐
            │   Title    │ ───────────────────▶ │   Connecting   │
            └─────┬──────┘                      └───────┬────────┘
                  │ any key (single-player)             │
                  ▼                                     ▼
            ┌────────────┐                      ┌────────────────┐
            │  Playing   │                      │ WaitingForPeer │ (host: show code)
            │ (local v1) │                      └───────┬────────┘
            └────────────┘                              │ JoinRequest (host only)
                                                        ▼
                                                ┌────────────────┐
                                                │  AwaitingAdmit │ (y/n prompt)
                                                └───────┬────────┘
                                                        │ HostDecision accept
                                                        ▼
                                                ┌────────────────┐
                                                │  MatchStarting │ (3-2-1 countdown)
                                                └───────┬────────┘
                                                        ▼
                                                ┌────────────────┐
                                 ┌──────────────│   MatchPlaying │
                                 │ disconnect   └───────┬────────┘
                                 ▼                      │ GameOver event
                           ┌────────────┐               ▼
                           │   Aborted  │       ┌────────────────┐
                           └────────────┘       │   MatchResult  │
                                                └────────────────┘
```

### 5.3 Garbage exchange

#### 5.3.1 Send table (authoritative)

| Clear type                           | Garbage rows sent |
|--------------------------------------|------------------:|
| Single (1 line)                      | 0                 |
| Double (2 lines)                     | 1                 |
| Triple (3 lines)                     | 2                 |
| Quad (4 lines)                       | 4                 |
| T-spin single                        | 2                 |
| T-spin double                        | 4                 |
| T-spin triple                        | 6                 |
| B2B bonus (consecutive quad/T-spin)  | +1 flat           |
| Combo (N consecutive clears ≥ 2)     | floor((N-1)/2)    |
| Perfect clear                        | +10 flat          |

- Arithmetic is on integers; no floating point. All additions are
  computed in u8 then saturated at 20.
- The B2B bonus is **+1 flat**, replacing the v0.1-draft "×1.5"
  language; rounding mode is therefore not needed and no test should
  assume one.
- Drain cap: up to **8 rows** inserted per piece-spawn. Any excess
  **stays queued** and drains on the following spawns; the queue is
  bounded at 40 rows (double-well) above which further incoming
  garbage is dropped and the sender's HUD flashes `capped`.

#### 5.3.2 Sudden-death (10-minute cap)

At `tick == 36000` (10 min × 60 Hz) both wells enter sudden-death:

- Garbage drain cap per spawn rises from 8 to 20.
- An extra garbage row is inserted into each well every 2 s,
  deterministically (same RNG derivation as §5.3.3), bottom-up, with
  one random gap column.
- First top-out wins as usual. If both top-out on the same tick, the
  winner is the player with the lower aggregate well height at the
  start of that tick; further ties are resolved by match_id parity
  (`host wins if match_id is even`). This last tiebreak is arbitrary
  on purpose — it must be deterministic and it is not worth further
  design.

#### 5.3.3 Garbage RNG derivation (pinned)

Garbage gap columns are drawn from an RNG independent of the 7-bag
stream:

```
garbage_rng(recipient, tick) =
    ChaCha20Rng::from_seed(
        blake3(match_seed_le_bytes ∥ [recipient_id] ∥ tick_le_bytes)
            .as_bytes()[..32]
    )
```

Called once per inserted row; the first `u8 % WELL_WIDTH` output is
the gap column. This derivation is part of the wire contract: any
conforming implementation must reproduce the same gap sequence from
the same `(match_seed, recipient_id, tick)` triple.

#### 5.3.4 HUD indicator thresholds

Pending-row indicator colour ramp (uses existing animation palette):

| Pending rows | Colour step      |
|-------------:|------------------|
| 0            | hidden           |
| 1–3          | `DIM`            |
| 4–7          | `OVERLAY`        |
| 8+           | `NEW_BEST` (red) |
| queue capped | flashing `NEW_BEST` at 4 Hz |

### 5.4 Disconnect & lag handling

- Heartbeat: client sends `Ping(nonce)` every 500 ms; server replies
  `Pong(nonce, server_tick)`. No pong for 3 s → client declares
  connection stalled and emits `Goodbye(ClientStalled)` before
  closing. Symmetrically, no ping for 3 s → server awards the match
  to the peer.
- If the *peer* disconnects mid-match, the surviving client gets a
  `Goodbye(PeerDisconnected)` and a win by default.
- Clean shutdown: Ctrl-C / SIGTERM sends `Goodbye(UserQuit)` before
  closing the socket. The shutdown path is exercised by an automated
  test (§6.2).

### 5.5 Security posture

#### 5.5.1 Transport and parsing

- TLS 1.3 only; `rustls` with the `ring` provider. No plaintext
  fallback.
- All frame-size, allocation, queue-depth, and frame-rate limits in
  §4.4 are enforced before the frame is decoded.
- Server ignores client-supplied `server_tick` fields; tick is a
  server-owned monotonic counter.
- Server validates every client `Input` against the simulation — an
  input that names a non-existent move (e.g. `HardDrop` during
  `Phase::GameOver`) is silently dropped; any reserved bit set
  increments the invalid-frame budget.
- Client sanitizes all server-origin strings before painting (§4.5).

#### 5.5.2 Matchmaking abuse controls

All limits in §4.4 are enforced. `HostRoom`, `JoinRoom`, concurrent
sessions, concurrent rooms, and per-node caps are each tracked with
a token bucket or counter. On breach, the offending frame receives
`Error(RateLimited|ServerFull|TooManySessions)` and the session is
closed.

#### 5.5.3 Logging & IP handling

- No persistent data about players is stored server-side beyond logs.
- Access log fields: `match_id`, `event`, `ip_hash`, `ts`.
- `ip_hash = blake3(salt ∥ client_ip)[..8]` where `salt` is a
  server-held 256-bit secret rotated every 24 h. Previous 7 salts are
  retained in memory so abuse correlation works across rotation
  boundaries. Salts are never written to disk; a restart rotates the
  salt.
- Log retention: 7 days, then deleted. Only `ip_hash` and `match_id`
  are present in retained logs; raw IPs exist only in ephemeral
  admission counters and are purged on session close.

#### 5.5.4 Metrics endpoint

A Prometheus `/metrics` endpoint exposes `concurrent_matches`,
`concurrent_sessions`, `tick_budget_p99`, `admission_rejects_total`,
and `invalid_frames_total`. It is **bound to `127.0.0.1:9090` only**
and is scraped by a local node-exporter sidecar; it is never exposed
on the public TLS listener. Self-hosters wanting remote metrics must
deploy their own reverse proxy with their own auth.

### 5.6 CLI surface

```
blocktxt                         # unchanged: single-player v0.1
blocktxt mp host [--server URL]  # host a room, print room code,
                                 #  prompt to accept each join request
blocktxt mp join CODE [--server URL]
```

`--server` defaults to the compiled-in official URL; overridable for
self-hosting. `BLOCKTXT_SERVER` env var is also honoured. **When both
are set, `--server` wins**; this matches common CLI convention.

`mp local` is **not** a v0.2 public subcommand. The two-player
single-process harness still exists behind the `internal-harness`
cargo feature (off by default in release builds) so CI can exercise
`sim::Match` end-to-end without a network, but it is neither
advertised in `--help` nor shipped in the release binary.

## 6. Tests plan

### 6.1 Red/green TDD — built test-first

The following are written as failing tests *before* their implementation
lands, per strict TDD:

- **`sim::Match` determinism** — property test, 10 000 random input
  sequences across two independent instances → identical state.
- **`proto` round-trip** — for every frame variant, `decode(encode(x))
  == x`; malformed bytes → specific `Error` variant (never panic).
- **`proto` size caps** — frames larger than 16 KiB are rejected
  without allocation; decoded frames never allocate > 64 KiB.
- **Version-mismatch refusal** — client with `proto_version = 2`
  against server v1 receives `Error(VersionMismatch)` and exits with
  status 3.
- **Garbage exchange table** — every row of §5.3.1 (single through
  perfect-clear, B2B +1, combo formula) produces the stated row count;
  queue cap at 40 drops excess; drain cap of 8 (normal) / 20
  (sudden-death) leaves the correct residue.
- **Sudden-death trigger** — at tick 36000 exactly, drip garbage
  engages; tiebreaks (height, match_id parity) resolve as specified.
- **Garbage RNG derivation** — given a fixed `match_seed`,
  `recipient_id`, and tick range, gap columns match a checked-in
  golden vector across macOS arm64 / macOS x86_64 / Linux x86_64.
- **Input bit layout** — every named bit encodes/decodes; reserved
  bits set → `Error(MalformedInput)` without panic.
- **Room-code charset** — generated codes never contain `I`, `L`,
  `O`, `U`; property test over 100 k samples.
- **Input-delay pipeline** — an input posted at client_tick T is
  applied at server_tick T+2 under nominal RTT; early inputs queue,
  late inputs are dropped and logged as `InputTooLate`.
- **Snapshot delta/keyframe round-trip** — apply a random walk of
  snapshots; baselines + deltas reconstruct the same `Match` state as
  re-decoding the keyframes directly.
- **Ack-window boundary** — with acks stalled for 32 snapshots a
  keyframe is emitted; at 60 snapshots the session closes with
  `Goodbye(ClientStalled)`.
- **Heartbeat stall auto-award** — simulate 3 s of silence; the live
  peer receives `Goodbye(PeerDisconnected)` within 3.2 s.
- **Terminal sanitization** — fuzz server-origin strings containing
  ANSI / CSI / BEL / NUL bytes; the rendered output is pure printable
  ASCII.
- **Tick-budget invariant** — server `step` for 2 active matches
  completes in < 1 ms on the reference CI VM (criterion bench gated
  as a test).

### 6.2 Integration (green-path, written after the TDD pieces above)

- **Loopback match**: spin up `blocktxt-server` on `127.0.0.1:0`,
  connect two in-process clients via `internal-harness`, script a
  30-second match, assert a `MatchResult` event is emitted and both
  clients exit cleanly.
- **Host admission**: scripted guest is refused when host replies
  `HostDecision(accept=false)`; guest sees `Error(HostDeclined)`.
- **Disconnect mid-match**: kill one client's socket; assert the
  other receives `Goodbye(PeerDisconnected)` within 3 s.
- **SIGTERM graceful shutdown**: spawn the host binary, send SIGTERM,
  assert `Goodbye(UserQuit)` is received by the peer and the process
  exits 0 within 2 s.
- **Corporate-proxy simulation**: run the loopback suite through
  `mitmproxy` in forward mode to confirm wss:443 still works.
- **TLS refusal**: plaintext `ws://` connection attempt → server
  drops with clear error, no process crash.
- **Rate-limit enforcement**: drive 11 `JoinRoom` from one IP in
  60 s; 11th returns `Error(RateLimited)`. Drive 4 `HostRoom` per
  minute; 4th returns `Error(RateLimited)`.
- **Admission cap**: spin up 256 matches, attempt a 257th, receive
  `Error(ServerFull)`; existing matches unaffected.
- **Frame-size enforcement**: send a 17 KiB binary frame; server
  closes session with `Error(FrameTooLarge)` and increments the
  invalid-frame budget.
- **Bandwidth budget**: 60-second loopback match, measure aggregate
  client→server and server→client bytes; both ends must be
  ≤ 8 KiB/s mean and ≤ 32 KiB/s peak.
- **Load test (200 matches)**: gated nightly CI job — 200 concurrent
  scripted matches against one server node; assert `tick_budget_p99
  < 4 ms`, `admission_rejects_total == 0`, zero `Goodbye(ClientStalled)`.
- **Cross-arch determinism**: run the determinism property test on
  macOS arm64, macOS x86_64, and Linux x86_64 in CI; seeds must
  produce identical SHA-256 digests of the final state.

### 6.3 CI additions

- New matrix entry: `cargo test -p blocktxt-sim -p blocktxt-proto
  -p blocktxt-server`, run on macOS arm64, macOS x86_64, and
  Linux x86_64.
- Fuzz target (`cargo-fuzz`) on `proto::decode` — 5-minute nightly
  run; any panic fails the job.
- `cargo deny check` gains an advisories gate for `rustls`, `tokio`,
  `tokio-tungstenite`, `blake3`.
- Nightly load-test job (§6.2) runs on a dedicated 1-vCPU / 512 MiB
  runner to validate the headline scale numbers.

### 6.4 Manual test plan (release-blocking)

Extends `docs/manual-test-plan.md` with a **Multiplayer** section:

- [ ] `mp host` prints a 5-char code; `mp join <code>` on a second
      machine shows the host a `y/N` prompt within 2 s.
- [ ] Host declines → guest sees `host declined` and exits cleanly.
- [ ] Line clear on host inserts garbage on guest well within one
      visible frame of flash-phase completion.
- [ ] Quad sends 4 rows; B2B quad sends 5 rows (4 + 1).
- [ ] 10-minute match reaches sudden-death and resolves.
- [ ] Ctrl-C on host ends the match cleanly for guest with
      `opponent disconnected` overlay.
- [ ] Wrong room code → "room not found" error, no crash.
- [ ] Server unreachable → "cannot reach blocktxt server" within 5 s,
      clean process exit, cooked terminal.
- [ ] Malicious-server dry run: point `--server` at a scripted server
      that emits ANSI-laden `Error.detail`; terminal remains clean.

## 7. Team

Veteran experts to hire for v0.2 multiplayer:

- **Veteran real-time multiplayer game networking engineer (1)** —
  lead for protocol, tick/sync model, garbage exchange, desync audit,
  ack-window/keyframe policy.
- **Veteran Rust systems engineer (1)** — extracts `blocktxt-sim`
  crate, enforces no-std-like purity (no clocks, no thread-locals),
  owns `blocktxt-proto` codec, size caps, and fuzzing.
- **Veteran async-Rust / tokio services engineer (1)** — builds
  `blocktxt-server` (WS accept, admission, matchmaker, match runner,
  graceful shutdown, observability).
- **Veteran CLI / TUI engineer (1)** — wires `mp host` / `mp join`
  subcommands, host-admit prompt, connection-status overlays,
  incoming-garbage HUD, terminal-safe error rendering, and
  integrates the net thread with the existing render loop without
  breaking single-player cadence.
- **Veteran application security engineer (0.5)** — reviews TLS
  config, input validation, admission limits, terminal-escape
  sanitization, IP-hash salt rotation, and the threat model; sign-off
  before public server goes live.
- **Veteran release/QA engineer (0.5)** — owns the manual multiplayer
  test plan, cross-platform matrix (macOS arm64, macOS x86_64,
  Linux x86_64), the nightly load-test job, and the "does
  single-player still work offline" regression gate.

Total: **4 full-time + 1 split** specialist hires on top of the
existing maintainers.

## 8. Implementation plan (sprints)

Each sprint is 1 calendar week. Ordering between specialists shown
with `→` for dependencies and `∥` for parallel work.

### Sprint 1 — Foundations (parallel everywhere)

- Rust systems ∥ networking: extract `blocktxt-sim` crate, swap RNG
  for `ChaCha20Rng`, delete all `Instant::now()` from sim, remove
  `dt` parameter. (TDD: determinism property test first, red.)
- Networking ∥ Rust systems: draft `blocktxt-proto` frame catalogue
  including `Input` bit layout, size caps, round-trip tests
  (red → green).
- CLI: add `blocktxt mp --help` stub subcommand skeleton behind a
  compile-time `multiplayer` feature flag, defaulted *off* until
  Sprint 3 so main stays shippable.
- Security: threat-model doc + `cargo deny` rules + IP-hash salt
  design merged.

### Sprint 2 — Server & internal harness

- Async-tokio: `blocktxt-server` accepts TLS-terminated WS
  connections, enforces all §4.4 limits, implements matchmaker with
  room codes, runs one `sim::Match` per room at 60 Hz.
- Networking → async-tokio: garbage-exchange logic (full §5.3 table,
  sudden-death, RNG derivation) lands in `sim::Match`; server
  implements ack window + keyframe policy + backpressure.
- Rust systems ∥ CLI: `internal-harness` feature exercises
  `sim::Match` end-to-end in a single process for integration tests.
- QA: multiplayer manual test plan draft v1, reviewed.

### Sprint 3 — Online play & polish

- CLI → networking: `mp host` / `mp join` wired to server; host-admit
  prompt, connection overlays ("waiting for opponent", "connecting…",
  "peer disconnected", "host declined"), terminal-safe error
  rendering.
- Networking ∥ CLI: input-delay pipeline, snapshot mirror, echo-ghost
  on client; incoming-garbage HUD column with §5.3.4 thresholds.
- Security: penetration pass on staging (rate-limit verification, TLS
  audit, fuzzing, ANSI-injection dry run, IP-hash correlation check).
- Async-tokio ∥ QA: observability (match_id structured logs with
  keyed-hash IPs; Prometheus `/metrics` on 127.0.0.1:9090) and
  nightly 200-match load test.
- CI: cross-arch determinism check on three platforms, fuzz job,
  release artifact for `blocktxt-server` (static musl binary).

### Sprint 4 — Release hardening

- QA leads the manual matrix across macOS arm64, macOS x86_64,
  Linux x86_64; all veterans on-call for triage.
- Load-test headline numbers (200 matches, bandwidth ≤ 8 KiB/s) must
  pass on the target 1-vCPU / 512 MiB VM before sign-off.
- Security sign-off before flipping DNS for the public server.
- Docs: README multiplayer section, self-hosting guide (including
  metrics-exposure caveat).
- Cut `blocktxt v0.2.0` + `blocktxt-server v0.1.0`.

## 9. Risks & mitigations

| Risk                                     | Likelihood | Mitigation                                              |
|------------------------------------------|------------|---------------------------------------------------------|
| Simulation desync between arches         | Med        | Fixed-point only; CI digest check on three platforms.   |
| Corporate networks block wss:443         | Low        | Use standard port; document HTTP_PROXY support.         |
| Server cost balloons                     | Low        | Match cap of 10 min; admission cap 256 matches/node.    |
| Single-player regresses                  | Med        | `multiplayer` feature is opt-in compile gate in Sprint 1–2; existing v0.1 manual test plan stays green every sprint. |
| Room-code collision                      | Very low   | 33 M codes; matchmaker rejects live collisions.         |
| Room-code hijack by brute force          | Low        | Host-admission step (§4.4) + `JoinRoom` bucket.         |
| Single-node outage wipes live matches    | Med        | Accepted for v0.2 beta; clean `Goodbye(ServerRestart)` + v0.3 multi-node roadmap. |
| ANSI escape injection via server strings | Low        | ASCII-only wire contract + client-side sanitizer (§4.5).|
| Snapshot memory balloon on stalled peer  | Low        | Bounded 32-snapshot ring + 60-snapshot disconnect.      |

## 10. Changelog

- v0.1 — initial scaffold: language/topology/sync/transport/scale
  decided; architecture, protocol, tests, team, 4-sprint plan laid
  out; strict non-goals recorded.
- v0.2 — round-1 review response. Added host-admission join flow and
  second-credential design; expanded abuse/resource controls
  (HostRoom, session, room, frame-size, invalid-frame, idle-room,
  admission-cap limits); defined bounded ack window + keyframe
  policy + outbound queue depth + backpressure rules; pinned
  single-node v0.2 deployment with v0.3 scale roadmap; bound
  `/metrics` to localhost; required terminal-safe rendering of all
  server strings and made `Error.detail` ASCII-only; defined keyed
  rotating IP-hash salt; removed `mp local` from public CLI surface
  (internal harness only); fixed architecture placeholder
  contradiction; fixed RTT vs one-way budget to explicit RTT;
  replaced snapshot-rate-limit restatement with bounded
  ack-window/keyframe policy; completed garbage table (combos,
  T-spins, perfect clear), replaced B2B ×1.5 with flat +1, defined
  queue cap and drain-cap overflow; defined sudden-death mechanics
  at 10-minute cap; added Ping frame alongside Pong and corrected
  direction; removed `dt` parameter from `sim::Match::step`; pinned
  garbage RNG derivation via blake3; pinned `Input` bit layout;
  removed `capabilities` from `Hello` in v1; documented
  `--server`-wins precedence over env var; defined HUD colour-ramp
  thresholds; added automated tests for snapshot pipeline,
  input-delay, heartbeat, bandwidth, rate-limits, SIGTERM, and
  200-match load; added macOS x86_64 to determinism CI; defined
  local echo-ghost render policy for active piece.
