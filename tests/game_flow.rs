use rusty_mines::minesweeper::{Board, CellState, GameState};

#[test]
fn test_complex_game_flow() {
    // 1. Initialize a small 5x5 board with 5 mines
    let mut board = Board::new(5, 5, 5);
    assert_eq!(board.state, GameState::Playing);

    // 2. Perform first click at (0,0).
    // This should trigger mine generation and ensure (0,0) and surroundings are safe.
    board.reveal(0, 0);
    assert!(!board.first_click);

    // Check that (0,0) is revealed and is not a mine
    let cell = board.get_cell(0, 0).unwrap();
    assert_eq!(cell.state, CellState::Revealed);
    assert!(!cell.is_mine);

    // 3. Toggle a flag at (4,4)
    board.toggle_flag(4, 4);
    assert_eq!(board.get_cell(4, 4).unwrap().state, CellState::Flagged);

    // 4. Try to reveal the flagged cell (should do nothing)
    board.reveal(4, 4);
    assert_eq!(board.get_cell(4, 4).unwrap().state, CellState::Flagged);

    // 5. Unflag it
    board.toggle_flag(4, 4);
    assert_eq!(board.get_cell(4, 4).unwrap().state, CellState::Hidden);
}

#[test]
fn test_win_condition_no_mines() {
    // Create a board with 0 mines. Revealing any cell should trigger a win if it clears the board.
    let mut board = Board::new(3, 3, 0);
    board.reveal(1, 1);

    // Since there are no mines, the first click reveals (1,1) and flood-fills the rest.
    assert_eq!(board.state, GameState::Won);
}
