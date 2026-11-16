# Difficulty, timer, and records design

## Intent and why
Add the standard Minesweeper difficulty progression and make successful times comparable within each difficulty. The current game is Easy. Moderate and Hard increase both board area and mine count. A visible timer and persistent per-mode best time provide immediate feedback and a goal across app launches.

## Goals
- Provide Easy 9×9 with 10 mines, Moderate 16×16 with 40 mines, and Hard 30×16 with 99 mines.
- Let the player switch difficulty with three clearly labeled buttons above the board.
- Start a fresh board and resize the window to fit when the difficulty changes.
- Start the timer on the first valid cell reveal, stop it on win or loss, and reset it on New Game or difficulty change.
- Show elapsed time to the left of New Game and the current mode's best time to its right throughout play.
- Persist the best winning time for each mode using Rune's existing preferences store.

## Non-goals
- No custom board dimensions, mine counts, pause control, leaderboard, record history, or record-reset UI.
- No new dependencies or custom persistence/timer infrastructure.
- No changes to reveal, flag, double-click adjacent reveal, first-click safety, or win/loss rules except making them operate on variable-sized boards.

## Anti-goals / failure conditions
- Hard mode must not be squeezed into the Easy window with unusably small cells.
- Flagging must not start the timer; a finished game's timer must not continue.
- A loss or a slower win must not replace a record.
- A record from one mode must not appear as another mode's record.
- Switching modes or starting a new game must not erase saved records.

## Constraints
- Build on Rust 2024 and the pinned Rune dependency; add no dependencies.
- Reuse Rune `AppSpec::poll`, `Tx` clock/wake support, `Tx::resize_window`, and `rune::prefs::Prefs`.
- Preserve existing cell colors, labels, input handlers, and accessible button roles.
- Tests must use in-memory preferences and Rune's virtual clock, not real user preferences or wall-clock sleeps.

## Approaches
### Chosen: dynamic board model and fitting windows
Represent difficulty as an enum with name, width, height, and mine count. Store cells in a `Vec<Cell>` and pass board dimensions to neighbor calculation. Difficulty handlers reset game state and request a fitting window size. This directly supports runtime mode changes and reuses the existing rules.

### Fixed maximum array
A fixed 30×16 backing array could avoid dynamic allocation, but every rule and test would still need active dimensions and it would add index/capacity complexity without a useful benefit.

### Fixed window with scaled cells or scrolling
Shrinking 30 columns into the current window makes cells too small. Scrolling a Minesweeper board hides relevant cells and makes play harder. Resizing the window is simpler and clearer.

## Design decisions
### Difficulty and game state
- Easy remains the initial mode and uses the current 9×9/10 rules.
- Moderate is 16×16/40; Hard is 30×16/99.
- Difficulty buttons are Easy, Moderate, and Hard in a row above the board. The active mode is visually distinguished with an accent fill; inactive modes use the existing card fill.
- Selecting a different mode starts an unstarted board with zero elapsed time and that mode's existing record. Selecting the active mode does nothing; New Game remains the reset action.
- Each mode change requests a window large enough for the existing 36-point cells, 4-point gaps, padding, and all controls. The initial Easy window is adjusted for the added controls.
- Mine placement, adjacency, flood reveal, flag counts, double-click reveal, and win detection use the active mode's dimensions and mine count.

### Timer
- Store timer values on Rune's logical clock in seconds so headless tests are deterministic.
- Before the first valid reveal, elapsed time is zero. That reveal records its start time; flagging, invalid input, and double-click on an invalid target do not start it.
- While Playing after the first reveal, the poll hook updates elapsed time and schedules the next wake. Display whole seconds as `m:ss` under a `Time` label.
- Win and loss store the finish time, so the displayed value remains fixed. New Game and difficulty change clear start/finish values.
- A very fast win may legitimately display and record `0:00`.

### Records and layout
- Keep one best whole-second value per difficulty. Load records when the app starts; missing or invalid values display `Best —`.
- On a win, compare the final whole-second elapsed value with the current mode's record. Update memory and persistent preferences only when there is no record or the new value is lower.
- Use stable preference keys `best_easy`, `best_moderate`, and `best_hard`. Preferences writes are best-effort: Rune retains the new value in memory if saving fails; do not interrupt the game with an error dialog.
- The bottom control row is `Time 0:00` on the left, `New Game` in the middle, and `Best 0:00` or `Best —` on the right. Timer and record use fixed-width mono text to avoid layout movement.
- Records remain visible before, during, and after a game. Records update only on wins.

## Implementation boundaries
Keep the implementation in the existing `src/main.rs` unless its size makes a focused module clearly beneficial. Reuse existing view and test helpers where practical. Update existing tests for dynamic dimensions while retaining their behavior coverage.

## Verification
- Test each mode's dimensions, mine count, first-click safety, adjacency, and win threshold.
- Test mode switching, New Game timer reset, and active-mode selection behavior through runtime input.
- Test timer start only on reveal, advancement through virtual frames, and stopping after win/loss.
- Test records per mode, faster-win replacement, slower-win/loss preservation, and persistence using a temporary preferences file.
- Test bottom-row labels and difficulty selection rendering.
- Run the complete test suite and `cargo build`; exercise difficulty switching and timer updates through Rune runtime calls.
