# Clear cell colors

## Intent
Make revealed cells with zero adjacent mines immediately distinguishable from covered cells; the current theme shades are too similar.

## Scope
Change only the fill color of revealed, empty, non-mine cells in `cell_view`. Leave covered cells, flags, numbers, mines, labels, and game behavior unchanged. No settings or new UI states.

## Approaches
- **Chosen:** use a muted accent fill for clear revealed cells. This creates a strong distinction while reusing Rune's theme colors.
- Use another neutral theme shade: smaller change in appearance, but risks preserving the same low-contrast problem.
- Add configurable colors: unnecessary for one fixed visual distinction.

## Verification
Add a focused rendering test showing that clear revealed and covered cells have different fills, then run the existing Rust test suite and build.
