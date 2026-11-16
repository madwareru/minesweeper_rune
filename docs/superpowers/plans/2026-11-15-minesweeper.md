# Minesweeper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a playable 9×9 Rune Minesweeper with 10 mines, safe first reveal, flags, flood reveal, win/loss, and restart.

**Architecture:** Keep game state, rules, rendering, and tests in `src/main.rs`, following Rune's hello example. Use an array of 81 cells and keyed Rune elements; no separate engine or renderer is needed at this size.

**Tech Stack:** Rust 2024, Rune at git revision `958743308f36fdf1c568c513badc96ddb0c7f040`, standard library only otherwise.

**Spec:** `docs/superpowers/specs/2026-11-15-minesweeper-design.md`

## Global Constraints

- Board is exactly 9×9 with 10 mines; first revealed cell and all its neighbors are safe.
- Left-click reveals; right-click toggles flags; empty cells flood-reveal; revealing all 71 safe cells wins.
- Show remaining mine count, status, and New Game; no timer, difficulty selector, persistence, or animation work.
- Keep implementation in `src/main.rs`; add no dependencies beyond pinned Rune.
- Use keyed, labeled elements and verify actual UI through Rune `Runtime`, PNG snapshots, and element-tree output.

## Review Focus

- First click on a corner or edge: its out-of-bounds neighbors must not panic or be mined; pin with `first_click_is_safe` using corner, edge, and center indices.
- Repeated reveal/flag input after win or loss: board must stay unchanged; pin with `finished_game_ignores_cell_input`.
- Flagged cells: primary reveal must not open them, and flags must not exceed one per cell; pin with `flagging_guards_reveal`.
- Flood reveal: must stop at numbered cells and never reveal flagged cells; pin with `empty_reveal_floods_and_respects_flags`.
- Restart: must clear reveals, flags, outcome, and mine placement while changing the deterministic seed; pin with `new_game_resets_the_board`.

---

### Task 1: Game rules

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `const SIZE: usize = 9`, `const MINES: usize = 10`, `const CELL_COUNT: usize = 81`.
- Produces: `Cell { mine: bool, revealed: bool, flagged: bool, adjacent: u8 }`.
- Produces: `Outcome::{Playing, Won, Lost}` and `State { cells: [Cell; CELL_COUNT], started: bool, outcome: Outcome, seed: u64 }`.
- Produces: `State::new(seed: u64) -> State`, `State::reveal(&mut self, index: usize)`, `State::toggle_flag(&mut self, index: usize)`, and `State::new_game(&mut self)`.
- Produces: `neighbors(index: usize) -> impl Iterator<Item = usize>` for orthogonal and diagonal in-bounds neighbors.

- [ ] **Step 1: Add the pinned Rune dependency and failing rule tests**

Add `rune = { git = "https://github.com/anomalyco/rune", rev = "958743308f36fdf1c568c513badc96ddb0c7f040" }`. In `src/main.rs` tests, cover the five Review Focus cases plus mine/adjacency counts and a forced win by revealing all safe cells. Use fixed seeds and direct `State` methods; assert exact counts and outcomes rather than board snapshots.

- [ ] **Step 2: Run the rule tests and verify they fail**

Run: `cargo test`
Expected: compilation failure because the game types and methods do not exist.

- [ ] **Step 3: Implement the minimal game model**

Implement the interfaces above. On first reveal, exclude that cell and its neighbors, shuffle the remaining indices with a small standard-library-only seeded PRNG, place exactly 10 mines, and compute adjacency. Use a `Vec<usize>` stack for flood reveal. Ignore out-of-range, flagged, already revealed, or finished-game input; on loss reveal mines, and on each reveal check for 71 revealed safe cells. `new_game` increments the seed and replaces state with `State::new(seed)`.

- [ ] **Step 4: Run the rule tests and verify they pass**

Run: `cargo test`
Expected: all rule tests PASS.

### Task 2: Rune UI and end-to-end verification

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: all Task 1 constants, fields, and methods.
- Produces: `view(s: &State, cx: &mut Cx<State>) -> El` and `main()` using `rune::run(AppSpec<State>)`.
- Produces: stable keys `cell-0` through `cell-80`, `new-game`, and `status`.

- [ ] **Step 1: Add failing headless UI tests**

Using `Runtime::new(State::new(1), view, (420.0, 520.0), Env { system_dark: true, scale: 2.0 })`, render a frame and click a cell through `find`, `pointer_down`, and `pointer_up`. Assert the clicked cell becomes revealed. Add a right-click flag test using Rune's secondary pointer API, a New Game click test, and accessible labels that distinguish covered, flagged, numbered, and mined cells.

- [ ] **Step 2: Run the UI tests and verify they fail**

Run: `cargo test`
Expected: compilation failure because `view` and the Rune app entry point do not exist.

- [ ] **Step 3: Implement the Rune view**

Follow `examples/hello`: use `tok::theme(true)`, a centered column, title/status, 9 rows of 9 fixed-size cell buttons, and New Game. Each cell gets its stable key, a descriptive label, `focusable(true)`, `.on_press` calling `reveal`, and `.on_secondary_press` calling `toggle_flag`. Show flags, adjacent counts, mines on loss, and win/loss text; use theme/accent colors with readable contrast. Configure `AppSpec::new("Minesweeper", (420.0, 520.0), State::default, view)` with deterministic scenes only if needed for snapshots.

- [ ] **Step 4: Run all tests and formatting**

Run: `cargo fmt --check && cargo test`
Expected: formatting clean and all tests PASS. If formatting differs, run `cargo fmt`, then repeat both checks.

- [ ] **Step 5: Check the shipped app headlessly**

Run: `cargo run -- --tree` and `cargo run -- --snap /tmp/minesweeper.png`; use Rune's scripted input/scene options to capture a revealed or flagged state as well. Inspect the PNG with the media reader and the JSON/tree output for all 81 keyed cells, status, and New Game. Expected: commands exit successfully, the board is legible and unclipped, and the tree reflects the same state as the tests.
