# Rust Learnings – Rusty-Mines

## 2026-03-27 – Borrow Checker and `board.index()` in Tests
**Learning:** Calling `board.cells[board.index(cx, cy)]` on a mutable borrow of `board` causes E0502 because `board.index()` takes `&self` (immutable borrow) while the `[]` index assignment needs `&mut self`, and they overlap lexically.
**Action:** Pre-compute the index into a local variable (`let idx = board.index(cx, cy);`) before the mutable indexing statement.

## 2026-03-27 – `Default` Derive and Enum Variants with Data
**Learning:** `#[derive(Default)]` on a struct fails if any field's type does not implement `Default`. Enums with data-carrying variants (`Reveal(usize, usize)`) cannot automatically derive `Default`.
**Action:** Either annotate the intended default variant with `#[default]` (Rust 1.62+), or provide a manual `impl Default` returning the unit/sentinel variant.
