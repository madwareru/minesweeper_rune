# Clear Cell Colors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make revealed cells with zero adjacent mines clearly distinct from covered cells.

**Architecture:** Keep all rendering in `cell_view`; add one fill branch using Rune's blue accent at 20% opacity. Test the actual `ElementInfo.fill` values through the existing headless runtime.

**Tech Stack:** Rust 2024, Rune `958743308f36fdf1c568c513badc96ddb0c7f040`

**Spec:** `docs/superpowers/specs/2026-11-15-clear-cell-colors-design.md`

## Global Constraints

Change only clear-cell fill. Do not alter game behavior, labels, other cell colors, dependencies, or add configuration.

## Review Focus

- Covered cells must retain `theme.card`; test this in Task 1 Step 1.
- Revealed numbered cells must retain `theme.control_track`; test this in Task 1 Step 1.
- Clear cells must use `tok::accent(0, true).alpha(0.2)`; test this in Task 1 Step 1.

---

### Task 1: Distinguish clear cells

**Files:**
- Modify: `src/main.rs:181-185` (fill selection)
- Test: `src/main.rs` test module

**Interfaces:**
- Consumes: existing `runtime(State) -> Runtime<State>` and `Runtime::elements()`.
- Produces: no new public interface.

- [ ] **Step 1: Write the failing test**

Add `clear_cells_have_a_distinct_fill`. Set cell 0 to revealed/clear, cell 1 to revealed with one adjacent mine, and leave cell 2 covered. Build the runtime, find each `ElementInfo` by key, and assert the three fills are respectively `tok::accent(0, true).alpha(0.2)`, `tok::theme(true).control_track`, and `tok::theme(true).card`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test clear_cells_have_a_distinct_fill`
Expected: FAIL because the clear cell currently uses `theme.control_track`.

- [ ] **Step 3: Implement the fill branch**

In `cell_view`, select `tok::accent(0, true).alpha(0.2)` when `cell.revealed && !cell.mine && cell.adjacent == 0`, before the existing revealed and covered branches.

- [ ] **Step 4: Run the focused test**

Run: `cargo test clear_cells_have_a_distinct_fill`
Expected: PASS.

- [ ] **Step 5: Verify the whole project**

Run: `cargo test && cargo build`
Expected: all existing and new tests pass; build exits successfully.
