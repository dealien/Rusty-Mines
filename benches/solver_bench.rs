use divan::{Bencher, black_box};
use rusty_mines::minesweeper::Board;
use rusty_mines::solver::Solver;

// -----------------------------------------------------------------------------
// Helper function to create deterministic boards for repeatable benchmarks
// -----------------------------------------------------------------------------
fn generate_deterministic_board(
    width: usize,
    height: usize,
    mut num_mines: usize,
    reveal_x: usize,
    reveal_y: usize,
) -> Board {
    let size = width * height;

    // Calculate the actual size of the protected region around the reveal point
    let mut protected_count = 0;
    for y in 0..height {
        for x in 0..width {
            if (x as isize - reveal_x as isize).abs() <= 1
                && (y as isize - reveal_y as isize).abs() <= 1
            {
                protected_count += 1;
            }
        }
    }

    let max_mines = size.saturating_sub(protected_count);
    if num_mines > max_mines {
        num_mines = max_mines;
    }

    let mut board = Board::new(width, height, num_mines).unwrap();
    board.num_mines = num_mines;

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
            let density = num_mines as f64 / size as f64;
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
    while mines_placed < num_mines && i < size {
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

    assert_eq!(mines_placed, num_mines, "Failed to place all deterministic mines.");

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

    // Reveal a starting cell to set up the board state for solver
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

fn generate_large_high_density_board() -> Board {
    generate_deterministic_board(50, 50, 900, 25, 25)
}

fn generate_empty_board() -> Board {
    generate_deterministic_board(50, 50, 0, 25, 25)
}

fn generate_large_low_density_board() -> Board {
    generate_deterministic_board(50, 50, 10, 25, 25)
}

macro_rules! bench_solver_method {
    ($name:ident, $board_generator:ident, $standard:expr, $subset:expr, $csp:expr, $probability:expr) => {
        #[divan::bench]
        fn $name(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let mut solver = Solver::new();
                    solver.settings.use_standard = $standard;
                    solver.settings.use_subset = $subset;
                    solver.settings.use_csp = $csp;
                    solver.settings.use_probability = $probability;
                    (solver, $board_generator())
                })
                .bench_local_values(|(mut solver, board)| {
                    black_box(solver.get_next_move(black_box(&board)));
                });
        }
    };
}

// -----------------------------------------------------------------------------
// Standard Deduction Benchmarks
// -----------------------------------------------------------------------------
bench_solver_method!(
    bench_standard_beginner,
    generate_beginner_board,
    true,
    false,
    false,
    false
);
bench_solver_method!(
    bench_standard_mid,
    generate_mid_size_board,
    true,
    false,
    false,
    false
);
bench_solver_method!(
    bench_standard_expert,
    generate_expert_board,
    true,
    false,
    false,
    false
);
bench_solver_method!(
    bench_standard_large_dense,
    generate_large_high_density_board,
    true,
    false,
    false,
    false
);
bench_solver_method!(
    bench_standard_large_empty,
    generate_empty_board,
    true,
    false,
    false,
    false
);
bench_solver_method!(
    bench_standard_large_sparse,
    generate_large_low_density_board,
    true,
    false,
    false,
    false
);

// -----------------------------------------------------------------------------
// Pattern Matching (Subset) Benchmarks
// -----------------------------------------------------------------------------
bench_solver_method!(
    bench_pattern_beginner,
    generate_beginner_board,
    false,
    true,
    false,
    false
);
bench_solver_method!(
    bench_pattern_mid,
    generate_mid_size_board,
    false,
    true,
    false,
    false
);
bench_solver_method!(
    bench_pattern_expert,
    generate_expert_board,
    false,
    true,
    false,
    false
);
bench_solver_method!(
    bench_pattern_large_dense,
    generate_large_high_density_board,
    false,
    true,
    false,
    false
);
bench_solver_method!(
    bench_pattern_large_empty,
    generate_empty_board,
    false,
    true,
    false,
    false
);
bench_solver_method!(
    bench_pattern_large_sparse,
    generate_large_low_density_board,
    false,
    true,
    false,
    false
);

// -----------------------------------------------------------------------------
// CSP Deduction Benchmarks
// -----------------------------------------------------------------------------
bench_solver_method!(
    bench_csp_beginner,
    generate_beginner_board,
    false,
    false,
    true,
    false
);
bench_solver_method!(
    bench_csp_mid,
    generate_mid_size_board,
    false,
    false,
    true,
    false
);
bench_solver_method!(
    bench_csp_expert,
    generate_expert_board,
    false,
    false,
    true,
    false
);
bench_solver_method!(
    bench_csp_large_dense,
    generate_large_high_density_board,
    false,
    false,
    true,
    false
);
bench_solver_method!(
    bench_csp_large_empty,
    generate_empty_board,
    false,
    false,
    true,
    false
);
bench_solver_method!(
    bench_csp_large_sparse,
    generate_large_low_density_board,
    false,
    false,
    true,
    false
);

// -----------------------------------------------------------------------------
// Probability Guess Benchmarks
// -----------------------------------------------------------------------------
bench_solver_method!(
    bench_probability_beginner,
    generate_beginner_board,
    false,
    false,
    false,
    true
);
bench_solver_method!(
    bench_probability_mid,
    generate_mid_size_board,
    false,
    false,
    false,
    true
);
bench_solver_method!(
    bench_probability_expert,
    generate_expert_board,
    false,
    false,
    false,
    true
);
bench_solver_method!(
    bench_probability_large_dense,
    generate_large_high_density_board,
    false,
    false,
    false,
    true
);
bench_solver_method!(
    bench_probability_large_empty,
    generate_empty_board,
    false,
    false,
    false,
    true
);
bench_solver_method!(
    bench_probability_large_sparse,
    generate_large_low_density_board,
    false,
    false,
    false,
    true
);

fn main() {
    divan::main();
}
