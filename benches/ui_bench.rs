use divan::{Bencher, black_box};
use rusty_mines::minesweeper::Board;
use rusty_mines::solver::SolverAction;
use rusty_mines::ui_helpers::{apply_action, compute_probabilities, get_color, probability_color};

// -----------------------------------------------------------------------------
// Helper function to create deterministic boards for repeatable benchmarks
// -----------------------------------------------------------------------------
fn generate_deterministic_board(
    width: usize,
    height: usize,
    num_mines: usize,
    reveal_x: usize,
    reveal_y: usize,
) -> Board {
    let mut board = Board::new(width, height, num_mines).unwrap();
    let mut mines_placed = 0;

    // Simple deterministic pattern: place mines sequentially skipping some cells
    // but avoid placing anything at the reveal coordinates and its neighbors
    for y in 0..height {
        for x in 0..width {
            if mines_placed >= num_mines {
                break;
            }
            if (x as isize - reveal_x as isize).abs() <= 1
                && (y as isize - reveal_y as isize).abs() <= 1
            {
                continue;
            }

            // Distribute mines somewhat evenly based on target density
            let density = num_mines as f64 / (width * height) as f64;
            let step = (1.0 / density).max(1.0) as usize;

            if (x + y * width) % step == 0 {
                let idx = board.index(x, y);
                if !board.cells[idx].is_mine {
                    board.cells[idx].is_mine = true;
                    mines_placed += 1;
                }
            }
        }
    }

    // If we missed the target because of the step, fill the rest deterministically
    let mut i = 0;
    while mines_placed < num_mines {
        let x = i % width;
        let y = i / width;
        if (x as isize - reveal_x as isize).abs() > 1 || (y as isize - reveal_y as isize).abs() > 1
        {
            let idx = board.index(x, y);
            if !board.cells[idx].is_mine {
                board.cells[idx].is_mine = true;
                mines_placed += 1;
            }
        }
        i += 1;
    }

    // Implement calculate_adjacent_mines here:
    for y in 0..height {
        for x in 0..width {
            if board.cells[board.index(x, y)].is_mine {
                continue;
            }
            let mut count = 0;
            for (nx, ny) in board.adjacent_cells(x, y) {
                if board.cells[board.index(nx, ny)].is_mine {
                    count += 1;
                }
            }
            let idx = board.index(x, y);
            board.cells[idx].adjacent_mines = count;
        }
    }

    board.first_click = false;
    board.unrevealed_safe_cells = (width * height) - num_mines;

    // Reveal a starting cell to set up the board state
    board.reveal(reveal_x, reveal_y);
    board
}

fn generate_beginner_board() -> Board {
    generate_deterministic_board(9, 9, 10, 4, 4)
}

fn generate_mid_size_board() -> Board {
    generate_deterministic_board(16, 16, 40, 8, 8)
}

fn generate_expert_board() -> Board {
    generate_deterministic_board(30, 16, 99, 15, 8)
}

// -----------------------------------------------------------------------------
// Benchmarks
// -----------------------------------------------------------------------------

#[divan::bench]
fn bench_compute_probabilities_beginner(bencher: Bencher) {
    bencher
        .with_inputs(generate_beginner_board)
        .bench_local_values(|board| {
            black_box(compute_probabilities(black_box(&board)));
        });
}

#[divan::bench]
fn bench_compute_probabilities_mid(bencher: Bencher) {
    bencher
        .with_inputs(generate_mid_size_board)
        .bench_local_values(|board| {
            black_box(compute_probabilities(black_box(&board)));
        });
}

#[divan::bench]
fn bench_compute_probabilities_expert(bencher: Bencher) {
    bencher
        .with_inputs(generate_expert_board)
        .bench_local_values(|board| {
            black_box(compute_probabilities(black_box(&board)));
        });
}

#[divan::bench]
fn bench_get_color(bencher: Bencher) {
    bencher.bench_local(|| {
        for i in 0..=9 {
            black_box(get_color(black_box(i)));
        }
    });
}

#[divan::bench]
fn bench_probability_color(bencher: Bencher) {
    bencher.bench_local(|| {
        for i in 0..=10 {
            let prob = i as f32 / 10.0;
            black_box(probability_color(black_box(prob)));
        }
    });
}

#[divan::bench]
fn bench_apply_action(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let board = generate_beginner_board();
            let mut actions = Vec::new();
            actions.push(SolverAction::Reveal(0, 0));
            actions.push(SolverAction::Flag(1, 1));
            actions.push(SolverAction::None);
            (board, actions)
        })
        .bench_local_values(|(mut board, actions)| {
            for action in actions {
                black_box(apply_action(&mut board, &action));
            }
        });
}

fn main() {
    divan::main();
}
