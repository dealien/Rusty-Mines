use rand::Rng;

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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameState {
    Playing,
    Won,
    Lost,
}

pub const MAX_WIDTH: usize = 50;
pub const MAX_HEIGHT: usize = 50;

pub struct Board {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
    pub num_mines: usize,
    pub unrevealed_safe_cells: usize,
    pub state: GameState,
    pub first_click: bool,
}

impl Board {
    /// Creates a new Minesweeper board.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_mines::minesweeper::Board;
    /// let board = Board::new(10, 10, 15).unwrap();
    /// assert_eq!(board.width, 10);
    /// assert_eq!(board.num_mines, 15);
    /// ```
    pub fn new(width: usize, height: usize, num_mines: usize) -> Option<Self> {
        if width == 0 || height == 0 || width > MAX_WIDTH || height > MAX_HEIGHT {
            return None;
        }

        let size = width.checked_mul(height)?;
        if num_mines >= size {
            return None;
        }

        let cells = vec![
            Cell {
                is_mine: false,
                adjacent_mines: 0,
                state: CellState::Hidden,
            };
            size
        ];

        let unrevealed_safe_cells = size.saturating_sub(num_mines);

        Some(Self {
            width,
            height,
            cells,
            num_mines,
            unrevealed_safe_cells,
            state: GameState::Playing,
            first_click: true,
        })
    }

    // Getting the 1D index from 2D coordinates
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Returns an iterator over the valid adjacent coordinates for a given cell.
    pub fn adjacent_cells(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> {
        let width = self.width as isize;
        let height = self.height as isize;
        let x = x as isize;
        let y = y as isize;

        (-1..=1).flat_map(move |dy| {
            (-1..=1).filter_map(move |dx| {
                if dx == 0 && dy == 0 {
                    return None;
                }
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0 && nx < width && ny >= 0 && ny < height {
                    Some((nx as usize, ny as usize))
                } else {
                    None
                }
            })
        })
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<&Cell> {
        if x < self.width && y < self.height {
            Some(&self.cells[self.index(x, y)])
        } else {
            None
        }
    }

    /// Returns an iterator over adjacent coordinates matching a specific state.
    pub fn adjacent_cells_with_state(
        &self,
        x: usize,
        y: usize,
        state: CellState,
    ) -> impl Iterator<Item = (usize, usize)> {
        self.adjacent_cells(x, y)
            .filter(move |&(nx, ny)| self.get_cell(nx, ny).is_some_and(|c| c.state == state))
    }

    /// Returns the number of adjacent cells matching a specific state.
    pub fn count_adjacent_with_state(&self, x: usize, y: usize, state: CellState) -> usize {
        self.adjacent_cells_with_state(x, y, state).count()
    }

    // Place mines randomly, ensuring the first clicked cell and its surroundings are safe
    fn place_mines_after_first_click(&mut self, first_x: usize, first_y: usize) {
        let mut rng = rand::thread_rng();
        let mut mines_placed = 0;

        // Make sure we do not place more mines than available cells minus the protected 3x3 area
        let protected_cells = 9.min(self.width * self.height);
        let max_mines = (self.width * self.height).saturating_sub(protected_cells);
        let actual_mines = self.num_mines.min(max_mines);

        while mines_placed < actual_mines {
            let x = rng.gen_range(0..self.width);
            let y = rng.gen_range(0..self.height);

            // Ensure first click and its surroundings are not mines
            if (x as isize - first_x as isize).abs() <= 1
                && (y as isize - first_y as isize).abs() <= 1
            {
                continue;
            }

            let idx = self.index(x, y);
            if !self.cells[idx].is_mine {
                self.cells[idx].is_mine = true;
                mines_placed += 1;
            }
        }

        // Update the actual number of mines in case we had to clamp them
        self.num_mines = actual_mines;
        self.unrevealed_safe_cells = (self.width * self.height) - self.num_mines;

        self.calculate_adjacent_mines();
        self.first_click = false;
    }

    fn calculate_adjacent_mines(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[self.index(x, y)].is_mine {
                    continue;
                }

                let mut count = 0;
                for (nx, ny) in self.adjacent_cells(x, y) {
                    let n_idx = self.index(nx, ny);
                    if self.cells[n_idx].is_mine {
                        count += 1;
                    }
                }

                let idx = self.index(x, y);
                self.cells[idx].adjacent_mines = count;
            }
        }
    }

    pub fn reveal(&mut self, x: usize, y: usize) {
        if self.state != GameState::Playing {
            return;
        }

        if x >= self.width || y >= self.height {
            return;
        }

        let idx = self.index(x, y);

        if self.cells[idx].state != CellState::Hidden {
            return; // Can't reveal flagged or already revealed cells
        }

        if self.first_click {
            self.place_mines_after_first_click(x, y);
        }

        self.cells[idx].state = CellState::Revealed;

        if self.cells[idx].is_mine {
            self.state = GameState::Lost;
            self.reveal_all_mines();
            return;
        } else {
            self.unrevealed_safe_cells -= 1;
        }

        if self.cells[idx].adjacent_mines == 0 {
            // Flood fill for empty cells
            let adjacent = self.adjacent_cells(x, y).collect::<Vec<_>>();
            for (nx, ny) in adjacent {
                self.reveal(nx, ny);
            }
        }

        self.check_win();
    }

    pub fn toggle_flag(&mut self, x: usize, y: usize) {
        if self.state != GameState::Playing {
            return;
        }

        if x >= self.width || y >= self.height {
            return;
        }

        let idx = self.index(x, y);

        match self.cells[idx].state {
            CellState::Hidden => {
                self.cells[idx].state = CellState::Flagged;
            }
            CellState::Flagged => {
                self.cells[idx].state = CellState::Hidden;
            }
            CellState::Revealed => {} // Can't flag revealed cells
        }
    }

    fn check_win(&mut self) {
        if self.unrevealed_safe_cells == 0 {
            self.state = GameState::Won;
        }
    }

    fn reveal_all_mines(&mut self) {
        for cell in &mut self.cells {
            if cell.is_mine {
                cell.state = CellState::Revealed;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_initialization() {
        let board = Board::new(10, 10, 15).unwrap();
        assert_eq!(board.width, 10);
        assert_eq!(board.height, 10);
        assert_eq!(board.cells.len(), 100);
        assert_eq!(board.num_mines, 15);
        assert_eq!(board.state, GameState::Playing);
        assert!(board.first_click);
    }

    #[test]
    fn test_first_click_places_mines() {
        let mut board = Board::new(5, 5, 5).unwrap();
        board.reveal(2, 2);
        assert!(!board.first_click);

        // Count placed mines
        let mines = board.cells.iter().filter(|c| c.is_mine).count();
        assert_eq!(mines, 5);

        // Ensure first click area is safe
        assert!(!board.get_cell(2, 2).unwrap().is_mine);
        for (nx, ny) in board.adjacent_cells(2, 2) {
            let cell = board.get_cell(nx, ny).unwrap();
            assert!(!cell.is_mine);
        }
    }

    #[test]
    fn test_flagging() {
        let mut board = Board::new(5, 5, 0).unwrap();
        board.toggle_flag(1, 1);
        assert_eq!(board.get_cell(1, 1).unwrap().state, CellState::Flagged);
        board.toggle_flag(1, 1);
        assert_eq!(board.get_cell(1, 1).unwrap().state, CellState::Hidden);
    }

    #[test]
    fn test_toggle_flag_out_of_bounds() {
        let mut board = Board::new(5, 5, 0).unwrap();
        // Test x out of bounds
        board.toggle_flag(5, 0);
        // Test y out of bounds
        board.toggle_flag(0, 5);
        // Test both out of bounds
        board.toggle_flag(5, 5);

        // Ensure no cell state changed (all should be Hidden)
        for cell in &board.cells {
            assert_eq!(cell.state, CellState::Hidden);
        }
    }

    #[test]
    fn test_invalid_board_parameters() {
        // Zero dimensions
        assert!(Board::new(0, 10, 5).is_none());
        assert!(Board::new(10, 0, 5).is_none());
        assert!(Board::new(0, 0, 0).is_none());

        // Dimensions exceeding MAX_WIDTH/MAX_HEIGHT
        assert!(Board::new(MAX_WIDTH + 1, 10, 5).is_none());
        assert!(Board::new(10, MAX_HEIGHT + 1, 5).is_none());

        // Too many mines (must be less than total cells)
        assert!(Board::new(10, 10, 100).is_none());
        assert!(Board::new(10, 10, 101).is_none());

        // Normal board
        assert!(Board::new(10, 10, 99).is_some());
    }
}
