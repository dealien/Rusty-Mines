#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use rusty_mines::minesweeper::{Board, CellState, GameState};

struct MinesweeperApp {
    board: Board,
    // Configuration for the next game
    cfg_width: usize,
    cfg_height: usize,
    cfg_mines: usize,
    // Track last board size to trigger resize
    last_board_size: (usize, usize),
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
            last_board_size: (0, 0), // Force resize on first frame
        }
    }
}

impl eframe::App for MinesweeperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if we need to resize the window
        if self.last_board_size != (self.board.width, self.board.height) {
            let cell_size = 30.0;
            let spacing = 2.0;
            // Estimated heights and widths
            let top_panel_height = 70.0; // Height of the two rows + separators
            let horizontal_padding = 24.0; // Padding on sides of the grid
            let vertical_padding = 20.0; // Padding on top/bottom of the grid

            let grid_width =
                self.board.width as f32 * cell_size + (self.board.width - 1) as f32 * spacing;
            // The top panel contains "Size: 10x10 Mines: 15 New Game" which is roughly 350px wide
            let top_panel_min_width = 350.0;

            let desired_width = (grid_width + horizontal_padding).max(top_panel_min_width);
            let desired_height = self.board.height as f32 * cell_size
                + (self.board.height - 1) as f32 * spacing
                + top_panel_height
                + vertical_padding;

            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                desired_width,
                desired_height,
            )));
            self.last_board_size = (self.board.width, self.board.height);
        }

        // Top Panel for game state / controls
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
                        }
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
                    }
                });
            });
        });

        // Central Panel for the grid
        egui::CentralPanel::default().show(ctx, |ui| {
            // Center the grid within the panel
            ui.vertical_centered(|ui| {
                ui.add_space(10.0); // A bit of padding on top

                egui::Grid::new("minesweeper_grid")
                    .spacing([2.0, 2.0])
                    .min_col_width(30.0)
                    .min_row_height(30.0)
                    .show(ui, |ui| {
                        for y in 0..self.board.height {
                            for x in 0..self.board.width {
                                let cell = self.board.get_cell(x, y).unwrap();
                                let (text, color) = match cell.state {
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

                                let fill_color =
                                    if cell.state == CellState::Revealed && !cell.is_mine {
                                        egui::Color32::from_gray(30)
                                    } else {
                                        egui::Color32::from_gray(80)
                                    };

                                let button = egui::Button::new(
                                    egui::RichText::new(text).color(color).strong(),
                                )
                                .min_size(egui::vec2(30.0, 30.0))
                                .fill(fill_color);

                                let response = ui.add(button);

                                if self.board.state == GameState::Playing {
                                    if response.clicked() {
                                        self.board.reveal(x, y);
                                    } else if response.secondary_clicked() {
                                        self.board.toggle_flag(x, y);
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

fn get_color(mines: u8) -> egui::Color32 {
    match mines {
        1 => egui::Color32::LIGHT_BLUE,
        2 => egui::Color32::LIGHT_GREEN,
        3 => egui::Color32::LIGHT_RED,
        4 => egui::Color32::from_rgb(0, 0, 139), // Dark blue
        5 => egui::Color32::from_rgb(139, 0, 0), // Dark red
        6 => egui::Color32::from_rgb(0, 139, 139), // Cyan
        7 => egui::Color32::BLACK,
        8 => egui::Color32::GRAY,
        _ => egui::Color32::WHITE,
    }
}

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
