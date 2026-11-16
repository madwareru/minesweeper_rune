# Minesweeper design

## Intent

Build a small, playable native Minesweeper game with Rune, starting from `examples/hello` and verifying the real UI headlessly.

## Scope

- One beginner board: 9×9 cells with 10 mines.
- The first revealed cell and its neighbors are safe.
- Left-click reveals; right-click toggles a flag.
- Revealing an empty cell flood-reveals its connected empty area and numbered boundary.
- A mine ends the game; revealing every safe cell wins it.
- Show remaining mine count, game status, and a New Game button.
- No difficulty selector, timer, persistence, or animation work.

## Constraints

- Use Rune pinned to dev revision `958743308f36fdf1c568c513badc96ddb0c7f040`.
- Keep the implementation in `src/main.rs`, following the hello example's `State`, `view`, `AppSpec`, and `Runtime` structure.
- Use no dependencies beyond Rune; deterministic pseudo-random placement is sufficient.
- Use keyed, labeled Rune elements so the game works with pointer input, accessibility, element-tree dumps, and snapshots.

## Approaches

- **Chosen: ordinary Rune elements.** One element per cell with Rune layout and input handlers. This is the smallest accessible implementation and supports built-in headless inspection.
- Custom canvas rendering would reduce element count but require manual hit-testing, drawing, accessibility, and more test code; unnecessary for 81 cells.
- A reusable game engine module would separate rules from UI, but one file is enough at this size and matches the requested simple starting point.

## Verification

Add focused Rust tests for board rules and Rune `Runtime` pointer interactions. Run formatting, tests, and headless PNG/tree commands; inspect the resulting render and tree, including revealed, flagged, and finished states.
