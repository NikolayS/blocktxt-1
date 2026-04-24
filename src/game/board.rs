use crate::game::piece::PieceKind;

/// 12-column × 48-row playfield (originality-pass dimensions).
///
/// Rows 0..24 are the hidden buffer (spawn area).
/// Rows 24..48 are the visible playfield.
/// Origin (0, 0) is top-left; x increases rightward, y increases downward.
///
/// The stadium-style 12×24 visible area is an intentional departure from the
/// canonical 10×20 playfield for trade-dress safety (see SPEC §1a).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Board {
    cells: [[Option<PieceKind>; 12]; 48],
}

/// Number of columns in the playfield (logical width).
pub const COLS: usize = 12;
/// Number of visible rows shown to the player.
pub const VISIBLE_ROWS: usize = 24;
/// Number of hidden buffer rows above the visible area.
pub const BUFFER_ROWS: usize = 24;
/// Total rows (visible + buffer).
pub const TOTAL_ROWS: usize = VISIBLE_ROWS + BUFFER_ROWS;

impl Board {
    /// Return an empty board with no occupied cells.
    pub fn empty() -> Self {
        Self {
            cells: [[None; COLS]; TOTAL_ROWS],
        }
    }

    /// Return true if the cell at (col, row) is occupied or out-of-bounds.
    ///
    /// Out-of-bounds coordinates always return true so that collision
    /// checks treat walls and floor as solid.
    pub fn is_occupied(&self, col: i32, row: i32) -> bool {
        if !(0..COLS as i32).contains(&col) || !(0..TOTAL_ROWS as i32).contains(&row) {
            return true;
        }
        self.cells[row as usize][col as usize].is_some()
    }

    /// Place a piece kind at (col, row). Panics if out of bounds.
    pub fn set(&mut self, col: usize, row: usize, piece: PieceKind) {
        self.cells[row][col] = Some(piece);
    }

    /// Return the piece kind at (col, row), or `None` if empty.
    ///
    /// Returns `None` for out-of-bounds coordinates.
    pub fn cell_kind(&self, col: usize, row: usize) -> Option<PieceKind> {
        if col >= COLS || row >= TOTAL_ROWS {
            return None;
        }
        self.cells[row][col]
    }

    /// Clear all fully-occupied rows, shift everything above down,
    /// and return the number of rows cleared.
    ///
    /// When no rows are full, the board is left completely untouched.
    pub fn clear_full_rows(&mut self) -> u8 {
        // First pass: count full rows. If zero, return without mutating
        // any cells — this avoids the off-by-one that would erase row 0.
        let cleared: u8 = self
            .cells
            .iter()
            .filter(|row| row.iter().all(|c| c.is_some()))
            .count() as u8;
        if cleared == 0 {
            return 0;
        }

        // Second pass: compact non-full rows downward, bottom-up.
        // `write_row` is the destination; iterate from the bottom up.
        let last = (TOTAL_ROWS - 1) as i64;
        let mut write_row: i64 = last;
        for read_row in (0..TOTAL_ROWS as i64).rev() {
            let r = read_row as usize;
            if self.cells[r].iter().all(|c| c.is_some()) {
                // Full row — skip (do not copy).
                continue;
            }
            self.cells[write_row as usize] = self.cells[r];
            write_row -= 1;
        }

        // Any rows above the final write_row were vacated — fill them
        // with empty cells. `write_row` now points one row above the
        // topmost compacted row.
        for r in 0..=write_row {
            self.cells[r as usize] = [None; COLS];
        }

        cleared
    }
}
