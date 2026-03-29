use divan::{Bencher, black_box};
use rusty_mines::minesweeper::Board;
use rusty_mines::solver::Solver;

fn generate_mid_size_board() -> Board {
    // Intermediate difficulty: 16x16 with 40 mines
    let mut board = Board::new(16, 16, 40).unwrap();
    // Reveal a starting cell to set up the board state for solver
    board.reveal(8, 8);
    board
}

fn generate_large_high_density_board() -> Board {
    // Extreme edge case: 50x50 with 900 mines
    let mut board = Board::new(50, 50, 900).unwrap();
    // Reveal a starting cell to set up the board state for solver
    board.reveal(25, 25);
    board
}

fn generate_empty_board() -> Board {
    // Extreme edge case: 50x50 with 0 mines
    let mut board = Board::new(50, 50, 0).unwrap();
    // Reveal a starting cell to set up the board state for solver
    board.reveal(25, 25);
    board
}

fn generate_large_low_density_board() -> Board {
    // Extreme edge case: 50x50 with 10 mines
    let mut board = Board::new(50, 50, 10).unwrap();
    // Reveal a starting cell to set up the board state for solver
    board.reveal(25, 25);
    board
}

// Standard Deduction
#[divan::bench]
fn bench_mid_board_standard_deduction(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            solver.settings.use_probability = false;
            (solver, generate_mid_size_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

#[divan::bench]
fn bench_large_dense_standard_deduction(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            solver.settings.use_probability = false;
            (solver, generate_large_high_density_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

// Pattern Matching
#[divan::bench]
fn bench_mid_board_pattern_matching(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_csp = false;
            solver.settings.use_probability = false;
            (solver, generate_mid_size_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

#[divan::bench]
fn bench_large_dense_pattern_matching(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_csp = false;
            solver.settings.use_probability = false;
            (solver, generate_large_high_density_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

// CSP Deduction
#[divan::bench]
fn bench_mid_board_csp_deduction(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_probability = false;
            (solver, generate_mid_size_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

#[divan::bench]
fn bench_large_dense_csp_deduction(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_probability = false;
            (solver, generate_large_high_density_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

// Probability Guess
#[divan::bench]
fn bench_mid_board_probability_guess(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            (solver, generate_mid_size_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

#[divan::bench]
fn bench_large_dense_probability_guess(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            (solver, generate_large_high_density_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

#[divan::bench]
fn bench_empty_board_probability_guess(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            (solver, generate_empty_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

#[divan::bench]
fn bench_large_low_density_probability_guess(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            (solver, generate_large_low_density_board())
        })
        .bench_local_values(|(mut solver, board)| {
            black_box(solver.get_next_move(black_box(&board)));
        });
}

fn main() {
    divan::main();
}
