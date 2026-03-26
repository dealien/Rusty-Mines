#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusty_mines::minesweeper::{Board, CellState, GameState};
use eframe::egui;

struct MinesweeperApp {
    board: Board,
}

impl Default for MinesweeperApp {
    fn default() -> Self {
        Self {
            board: Board::new(10, 10, 15), // Default 10x10 with 15 mines
        }
    }
}

impl eframe::App for MinesweeperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top Panel for game state / controls
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Rusty Mines");
                ui.separator();
                
                let state_text = match self.board.state {
                    GameState::Playing => "Playing ☺️",
                    GameState::Won => "You Won! 😎",
                    GameState::Lost => "Game Over 😵",
                };
                
                ui.label(state_text);
                
                if ui.button("Restart").clicked() {
                    self.board = Board::new(self.board.width, self.board.height, self.board.num_mines);
                }
            });
        });

        // Central Panel for the grid
        egui::CentralPanel::default().show(ctx, |ui| {
            // Center the grid within the panel
            ui.vertical_centered(|ui| {
                ui.add_space(20.0); // A bit of padding on top
                
                egui::Grid::new("minesweeper_grid").spacing([2.0, 2.0]).show(ui, |ui| {
                    for y in 0..self.board.height {
                        for x in 0..self.board.width {
                            let cell = self.board.get_cell(x, y).unwrap();
                            let (text, color) = match cell.state {
                                CellState::Hidden => ("   ".to_string(), egui::Color32::from_gray(200)),
                                CellState::Flagged => (" 🚩".to_string(), egui::Color32::from_gray(200)),
                                CellState::Revealed => {
                                    if cell.is_mine {
                                        (" 💣".to_string(), egui::Color32::RED)
                                    } else if cell.adjacent_mines > 0 {
                                        (format!(" {} ", cell.adjacent_mines), get_color(cell.adjacent_mines))
                                    } else {
                                        ("   ".to_string(), egui::Color32::from_gray(60))
                                    }
                                }
                            };

                            let fill_color = if cell.state == CellState::Revealed && !cell.is_mine {
                                egui::Color32::from_gray(30)
                            } else {
                                egui::Color32::from_gray(80)
                            };

                            let button = egui::Button::new(
                                egui::RichText::new(text).color(color).strong()
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
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 450.0])
            .with_min_inner_size([300.0, 300.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Rusty Mines",
        options,
        Box::new(|_cc| Ok(Box::new(MinesweeperApp::default()))),
    )
}
