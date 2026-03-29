use criterion::{Criterion, black_box, criterion_group, criterion_main};
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

fn bench_solver(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver");

    let mid_board = generate_mid_size_board();
    let large_dense_board = generate_large_high_density_board();
    let empty_board = generate_empty_board();
    let large_low_density_board = generate_large_low_density_board();

    // Standard Deduction
    group.bench_function("mid_board_standard_deduction", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            solver.settings.use_probability = false;
            black_box(solver.get_next_move(black_box(&mid_board)));
        })
    });

    group.bench_function("large_dense_standard_deduction", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            solver.settings.use_probability = false;
            black_box(solver.get_next_move(black_box(&large_dense_board)));
        })
    });

    // Pattern Matching
    group.bench_function("mid_board_pattern_matching", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_csp = false;
            solver.settings.use_probability = false;
            black_box(solver.get_next_move(black_box(&mid_board)));
        })
    });

    group.bench_function("large_dense_pattern_matching", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_csp = false;
            solver.settings.use_probability = false;
            black_box(solver.get_next_move(black_box(&large_dense_board)));
        })
    });

    // CSP Deduction
    group.bench_function("mid_board_csp_deduction", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_probability = false;
            black_box(solver.get_next_move(black_box(&mid_board)));
        })
    });

    group.bench_function("large_dense_csp_deduction", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_probability = false;
            black_box(solver.get_next_move(black_box(&large_dense_board)));
        })
    });

    // Probability Guess
    group.bench_function("mid_board_probability_guess", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            black_box(solver.get_next_move(black_box(&mid_board)));
        })
    });

    group.bench_function("large_dense_probability_guess", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            black_box(solver.get_next_move(black_box(&large_dense_board)));
        })
    });

    group.bench_function("empty_board_probability_guess", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            black_box(solver.get_next_move(black_box(&empty_board)));
        })
    });

    group.bench_function("large_low_density_probability_guess", |b| {
        b.iter(|| {
            let mut solver = Solver::new();
            solver.settings.use_standard = false;
            solver.settings.use_subset = false;
            solver.settings.use_csp = false;
            black_box(solver.get_next_move(black_box(&large_low_density_board)));
        })
    });

    group.finish();
}

criterion_group!(benches, bench_solver);
criterion_main!(benches);
