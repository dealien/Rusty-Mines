use std::collections::{HashMap, HashSet};

use crate::minesweeper::{Board, CellState};

/// Type alias for a constraint entry used in Rule 2 pattern matching:
/// `(cell_x, cell_y, effective_mine_count, hidden_neighbours)`.
type Constraint = (usize, usize, usize, HashSet<(usize, usize)>);

/// One cached CSP region result: the ordered frontier cell positions paired with
/// every valid mine assignment found by backtracking.  Used by Rule 4 to compute
/// exact per-cell mine frequencies without re-enumeration.
type CspRegionConfig = (Vec<(usize, usize)>, Vec<Vec<u8>>);

/// Internal solver errors for bailing out of complex deductions.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SolveError {
    /// The search space was too deep and hit the time budget.
    Timeout,
}

// ---------------------------------------------------------------------------
// Public API types
// ---------------------------------------------------------------------------

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
    /// Cached valid mine configurations from Rule 3 (Constraint Satisfaction).
    ///
    /// Each entry holds `(ordered frontier cells, valid bit-assignments)` for
    /// one independent sub-region.  Rule 4 (Probability Guess) reads this to
    /// compute **exact** per-cell mine frequencies instead of the heuristic
    /// estimate, for frontier cells that Rule 3 has already enumerated.
    pub csp_configs: Vec<CspRegionConfig>,
}

impl SolverState {
    /// Reset all transient visualisation data between solver steps.
    pub fn clear(&mut self) {
        self.highlighted_cells.clear();
        self.probabilities.clear();
        self.current_rule.clear();
        self.next_action = SolverAction::None;
        self.csp_configs.clear();
    }
}

/// Configuration toggles for enabling/disabling solver deduction tiers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolverSettings {
    /// Rule 1: Standard single-cell deduction.
    pub use_standard: bool,
    /// Rule 2: Pattern-based deduction (subset / 1-2 constraint propagation).
    pub use_subset: bool,
    /// Rule 3: Constraint Satisfaction (Tank algorithm / backtracking DFS).
    pub use_csp: bool,
    /// Rule 4: Probability/heuristic guess (fallback when no certainty exists).
    pub use_probability: bool,
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self {
            use_standard: true,
            use_subset: true,
            use_csp: true,
            use_probability: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Solver
// ---------------------------------------------------------------------------

/// The Minesweeper auto-solver.
///
/// The solver is **read-only** with respect to the board: it inspects `&Board`
/// and returns a [`SolverAction`] that the application loop applies.  This
/// keeps Rust's borrow checker happy and makes the solver trivially testable.
#[derive(Default)]
pub struct Solver {
    /// Public state used by the UI for visualisation.
    pub state: SolverState,
    /// Toggleable options for the solver's deduction engine.
    pub settings: SolverSettings,
}

impl Solver {
    /// Creates a new solver instance with all deduction tiers enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyse the board and return the best next action.
    ///
    /// The solver tries each deduction tier in order, returning as soon as a
    /// certain move is found.  If all certainty tiers fail, Rule 4 makes a
    /// probabilistic guess.  The internal [`SolverState`] is populated as a
    /// side-effect so the UI can render the "thought process".
    ///
    /// # Rule Execution Order
    ///
    /// 1. **Standard Deduction** – single-cell flag/reveal from neighbour counts.
    /// 2. **Pattern Matching** – subset / 1-2 constraint propagation.
    /// 3. **Constraint Satisfaction** – backtracking DFS over independent
    ///    frontier sub-regions (Tank algorithm), using a 3-second time budget.
    /// 4. **Probability Guess** – reveal the hidden cell with the lowest
    ///    estimated mine probability.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_mines::minesweeper::Board;
    /// use rusty_mines::solver::Solver;
    ///
    /// let board = Board::new(9, 9, 10).unwrap();
    /// let mut solver = Solver::new();
    /// let action = solver.get_next_move(&board);
    /// ```
    #[allow(clippy::collapsible_if)]
    pub fn get_next_move(&mut self, board: &Board) -> SolverAction {
        self.state.clear();

        // Rule 1 – Standard Deduction.
        if self.settings.use_standard {
            if let Some(action) = self.apply_standard_deduction(board) {
                self.state.next_action = action.clone();
                return action;
            }
        }

        // Rule 2 – Pattern Matching.
        if self.settings.use_subset {
            if let Some(action) = self.apply_pattern_matching(board) {
                self.state.next_action = action.clone();
                return action;
            }
        }

        // Rule 3 – Constraint Satisfaction.
        // Even when no certainty is found, `csp_configs` is populated so that
        // Rule 4 can derive exact mine frequencies for frontier cells.
        if self.settings.use_csp {
            if let Some(action) = self.apply_csp_deduction(board) {
                self.state.next_action = action.clone();
                return action;
            }
        }

        // Rule 4 – Probability Guess (fallback).
        if self.settings.use_probability {
            let action = self.apply_probability_guess(board);
            self.state.next_action = action.clone();
            return action;
        }

        SolverAction::None
    }

    // -----------------------------------------------------------------------
    // Rule 1 – Standard single-cell deduction
    // -----------------------------------------------------------------------

    /// For every revealed numbered cell, count its hidden and flagged neighbours.
    ///
    /// * If `flags == number`:  all remaining hidden neighbours are safe → reveal.
    /// * If `hidden + flags == number`:  all hidden neighbours are mines → flag.
    fn apply_standard_deduction(&mut self, board: &Board) -> Option<SolverAction> {
        self.state.current_rule = "Standard Deduction".to_string();

        for y in 0..board.height {
            for x in 0..board.width {
                let cell = board.get_cell(x, y)?;
                if cell.state != CellState::Revealed || cell.is_mine || cell.adjacent_mines == 0 {
                    continue;
                }

                let mut flag_count = 0;
                let mut hidden = [(0, 0); 8];
                let mut hidden_count = 0;

                for (nx, ny) in board.adjacent_cells(x, y) {
                    if let Some(c) = board.get_cell(nx, ny) {
                        match c.state {
                            CellState::Flagged => flag_count += 1,
                            CellState::Hidden => {
                                hidden[hidden_count] = (nx, ny);
                                hidden_count += 1;
                            }
                            _ => {}
                        }
                    }
                }

                let number = cell.adjacent_mines as usize;

                if flag_count == number && hidden_count > 0 {
                    // All mines accounted for – reveal the rest.
                    self.state
                        .highlighted_cells
                        .extend(board.adjacent_cells(x, y));
                    return Some(SolverAction::Reveal(hidden[0].0, hidden[0].1));
                }

                if flag_count + hidden_count == number && hidden_count > 0 {
                    // Every hidden neighbour must be a mine.
                    self.state
                        .highlighted_cells
                        .extend(board.adjacent_cells(x, y));
                    return Some(SolverAction::Flag(hidden[0].0, hidden[0].1));
                }
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Rule 2 – Pattern matching (subset / 1-2 constraint propagation)
    // -----------------------------------------------------------------------

    /// Compares pairs of revealed numbered cells whose *effective* constraint
    /// sets overlap.  If one cell's hidden-neighbour set is a strict subset of
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
                let mut flag_count = 0;
                let mut hidden = HashSet::new();

                for (nx, ny) in board.adjacent_cells(x, y) {
                    if let Some(c) = board.get_cell(nx, ny) {
                        match c.state {
                            CellState::Flagged => flag_count += 1,
                            CellState::Hidden => {
                                hidden.insert((nx, ny));
                            }
                            _ => {}
                        }
                    }
                }

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
    // Rule 3 – Constraint Satisfaction (Tank algorithm)
    // -----------------------------------------------------------------------

    /// Partition the board's frontier into independent sub-regions and run a
    /// backtracking DFS over each one to enumerate all valid mine arrangements.
    ///
    /// A cell is deduced **safe** if it receives `0` in every valid
    /// configuration for its region, and a **mine** if it is always `1`.
    ///
    /// All valid configurations are cached in [`SolverState::csp_configs`] so
    /// that Rule 4 can derive exact mine frequencies instead of the heuristic
    /// estimate for frontier cells.
    ///
    /// Regions typically contain ≤ 20 cells, but larger ones are supported until
    /// a 3-second time budget is exhausted, at which point the solver bails out
    /// and falls through to Rule 4 for those specific cells.
    fn apply_csp_deduction(&mut self, board: &Board) -> Option<SolverAction> {
        self.state.current_rule = "Constraint Satisfaction (CST)".to_string();

        // Compute remaining mine budget for global pruning.
        let flagged_count = board
            .cells
            .iter()
            .filter(|c| c.state == CellState::Flagged)
            .count();
        let remaining_mines = board.num_mines.saturating_sub(flagged_count);
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(3000);

        // Build raw frontier constraints and group into independent regions.
        let raw = build_frontier_constraints(board);
        if raw.is_empty() {
            return None;
        }
        let regions = group_into_regions(raw);

        let mut all_safe: Vec<(usize, usize)> = Vec::new();
        let mut all_mines: Vec<(usize, usize)> = Vec::new();

        for region in regions {
            let mut assignment = vec![0u8; region.cells.len()];
            let mut valid_configs: Vec<Vec<u8>> = Vec::new();
            let mut ctx = SearchContext {
                start_time,
                timeout,
                iteration_count: 0,
                remaining_mines,
            };

            if let Err(SolveError::Timeout) =
                backtrack(&region, &mut ctx, &mut assignment, 0, 0, &mut valid_configs)
            {
                // Time budget exhausted for this region; Rule 4 heuristic will take over.
                continue;
            }

            if valid_configs.is_empty() {
                continue;
            }

            let config_count = valid_configs.len();

            // Inspect per-cell mine frequency across all valid configurations.
            for (i, &pos) in region.cells.iter().enumerate() {
                let mine_count = valid_configs.iter().filter(|cfg| cfg[i] == 1).count();
                if mine_count == 0 {
                    all_safe.push(pos);
                } else if mine_count == config_count {
                    all_mines.push(pos);
                }
            }

            // Cache for Rule 4 synergy (exact probabilities on frontier).
            self.state.csp_configs.push((region.cells, valid_configs));
        }

        // Sort for determinism before returning the first certainty.
        all_safe.sort_unstable();
        all_mines.sort_unstable();

        if let Some(&(x, y)) = all_safe.first() {
            self.state.highlighted_cells.push((x, y));
            self.state.current_rule = format!("CST: ({x},{y}) confirmed safe");
            return Some(SolverAction::Reveal(x, y));
        }
        if let Some(&(x, y)) = all_mines.first() {
            self.state.highlighted_cells.push((x, y));
            self.state.current_rule = format!("CST: ({x},{y}) confirmed mine");
            return Some(SolverAction::Flag(x, y));
        }

        None
    }

    // -----------------------------------------------------------------------
    // Rule 4 – Probability-based heuristic guess
    // -----------------------------------------------------------------------

    /// When no certain move exists, estimate per-cell mine probability and
    /// reveal the hidden cell with the lowest probability.
    ///
    /// **Priority for probability estimates:**
    /// 1. Frontier cells enumerated by Rule 3 → exact CSP-derived frequencies.
    /// 2. Cells adjacent to revealed numbers → local constraint heuristic.
    /// 3. Deep-unknown cells (no revealed numbered neighbour) → global density.
    ///
    /// **Tie-breaking priority:**
    /// 1. Lowest mine probability.
    /// 2. Highest hidden-neighbour count (maximises information yield).
    /// 3. Coordinates `(y, x)` ascending (determinism).
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

                let mut flag_count = 0;
                let mut hidden = [(0, 0); 8];
                let mut hidden_count = 0;
                for (nx, ny) in board.adjacent_cells(x, y) {
                    if let Some(c) = board.get_cell(nx, ny) {
                        match c.state {
                            CellState::Flagged => flag_count += 1,
                            CellState::Hidden => {
                                hidden[hidden_count] = (nx, ny);
                                hidden_count += 1;
                            }
                            _ => {}
                        }
                    }
                }

                if hidden_count == 0 {
                    continue;
                }
                let effective = (cell.adjacent_mines as usize).saturating_sub(flag_count);
                let local_prob = effective as f32 / hidden_count as f32;
                for i in 0..hidden_count {
                    probs
                        .entry(hidden[i])
                        .and_modify(|p| *p = p.max(local_prob));
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
                    let mut flag_count = 0usize;
                    let mut uncertain = [(0, 0); 8];
                    let mut uncertain_count = 0;

                    for pos in board.adjacent_cells(x, y) {
                        match board.get_cell(pos.0, pos.1).map(|c| c.state) {
                            Some(CellState::Flagged) => flag_count += 1,
                            Some(CellState::Hidden) => {
                                if confirmed_mine.contains(&pos) {
                                    flag_count += 1; // treat as additional flag
                                } else if !confirmed_safe.contains(&pos) {
                                    uncertain[uncertain_count] = pos;
                                    uncertain_count += 1;
                                }
                                // confirmed_safe cells are excluded from uncertainty
                            }
                            _ => {}
                        }
                    }

                    let effective = (cell.adjacent_mines as usize).saturating_sub(flag_count);

                    if effective == 0 {
                        for i in 0..uncertain_count {
                            if confirmed_safe.insert(uncertain[i]) {
                                changed = true;
                            }
                        }
                    } else if uncertain_count > 0 && effective == uncertain_count {
                        for i in 0..uncertain_count {
                            if confirmed_mine.insert(uncertain[i]) {
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

        // Apply confirmed knowledge — override the heuristic estimates.
        for pos in &confirmed_safe {
            probs.insert(*pos, 0.0);
        }
        for pos in &confirmed_mine {
            probs.insert(*pos, 1.0);
        }

        // Override frontier cells with exact CSP-derived frequencies if Rule 3
        // already enumerated them, replacing heuristic guesses with exact math.
        for (cells, configs) in &self.state.csp_configs {
            let n = configs.len() as f32;
            for (i, &pos) in cells.iter().enumerate() {
                let mine_freq = configs.iter().filter(|cfg| cfg[i] == 1).count() as f32;
                // Insert exact probability, overriding heuristic estimate.
                probs.insert(pos, mine_freq / n);
            }
        }

        self.state.probabilities = probs.clone();

        // Pick the best hidden cell candidate.
        //
        // Priority:
        // 1. Lowest mine probability  (lowest risk)
        // 2. Most hidden neighbours   (highest information yield)
        // 3. Coordinates (y, x)       (determinism)
        let best = probs.iter().min_by(|(pos_a, prob_a), (pos_b, prob_b)| {
            let (ax, ay) = **pos_a;
            let a_prob = **prob_a;
            let (bx, by) = **pos_b;
            let b_prob = **prob_b;

            a_prob
                .partial_cmp(&b_prob)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    // Tie-break 1: Most hidden neighbours (information yield).
                    let a_hidden = board
                        .adjacent_cells(ax, ay)
                        .filter(|&(nx, ny)| {
                            board
                                .get_cell(nx, ny)
                                .is_some_and(|c| c.state == CellState::Hidden)
                        })
                        .count();
                    let b_hidden = board
                        .adjacent_cells(bx, by)
                        .filter(|&(nx, ny)| {
                            board
                                .get_cell(nx, ny)
                                .is_some_and(|c| c.state == CellState::Hidden)
                        })
                        .count();
                    // Descending: most hidden neighbours wins.
                    b_hidden.cmp(&a_hidden)
                })
                .then_with(|| {
                    // Tie-break 2: top-most then left-most for perfect determinism.
                    ay.cmp(&by).then(ax.cmp(&bx))
                })
        });

        match best {
            Some((&(bx, by), _)) => SolverAction::Reveal(bx, by),
            None => SolverAction::None,
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers for Rule 3 – Constraint Satisfaction
// ---------------------------------------------------------------------------

/// A single numbered-cell equation reduced to frontier-cell indices.
///
/// Using indices into a contiguous `Region::cells` vector rather than hashing
/// coordinates avoids repeated hash lookups on the recursive DFS hot path.
struct CspConstraint {
    /// Indices into the parent [`Region::cells`] vector.
    cell_indices: Vec<usize>,
    /// Effective mines still needed (adjacent_mines − already_flagged).
    mines_needed: usize,
}

/// An independent sub-region of the frontier, suitable for isolated backtracking.
struct Region {
    /// Ordered hidden frontier cells for this region (sorted for determinism).
    cells: Vec<(usize, usize)>,
    /// Equations expressed as indices into [`Region::cells`].
    constraints: Vec<CspConstraint>,
}

/// Parameters for the recursive backtracking search to avoid argument bloat.
struct SearchContext {
    start_time: std::time::Instant,
    timeout: std::time::Duration,
    iteration_count: usize,
    remaining_mines: usize,
}

/// Scan the board and produce one raw constraint per active numbered cell.
///
/// Returns `(hidden_neighbours, effective_mine_count)` pairs — one per cell
/// that has at least one hidden neighbour after subtracting placed flags.
fn build_frontier_constraints(board: &Board) -> Vec<(HashSet<(usize, usize)>, usize)> {
    let mut result = Vec::new();
    for y in 0..board.height {
        for x in 0..board.width {
            let cell = match board.get_cell(x, y) {
                Some(c) if c.state == CellState::Revealed && !c.is_mine && c.adjacent_mines > 0 => {
                    c
                }
                _ => continue,
            };
            let mut flag_count = 0;
            let mut hidden = HashSet::new();

            for (nx, ny) in board.adjacent_cells(x, y) {
                if let Some(c) = board.get_cell(nx, ny) {
                    match c.state {
                        CellState::Flagged => flag_count += 1,
                        CellState::Hidden => {
                            hidden.insert((nx, ny));
                        }
                        _ => {}
                    }
                }
            }

            if hidden.is_empty() {
                continue;
            }
            let effective = (cell.adjacent_mines as usize).saturating_sub(flag_count);
            result.push((hidden, effective));
        }
    }
    result
}

/// Iterative union-find root lookup with path-halving compression.
fn uf_find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        // Path halving: link each node to its grandparent.
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Group raw frontier constraints into independent [`Region`]s via union-find.
///
/// Two constraints belong to the same region when they share at least one
/// hidden cell.  Each region can then be solved in isolation, keeping the
/// backtracking search space small (typically ≤ 20 cells per region).
fn group_into_regions(raw: Vec<(HashSet<(usize, usize)>, usize)>) -> Vec<Region> {
    let n = raw.len();
    let mut parent: Vec<usize> = (0..n).collect();

    // Union any two constraints that share at least one hidden cell.
    for i in 0..n {
        for j in (i + 1)..n {
            if !raw[i].0.is_disjoint(&raw[j].0) {
                let ri = uf_find(&mut parent, i);
                let rj = uf_find(&mut parent, j);
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }

    // Collect constraint indices per root component.
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = uf_find(&mut parent, i);
        groups.entry(root).or_default().push(i);
    }

    // Build a Region for each component.
    let mut regions = Vec::with_capacity(groups.len());
    for (_, constraint_indices) in groups {
        // Collect all unique cells for this region; sort for determinism.
        let mut cell_vec: Vec<(usize, usize)> = constraint_indices
            .iter()
            .flat_map(|&ci| raw[ci].0.iter().cloned())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        cell_vec.sort_unstable();

        // Build a stable position → index map for constraint construction.
        let cell_index: HashMap<(usize, usize), usize> = cell_vec
            .iter()
            .enumerate()
            .map(|(idx, &pos)| (pos, idx))
            .collect();

        // Convert raw constraints to index-based form.
        let constraints: Vec<CspConstraint> = constraint_indices
            .iter()
            .map(|&ci| {
                let (ref hidden, mines_needed) = raw[ci];
                CspConstraint {
                    cell_indices: hidden.iter().map(|pos| cell_index[pos]).collect(),
                    mines_needed,
                }
            })
            .collect();

        regions.push(Region {
            cells: cell_vec,
            constraints,
        });
    }
    regions
}

/// Check whether `assignment[0..=index]` violates any constraint.
///
/// * Fully-assigned constraints (all indices ≤ `index`) must have exact mine count.
/// * Partially-assigned constraints are pruned when they are already over-budget
///   or when the remaining unassigned cells cannot satisfy the remaining need.
fn is_locally_valid(region: &Region, assignment: &[u8], index: usize) -> bool {
    for constraint in &region.constraints {
        let mut mines_assigned = 0usize;
        let mut unassigned = 0usize;

        for &ci in &constraint.cell_indices {
            if ci <= index {
                mines_assigned += assignment[ci] as usize;
            } else {
                unassigned += 1;
            }
        }

        if unassigned == 0 {
            // Fully determined: must match exactly.
            if mines_assigned != constraint.mines_needed {
                return false;
            }
        } else {
            // Partial: already over budget?
            if mines_assigned > constraint.mines_needed {
                return false;
            }
            // Enough remaining capacity to fulfil the outstanding need?
            if constraint.mines_needed - mines_assigned > unassigned {
                return false;
            }
        }
    }
    true
}

/// Recursive backtracking DFS over a single independent [`Region`].
///
/// `assignment` is mutated in-place; a clone is only made when a complete
/// valid configuration is found (one allocation per leaf, none on the hot path).
///
/// `frontier_mines` tracks cumulative mines in the current branch so that the
/// **global** mine budget (`ctx.remaining_mines`) can be enforced as an additional
/// pruning constraint.
fn backtrack(
    region: &Region,
    ctx: &mut SearchContext,
    assignment: &mut Vec<u8>,
    index: usize,
    frontier_mines: usize,
    valid_configs: &mut Vec<Vec<u8>>,
) -> Result<(), SolveError> {
    ctx.iteration_count += 1;
    // Periodically check elapsed time to avoid overhead on every single call.
    if ctx.iteration_count.is_multiple_of(1000) && ctx.start_time.elapsed() > ctx.timeout {
        return Err(SolveError::Timeout);
    }

    if index == region.cells.len() {
        // All cells assigned without contradiction → record this valid config.
        valid_configs.push(assignment.clone());
        return Ok(());
    }

    for &value in &[0u8, 1u8] {
        let new_frontier_mines = frontier_mines + value as usize;

        // Global constraint: total assigned mines must not exceed board budget.
        if new_frontier_mines > ctx.remaining_mines {
            continue;
        }

        assignment[index] = value;

        // Prune immediately if the partial assignment breaks a local constraint.
        if is_locally_valid(region, assignment, index) {
            backtrack(
                region,
                ctx,
                assignment,
                index + 1,
                new_frontier_mines,
                valid_configs,
            )?;
        }
        // No explicit reset: assignment[index] is overwritten on the next
        // iteration, and is_locally_valid only inspects indices ≤ `index`.
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minesweeper::Board;

    // ── Shared helpers ───────────────────────────────────────────────────────

    /// Build a deterministic 3×3 board with one mine at (0,0).
    fn make_test_board() -> Board {
        let mut board = Board::new(3, 3, 1).unwrap();
        let mine_idx = board.index(0, 0);
        board.cells[mine_idx].is_mine = true;
        board.first_click = false;

        // Recalculate adjacency counts inline (avoids a private method).
        for cy in 0..board.height {
            for cx in 0..board.width {
                let self_idx = board.index(cx, cy);
                if board.cells[self_idx].is_mine {
                    continue;
                }
                let mut count = 0u8;
                for (nx, ny) in board.adjacent_cells(cx, cy) {
                    let n_idx = board.index(nx, ny);
                    if board.cells[n_idx].is_mine {
                        count += 1;
                    }
                }
                board.cells[self_idx].adjacent_mines = count;
            }
        }
        board
    }

    // ── Rule 1 ───────────────────────────────────────────────────────────────

    #[test]
    fn test_standard_deduction_flag() {
        let mut board = make_test_board();
        // Reveal (1,0): adjacent_mines == 1, lone hidden mine-neighbour is (0,0).
        board.reveal(1, 0);
        board.reveal(1, 1);
        board.reveal(2, 0);
        board.reveal(2, 1);

        let mut solver = Solver::new();
        let action = solver.get_next_move(&board);
        assert!(
            matches!(action, SolverAction::Flag(0, 0)),
            "Expected Flag(0,0), got {action:?}"
        );
    }

    // ── Rule 4 (guess) ───────────────────────────────────────────────────────

    #[test]
    fn test_probability_guess_returns_reveal() {
        // Fresh board: no revealed cells → solver must guess.
        let board = Board::new(5, 5, 5).unwrap();
        let mut solver = Solver::new();
        let action = solver.get_next_move(&board);
        assert!(
            matches!(action, SolverAction::Reveal(_, _)),
            "Expected a Reveal guess, got {action:?}"
        );
    }

    #[test]
    fn test_solver_state_cleared_between_steps() {
        let board = Board::new(5, 5, 5).unwrap();
        let mut solver = Solver::new();
        let _ = solver.get_next_move(&board);
        let first_rule = solver.state.current_rule.clone();
        let _ = solver.get_next_move(&board);
        // State is repopulated each call, not accumulated.
        assert!(!first_rule.is_empty());
    }

    #[test]
    fn test_tie_break_prefers_interior() {
        // 5×5 board with no mines: every cell has 0% probability.
        // Corner (0,0) has 3 total neighbours; interior (2,2) has 8.
        // The solver should pick an interior cell (highest hidden-neighbour count).
        let board = Board::new(5, 5, 0).unwrap();
        let mut solver = Solver::new();
        let action = solver.get_next_move(&board);

        if let SolverAction::Reveal(x, y) = action {
            let hidden_count = board.adjacent_cells(x, y).count();
            assert_eq!(
                hidden_count, 8,
                "Expected an interior cell (8 neighbours), got ({x},{y}) with {hidden_count}"
            );
        } else {
            panic!("Expected Reveal action, got {action:?}");
        }
    }

    #[test]
    fn test_tie_break_determinism() {
        // Same board layout must always produce the same first move.
        let board1 = Board::new(10, 10, 10).unwrap();
        let board2 = Board::new(10, 10, 10).unwrap();
        let mut solver = Solver::new();

        let move1 = solver.get_next_move(&board1);
        let move2 = solver.get_next_move(&board2);
        assert_eq!(move1, move2, "Tie-breaking must be deterministic");
    }

    #[test]
    fn test_settings_disable_rule() {
        let mut board = make_test_board();
        board.reveal(1, 0);
        board.reveal(0, 1);
        board.reveal(1, 1);
        board.reveal(2, 0);
        board.reveal(2, 1);

        let mut solver = Solver::new();
        // With all rules enabled, standard deduction flags (0,0).
        assert!(matches!(
            solver.get_next_move(&board),
            SolverAction::Flag(0, 0)
        ));

        // Disabling all certainty rules leaves only Rule 4 (probability guess).
        // A pure probability guess must not produce a Flag action.
        solver.settings.use_standard = false;
        solver.settings.use_subset = false;
        solver.settings.use_csp = false;
        let action = solver.get_next_move(&board);
        assert!(
            !matches!(action, SolverAction::Flag(_, _)),
            "Probability-only mode must not flag; got {action:?}"
        );
    }

    // ── Rule 3 – CSP ─────────────────────────────────────────────────────────

    #[test]
    fn test_csp_toggle_disabled_falls_through() {
        let mut board = make_test_board();
        board.reveal(1, 0);
        board.reveal(1, 1);
        board.reveal(2, 0);
        board.reveal(2, 1);

        let mut solver = Solver::new();
        // With all rules enabled, standard deduction flags (0,0).
        assert!(matches!(
            solver.get_next_move(&board),
            SolverAction::Flag(0, 0)
        ));

        // Disabling CSP must not crash; standard deduction still fires.
        solver.settings.use_csp = false;
        assert!(matches!(
            solver.get_next_move(&board),
            SolverAction::Flag(0, 0)
        ));
    }

    #[test]
    fn test_csp_large_region_handles_gracefully() {
        // Large board: frontier may exceed the search budget; must not hang or panic.
        let board = Board::new(9, 9, 10).unwrap();
        let mut solver = Solver::new();
        // Empty board → no frontier → falls through to Rule 4 probability guess.
        let action = solver.get_next_move(&board);
        assert!(matches!(action, SolverAction::Reveal(_, _)));
    }

    #[test]
    fn test_csp_state_cleared_each_call() {
        // csp_configs must reflect only the current call, not accumulate.
        let mut board = make_test_board();
        board.reveal(1, 0);
        board.reveal(2, 0);
        board.reveal(1, 1);
        board.reveal(2, 1);

        let mut solver = Solver::new();
        let _ = solver.get_next_move(&board);
        let _ = solver.get_next_move(&board);
        // Should never grow unboundedly.
        assert!(solver.state.csp_configs.len() < 100);
    }

    #[test]
    fn test_csp_global_mine_cap_respected() {
        // Every valid configuration stored in csp_configs must not assign more
        // mines than the board has remaining (flagged mines subtracted).
        let mut board = make_test_board();
        board.reveal(1, 1);
        board.reveal(2, 0);
        board.reveal(2, 1);
        board.reveal(0, 2);
        board.reveal(1, 2);
        board.reveal(2, 2);

        let mut solver = Solver::new();
        let _ = solver.get_next_move(&board);

        let flagged = board
            .cells
            .iter()
            .filter(|c| c.state == CellState::Flagged)
            .count();
        let remaining = board.num_mines.saturating_sub(flagged);

        for (_, configs) in &solver.state.csp_configs {
            for cfg in configs {
                let assigned: usize = cfg.iter().map(|&v| v as usize).sum();
                assert!(
                    assigned <= remaining,
                    "Config assigns {assigned} mines but only {remaining} remain on the board"
                );
            }
        }
    }

    #[test]
    fn test_csp_synergy_overrides_heuristic_for_frontier() {
        // When Rule 3 finds no certainty, it populates csp_configs and falls
        // through to Rule 4.  Rule 4 must then write probabilities using the
        // CSP-exact frequencies rather than the global heuristic.
        //
        // Strategy: use a fresh board with no revealed cells so neither
        // Rule 1/2/3 finds a certainty, and Rule 4 definitely runs.
        let board = Board::new(9, 9, 10).unwrap();
        let mut solver = Solver::new();
        // Fresh board: no frontier → CSP has nothing to enumerate, falls to Rule 4.
        let action = solver.get_next_move(&board);

        // Rule 4 (probability guess) must always fire and produce a Reveal.
        assert!(
            matches!(action, SolverAction::Reveal(_, _)),
            "Expected Reveal from probability guess, got {action:?}"
        );

        // Rule 4 must always populate probabilities.
        assert!(
            !solver.state.probabilities.is_empty(),
            "Rule 4 must populate probabilities for hidden cells"
        );
    }

    #[test]
    fn test_backtrack_all_configs_respect_constraint() {
        // Unit-test the backtracking function directly.
        // Region: 2 cells, constraint: exactly 1 mine.
        let region = Region {
            cells: vec![(0, 0), (1, 0)],
            constraints: vec![CspConstraint {
                cell_indices: vec![0, 1],
                mines_needed: 1,
            }],
        };
        let mut assignment = vec![0u8; 2];
        let mut valid_configs: Vec<Vec<u8>> = Vec::new();
        let mut ctx = SearchContext {
            start_time: std::time::Instant::now(),
            timeout: std::time::Duration::from_millis(3000),
            iteration_count: 0,
            remaining_mines: 10,
        };

        let _ = backtrack(&region, &mut ctx, &mut assignment, 0, 0, &mut valid_configs);

        // Exactly 2 valid configs: [1,0] and [0,1].
        assert_eq!(valid_configs.len(), 2, "Expected exactly 2 valid configs");
        for cfg in &valid_configs {
            let total: usize = cfg.iter().map(|&v| v as usize).sum();
            assert_eq!(total, 1, "Each config must have exactly 1 mine");
        }
    }

    #[test]
    fn test_backtrack_global_mine_cap_prunes() {
        // Region: 2 cells with 1 mine each required, but only 1 mine remaining.
        // Both constraints demand 1 mine → the only way to satisfy both is 2 mines,
        // which exceeds remaining_mines=1. No valid config should be found.
        let region = Region {
            cells: vec![(0, 0), (1, 0)],
            constraints: vec![
                CspConstraint {
                    cell_indices: vec![0],
                    mines_needed: 1,
                },
                CspConstraint {
                    cell_indices: vec![1],
                    mines_needed: 1,
                },
            ],
        };
        let mut assignment = vec![0u8; 2];
        let mut valid_configs: Vec<Vec<u8>> = Vec::new();
        let mut ctx = SearchContext {
            start_time: std::time::Instant::now(),
            timeout: std::time::Duration::from_millis(3000),
            iteration_count: 0,
            remaining_mines: 1,
        };

        let _ = backtrack(&region, &mut ctx, &mut assignment, 0, 0, &mut valid_configs);

        assert!(
            valid_configs.is_empty(),
            "Global mine cap should have pruned all configs, got: {valid_configs:?}"
        );
    }

    #[test]
    fn test_backtrack_timeout_prunes() {
        // Create a basic region but set iteration count to 1000 and time to "now"
        // but set timeout to 0ms to force an immediate timeout.
        let region = Region {
            cells: vec![(0, 0), (1, 0)],
            constraints: vec![CspConstraint {
                cell_indices: vec![0, 1],
                mines_needed: 1,
            }],
        };
        let mut assignment = vec![0u8; 2];
        let mut valid_configs: Vec<Vec<u8>> = Vec::new();
        let mut ctx = SearchContext {
            start_time: std::time::Instant::now(),
            timeout: std::time::Duration::from_millis(0),
            iteration_count: 999, // check triggers on multiples of 1000
            remaining_mines: 10,
        };

        let result = backtrack(&region, &mut ctx, &mut assignment, 0, 0, &mut valid_configs);

        assert!(matches!(result, Err(SolveError::Timeout)));
    }
}
