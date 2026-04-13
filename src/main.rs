#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui;
use rusty_mines::minesweeper::{Board, CellState, GameState};
use rusty_mines::solver::{Solver, SolverAction};
use rusty_mines::ui_helpers::{apply_action, compute_probabilities, get_color, probability_color};

// ─── Probability display mode ─────────────────────────────────────────────────

/// Controls when mine-probability numbers are shown on hidden cells.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ProbabilityMode {
    /// Never display probabilities.
    Off,
    /// Only when the solver has computed a move (state is populated).
    WhenInUse,
    /// Always recompute and display on every frame.
    Always,
}

// ─── Application State ───────────────────────────────────────────────────────

struct MinesweeperApp {
    board: Board,
    cfg_width: usize,
    cfg_height: usize,
    cfg_mines: usize,
    last_board_size: (usize, usize),

    solver: Solver,
    solver_auto_play: bool,
    solver_speed_ms: u64,
    last_solver_step: Instant,
    show_solver_panel: bool,
    show_history_panel: bool,
    probability_mode: ProbabilityMode,

    action_history: Vec<String>,
}

impl Default for MinesweeperApp {
    fn default() -> Self {
        let width = 25;
        let height = 25;
        let mines = 100;
        Self {
            board: Board::new(width, height, mines)
                .unwrap_or_else(|| Board::new(10, 10, 10).unwrap()),
            cfg_width: width,
            cfg_height: height,
            cfg_mines: mines,
            last_board_size: (0, 0),

            solver: Solver::new(),
            solver_auto_play: false,
            solver_speed_ms: 50,
            last_solver_step: Instant::now(),
            show_solver_panel: true,
            show_history_panel: true,
            probability_mode: ProbabilityMode::Always,

            action_history: Vec::new(),
        }
    }
}

// ─── App ─────────────────────────────────────────────────────────────────────

impl eframe::App for MinesweeperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Auto-resize main window ──────────────────────────────────────────
        if self.last_board_size != (self.board.width, self.board.height) {
            let cell_size = 30.0;
            let spacing = 2.0;
            let top_panel_height = 70.0;
            let horizontal_padding = 24.0;
            let vertical_padding = 20.0;

            let grid_width = self.board.width as f32 * cell_size
                + (self.board.width.saturating_sub(1)) as f32 * spacing;
            let desired_width = (grid_width + horizontal_padding).max(380.0);
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

        // ── Non-blocking auto-play ───────────────────────────────────────────
        if self.solver_auto_play && self.board.state == GameState::Playing {
            if self.last_solver_step.elapsed() >= Duration::from_millis(self.solver_speed_ms) {
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

        // ── Pre-compute values shared with popup viewports ───────────────────
        let can_step = self.board.state == GameState::Playing;
        let auto_play_active = self.solver_auto_play;
        let current_rule = self.solver.state.current_rule.clone();
        let highlight_count = self.solver.state.highlighted_cells.len();

        // Deferred actions from popup windows (avoids simultaneous mut borrows).
        let mut do_step = false;
        let mut do_toggle_auto = false;
        let mut solver_closed = false;
        let mut history_closed = false;
        let mut clear_history = false;
        let mut jump_to_bottom = false;

        // ── Solver popup window (separate OS window) ─────────────────────────
        if self.show_solver_panel {
            let speed_ms = &mut self.solver_speed_ms;
            let prob_mode = &mut self.probability_mode;
            let settings = &mut self.solver.settings;

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("solver_panel"),
                egui::ViewportBuilder::default()
                    .with_title("Auto-Solver")
                    .with_resizable(false)
                    .with_inner_size([300.0_f32, 380.0]),
                |ctx, _class| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        solver_closed = true;
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label(
                            egui::RichText::new(format!("Rule: {current_rule}"))
                                .italics()
                                .weak(),
                        );
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(can_step, egui::Button::new("⏭ Step"))
                                .clicked()
                            {
                                do_step = true;
                            }
                            let auto_label = if auto_play_active {
                                "⏸ Pause"
                            } else {
                                "▶ Auto-Play"
                            };
                            if ui
                                .add_enabled(can_step, egui::Button::new(auto_label))
                                .clicked()
                            {
                                do_toggle_auto = true;
                            }
                        });

                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Speed:");
                            let mut spd = *speed_ms as f32;
                            ui.add(
                                egui::Slider::new(&mut spd, 50.0..=2000.0)
                                    .suffix(" ms")
                                    .logarithmic(true),
                            );
                            *speed_ms = spd as u64;
                        });

                        ui.separator();

                        ui.label("Logic Tiers:");
                        ui.checkbox(&mut settings.use_standard, "Rule 1: Standard Deduction");
                        ui.checkbox(&mut settings.use_subset, "Rule 2: Subset Patterns");
                        ui.checkbox(&mut settings.use_csp, "Rule 3: Constraint Satisfaction");
                        ui.checkbox(
                            &mut settings.use_probability,
                            "Rule 4: Probability/Heuristic",
                        );

                        ui.separator();

                        ui.label("Probabilities:");
                        ui.radio_value(prob_mode, ProbabilityMode::Off, "Off");
                        ui.radio_value(prob_mode, ProbabilityMode::WhenInUse, "Show when in use");
                        ui.radio_value(prob_mode, ProbabilityMode::Always, "Always show");

                        if highlight_count > 0 {
                            ui.separator();
                            ui.label(
                                egui::RichText::new(format!("Evaluating {highlight_count} cells"))
                                    .color(egui::Color32::YELLOW),
                            );
                        }
                    });
                },
            );

            if solver_closed {
                self.show_solver_panel = false;
            }
        }

        // ── History popup window (separate OS window) ────────────────────────
        if self.show_history_panel {
            let history_slice: &[String] = &self.action_history;
            let total = history_slice.len();

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("history_panel"),
                egui::ViewportBuilder::default()
                    .with_title("Solver History")
                    .with_inner_size([320.0_f32, 340.0]),
                |ctx, _class| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        history_closed = true;
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{total} moves"));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("Clear").clicked() {
                                        clear_history = true;
                                    }
                                    if total > 0 && ui.small_button("⬇").clicked() {
                                        jump_to_bottom = true;
                                    }
                                },
                            );
                        });
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for (i, entry) in history_slice.iter().enumerate() {
                                    let color = if entry.starts_with("Flag") {
                                        egui::Color32::from_rgb(255, 140, 0)
                                    } else {
                                        egui::Color32::LIGHT_GREEN
                                    };
                                    let response = ui.label(
                                        egui::RichText::new(format!("{:>4}. {entry}", i + 1))
                                            .color(color)
                                            .monospace()
                                            .size(11.0),
                                    );
                                    if jump_to_bottom && i == total - 1 {
                                        response.scroll_to_me(Some(egui::Align::BOTTOM));
                                    }
                                }
                            });
                    });
                },
            );

            if history_closed {
                self.show_history_panel = false;
            }
        }

        // ── Apply deferred actions ───────────────────────────────────────────
        if clear_history {
            self.action_history.clear();
        }
        if do_toggle_auto {
            self.solver_auto_play = !self.solver_auto_play;
            if self.solver_auto_play {
                self.last_solver_step = Instant::now();
            }
        }
        if do_step && can_step {
            let action = self.solver.get_next_move(&self.board);
            if let Some(desc) = apply_action(&mut self.board, &action) {
                let rule = self.solver.state.current_rule.clone();
                self.action_history.push(format!("{desc}  [{rule}]"));
            }
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
                            if let Some(new_board) = Board::new(
                                self.board.width,
                                self.board.height,
                                self.board.num_mines,
                            ) {
                                self.board = new_board;
                            }
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
                            .range(5..=50)
                            .speed(0.1),
                    );
                    ui.label("x");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg_height)
                            .range(5..=50)
                            .speed(0.1),
                    );
                    ui.separator();
                    ui.label("Mines:");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg_mines)
                            .range(1..=(self.cfg_width * self.cfg_height - 9))
                            .speed(0.1),
                    );

                    if ui.button("New Game").clicked()
                        && let Some(new_board) =
                            Board::new(self.cfg_width, self.cfg_height, self.cfg_mines)
                    {
                        self.board = new_board;
                        self.solver.state.clear();
                        self.solver_auto_play = false;
                        self.action_history.clear();
                    }

                    if ui.button("Solve New Game").clicked()
                        && let Some(new_board) =
                            Board::new(self.cfg_width, self.cfg_height, self.cfg_mines)
                    {
                        self.board = new_board;
                        self.solver.state.clear();
                        self.solver_auto_play = true;
                        self.last_solver_step = Instant::now();
                        self.action_history.clear();
                    }
                });
            });
        });

        // ── Resolve probability map for this frame ────────────────────────────
        let probabilities: HashMap<(usize, usize), f32> = if self.board.state != GameState::Playing
        {
            HashMap::new()
        } else {
            match self.probability_mode {
                ProbabilityMode::Off => HashMap::new(),
                ProbabilityMode::WhenInUse => self.solver.state.probabilities.clone(),
                ProbabilityMode::Always => compute_probabilities(&self.board),
            }
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
                                let Some(cell) = self.board.get_cell(x, y) else {
                                    continue;
                                };

                                const NUMBER_STRINGS: [&str; 9] = [
                                    "   ", " 1 ", " 2 ", " 3 ", " 4 ", " 5 ", " 6 ", " 7 ", " 8 ",
                                ];
                                let (text, text_color) = match cell.state {
                                    CellState::Hidden => ("   ", egui::Color32::from_gray(200)),
                                    CellState::Flagged => (" 🚩", egui::Color32::from_gray(200)),
                                    CellState::Revealed => {
                                        if cell.is_mine {
                                            (" 💣", egui::Color32::RED)
                                        } else if cell.adjacent_mines > 0 {
                                            let idx = (cell.adjacent_mines as usize).min(8);
                                            (NUMBER_STRINGS[idx], get_color(cell.adjacent_mines))
                                        } else {
                                            ("   ", egui::Color32::from_gray(60))
                                        }
                                    }
                                };

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

                                // Probability overlay
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

                                // Click handlers
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
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([200.0, 200.0])
            // Enable multi-viewport so child windows can pop out.
            .with_app_id("rusty-mines"),
        ..Default::default()
    };

    eframe::run_native(
        "Rusty Mines",
        options,
        Box::new(|_cc| Ok(Box::new(MinesweeperApp::default()))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Color32;

    #[test]
    fn test_get_color() {
        assert_eq!(get_color(1), Color32::LIGHT_BLUE);
        assert_eq!(get_color(2), Color32::LIGHT_GREEN);
        assert_eq!(get_color(3), Color32::LIGHT_RED);
        assert_eq!(get_color(4), Color32::from_rgb(0, 0, 139));
        assert_eq!(get_color(5), Color32::from_rgb(139, 0, 0));
        assert_eq!(get_color(6), Color32::from_rgb(0, 139, 139));
        assert_eq!(get_color(7), Color32::BLACK);
        assert_eq!(get_color(8), Color32::GRAY);

        // Edge cases and fallbacks
        assert_eq!(get_color(0), Color32::WHITE);
        assert_eq!(get_color(9), Color32::WHITE);
        assert_eq!(get_color(100), Color32::WHITE);
        assert_eq!(get_color(255), Color32::WHITE);
    }
}
