# 🦀💣 Rusty Mines

[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/dealien/Rusty-Mines?utm_source=badge)

Welcome to **Rusty Mines**—a modern, blazingly fast Minesweeper clone built in Rust using the `egui` framework. Whether you want to click away or watch an AI dismantle the minefield, this project has you covered. 🤖✨

## ✨ Features

- 🎮 **Classic Minesweeper Gameplay** — The timeless puzzle of clearing a minefield, with configurable board sizes (up to 50×50) and adjustable mine counts.
- 🤖 **Advanced Auto-Solver** — A multi-tiered logic engine that deduces safe cells and flags using four escalating rules, from simple arithmetic to full constraint satisfaction.
- 📊 **Live Probability Overlay** — Visualize the real-time calculated chance of a mine under every hidden cell, color-coded from green (safe) to red (danger).
- 📜 **Action History Panel** — A dedicated log that tracks every solver move and labels which rule produced each deduction.
- ⚙️ **Interactive UI** — Built with `eframe`/`egui`, featuring auto-resizing windows, detachable Solver and History panels, and an adjustable solve speed from step-by-step to full auto.

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
2. Choose which logic tiers to enable using the checkboxes.
3. Click **▶ Auto-Play** to watch it go, or **⏭ Step** to move one deduction at a time.
4. Adjust the **Speed** slider to control how fast auto-play runs.
5. Toggle **Probabilities** to visualize the solver's reasoning live.

## 🧠 Solver Algorithms

The solver tries each rule in order and returns as soon as it finds a certain move. Only Rule 4 is a heuristic — Rules 1–3 are deterministic deductions that are guaranteed to be correct.

---

### Rule 1 — Standard Deduction

The simplest and most commonly fired rule. For every revealed numbered cell, the solver counts its **flagged** and **hidden** neighbours and compares them to the cell's mine count.

Two deductions are possible:

| Condition | Conclusion |
| --- | --- |
| `flags == number` | All remaining hidden neighbours are **safe** → reveal them |
| `flags + hidden == number` | All hidden neighbours must be **mines** → flag them |

**Example:**

```text
[1][1][0]
[1][2][1]
[ ][1][F]
```

The `2` has 1 flagged neighbour (`F`) and 1 hidden neighbour (`[ ]` at bottom-left). Since `flags + hidden == 2 == number`, that hidden cell must be a mine → **Flag it**.

---

### Rule 2 — Subset / Pattern Matching

When Rule 1 cannot fire, the solver looks at *pairs* of numbered cells and checks whether one cell's hidden-neighbour set is a **strict subset** of the other's.

If set **B** ⊂ set **A**, we can subtract the constraints:

```text
mines(A) − mines(B) = mines(A \ B)
```

This tells us exactly how many mines are in the *difference* cells (A minus B).

**Two outcomes are possible:**

- If `mines(A) − mines(B) == 0` → all cells in `A \ B` are **safe**.
- If `mines(A) − mines(B) == |A \ B|` → all cells in `A \ B` are **mines**.

**Example (the classic 1-2 pattern):**

```text
[1][2]
[A][B][C]
```

- Cell `1` sees hidden neighbours: `{A, B}`, needs 1 mine.
- Cell `2` sees hidden neighbours: `{A, B, C}`, needs 2 mines.

Since `{A, B}` ⊂ `{A, B, C}`:

```text
mines({A,B,C}) − mines({A,B}) = 2 − 1 = 1 mine in {C}
```

So `C` must be a mine → **Flag it**.

---

### Rule 3 — Constraint Satisfaction (CSP / Tank Algorithm)

When no pair-wise subset relationship resolves a cell, the solver shifts to a full **Constraint Satisfaction Problem** approach. It treats every hidden frontier cell as a boolean variable (`0` = safe, `1` = mine) and every revealed numbered cell as a linear equation over those variables.

**Step 1 — Build the frontier.** Collect all hidden cells that touch at least one revealed number. Each numbered cell contributes one equation:

```text
x₁ + x₂ + ... + xₙ = k   (where k = number − already_flagged)
```

**Step 2 — Partition into independent regions.** Two cells are in the same region if they appear together in any equation. This is solved with a **union-find** data structure, keeping each region small (typically ≤ 20 cells).

**Step 3 — Backtrack over each region.** For each region, a recursive DFS assigns `0` or `1` to every cell. At each step, any unsatisfied constraint is checked for early pruning. Every complete assignment that satisfies all constraints is recorded as a *valid configuration*.

**Step 4 — Analyze configurations.** After enumeration, the solver counts how often each cell is a mine across all valid configurations:

```text
If mine_count(cell) == 0              → cell is SAFE in every valid world → reveal
If mine_count(cell) == config_count   → cell is a MINE in every valid world → flag
```

A **3-second time budget** guards against exponentially large regions; if the budget is exhausted, the solver falls through to Rule 4.

The valid configurations are also cached so Rule 4 can read exact probabilities rather than using a heuristic.

---

### Rule 4 — Probability Guess

When no certain move exists, the solver computes the **estimated mine probability** for every hidden cell and reveals the one with the lowest risk.

Probability estimates are produced in three tiers of accuracy:

1. **Exact (CSP-derived):** If Rule 3 enumerated a cell's region, use the exact frequency:

    ```text
    P(cell is mine) = valid_configs_where_cell_is_mine / total_valid_configs
    ```

2. **Local heuristic:** For frontier cells not covered by Rule 3, compute a per-constraint estimate:

    ```text
    P(cell is mine) = effective_mines / hidden_neighbours
    ```

    The maximum estimate across all constraints touching a cell is used (conservative blend).

3. **Global density:** For deep-unknown cells with no revealed neighbours at all:

    ```text
    P(cell is mine) = remaining_mines / total_hidden_cells
    ```

The solver then runs **iterative constraint propagation** to refine the local estimates — if a cell is confirmed safe or a mine by logic, constraints that contain it are updated, which may cascade and confirm further cells. This loop repeats until no new information is gained.

**Tie-breaking** (when two cells share the same probability):

1. Prefer the cell with the **most hidden neighbours** (maximises information yield).
2. Then prefer top-most, then left-most (for determinism).

## 💡 Tech Stack

- **[Rust](https://www.rust-lang.org/):** The core language, providing safety and performance.
- **[egui](https://github.com/emilk/egui) & [eframe](https://github.com/emilk/egui/tree/master/crates/eframe):** An easy-to-use, immediate mode GUI library for Rust.

---

*Happy sweeping! Try not to click on a 💣!*
