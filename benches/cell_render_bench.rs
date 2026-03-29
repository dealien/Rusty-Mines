use criterion::{Criterion, black_box, criterion_group, criterion_main};

// We will replicate the logic inside `src/main.rs:140` to benchmark it standalone
// For simplicity we create a mock version of the `Cell` struct.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CellState {
    Hidden,
    Flagged,
    Revealed,
}

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub is_mine: bool,
    pub adjacent_mines: u8,
    pub state: CellState,
}

fn get_color(mines: u8) -> u32 {
    // Just mock a color for benchmark
    mines as u32
}

fn render_cell_old(cell: &Cell) -> (String, u32) {
    match cell.state {
        CellState::Hidden => ("   ".to_string(), 200),
        CellState::Flagged => (" 🚩".to_string(), 200),
        CellState::Revealed => {
            if cell.is_mine {
                (" 💣".to_string(), 255) // RED
            } else if cell.adjacent_mines > 0 {
                (
                    format!(" {} ", cell.adjacent_mines),
                    get_color(cell.adjacent_mines),
                )
            } else {
                ("   ".to_string(), 60)
            }
        }
    }
}

fn render_cell_new(cell: &Cell) -> (&'static str, u32) {
    const NUMBER_STRINGS: [&str; 9] = [
        "   ", " 1 ", " 2 ", " 3 ", " 4 ", " 5 ", " 6 ", " 7 ", " 8 ",
    ];
    match cell.state {
        CellState::Hidden => ("   ", 200),
        CellState::Flagged => (" 🚩", 200),
        CellState::Revealed => {
            if cell.is_mine {
                (" 💣", 255) // RED
            } else if cell.adjacent_mines > 0 {
                let idx = (cell.adjacent_mines as usize).min(8);
                (NUMBER_STRINGS[idx], get_color(cell.adjacent_mines))
            } else {
                ("   ", 60)
            }
        }
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let cells = vec![
        Cell {
            is_mine: false,
            adjacent_mines: 0,
            state: CellState::Hidden,
        },
        Cell {
            is_mine: false,
            adjacent_mines: 0,
            state: CellState::Flagged,
        },
        Cell {
            is_mine: true,
            adjacent_mines: 0,
            state: CellState::Revealed,
        },
        Cell {
            is_mine: false,
            adjacent_mines: 0,
            state: CellState::Revealed,
        },
        Cell {
            is_mine: false,
            adjacent_mines: 1,
            state: CellState::Revealed,
        },
        Cell {
            is_mine: false,
            adjacent_mines: 2,
            state: CellState::Revealed,
        },
        Cell {
            is_mine: false,
            adjacent_mines: 3,
            state: CellState::Revealed,
        },
        Cell {
            is_mine: false,
            adjacent_mines: 8,
            state: CellState::Revealed,
        },
    ];

    c.bench_function("render_cell_old", |b| {
        b.iter(|| {
            for cell in &cells {
                black_box(render_cell_old(black_box(cell)));
            }
        });
    });

    c.bench_function("render_cell_new", |b| {
        b.iter(|| {
            for cell in &cells {
                black_box(render_cell_new(black_box(cell)));
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
