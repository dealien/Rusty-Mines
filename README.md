# 🦀💣 Rusty Mines

Welcome to **Rusty Mines**—a modern, blazingly fast Minesweeper clone built in Rust using the `egui` framework! It's not just a game; it's a testbed for automated logic solving. Whether you want to click away or watch an AI dismantle the minefield, this project has you covered. 🤖✨

## ✨ Features

- 🎮 **Classic Minesweeper Gameplay:** Enjoy the timeless puzzle of clearing a minefield without detonating any hidden explosives. Features custom board sizes (up to 50x50) and adjustable mine counts.
- 🤖 **Advanced Auto-Solver:** Watch the computer play for you! The built-in solver uses a multi-tiered logic system to deduce safe cells and flags:
  - **Rule 1: Standard Deduction:** Simple neighbor counting for obvious flags and reveals.
    - *Example:* If a revealed cell is a `1` and has exactly one hidden neighbor, that neighbor must be a mine. If a `1` has one flagged neighbor and one hidden neighbor, the hidden one is safe.
  - **Rule 2: Subset Patterns:** Advanced logic for overlapping constraints (e.g., the classic 1-2 or 1-1 patterns).
    - *Example:* If two adjacent revealed cells share a set of hidden neighbors, the solver can deduce that the mines required by the first cell satisfy the requirements of the second, allowing it to flag or clear the non-shared neighbors safely.
  - **Rule 3: Constraint Satisfaction (CSP):** Complex deductions treating cells as variables in equations, resolving the entire frontier at once.
    - *Example:* When simple overlaps fail, CSP maps out all remaining hidden cells bordering the revealed area and finds every possible valid arrangement of mines. If a cell is safe in *all* valid arrangements, it's definitively safe to click.
  - **Rule 4: Probability/Heuristic:** When deterministic logic fails, the solver calculates the statistical probability of mines to make the safest possible guess.
    - *Example:* If forced into a 50/50 corner, or an area entirely surrounded by mines, it will compute the exact odds of every single hidden cell on the board based on remaining mine count and local constraints, clicking the cell with the lowest percentage (e.g. `12%` danger).
- 📊 **Live Probability Overlay:** Turn on probability mode to see the real-time calculated chance of a mine existing under every hidden cell! (Green = Safe, Red = Danger).
- 📜 **Detailed Action History:** A dedicated history panel tracks every move the solver makes, indicating which logic rule was used for the deduction.
- ⚙️ **Interactive UI:** Built with `eframe`/`egui`, featuring dynamic auto-resizing, separate pop-out windows for the Solver and History panels, and adjustable solve speeds (from blazing fast to step-by-step).

## 🚀 Installation & Usage

Make sure you have [Rust and Cargo](https://rustup.rs/) installed on your system.

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/rusty-mines.git
   cd rusty-mines
   ```

2. Run the game in release mode for optimal performance:

   ```bash
   cargo run --release
   ```

## 🛠️ How to Play

### Manual Play 🖱️
- **Left Click:** Reveal a cell.
- **Right Click:** Flag a cell you suspect is a mine.
- Adjust the board size and mine count at the top and click **New Game**.

### Using the Solver 🤖
1. Open the **🤖 Solver** panel (toggleable at the top right).
2. Choose your active logic tiers using the checkboxes.
3. Click **▶ Auto-Play** to watch it go, or **⏭ Step** to move one deduction at a time.
4. Adjust the **Speed** slider to control how fast the auto-play runs.
5. Toggle the **Probabilities** setting to visualize the solver's brain at work!

## 💡 Tech Stack

- **[Rust](https://www.rust-lang.org/):** The core language, providing safety and performance.
- **[egui](https://github.com/emilk/egui) & [eframe](https://github.com/emilk/egui/tree/master/crates/eframe):** An easy-to-use, immediate mode GUI library for Rust.

---

*Happy sweeping! Try not to click on a 💣!*
