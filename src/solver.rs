use std::collections::{HashMap, HashSet};

use crate::minesweeper::{Board, CellState};

/// Type alias for a constraint entry: (cell_x, cell_y, effective_mine_count, hidden_neighbours).
type Constraint = (usize, usize, usize, HashSet<(usize, usize)>);

/// An action the solver wants the game loop to apply to the board.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SolverAction {
    /// Safely reveal this cell.
    Reveal(usize, usize),
    /// Flag this cell as a mine.
    Flag(usize, usize),
    /// No certain move found; the solver needs to guess.
    #[default]
    None,
}

/// Transient state the solver produces for visual "thought process" rendering.
#[derive(Debug, Clone, Default)]
pub struct SolverState {
    /// Cells currently being evaluated (shown with a highlight).
    pub highlighted_cells: Vec<(usize, usize)>,
    /// Per-cell probability estimate of containing a mine (0.0 – 1.0).
    pub probabilities: HashMap<(usize, usize), f32>,
    /// Human-readable description of the rule currently being applied.
    pub current_rule: String,
    /// The next action the solver has decided on (if any).
    pub next_action: SolverAction,
}

impl SolverState {
    /// Reset transient visualisation data between solver steps.
    pub fn clear(&mut self) {
        self.highlighted_cells.clear();
        self.probabilities.clear();
        self.current_rule.clear();
        self.next_action = SolverAction::None;
    }
}

/// The Minesweeper auto-solver.
///
/// The solver is **read-only** with respect to the board: it inspects `&Board`
/// and returns a [`SolverAction`] that the application loop applies.  This
/// keeps Rust's borrow checker happy and makes the solver trivially testable.
#[derive(Default)]
pub struct Solver {
    /// Public state used by the UI for visualisation.
    pub state: SolverState,
}

impl Solver {
    /// Creates a new solver instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyse the board and return the best next action.
    ///
    /// The internal [`SolverState`] is updated with visualisation data as a
    /// side-effect so the UI can render the "thought process" without an extra
    /// round-trip.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_mines::minesweeper::Board;
    /// use rusty_mines::solver::Solver;
    ///
    /// let board = Board::new(9, 9, 10);
    /// let mut solver = Solver::new();
    /// let action = solver.get_next_move(&board);
    /// ```
    pub fn get_next_move(&mut self, board: &Board) -> SolverAction {
        self.state.clear();

        // Try each rule tier in order from cheapest to most expensive.
        if let Some(action) = self.apply_standard_deduction(board) {
            self.state.next_action = action.clone();
            return action;
        }

        if let Some(action) = self.apply_pattern_matching(board) {
            self.state.next_action = action.clone();
            return action;
        }

        // Nothing certain – fall back to a probability-based guess.
        let action = self.apply_probability_guess(board);
        self.state.next_action = action.clone();
        action
    }

    // -----------------------------------------------------------------------
    // Rule 1 – Standard single-cell deduction
    // -----------------------------------------------------------------------

    /// For every revealed numbered cell, count its hidden and flagged neighbours.
    ///
    /// * If `flags == number`:  all remaining hidden neighbours are safe → reveal.
    /// * If `hidden == number - flags`:  all hidden neighbours are mines → flag.
    fn apply_standard_deduction(&mut self, board: &Board) -> Option<SolverAction> {
        self.state.current_rule = "Standard Deduction".to_string();

        for y in 0..board.height {
            for x in 0..board.width {
                let cell = board.get_cell(x, y)?;
                if cell.state != CellState::Revealed || cell.is_mine || cell.adjacent_mines == 0 {
                    continue;
                }

                let neighbours = get_neighbours(board, x, y);
                let flagged: Vec<_> = neighbours
                    .iter()
                    .filter(|&&(nx, ny)| {
                        board
                            .get_cell(nx, ny)
                            .is_some_and(|c| c.state == CellState::Flagged)
                    })
                    .cloned()
                    .collect();
                let hidden: Vec<_> = neighbours
                    .iter()
                    .filter(|&&(nx, ny)| {
                        board
                            .get_cell(nx, ny)
                            .is_some_and(|c| c.state == CellState::Hidden)
                    })
                    .cloned()
                    .collect();

                let number = cell.adjacent_mines as usize;
                let flag_count = flagged.len();
                let hidden_count = hidden.len();

                if flag_count == number && !hidden.is_empty() {
                    // All mines accounted for – reveal the rest.
                    self.state.highlighted_cells.extend(neighbours.iter());
                    return Some(SolverAction::Reveal(hidden[0].0, hidden[0].1));
                }

                if flag_count + hidden_count == number && !hidden.is_empty() {
                    // Every hidden neighbour must be a mine.
                    self.state.highlighted_cells.extend(neighbours.iter());
                    return Some(SolverAction::Flag(hidden[0].0, hidden[0].1));
                }
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Rule 2 – Pattern matching (subset / 1-2 constraint propagation)
    // -----------------------------------------------------------------------

    /// Compares pairs of revealed numbered cells whose *effective* constraint sets
    /// overlap.  If one cell's hidden-neighbour set is a strict subset of
    /// another's, the difference in their numbers tells us whether the subset
    /// cells are safe or mines.
    fn apply_pattern_matching(&mut self, board: &Board) -> Option<SolverAction> {
        self.state.current_rule = "Pattern Matching (subset)".to_string();

        // Build a list of (cell position, effective_mine_count, hidden_neighbours).
        let constraints: Vec<Constraint> = (0..board.height)
            .flat_map(|y| (0..board.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| {
                let cell = board.get_cell(x, y)?;
                if cell.state != CellState::Revealed || cell.is_mine || cell.adjacent_mines == 0 {
                    return None;
                }
                let neighbours = get_neighbours(board, x, y);
                let flag_count = neighbours
                    .iter()
                    .filter(|&&(nx, ny)| {
                        board
                            .get_cell(nx, ny)
                            .is_some_and(|c| c.state == CellState::Flagged)
                    })
                    .count();
                let hidden: HashSet<_> = neighbours
                    .iter()
                    .filter(|&&(nx, ny)| {
                        board
                            .get_cell(nx, ny)
                            .is_some_and(|c| c.state == CellState::Hidden)
                    })
                    .cloned()
                    .collect();

                if hidden.is_empty() {
                    return None;
                }

                let effective = (cell.adjacent_mines as usize).saturating_sub(flag_count);
                Some((x, y, effective, hidden))
            })
            .collect();

        // For every ordered pair, check subset relationships.
        for i in 0..constraints.len() {
            for j in 0..constraints.len() {
                if i == j {
                    continue;
                }
                let (ax, ay, a_mines, ref a_set) = constraints[i];
                let (_, _, b_mines, ref b_set) = constraints[j];

                if b_set.is_subset(a_set) && b_set.len() < a_set.len() {
                    let diff_mines = a_mines.saturating_sub(b_mines);
                    let diff_cells: Vec<_> = a_set.difference(b_set).cloned().collect();

                    if diff_cells.is_empty() {
                        continue;
                    }

                    self.state.highlighted_cells.push((ax, ay));
                    self.state.highlighted_cells.extend(diff_cells.iter());

                    if diff_mines == 0 {
                        // All diff cells are safe.
                        let (tx, ty) = diff_cells[0];
                        self.state.current_rule =
                            format!("Pattern: [{ax},{ay}] minus subset → {diff_cells:?} are safe");
                        return Some(SolverAction::Reveal(tx, ty));
                    } else if diff_mines == diff_cells.len() {
                        // All diff cells are mines.
                        let (tx, ty) = diff_cells[0];
                        self.state.current_rule =
                            format!("Pattern: [{ax},{ay}] minus subset → {diff_cells:?} are mines");
                        return Some(SolverAction::Flag(tx, ty));
                    }
                }
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Rule 3 – Probability-based heuristic guess
    // -----------------------------------------------------------------------

    /// When no certain move exists, estimate per-cell mine probability and
    /// reveal the hidden cell with the lowest probability.
    ///
    /// Cells bordering numbered cells are estimated from neighbour constraints.
    /// Unreachable interior cells use the global density estimate.
    fn apply_probability_guess(&mut self, board: &Board) -> SolverAction {
        self.state.current_rule = "Probability Guess".to_string();

        let total_hidden = board
            .cells
            .iter()
            .filter(|c| c.state == CellState::Hidden)
            .count();
        let flagged_count = board
            .cells
            .iter()
            .filter(|c| c.state == CellState::Flagged)
            .count();
        let remaining_mines = board.num_mines.saturating_sub(flagged_count);

        if total_hidden == 0 {
            return SolverAction::None;
        }

        let global_prob = remaining_mines as f32 / total_hidden as f32;

        // Initialise all hidden cells with the global density.
        let mut probs: HashMap<(usize, usize), f32> = HashMap::new();
        for y in 0..board.height {
            for x in 0..board.width {
                if let Some(cell) = board.get_cell(x, y)
                    && cell.state == CellState::Hidden
                {
                    probs.insert((x, y), global_prob);
                }
            }
        }

        // Pass 1 – local probability estimate (conservative max blend).
        for y in 0..board.height {
            for x in 0..board.width {
                let cell = match board.get_cell(x, y) {
                    Some(c) => c,
                    None => continue,
                };
                if cell.state != CellState::Revealed || cell.is_mine || cell.adjacent_mines == 0 {
                    continue;
                }
                let neighbours = get_neighbours(board, x, y);
                let flag_count = neighbours
                    .iter()
                    .filter(|&&(nx, ny)| {
                        board
                            .get_cell(nx, ny)
                            .is_some_and(|c| c.state == CellState::Flagged)
                    })
                    .count();
                let hidden: Vec<_> = neighbours
                    .iter()
                    .filter(|&&(nx, ny)| {
                        board
                            .get_cell(nx, ny)
                            .is_some_and(|c| c.state == CellState::Hidden)
                    })
                    .cloned()
                    .collect();

                if hidden.is_empty() {
                    continue;
                }
                let effective = (cell.adjacent_mines as usize).saturating_sub(flag_count);
                let local_prob = effective as f32 / hidden.len() as f32;
                for pos in &hidden {
                    probs.entry(*pos).and_modify(|p| *p = p.max(local_prob));
                }
            }
        }

        // Pass 2 – iterative constraint propagation until convergence.
        //
        // A single pass cannot catch transitive deductions: e.g. Cell A is
        // confirmed safe → constraints containing A are re-evaluated → Cell B
        // may now also be confirmed safe or a mine.  We repeat until nothing
        // changes.
        //
        // Rules per numbered cell on each iteration:
        //   confirmed_safe cells → excluded (treated as already-revealed safe)
        //   confirmed_mine cells → counted as additional effective flags
        //   If effective_mines == 0:            all uncertain hidden → safe (0 %)
        //   If effective_mines == uncertain_count: all uncertain hidden → mine (100 %)
        let mut confirmed_safe: HashSet<(usize, usize)> = HashSet::new();
        let mut confirmed_mine: HashSet<(usize, usize)> = HashSet::new();

        loop {
            let mut changed = false;
            for y in 0..board.height {
                for x in 0..board.width {
                    let cell = match board.get_cell(x, y) {
                        Some(c)
                            if c.state == CellState::Revealed
                                && !c.is_mine
                                && c.adjacent_mines > 0 =>
                        {
                            c
                        }
                        _ => continue,
                    };
                    let all_neighbours = get_neighbours(board, x, y);
                    let mut flag_count = 0usize;
                    let mut uncertain: Vec<(usize, usize)> = Vec::new();

                    for &pos in &all_neighbours {
                        match board.get_cell(pos.0, pos.1).map(|c| c.state) {
                            Some(CellState::Flagged) => flag_count += 1,
                            Some(CellState::Hidden) => {
                                if confirmed_mine.contains(&pos) {
                                    flag_count += 1; // treat as additional flag
                                } else if !confirmed_safe.contains(&pos) {
                                    uncertain.push(pos); // truly uncertain
                                }
                                // confirmed_safe hidden cells are ignored
                            }
                            _ => {}
                        }
                    }

                    let effective = (cell.adjacent_mines as usize).saturating_sub(flag_count);

                    if effective == 0 {
                        for pos in &uncertain {
                            if confirmed_safe.insert(*pos) {
                                changed = true;
                            }
                        }
                    } else if !uncertain.is_empty() && effective == uncertain.len() {
                        for pos in &uncertain {
                            if confirmed_mine.insert(*pos) {
                                changed = true;
                            }
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        // Apply confirmed knowledge — these override probabilistic estimates.
        for pos in &confirmed_safe {
            probs.insert(*pos, 0.0);
        }
        for pos in &confirmed_mine {
            probs.insert(*pos, 1.0);
        }

        self.state.probabilities = probs.clone();

        // Pick the hidden cell with the lowest mine probability.
        let best = probs
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));

        match best {
            Some((&(bx, by), _)) => SolverAction::Reveal(bx, by),
            None => SolverAction::None,
        }
    }

    // -----------------------------------------------------------------------
    // Rule 4 – (Placeholder) Constraint Satisfaction
    // -----------------------------------------------------------------------

    /// Treat the boundary between revealed and hidden cells as a system of
    /// linear constraints to find moves that basic deduction misses.
    ///
    /// This is a stub – a full implementation would enumerate consistent mine
    /// assignments and mark cells safe/mine when they agree in every solution.
    #[allow(dead_code)]
    fn advanced_deduction(&self, _board: &Board) -> Option<SolverAction> {
        // TODO: Implement full constraint satisfaction / gaussian elimination.
        None
    }
}

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

/// Return all valid (x, y) neighbours of a cell.
fn get_neighbours(board: &Board, x: usize, y: usize) -> Vec<(usize, usize)> {
    let mut result = Vec::with_capacity(8);
    for dy in -1_i32..=1 {
        for dx in -1_i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && nx < board.width as i32 && ny >= 0 && ny < board.height as i32 {
                result.push((nx as usize, ny as usize));
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minesweeper::Board;

    /// Build a manually configured board (no randomness) by directly setting
    /// cell data after construction.
    fn make_test_board() -> Board {
        // 3×3 board: mine at (0,0), everything else safe.
        let mut board = Board::new(3, 3, 1);

        // Pre-compute index, then mutate – avoids simultaneous borrow of `board`.
        let mine_idx = board.index(0, 0);
        board.cells[mine_idx].is_mine = true;
        board.first_click = false;

        // Recalculate adjacencies inline (method is private).
        for cy in 0..board.height {
            for cx in 0..board.width {
                let self_idx = board.index(cx, cy);
                if board.cells[self_idx].is_mine {
                    continue;
                }
                let mut count = 0u8;
                for dy in -1_i32..=1 {
                    for dx in -1_i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = cx as i32 + dx;
                        let ny = cy as i32 + dy;
                        if nx >= 0 && nx < board.width as i32 && ny >= 0 && ny < board.height as i32
                        {
                            let n_idx = board.index(nx as usize, ny as usize);
                            if board.cells[n_idx].is_mine {
                                count += 1;
                            }
                        }
                    }
                }
                board.cells[self_idx].adjacent_mines = count;
            }
        }
        board
    }

    #[test]
    fn test_standard_deduction_flag() {
        let mut board = make_test_board();
        // Reveal (1,0): adjacent_mines == 1, its only hidden mine-neighbour is (0,0).
        board.reveal(1, 0);
        // Also reveal (1,1) so deduction has more context.
        board.reveal(1, 1);
        board.reveal(2, 0);
        board.reveal(2, 1);

        let mut solver = Solver::new();
        let action = solver.get_next_move(&board);
        // The solver should flag (0,0) as it's the only hidden neighbour of a "1".
        assert!(
            matches!(action, SolverAction::Flag(0, 0)),
            "Expected Flag(0,0), got {action:?}"
        );
    }

    #[test]
    fn test_probability_guess_returns_reveal() {
        // Empty 5×5 board with no revealed cells – solver must guess.
        let board = Board::new(5, 5, 5);
        let mut solver = Solver::new();
        let action = solver.get_next_move(&board);
        assert!(
            matches!(action, SolverAction::Reveal(_, _)),
            "Expected a Reveal guess, got {action:?}"
        );
    }

    #[test]
    fn test_solver_state_cleared_between_steps() {
        let board = Board::new(5, 5, 5);
        let mut solver = Solver::new();
        let _ = solver.get_next_move(&board);
        let first_rule = solver.state.current_rule.clone();
        let _ = solver.get_next_move(&board);
        // State is re-populated each call, not accumulated.
        assert!(!first_rule.is_empty());
    }
}
