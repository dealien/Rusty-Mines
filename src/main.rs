#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

use eframe::egui;
use rusty_mines::minesweeper::{Board, CellState, GameState};
use rusty_mines::solver::{Solver, SolverAction};

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
    show_probabilities: bool,
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
            show_probabilities: true,
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
                apply_action(&mut self.board, &action);
                self.last_solver_step = Instant::now();
                if action == SolverAction::None {
                    self.solver_auto_play = false; // nothing to do
                }
            }
            // Keep repainting so auto-play continues.
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
                        }
                        ui.separator();
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
                    }
                });
            });
        });

        // ── Solver control window ─────────────────────────────────────────────
        if self.show_solver_panel {
            egui::Window::new("🤖 Auto-Solver")
                .resizable(false)
                .collapsible(true)
                .default_pos([10.0, 120.0])
                .show(ctx, |ui| {
                    // Status line
                    let rule_text = if self.solver.state.current_rule.is_empty() {
                        "Idle".to_string()
                    } else {
                        self.solver.state.current_rule.clone()
                    };
                    ui.label(egui::RichText::new(format!("Rule: {rule_text}")).italics());
                    ui.separator();

                    // Manual controls
                    ui.horizontal(|ui| {
                        let can_step = self.board.state == GameState::Playing;
                        if ui
                            .add_enabled(can_step, egui::Button::new("⏭ Step"))
                            .clicked()
                        {
                            let action = self.solver.get_next_move(&self.board);
                            apply_action(&mut self.board, &action);
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

                    // Visualization toggles
                    ui.checkbox(&mut self.show_probabilities, "Show probabilities");
                    let highlight_count = self.solver.state.highlighted_cells.len();
                    if highlight_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("Evaluating {highlight_count} cells"))
                                .color(egui::Color32::YELLOW),
                        );
                    }
                });
        }

        // ── Central panel – grid ─────────────────────────────────────────────
        // Snapshot visualisation data so we don't borrow self inside the closure.
        let highlighted: std::collections::HashSet<(usize, usize)> = self
            .solver
            .state
            .highlighted_cells
            .iter()
            .cloned()
            .collect();
        let probabilities = self.solver.state.probabilities.clone();
        let show_probs = self.show_probabilities;
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

                                // ── Fill colour (resolved vs highlighted) ─
                                let is_highlighted = highlighted.contains(&(x, y));
                                let is_next = matches!(&next_action,
                                    SolverAction::Reveal(tx, ty) | SolverAction::Flag(tx, ty)
                                    if *tx == x && *ty == y);

                                let fill_color = if is_next {
                                    // The cell about to be acted on: bright accent
                                    egui::Color32::from_rgb(255, 180, 0)
                                } else if is_highlighted {
                                    // Cells being evaluated: subtle green tint
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

                                // ── Probability overlay on hidden cells ───
                                if show_probs && cell.state == CellState::Hidden
                                    && let Some(&prob) = probabilities.get(&(x, y)) {
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

// ─── Action applicator ───────────────────────────────────────────────────────

/// Apply a [`SolverAction`] returned by the solver to the board.
fn apply_action(board: &mut Board, action: &SolverAction) {
    match *action {
        SolverAction::Reveal(x, y) => board.reveal(x, y),
        SolverAction::Flag(x, y) => {
            // Only flag if the cell is currently hidden.
            if let Some(cell) = board.get_cell(x, y)
                && cell.state == CellState::Hidden {
                    board.toggle_flag(x, y);
                }
        }
        SolverAction::None => {}
    }
}

/// Map a mine-probability (0.0–1.0) to a display colour for the overlay text.
fn probability_color(prob: f32) -> egui::Color32 {
    // Green (safe) → yellow → red (dangerous)
    let r = (prob * 2.0 * 255.0).min(255.0) as u8;
    let g = ((1.0 - prob) * 2.0 * 255.0).min(255.0) as u8;
    egui::Color32::from_rgb(r, g, 60)
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
