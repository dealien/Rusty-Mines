#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui;
use rusty_mines::minesweeper::{Board, CellState, GameState};
use rusty_mines::solver::{Solver, SolverAction};

// ─── Probability display mode ─────────────────────────────────────────────────

/// Controls when mine-probability numbers are shown on hidden cells.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ProbabilityMode {
    /// Never display probabilities.
    Off,
    /// Only display probabilities when the solver has computed a move (i.e. the
    /// solver was last stepped or auto-played and its state is populated).
    WhenInUse,
    /// Always display probabilities, re-computing on every frame so they stay
    /// current after manual moves or flags.
    Always,
}

// ─── Application State ───────────────────────────────────────────────────────

struct MinesweeperApp {
    // Game state
    board: Board,
    cfg_width: usize,
    cfg_height: usize,
    cfg_mines: usize,
    last_board_size: (usize, usize),

    // Solver state
    solver: Solver,
    solver_auto_play: bool,
    solver_speed_ms: u64, // milliseconds between auto-play steps
    last_solver_step: Instant,
    show_solver_panel: bool,
    show_history_panel: bool,
    probability_mode: ProbabilityMode,

    // History log
    action_history: Vec<String>,
}

impl Default for MinesweeperApp {
    fn default() -> Self {
        let width = 10;
        let height = 10;
        let mines = 15;
        Self {
            board: Board::new(width, height, mines),
            cfg_width: width,
            cfg_height: height,
            cfg_mines: mines,
            last_board_size: (0, 0),

            solver: Solver::new(),
            solver_auto_play: false,
            solver_speed_ms: 300,
            last_solver_step: Instant::now(),
            show_solver_panel: true,
            show_history_panel: true,
            probability_mode: ProbabilityMode::WhenInUse,

            action_history: Vec::new(),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn get_color(mines: u8) -> egui::Color32 {
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
fn probability_color(prob: f32) -> egui::Color32 {
    // Green (safe) → yellow → red (dangerous)
    let r = (prob * 2.0 * 255.0).min(255.0) as u8;
    let g = ((1.0 - prob) * 2.0 * 255.0).min(255.0) as u8;
    egui::Color32::from_rgb(r, g, 60)
}

/// Apply a [`SolverAction`] returned by the solver to the board.
/// Returns a human-readable description for the history log.
fn apply_action(board: &mut Board, action: &SolverAction) -> Option<String> {
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

/// Compute per-cell mine probabilities directly from the board without
/// running the full solver pipeline. Used for "Always" mode so probabilities
/// stay current after manual moves.
fn compute_probabilities(board: &Board) -> HashMap<(usize, usize), f32> {
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
            if let Some(cell) = board.get_cell(x, y)
                && cell.state == CellState::Hidden
            {
                probs.insert((x, y), global_prob);
            }
        }
    }

    // Refine using each revealed numbered cell.
    for y in 0..board.height {
        for x in 0..board.width {
            let cell = match board.get_cell(x, y) {
                Some(c) => c,
                None => continue,
            };
            if cell.state != CellState::Revealed || cell.is_mine || cell.adjacent_mines == 0 {
                continue;
            }
            let mut flag_count = 0usize;
            let mut hidden_neighbours: Vec<(usize, usize)> = Vec::new();

            for dy in -1_i32..=1 {
                for dx in -1_i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && nx < board.width as i32 && ny >= 0 && ny < board.height as i32 {
                        let nx = nx as usize;
                        let ny = ny as usize;
                        match board.get_cell(nx, ny).map(|c| c.state) {
                            Some(CellState::Flagged) => flag_count += 1,
                            Some(CellState::Hidden) => hidden_neighbours.push((nx, ny)),
                            _ => {}
                        }
                    }
                }
            }

            if hidden_neighbours.is_empty() {
                continue;
            }
            let effective = (cell.adjacent_mines as usize).saturating_sub(flag_count);
            let local_prob = effective as f32 / hidden_neighbours.len() as f32;
            for pos in &hidden_neighbours {
                probs.entry(*pos).and_modify(|p| *p = p.max(local_prob));
            }
        }
    }

    probs
}

// ─── App ─────────────────────────────────────────────────────────────────────

impl eframe::App for MinesweeperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Auto-resize window when board dimensions change ──────────────────
        if self.last_board_size != (self.board.width, self.board.height) {
            let cell_size = 30.0;
            let spacing = 2.0;
            let top_panel_height = 70.0;
            let horizontal_padding = 24.0;
            let vertical_padding = 20.0;

            let grid_width = self.board.width as f32 * cell_size
                + (self.board.width.saturating_sub(1)) as f32 * spacing;
            let top_panel_min_width = 380.0;
            let desired_width = (grid_width + horizontal_padding).max(top_panel_min_width);
            let desired_height = self.board.height as f32 * cell_size
                + (self.board.height.saturating_sub(1)) as f32 * spacing
                + top_panel_height
                + vertical_padding;

            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                desired_width,
                desired_height,
            )));
            self.last_board_size = (self.board.width, self.board.height);
        }

        // ── Non-blocking solver auto-play ────────────────────────────────────
        if self.solver_auto_play && self.board.state == GameState::Playing {
            let elapsed = self.last_solver_step.elapsed();
            if elapsed >= Duration::from_millis(self.solver_speed_ms) {
                let action = self.solver.get_next_move(&self.board);
                if let Some(desc) = apply_action(&mut self.board, &action) {
                    let rule = self.solver.state.current_rule.clone();
                    self.action_history.push(format!("{desc}  [{rule}]"));
                }
                self.last_solver_step = Instant::now();
                if action == SolverAction::None {
                    self.solver_auto_play = false;
                }
            }
            ctx.request_repaint_after(Duration::from_millis(self.solver_speed_ms));
        }

        // ── Top panel ────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Rusty Mines");
                    ui.separator();

                    let state_text = match self.board.state {
                        GameState::Playing => "Playing ☺️",
                        GameState::Won => "You Won! 😎",
                        GameState::Lost => "Game Over 😵",
                    };
                    ui.label(state_text);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Restart").clicked() {
                            self.board = Board::new(
                                self.board.width,
                                self.board.height,
                                self.board.num_mines,
                            );
                            self.solver.state.clear();
                            self.solver_auto_play = false;
                            self.action_history.clear();
                        }
                        ui.separator();
                        ui.toggle_value(&mut self.show_history_panel, "📜 History");
                        ui.toggle_value(&mut self.show_solver_panel, "🤖 Solver");
                    });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg_width)
                            .range(5..=30)
                            .speed(0.1),
                    );
                    ui.label("x");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg_height)
                            .range(5..=30)
                            .speed(0.1),
                    );
                    ui.separator();
                    ui.label("Mines:");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg_mines)
                            .range(1..=(self.cfg_width * self.cfg_height - 9))
                            .speed(0.1),
                    );

                    if ui.button("New Game").clicked() {
                        self.board = Board::new(self.cfg_width, self.cfg_height, self.cfg_mines);
                        self.solver.state.clear();
                        self.solver_auto_play = false;
                        self.action_history.clear();
                    }
                });
            });
        });

        // ── Solver control window ─────────────────────────────────────────────
        // Rendered as a free-floating egui::Window so it never overlaps the grid.
        if self.show_solver_panel {
            egui::Window::new("🤖 Auto-Solver")
                .resizable(false)
                .collapsible(true)
                // Default to top-right corner; user can drag it anywhere.
                .default_pos(egui::pos2(ctx.screen_rect().right() + 10.0, 80.0))
                .show(ctx, |ui| {
                    // Current rule / status
                    let rule_text = if self.solver.state.current_rule.is_empty() {
                        "Idle".to_string()
                    } else {
                        self.solver.state.current_rule.clone()
                    };
                    ui.label(egui::RichText::new(format!("Rule: {rule_text}")).italics());
                    ui.separator();

                    // Controls row
                    ui.horizontal(|ui| {
                        let can_step = self.board.state == GameState::Playing;
                        if ui
                            .add_enabled(can_step, egui::Button::new("⏭ Step"))
                            .clicked()
                        {
                            let action = self.solver.get_next_move(&self.board);
                            if let Some(desc) = apply_action(&mut self.board, &action) {
                                let rule = self.solver.state.current_rule.clone();
                                self.action_history.push(format!("{desc}  [{rule}]"));
                            }
                        }

                        let auto_label = if self.solver_auto_play {
                            "⏸ Pause"
                        } else {
                            "▶ Auto-Play"
                        };
                        if ui
                            .add_enabled(can_step, egui::Button::new(auto_label))
                            .clicked()
                        {
                            self.solver_auto_play = !self.solver_auto_play;
                            if self.solver_auto_play {
                                self.last_solver_step = Instant::now();
                            }
                        }
                    });

                    ui.separator();

                    // Speed slider
                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        let mut speed_ms = self.solver_speed_ms as f32;
                        ui.add(
                            egui::Slider::new(&mut speed_ms, 50.0..=2000.0)
                                .suffix(" ms")
                                .logarithmic(true),
                        );
                        self.solver_speed_ms = speed_ms as u64;
                    });

                    ui.separator();

                    // Probability mode selector
                    ui.label("Probabilities:");
                    ui.radio_value(&mut self.probability_mode, ProbabilityMode::Off, "Off");
                    ui.radio_value(
                        &mut self.probability_mode,
                        ProbabilityMode::WhenInUse,
                        "Show when in use",
                    );
                    ui.radio_value(
                        &mut self.probability_mode,
                        ProbabilityMode::Always,
                        "Always show",
                    );

                    ui.separator();

                    let highlight_count = self.solver.state.highlighted_cells.len();
                    if highlight_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("Evaluating {highlight_count} cells"))
                                .color(egui::Color32::YELLOW),
                        );
                    }
                });
        }

        // ── History window ────────────────────────────────────────────────────
        if self.show_history_panel {
            egui::Window::new("📜 Solver History")
                .resizable(true)
                .collapsible(true)
                .default_pos(egui::pos2(ctx.screen_rect().right() + 10.0, 280.0))
                .default_size([220.0, 300.0])
                .show(ctx, |ui| {
                    let total = self.action_history.len();
                    ui.horizontal(|ui| {
                        ui.label(format!("{total} moves"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Clear").clicked() {
                                self.action_history.clear();
                            }
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for (i, entry) in self.action_history.iter().enumerate() {
                                let color = if entry.starts_with("Flag") {
                                    egui::Color32::from_rgb(255, 140, 0)
                                } else {
                                    egui::Color32::LIGHT_GREEN
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:>4}. {entry}", i + 1))
                                        .color(color)
                                        .monospace()
                                        .size(11.0),
                                );
                            }
                        });
                });
        }

        // ── Resolve probability map for this frame ────────────────────────────
        // Always mode: re-compute every frame from scratch to reflect manual moves.
        // WhenInUse mode: use whatever the solver last stored.
        // Off mode: empty map.
        let probabilities: HashMap<(usize, usize), f32> = match self.probability_mode {
            ProbabilityMode::Off => HashMap::new(),
            ProbabilityMode::WhenInUse => self.solver.state.probabilities.clone(),
            ProbabilityMode::Always => compute_probabilities(&self.board),
        };

        // ── Central panel – grid ──────────────────────────────────────────────
        let highlighted: std::collections::HashSet<(usize, usize)> = self
            .solver
            .state
            .highlighted_cells
            .iter()
            .cloned()
            .collect();
        let next_action = self.solver.state.next_action.clone();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);

                egui::Grid::new("minesweeper_grid")
                    .spacing([2.0, 2.0])
                    .min_col_width(30.0)
                    .min_row_height(30.0)
                    .show(ui, |ui| {
                        for y in 0..self.board.height {
                            for x in 0..self.board.width {
                                let cell = self.board.get_cell(x, y).unwrap();

                                // ── Cell text & base color ────────────────
                                let (text, text_color) = match cell.state {
                                    CellState::Hidden => {
                                        ("   ".to_string(), egui::Color32::from_gray(200))
                                    }
                                    CellState::Flagged => {
                                        (" 🚩".to_string(), egui::Color32::from_gray(200))
                                    }
                                    CellState::Revealed => {
                                        if cell.is_mine {
                                            (" 💣".to_string(), egui::Color32::RED)
                                        } else if cell.adjacent_mines > 0 {
                                            (
                                                format!(" {} ", cell.adjacent_mines),
                                                get_color(cell.adjacent_mines),
                                            )
                                        } else {
                                            ("   ".to_string(), egui::Color32::from_gray(60))
                                        }
                                    }
                                };

                                // ── Fill colour ───────────────────────────
                                let is_highlighted = highlighted.contains(&(x, y));
                                let is_next = matches!(&next_action,
                                    SolverAction::Reveal(tx, ty) | SolverAction::Flag(tx, ty)
                                    if *tx == x && *ty == y);

                                let fill_color = if is_next {
                                    egui::Color32::from_rgb(255, 180, 0)
                                } else if is_highlighted {
                                    egui::Color32::from_rgb(40, 100, 60)
                                } else if cell.state == CellState::Revealed && !cell.is_mine {
                                    egui::Color32::from_gray(30)
                                } else {
                                    egui::Color32::from_gray(80)
                                };

                                let button = egui::Button::new(
                                    egui::RichText::new(text).color(text_color).strong(),
                                )
                                .min_size(egui::vec2(30.0, 30.0))
                                .fill(fill_color);

                                let response = ui.add(button);

                                // ── Probability overlay ───────────────────
                                if cell.state == CellState::Hidden
                                    && let Some(&prob) = probabilities.get(&(x, y))
                                {
                                    let pct = (prob * 100.0).round() as u32;
                                    let prob_color = probability_color(prob);
                                    ui.painter().text(
                                        response.rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        format!("{pct}%"),
                                        egui::FontId::proportional(9.0),
                                        prob_color,
                                    );
                                }

                                // ── Click handlers ────────────────────────
                                if self.board.state == GameState::Playing {
                                    if response.clicked() {
                                        self.board.reveal(x, y);
                                        self.solver.state.clear();
                                    } else if response.secondary_clicked() {
                                        self.board.toggle_flag(x, y);
                                        self.solver.state.clear();
                                    }
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_min_inner_size([200.0, 200.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rusty Mines",
        options,
        Box::new(|_cc| Ok(Box::new(MinesweeperApp::default()))),
    )
}
