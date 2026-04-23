/// The seven piece kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

/// The four rotation states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Zero,
    R,
    Two,
    L,
}

/// A piece on the board: kind, rotation state, and grid origin.
///
/// `origin` is the top-left corner of the piece's bounding box in
/// board coordinates (col, row).  Cell offsets in the shape tables are
/// relative to this origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub kind: PieceKind,
    pub rotation: Rotation,
    pub origin: (i32, i32),
}

impl Piece {
    /// Return absolute board coordinates of this piece's occupied cells.
    pub fn cells(&self) -> Vec<(i32, i32)> {
        let offsets = shape_offsets(self.kind, self.rotation);
        offsets
            .iter()
            .map(|&(dc, dr)| (self.origin.0 + dc, self.origin.1 + dr))
            .collect()
    }
}

/// Cell offsets (col_delta, row_delta) relative to the piece origin.
///
/// Shape tables for all four rotation states: Zero, R, Two, L.
/// Derived by applying 90° CW rotation to the Zero state coordinates
/// within each piece's canonical bounding box.
///
/// I uses a 4×4 bounding box; O uses a 2×2 box; JLSTZ use a 3×3 box.
/// The origin (top-left of the bounding box) is held fixed across
/// rotation states so that kick offsets translate cleanly.
fn shape_offsets(kind: PieceKind, rotation: Rotation) -> &'static [(i32, i32)] {
    match rotation {
        Rotation::Zero => shape_offsets_zero(kind),
        Rotation::R => shape_offsets_r(kind),
        Rotation::Two => shape_offsets_two(kind),
        Rotation::L => shape_offsets_l(kind),
    }
}

// ---------------------------------------------------------------------------
// Zero rotation (spawning orientation)
// ---------------------------------------------------------------------------

/// Zero-rotation cell offsets (col, row) relative to bounding-box top-left.
///
/// Bounding box convention:
///   I, J, L, S, T, Z — 4-wide (cols 0..4), 1–2 rows.
///   O                 — 2-wide (cols 0..2), 2 rows.
fn shape_offsets_zero(kind: PieceKind) -> &'static [(i32, i32)] {
    match kind {
        // I: 4×4 bounding box. Cells in row 1 (0-indexed).
        //    . . . .
        //    X X X X
        //    . . . .
        //    . . . .
        PieceKind::I => &[(0, 1), (1, 1), (2, 1), (3, 1)],
        // O: 2×2 bounding box.
        //    X X
        //    X X
        PieceKind::O => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        // T: 3×3 bounding box.
        //    . X .
        //    X X X
        //    . . .
        PieceKind::T => &[(1, 0), (0, 1), (1, 1), (2, 1)],
        // S: 3×3 bounding box.
        //    . X X
        //    X X .
        //    . . .
        PieceKind::S => &[(1, 0), (2, 0), (0, 1), (1, 1)],
        // Z: 3×3 bounding box.
        //    X X .
        //    . X X
        //    . . .
        PieceKind::Z => &[(0, 0), (1, 0), (1, 1), (2, 1)],
        // J: 3×3 bounding box.
        //    X . .
        //    X X X
        //    . . .
        PieceKind::J => &[(0, 0), (0, 1), (1, 1), (2, 1)],
        // L: 3×3 bounding box.
        //    . . X
        //    X X X
        //    . . .
        PieceKind::L => &[(2, 0), (0, 1), (1, 1), (2, 1)],
    }
}

// ---------------------------------------------------------------------------
// R rotation (90° CW from Zero)
// ---------------------------------------------------------------------------

/// R-rotation cell offsets (col, row) relative to bounding-box top-left.
///
/// Derived by rotating Zero-state cells 90° CW within the bounding box:
///   (col, row) → (box_rows - 1 - row, col)   [for square boxes]
/// I box is 4×4; O box is 2×2; JLSTZ box is 3×3.
fn shape_offsets_r(kind: PieceKind) -> &'static [(i32, i32)] {
    match kind {
        // I: 4×4 box. 90° CW: Zero row1 → R col2.
        //    . . X .
        //    . . X .
        //    . . X .
        //    . . X .
        PieceKind::I => &[(2, 0), (2, 1), (2, 2), (2, 3)],
        // O: rotation is identity (2×2 symmetric).
        PieceKind::O => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        // T: 3×3 box. 90° CW.
        //    . X .
        //    . X X
        //    . X .
        PieceKind::T => &[(1, 0), (1, 1), (2, 1), (1, 2)],
        // S: 3×3 box. 90° CW.
        //    . X .
        //    . X X
        //    . . X
        PieceKind::S => &[(1, 0), (1, 1), (2, 1), (2, 2)],
        // Z: 3×3 box. 90° CW.
        //    . . X
        //    . X X
        //    . X .
        PieceKind::Z => &[(2, 0), (1, 1), (2, 1), (1, 2)],
        // J: 3×3 box. 90° CW.
        //    . X X
        //    . X .
        //    . X .
        PieceKind::J => &[(1, 0), (2, 0), (1, 1), (1, 2)],
        // L: 3×3 box. 90° CW.
        //    . X .
        //    . X .
        //    . X X
        PieceKind::L => &[(1, 0), (1, 1), (1, 2), (2, 2)],
    }
}

// ---------------------------------------------------------------------------
// Two rotation (180° from Zero)
// ---------------------------------------------------------------------------

/// Two-rotation cell offsets (col, row) relative to bounding-box top-left.
///
/// Derived by rotating Zero 180° within the bounding box:
///   (col, row) → (box_cols - 1 - col, box_rows - 1 - row)
fn shape_offsets_two(kind: PieceKind) -> &'static [(i32, i32)] {
    match kind {
        // I: 4×4 box. 180°: Zero row1 → Two row2.
        //    . . . .
        //    . . . .
        //    X X X X
        //    . . . .
        PieceKind::I => &[(0, 2), (1, 2), (2, 2), (3, 2)],
        // O: rotation is identity.
        PieceKind::O => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        // T: 3×3 box. 180°.
        //    . . .
        //    X X X
        //    . X .
        PieceKind::T => &[(0, 1), (1, 1), (2, 1), (1, 2)],
        // S: 3×3 box. 180°.
        //    . . .
        //    . X X
        //    X X .
        PieceKind::S => &[(1, 1), (2, 1), (0, 2), (1, 2)],
        // Z: 3×3 box. 180°.
        //    . . .
        //    X X .
        //    . X X
        PieceKind::Z => &[(0, 1), (1, 1), (1, 2), (2, 2)],
        // J: 3×3 box. 180°.
        //    . . .
        //    X X X
        //    . . X
        PieceKind::J => &[(0, 1), (1, 1), (2, 1), (2, 2)],
        // L: 3×3 box. 180°.
        //    . . .
        //    X X X
        //    X . .
        PieceKind::L => &[(0, 1), (1, 1), (2, 1), (0, 2)],
    }
}

// ---------------------------------------------------------------------------
// L rotation (90° CCW from Zero; equivalently 270° CW)
// ---------------------------------------------------------------------------

/// L-rotation cell offsets (col, row) relative to bounding-box top-left.
///
/// Derived by rotating Zero 90° CCW within the bounding box:
///   (col, row) → (row, box_cols - 1 - col)   [for square boxes]
fn shape_offsets_l(kind: PieceKind) -> &'static [(i32, i32)] {
    match kind {
        // I: 4×4 box. 90° CCW: Zero row1 → L col1.
        //    . X . .
        //    . X . .
        //    . X . .
        //    . X . .
        PieceKind::I => &[(1, 0), (1, 1), (1, 2), (1, 3)],
        // O: rotation is identity.
        PieceKind::O => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        // T: 3×3 box. 90° CCW.
        //    . X .
        //    X X .
        //    . X .
        PieceKind::T => &[(1, 0), (0, 1), (1, 1), (1, 2)],
        // S: 3×3 box. 90° CCW.
        //    X . .
        //    X X .
        //    . X .
        PieceKind::S => &[(0, 0), (0, 1), (1, 1), (1, 2)],
        // Z: 3×3 box. 90° CCW.
        //    . X .
        //    X X .
        //    X . .
        PieceKind::Z => &[(1, 0), (0, 1), (1, 1), (0, 2)],
        // J: 3×3 box. 90° CCW.
        //    . X .
        //    . X .
        //    X X .
        PieceKind::J => &[(1, 0), (1, 1), (0, 2), (1, 2)],
        // L: 3×3 box. 90° CCW.
        //    X X .
        //    . X .
        //    . X .
        PieceKind::L => &[(0, 0), (1, 0), (1, 1), (1, 2)],
    }
}

/// Spawn a piece at the Guideline-inspired spawn position.
///
/// Per SPEC §4 (round-2 decision):
/// - O: 2-wide bbox top-left at (col=4, row=18) → cells at cols 4..=5.
/// - I: 4-wide bbox top-left at (col=3, row=18) → bbox cols 3..7.
/// - J, L, S, T, Z: 4-wide bbox top-left at (col=3, row=18) → bbox cols 3..7.
pub fn spawn(kind: PieceKind) -> Piece {
    let origin = match kind {
        PieceKind::O => (4, 18),
        _ => (3, 18),
    };
    Piece {
        kind,
        rotation: Rotation::Zero,
        origin,
    }
}
