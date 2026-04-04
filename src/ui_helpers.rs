use eframe::egui;
use std::collections::HashMap;

use crate::minesweeper::{Board, CellState};
use crate::solver::SolverAction;

pub fn get_color(mines: u8) -> egui::Color32 {
    match mines {
        1 => egui::Color32::LIGHT_BLUE,
        2 => egui::Color32::LIGHT_GREEN,
        3 => egui::Color32::LIGHT_RED,
        4 => egui::Color32::from_rgb(0, 0, 139),
        5 => egui::Color32::from_rgb(139, 0, 0),
        6 => egui::Color32::from_rgb(0, 139, 139),
        7 => egui::Color32::BLACK,
        8 => egui::Color32::GRAY,
        _ => egui::Color32::WHITE,
    }
}

/// Map a mine-probability (0.0–1.0) to a display colour for the overlay text.
pub fn probability_color(prob: f32) -> egui::Color32 {
    if prob <= 0.0 {
        return egui::Color32::LIGHT_BLUE;
    }
    // Green (safe) → yellow → red (dangerous)
    let r = (prob * 2.0 * 255.0).min(255.0) as u8;
    let g = ((1.0 - prob) * 2.0 * 255.0).min(255.0) as u8;
    egui::Color32::from_rgb(r, g, 60)
}

/// Apply a [`SolverAction`] to the board, returning a history description.
pub fn apply_action(board: &mut Board, action: &SolverAction) -> Option<String> {
    match *action {
        SolverAction::Reveal(x, y) => {
            board.reveal(x, y);
            Some(format!("Reveal  ({x}, {y})"))
        }
        SolverAction::Flag(x, y) => {
            if let Some(cell) = board.get_cell(x, y)
                && cell.state == CellState::Hidden
            {
                board.toggle_flag(x, y);
                return Some(format!("Flag    ({x}, {y})"));
            }
            None
        }
        SolverAction::None => None,
    }
}

/// Compute mine probabilities directly from board state (two-pass algorithm).
///
/// * **Pass 1**: global density + local max-blend heuristic.
/// * **Pass 2**: definitive override — satisfied constraints produce 0 %;
///   fully-constrained hidden sets produce 100 %.
pub fn compute_probabilities(board: &Board) -> HashMap<(usize, usize), f32> {
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
        return HashMap::new();
    }

    let global_prob = remaining_mines as f32 / total_hidden as f32;

    let mut probs: HashMap<(usize, usize), f32> = HashMap::new();
    for y in 0..board.height {
        for x in 0..board.width {
            if board
                .get_cell(x, y)
                .is_some_and(|c| c.state == CellState::Hidden)
            {
                probs.insert((x, y), global_prob);
            }
        }
    }

    // Shared neighbour-gathering closure.
    let neighbours = |cx: usize, cy: usize| -> (usize, Vec<(usize, usize)>) {
        let mut flags = 0usize;
        let mut hidden = Vec::new();
        for (nx, ny) in board.adjacent_cells(cx, cy) {
            match board.get_cell(nx, ny).map(|c| c.state) {
                Some(CellState::Flagged) => flags += 1,
                Some(CellState::Hidden) => hidden.push((nx, ny)),
                _ => {}
            }
        }
        (flags, hidden)
    };

    // Pass 1 – local max-blend heuristic.
    for y in 0..board.height {
        for x in 0..board.width {
            let cell = match board.get_cell(x, y) {
                Some(c) if c.state == CellState::Revealed && !c.is_mine && c.adjacent_mines > 0 => {
                    c
                }
                _ => continue,
            };
            let (flag_count, hidden) = neighbours(x, y);
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
    // Same algorithm as apply_probability_guess in solver.rs:
    // confirmed_safe cells are excluded from uncertainty counts,
    // confirmed_mine cells are treated as additional flags.
    // Repeats until no new deductions fire.
    let mut confirmed_safe: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();
    let mut confirmed_mine: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();

    loop {
        let mut changed = false;
        for y in 0..board.height {
            for x in 0..board.width {
                let cell = match board.get_cell(x, y) {
                    Some(c)
                        if c.state == CellState::Revealed && !c.is_mine && c.adjacent_mines > 0 =>
                    {
                        c
                    }
                    _ => continue,
                };
                // neighbours() returns (base_flag_count, all_hidden_neighbours).
                let (base_flags, raw_hidden) = neighbours(x, y);
                let mut extra_flags = 0usize;
                let mut uncertain: Vec<(usize, usize)> = Vec::new();
                for pos in &raw_hidden {
                    if confirmed_mine.contains(pos) {
                        extra_flags += 1;
                    } else if !confirmed_safe.contains(pos) {
                        uncertain.push(*pos);
                    }
                }
                let effective =
                    (cell.adjacent_mines as usize).saturating_sub(base_flags + extra_flags);
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

    for pos in &confirmed_safe {
        probs.insert(*pos, 0.0);
    }
    for pos in &confirmed_mine {
        probs.insert(*pos, 1.0);
    }

    probs
}
