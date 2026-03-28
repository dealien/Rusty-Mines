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

pub struct Board {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
    pub num_mines: usize,
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
    /// let board = Board::new(10, 10, 15);
    /// assert_eq!(board.width, 10);
    /// assert_eq!(board.num_mines, 15);
    /// ```
    pub fn new(width: usize, height: usize, num_mines: usize) -> Self {
        let cells = vec![
            Cell {
                is_mine: false,
                adjacent_mines: 0,
                state: CellState::Hidden,
            };
            width * height
        ];

        Self {
            width,
            height,
            cells,
            num_mines,
            state: GameState::Playing,
            first_click: true,
        }
    }

    // Getting the 1D index from 2D coordinates
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<&Cell> {
        if x < self.width && y < self.height {
            Some(&self.cells[self.index(x, y)])
        } else {
            None
        }
    }

    // Place mines efficiently, ensuring the first clicked cell and its surroundings are safe.
    // This avoids the Coupon Collector's Problem at high mine densities by picking from available cells.
    fn place_mines_after_first_click(&mut self, first_x: usize, first_y: usize) {
        let mut rng = rand::thread_rng();

        // Make sure we do not place more mines than available cells minus the protected 3x3 area
        let protected_cells = 9.min(self.width * self.height);
        let max_mines = (self.width * self.height).saturating_sub(protected_cells);
        let actual_mines = self.num_mines.min(max_mines);

        if actual_mines > 0 {
            // Collect all valid indices that can be mined
            let mut available_indices =
                Vec::with_capacity(self.width * self.height - protected_cells);
            for y in 0..self.height {
                for x in 0..self.width {
                    // Ensure first click and its surroundings are not mines
                    if (x as isize - first_x as isize).abs() <= 1
                        && (y as isize - first_y as isize).abs() <= 1
                    {
                        continue;
                    }
                    available_indices.push(self.index(x, y));
                }
            }

            // Sample indices uniformly
            let selected =
                rand::seq::index::sample(&mut rng, available_indices.len(), actual_mines);
            for idx in selected.into_iter() {
                let board_idx = available_indices[idx];
                self.cells[board_idx].is_mine = true;
            }
        }

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
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }

                        let nx = x as isize + dx;
                        let ny = y as isize + dy;

                        if nx >= 0
                            && nx < self.width as isize
                            && ny >= 0
                            && ny < self.height as isize
                        {
                            let n_idx = self.index(nx as usize, ny as usize);
                            if self.cells[n_idx].is_mine {
                                count += 1;
                            }
                        }
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
        }

        if self.cells[idx].adjacent_mines == 0 {
            // Flood fill for empty cells
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }

                    let nx = x as isize + dx;
                    let ny = y as isize + dy;

                    if nx >= 0 && nx < self.width as isize && ny >= 0 && ny < self.height as isize {
                        self.reveal(nx as usize, ny as usize);
                    }
                }
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
        let mut won = true;
        for cell in &self.cells {
            if !cell.is_mine && cell.state != CellState::Revealed {
                won = false;
                break;
            }
        }

        if won {
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
        let board = Board::new(10, 10, 15);
        assert_eq!(board.width, 10);
        assert_eq!(board.height, 10);
        assert_eq!(board.cells.len(), 100);
        assert_eq!(board.num_mines, 15);
        assert_eq!(board.state, GameState::Playing);
        assert!(board.first_click);
    }

    #[test]
    fn test_first_click_places_mines() {
        let mut board = Board::new(5, 5, 5);
        board.reveal(2, 2);
        assert!(!board.first_click);

        // Count placed mines
        let mines = board.cells.iter().filter(|c| c.is_mine).count();
        assert_eq!(mines, 5);

        // Ensure first click area is safe
        for dy in -1..=1 {
            for dx in -1..=1 {
                let nx = 2 + dx;
                let ny = 2 + dy;
                let cell = board.get_cell(nx as usize, ny as usize).unwrap();
                assert!(!cell.is_mine);
            }
        }
    }

    #[test]
    fn test_flagging() {
        let mut board = Board::new(5, 5, 0);
        board.toggle_flag(1, 1);
        assert_eq!(board.get_cell(1, 1).unwrap().state, CellState::Flagged);
        board.toggle_flag(1, 1);
        assert_eq!(board.get_cell(1, 1).unwrap().state, CellState::Hidden);
    }
}
