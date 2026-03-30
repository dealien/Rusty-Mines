use divan::{Bencher, black_box};
use rusty_mines::minesweeper::Board;

// -----------------------------------------------------------------------------
// Helper function to create deterministic boards for repeatable benchmarks
// -----------------------------------------------------------------------------
fn generate_deterministic_board(width: usize, height: usize, num_mines: usize) -> Board {
    let mut board = Board::new(width, height, num_mines).unwrap();
    let mut mines_placed = 0;

    // Simple deterministic pattern: place mines sequentially skipping some cells
    // but avoid placing anything at the top-left (0,0) and its neighbors
    // so we can test reveals starting there safely.
    for y in 0..height {
        for x in 0..width {
            if mines_placed >= num_mines {
                break;
            }
            if x <= 1 && y <= 1 {
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
        if x > 1 || y > 1 {
            let idx = board.index(x, y);
            if !board.cells[idx].is_mine {
                board.cells[idx].is_mine = true;
                mines_placed += 1;
            }
        }
        i += 1;
    }

    // Implement calculate_adjacent_mines here since it is private:
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

    board
}

// -----------------------------------------------------------------------------
// Board::new benchmarks
// -----------------------------------------------------------------------------
#[divan::bench]
fn bench_board_new_beginner(bencher: Bencher) {
    bencher.bench_local(|| {
        black_box(Board::new(black_box(9), black_box(9), black_box(10)));
    });
}

#[divan::bench]
fn bench_board_new_intermediate(bencher: Bencher) {
    bencher.bench_local(|| {
        black_box(Board::new(black_box(16), black_box(16), black_box(40)));
    });
}

#[divan::bench]
fn bench_board_new_expert(bencher: Bencher) {
    bencher.bench_local(|| {
        black_box(Board::new(black_box(30), black_box(16), black_box(99)));
    });
}

#[divan::bench]
fn bench_board_new_50x50_low_density(bencher: Bencher) {
    bencher.bench_local(|| {
        black_box(Board::new(black_box(50), black_box(50), black_box(10)));
    });
}

#[divan::bench]
fn bench_board_new_50x50_high_density(bencher: Bencher) {
    bencher.bench_local(|| {
        black_box(Board::new(black_box(50), black_box(50), black_box(900)));
    });
}

// -----------------------------------------------------------------------------
// Board::reveal (cascading / flood fill) benchmarks
// -----------------------------------------------------------------------------

#[divan::bench]
fn bench_reveal_cascade_beginner(bencher: Bencher) {
    bencher
        .with_inputs(|| generate_deterministic_board(9, 9, 10))
        .bench_local_values(|mut board| {
            board.reveal(0, 0);
            black_box(board);
        });
}

#[divan::bench]
fn bench_reveal_cascade_intermediate(bencher: Bencher) {
    bencher
        .with_inputs(|| generate_deterministic_board(16, 16, 40))
        .bench_local_values(|mut board| {
            board.reveal(0, 0);
            black_box(board);
        });
}

#[divan::bench]
fn bench_reveal_cascade_expert(bencher: Bencher) {
    bencher
        .with_inputs(|| generate_deterministic_board(30, 16, 99))
        .bench_local_values(|mut board| {
            board.reveal(0, 0);
            black_box(board);
        });
}

#[divan::bench]
fn bench_reveal_cascade_50x50_low_density(bencher: Bencher) {
    bencher
        .with_inputs(|| generate_deterministic_board(50, 50, 10))
        .bench_local_values(|mut board| {
            board.reveal(0, 0);
            black_box(board);
        });
}

#[divan::bench]
fn bench_reveal_cascade_50x50_high_density(bencher: Bencher) {
    bencher
        .with_inputs(|| generate_deterministic_board(50, 50, 900))
        .bench_local_values(|mut board| {
            board.reveal(0, 0);
            black_box(board);
        });
}

// -----------------------------------------------------------------------------
// Adjacent Cells iteration benchmarks
// -----------------------------------------------------------------------------

#[divan::bench]
fn bench_adjacent_cells_iteration(bencher: Bencher) {
    // Generate an intermediate board, then benchmark how long it takes to iterate
    // over adjacent cells for all cells on the board.
    let board = Board::new(16, 16, 40).unwrap();
    bencher.bench_local(|| {
        for y in 0..board.height {
            for x in 0..board.width {
                for (nx, ny) in board.adjacent_cells(x, y) {
                    black_box((nx, ny));
                }
            }
        }
    });
}

fn main() {
    divan::main();
}
