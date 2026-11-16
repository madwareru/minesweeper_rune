# Double-Click Adjacent Reveal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reveal a numbered cell's unflagged neighbors by double-click when its neighboring flag count matches its number.

**Architecture:** Add one `State` method that validates the target and flag count, then delegates each neighbor to `State::reveal`. Bind it with Rune's `on_double_press`; do not implement click timing or duplicate reveal rules.

**Tech Stack:** Rust 2024, Rune `958743308f36fdf1c568c513badc96ddb0c7f040`

**Spec:** `docs/superpowers/specs/2026-11-15-double-click-reveal-design.md`

## Global Constraints

No new dependencies, configuration, keyboard behavior, or custom double-click timing. Single-click and right-click behavior remain unchanged. Reuse `State::reveal` for flood reveal, loss, and win handling.

## Review Focus

- A mismatched flag count must leave the board unchanged; pin in Task 1 Step 1.
- Flagged neighbors must stay covered and flagged; pin in Task 1 Step 1.
- Incorrect flags with a matching count may reveal a mine and lose; pin in Task 1 Step 1.
- Covered, mine, clear, finished, and out-of-range targets must do nothing; pin in Task 1 Step 1.
- Two real runtime clicks must trigger the action; pin in Task 1 Step 1.

---

### Task 1: Add double-click adjacent reveal

**Files:**
- Modify: `src/main.rs:40-117` (state operation)
- Modify: `src/main.rs:174-190` (double-press binding)
- Test: `src/main.rs` test module

**Interfaces:**
- Consumes: existing `State::reveal(&mut self, index: usize)`, `neighbors(index)`, and test `click` helper.
- Produces: `State::reveal_adjacent(&mut self, index: usize)`.

- [ ] **Step 1: Write failing tests**

Add `reveal_adjacent_requires_a_matching_flag_count`: use a started state with revealed cell 40 showing 1; first verify zero flags leaves the board unchanged, then flag mined cell 30, call `reveal_adjacent(40)`, and verify cell 31 is revealed while cell 30 remains flagged and covered.

Add `reveal_adjacent_validates_the_target`: verify covered, revealed mine, revealed clear, finished, and `CELL_COUNT` targets do not mutate state.

Add `reveal_adjacent_trusts_flag_count_not_flag_correctness`: flag a safe neighbor of revealed cell 40 showing 1, leave an actual neighboring mine unflagged, call `reveal_adjacent(40)`, and verify `Outcome::Lost`.

Add `double_click_reveals_adjacent_cells`: build a runtime with cell 40 revealed/showing 1 and mined cell 30 flagged, call the existing `click` helper twice on `cell-40`, and verify cell 31 becomes revealed.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test reveal_adjacent`
Expected: FAIL because `State::reveal_adjacent` does not exist.

- [ ] **Step 3: Implement the state operation and event binding**

Add `State::reveal_adjacent(&mut self, index: usize)`. Return unless the index is valid, the game is playing, and the cell is a revealed non-mine with `adjacent > 0`. Count flagged neighbors and return unless the count equals `adjacent`; then call `self.reveal(neighbor)` for every neighbor. In `cell_view`, add `.on_double_press(move |state: &mut State| state.reveal_adjacent(index))`.

- [ ] **Step 4: Run the focused tests**

Run: `cargo test reveal_adjacent && cargo test double_click_reveals_adjacent_cells`
Expected: all four new tests PASS.

- [ ] **Step 5: Verify the whole project**

Run: `cargo test && cargo build`
Expected: all existing and new tests pass; build exits successfully.
