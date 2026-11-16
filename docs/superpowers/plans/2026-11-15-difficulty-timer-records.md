# Difficulty, Timer, and Records Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add classic difficulty modes, a correctly lifecycle-managed timer, and persistent per-mode best times with the requested control layout.

**Architecture:** Replace fixed board constants with a `Difficulty` enum and dynamic cell vector while preserving the existing rules. Use Rune's logical clock and poll hook for timing, `Prefs` for records, and transaction-aware handlers for difficulty/window changes.

**Tech Stack:** Rust 2024, Rune `958743308f36fdf1c568c513badc96ddb0c7f040` (`AppSpec::poll`, `Tx`, `Prefs`); stdlib only otherwise.

**Spec:** `docs/superpowers/specs/2026-11-15-difficulty-timer-records-design.md`

## Global Constraints

No new dependencies, custom boards, pause, leaderboard, record history, or record-reset UI. Preserve current game rules and interactions. Tests use in-memory/temporary preferences and virtual time, never real user preferences or sleeps. Do not make Git commits unless separately requested.

## Review Focus

- Hard-board neighbor calculations must not wrap across rows; Task 1 tests corners and all cells in every mode.
- First-click safety and win thresholds must use active dimensions/mine counts; Task 1 tests every mode.
- Flagging and invalid input must not start time; Task 3 tests the timer lifecycle.
- Switching modes/New Game must reset time but preserve records; Tasks 3 and 4 test both.
- Losses, slower wins, and other modes must not replace a record; Task 4 tests each.

---

### Task 1: Make game rules difficulty-aware

**Files:**
- Modify: `src/main.rs` model, neighbor calculation, game rules, scenes, and tests

**Interfaces:**
- Produces: `Difficulty::{Easy, Moderate, Hard}`, `Difficulty::ALL`, `width()`, `height()`, `mines()`, `name()`, and `index()`; `State { difficulty, cells: Vec<Cell>, .. }`; `neighbors(difficulty: Difficulty, index: usize)`.
- Consumes: existing game behavior and tests, migrated from fixed constants to `state.difficulty`/`state.cells.len()`.

- [ ] **Step 1: Write the failing difficulty test**

Add `difficulty_presets_drive_board_rules`: assert Easy/Moderate/Hard are 9×9/10, 16×16/40, and 30×16/99. For each mode, construct a state, assert `cells.len() == width * height`, reveal a central first cell, and assert first-click safety, exact mine count, and correct adjacency for every cell using the new `neighbors(difficulty, index)`.

- [ ] **Step 2: Watch the test fail**

Run: `cargo test difficulty_presets_drive_board_rules`
Expected: FAIL because `Difficulty` and dynamic board state do not exist.

- [ ] **Step 3: Implement dynamic boards**

Add the difficulty enum and values. Replace `SIZE`, `MINES`, and `CELL_COUNT` uses with active difficulty values and `cells.len()`; store `Vec<Cell>`. Make `neighbors` use the supplied width/height. Update `cell_view`, board construction, scenes, and existing tests without changing behavior. Keep `State::new(seed)` creating Easy; record storage is introduced in Task 4.

- [ ] **Step 4: Verify dynamic gameplay**

Run: `cargo test`
Expected: the new test and all migrated existing tests PASS, including double-click and edge flood behavior.

### Task 2: Add difficulty controls and fitting windows

**Files:**
- Modify: `src/main.rs` difficulty state transitions, view, app/test window sizing

**Interfaces:**
- Consumes: Task 1 `Difficulty` API.
- Produces: `State::set_difficulty(&mut self, difficulty: Difficulty)`, `window_size(difficulty: Difficulty) -> (f32, f32)`, and button keys `difficulty-easy`, `difficulty-moderate`, `difficulty-hard`.

- [ ] **Step 1: Write failing UI tests**

Add `difficulty_buttons_start_a_fresh_selected_board`: start Easy, reveal and flag cells, click Moderate, and assert the mode, 256 cells, unstarted/Playing state, default cells, and incremented seed. Click the active Moderate button and assert no second reset. Repeat with Hard and assert 480 cells. Assert all three buttons exist with accessible labels.

- [ ] **Step 2: Watch the UI test fail**

Run: `cargo test difficulty_buttons_start_a_fresh_selected_board`
Expected: FAIL because difficulty buttons are absent.

- [ ] **Step 3: Implement controls and window sizing**

Add `set_difficulty` as a no-op for the active mode and a fresh seeded board otherwise. Add the three buttons above the board with active accent/inactive card fills. Use `on_press_tx` to change difficulty and `tx.resize_window(window_size(difficulty))`. `window_size` retains 36-point cells, 4-point gaps, 32-point horizontal padding, 420-point minimum width, and 248 points beyond board height. Use it for the initial app and test runtime.

- [ ] **Step 4: Verify difficulty UI and regression suite**

Run: `cargo test difficulty_buttons_start_a_fresh_selected_board && cargo test`
Expected: new UI test and full suite PASS.

### Task 3: Add the logical-clock timer

**Files:**
- Modify: `src/main.rs` state, reveal/input handlers, poll hook, bottom control row, tests

**Interfaces:**
- Consumes: existing reveal and reset operations.
- Produces: `State` clock/start/finish fields, `elapsed_seconds() -> u64`, `format_time(seconds: u64) -> String`, `poll(&mut State, f64, &mut Tx) -> bool`, and text key `timer`.

- [ ] **Step 1: Write failing timer tests**

Add `timer_starts_only_on_reveal_and_stops_at_game_end`: install `poll` in the test runtime, verify flagging leaves `Time 0:00`, reveal at virtual time 0, advance frames to 65 seconds and verify `Time 1:05`, then force a loss and verify subsequent frames do not change the value. Add `new_game_and_difficulty_reset_the_timer`: after time advances, verify New Game and a mode change each restore `Time 0:00`. Unit-test `format_time` for 0, 65, and 3600 seconds.

- [ ] **Step 2: Watch timer tests fail**

Run: `cargo test timer_ && cargo test new_game_and_difficulty_reset_the_timer`
Expected: FAIL because timer state/output does not exist.

- [ ] **Step 3: Implement timer lifecycle and display**

Store the current logical clock plus optional start/finish times. Update the clock from `on_press_tx`/`on_double_press_tx` before reveal operations; set start only on the first valid reveal and finish on win/loss. `elapsed_seconds` uses finish when present, otherwise current clock, and floors to whole seconds. In `poll`, update the clock; while started and Playing, request a one-second wake and return true so the label refreshes. Reset start/finish on New Game/mode change. Render mono `Time {m:ss}` left of New Game; install `poll` in `AppSpec` and the test runtime.

- [ ] **Step 4: Verify timer and regression suite**

Run: `cargo test timer_ && cargo test new_game_and_difficulty_reset_the_timer && cargo test`
Expected: new timer tests and full suite PASS.

### Task 4: Add persistent per-mode records

**Files:**
- Modify: `src/main.rs` preferences/state initialization, win handling, record display, tests

**Interfaces:**
- Consumes: Task 3 final `elapsed_seconds()` and difficulty `index()`/stable names.
- Produces: `State::with_prefs(seed: u64, prefs: Prefs)`, records indexed by `Difficulty::index()`, keys `best_easy`/`best_moderate`/`best_hard`, and text key `record`.

- [ ] **Step 1: Write failing record tests**

Add `records_are_per_mode_and_only_improve`: use memory preferences; win Easy at 12 seconds and assert 12, start a new game and win at 20 seconds and assert 12, switch to Moderate and assert no record, win Moderate at 30 seconds and assert Easy remains 12, then verify a loss does not alter the current mode's record. Add `records_reload_from_preferences`: write a record through `Prefs::at` in a unique temporary path, construct another state from the same file, assert the loaded value, and remove the temporary file/directory.

Add a runtime assertion that `record` displays `Best —` when absent and `Best 0:12` after a 12-second win.

- [ ] **Step 2: Watch record tests fail**

Run: `cargo test records_`
Expected: FAIL because record state/output does not exist.

- [ ] **Step 3: Implement record loading, updating, and display**

Store `Prefs` and `[Option<u64>; 3]`; `State::new` uses `Prefs::memory`, while `Default` uses `with_prefs(0, Prefs::open("com.madware.minesweeper"))`. Load each stable key. On a win, update memory and call `prefs.set` only for a missing or lower record, ignoring save errors after the in-memory update. Preserve preferences and records across reset/mode changes. Render mono `Best {m:ss}` or `Best —` right of New Game.

- [ ] **Step 4: Verify records and regression suite**

Run: `cargo test records_ && cargo test`
Expected: new record tests and full suite PASS.

- [ ] **Step 5: Build the deliverable and inspect workspace**

Run: `cargo build && git status --short`
Expected: build succeeds; no unexpected generated files outside ignored build output or the planned docs/source changes.
