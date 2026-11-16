# Double-click adjacent reveal

## Intent
Let players quickly open the covered neighbors of a revealed numbered cell once its neighboring flag count equals the displayed number.

## Scope
Add a `State::reveal_adjacent(index)` operation and bind it to Rune's existing `on_double_press` event for each cell. Reuse `State::reveal` for every unflagged neighbor so flood reveal, mine loss, and win detection keep their existing behavior. Single-click and right-click behavior remain unchanged.

A double-click acts only while playing, on a revealed, non-mine cell with `adjacent > 0`, when the exact neighboring flag count matches `adjacent`. Otherwise it does nothing. Flags are trusted, not checked for correctness; a wrong set of flags with the right count can reveal a mine and lose, as in standard Minesweeper.

No new dependencies, configuration, keyboard behavior, or custom double-click timing.

## Approaches
- **Chosen:** one state method plus `on_double_press`, reusing `reveal` for neighbors. This is the smallest change and preserves all existing game rules.
- Implement separate flood/loss/win logic for double-click: duplicates `reveal` and risks divergent behavior.
- Track click timing manually: unnecessary because Rune already supplies double-press events.

## Verification
Test matching and mismatched flag counts, flagged-neighbor protection, incorrect flags causing a loss, invalid targets, and a real two-click runtime interaction. Run the full Rust test suite and build.
