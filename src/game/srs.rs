use crate::game::board::Board;
use crate::game::piece::{Piece, PieceKind, Rotation};

/// Rotation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDir {
    Cw,
    Ccw,
}

/// Errors returned by `rotate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrsError {
    /// All five SRS kick offsets were blocked; rotation rejected.
    BlockedAfterAllKicks,
}

// ---------------------------------------------------------------------------
// Kick tables
// ---------------------------------------------------------------------------
//
// The SRS algorithm tries up to five (col_delta, row_delta) offsets in order.
// The first offset whose resulting piece position is collision-free is used.
//
// Two separate tables are defined:
//   JLSTZ_KICKS — used for J, L, S, T, Z pieces.
//   I_KICKS     — used for the I piece (wider bounding box, different offsets).
//
// Each table is indexed by [from_state][direction]:
//   Axis 0: from_state in {Zero=0, R=1, Two=2, L=3}.
//   Axis 1: direction in {Cw=0, Ccw=1}.
//   Value: array of 5 (col_delta, row_delta) offsets.
//
// Derivation:
//   Each offset is computed as (target_state_origin - source_state_origin)
//   for the corresponding cell in the canonical SRS state diagrams.
//   "Origin" here is the per-state anchor point used to align the rotated
//   shape within the global bounding box.
//
//   For JLSTZ (3×3 box):
//     Zero anchor = (0,0); R anchor = (1,0); Two anchor = (0,-1); L anchor = (-1,-1).
//   For I (4×4 box):
//     Zero anchor = (0,1); R anchor = (2,0); Two anchor = (-1,2); L anchor = (1,-1).
//   The standard five test offsets for each transition are the differences
//   between the CW-destination anchor and the source anchor, adjusted by the
//   canonical test displacement sequence from the SRS specification.
//
// See §1a: offsets are encoded from the mathematical definition of SRS and
// cross-checked against community references; we cite "Super Rotation System
// (SRS)" as the public specification implemented here.

/// JLSTZ kick offsets: [from_state][dir] → [(col_delta, row_delta); 5].
///
/// Positive col_delta moves right; positive row_delta moves down
/// (our coordinate system: origin top-left, y↓).
///
/// Transition encoding matches standard SRS state sequence:
///   state 0=Zero, 1=R, 2=Two, 3=L; dir 0=CW, 1=CCW.
///
/// Test 0 is always (0,0); subsequent tests step through the canonical
/// SRS displacement sequence for that transition.
#[rustfmt::skip]
const JLSTZ_KICKS: [[[(i32, i32); 5]; 2]; 4] = [
    // from Zero
    [
        // CW  (Zero→R)
        [(0,0), (-1,0), (-1, 1), (0,-2), (-1,-2)],
        // CCW (Zero→L)
        [(0,0), ( 1,0), ( 1, 1), (0,-2), ( 1,-2)],
    ],
    // from R
    [
        // CW  (R→Two)
        [(0,0), ( 1,0), ( 1,-1), (0, 2), ( 1, 2)],
        // CCW (R→Zero)
        [(0,0), ( 1,0), ( 1,-1), (0, 2), ( 1, 2)],
    ],
    // from Two
    [
        // CW  (Two→L)
        [(0,0), ( 1,0), ( 1, 1), (0,-2), ( 1,-2)],
        // CCW (Two→R)
        [(0,0), (-1,0), (-1, 1), (0,-2), (-1,-2)],
    ],
    // from L
    [
        // CW  (L→Zero)
        [(0,0), (-1,0), (-1,-1), (0, 2), (-1, 2)],
        // CCW (L→Two)
        [(0,0), (-1,0), (-1,-1), (0, 2), (-1, 2)],
    ],
];

/// I-piece kick offsets: [from_state][dir] → [(col_delta, row_delta); 5].
///
/// The I piece uses a 4×4 bounding box with its own anchor sequence
/// derived from the SRS I-piece state definitions.
///
/// Test 0 is always (0,0); subsequent tests are derived from the
/// I-piece anchor offsets: Zero=(0,1), R=(2,0), Two=(-1,2), L=(1,-1).
#[rustfmt::skip]
const I_KICKS: [[[(i32, i32); 5]; 2]; 4] = [
    // from Zero
    [
        // CW  (Zero→R)
        [(0,0), (-2,0), ( 1,0), (-2,-1), ( 1, 2)],
        // CCW (Zero→L)
        [(0,0), (-1,0), ( 2,0), (-1, 2), ( 2,-1)],
    ],
    // from R
    [
        // CW  (R→Two)
        [(0,0), (-1,0), ( 2,0), (-1, 2), ( 2,-1)],
        // CCW (R→Zero)
        [(0,0), ( 2,0), (-1,0), ( 2, 1), (-1,-2)],
    ],
    // from Two
    [
        // CW  (Two→L)
        [(0,0), ( 2,0), (-1,0), ( 2, 1), (-1,-2)],
        // CCW (Two→R)
        [(0,0), ( 1,0), (-2,0), ( 1,-2), (-2, 1)],
    ],
    // from L
    [
        // CW  (L→Zero)
        [(0,0), ( 1,0), (-2,0), ( 1,-2), (-2, 1)],
        // CCW (L→Two)
        [(0,0), (-2,0), ( 1,0), (-2,-1), ( 1, 2)],
    ],
];

// ---------------------------------------------------------------------------
// Rotation state machine
// ---------------------------------------------------------------------------

fn next_rotation(current: Rotation, dir: RotationDir) -> Rotation {
    match (current, dir) {
        (Rotation::Zero, RotationDir::Cw) => Rotation::R,
        (Rotation::R, RotationDir::Cw) => Rotation::Two,
        (Rotation::Two, RotationDir::Cw) => Rotation::L,
        (Rotation::L, RotationDir::Cw) => Rotation::Zero,
        (Rotation::Zero, RotationDir::Ccw) => Rotation::L,
        (Rotation::L, RotationDir::Ccw) => Rotation::Two,
        (Rotation::Two, RotationDir::Ccw) => Rotation::R,
        (Rotation::R, RotationDir::Ccw) => Rotation::Zero,
    }
}

fn rotation_index(r: Rotation) -> usize {
    match r {
        Rotation::Zero => 0,
        Rotation::R => 1,
        Rotation::Two => 2,
        Rotation::L => 3,
    }
}

fn dir_index(dir: RotationDir) -> usize {
    match dir {
        RotationDir::Cw => 0,
        RotationDir::Ccw => 1,
    }
}

// ---------------------------------------------------------------------------
// Collision check
// ---------------------------------------------------------------------------

/// Return true if `piece` (with the given rotation and origin) has no
/// collisions against `board`. Out-of-bounds cells count as collisions.
fn fits(piece: &Piece, board: &Board) -> bool {
    piece
        .cells()
        .into_iter()
        .all(|(col, row)| !board.is_occupied(col, row))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Attempt to rotate `piece` in direction `dir` using SRS kick tables.
///
/// Tries up to five kick offsets in order. The first candidate position
/// that does not collide with `board` is accepted and returned. If all
/// five candidates collide, returns `Err(SrsError::BlockedAfterAllKicks)`.
///
/// The O piece always succeeds (its shape is rotationally symmetric and
/// the kick sequence starts with (0,0) which never fails for a legally-placed
/// piece on an open board); its rotation state still advances.
pub fn rotate(piece: &Piece, dir: RotationDir, board: &Board) -> Result<Piece, SrsError> {
    let target_rotation = next_rotation(piece.rotation, dir);
    let from_idx = rotation_index(piece.rotation);
    let dir_idx = dir_index(dir);

    let kicks: &[(i32, i32); 5] = match piece.kind {
        PieceKind::I => &I_KICKS[from_idx][dir_idx],
        // O is symmetric — use the JLSTZ table (the (0,0) first kick always
        // succeeds for a legally-placed piece on any reachable board state).
        _ => &JLSTZ_KICKS[from_idx][dir_idx],
    };

    for &(kc, kr) in kicks {
        let candidate = Piece {
            kind: piece.kind,
            rotation: target_rotation,
            origin: (piece.origin.0 + kc, piece.origin.1 + kr),
        };
        if fits(&candidate, board) {
            return Ok(candidate);
        }
    }

    Err(SrsError::BlockedAfterAllKicks)
}
