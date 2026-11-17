# Minesweeper Rune

## Project overview

This repository is a small native desktop Minesweeper game. It builds one executable, `minesweeper_rune`, from the Rust sources in `src/`.

The technology stack is:

- Rust, edition 2024.
- Rune for the application lifecycle, declarative UI, input, logical clock, preferences, and headless runtime testing.
- The Rust standard library for all game logic; Rune is the only direct dependency.

Rune is pinned to git revision `958743308f36fdf1c568c513badc96ddb0c7f040`. The `[patch.crates-io]` entries for `wgpu-hal`, `winit`, `parley_engine`, `accesskit_windows`, and `accesskit_macos` use the same revision. Keep these revisions aligned when changing the dependency. `Cargo.lock` pins the complete dependency graph. There is no `rust-toolchain` file or explicit `rust-version`; use a toolchain that supports Rust 2024.

The game supports:

| Difficulty | Board | Mines |
| --- | --- | --- |
| Easy | 9×9 | 10 |
| Moderate | 16×16 | 40 |
| Hard | 30×16 | 99 |

The first revealed cell and its neighbors are always safe. Primary click reveals, secondary click toggles a flag, and double-click on a revealed number reveals its neighbors when the adjacent flag count matches that number. Empty cells flood-reveal. The game also has restart, dynamic window sizing, a logical-clock timer, persistent best winning times per difficulty, and subtle inactivity hints.

## Repository layout

- `Cargo.toml`: package metadata, the pinned Rune dependency, and git patches for selected transitive dependencies.
- `Cargo.lock`: resolved application dependencies.
- `src/main.rs`: application entry point, lifecycle polling, and scripted scenes.
- `src/model.rs`: all data definitions, state construction, and `State`/`GameState` delegation.
- `src/game.rs`: UI-free gameplay logic and game-rule tests.
- `src/view.rs`: Rune rendering, interaction handlers, and headless UI tests.
- `.gitignore`: excludes `/target`.

There are no separate library, engine, renderer, asset, integration-test, CI, packaging, or deployment directories.

## Runtime architecture

`main()` calls `rune::run` with an `AppSpec<State>`. `AppSpec::new` connects the window title and size, `State::default`, and the root `view` function. The `poll` hook advances the logical clock, updates hints, and requests a one-second wake while a game is active; Rune's renderer owns hint animation frames. Named `AppSpec::scenes` (`revealed`, `flagged`, `lost`, and `won`) set up deterministic states for Rune's headless tools.

The code is organized by responsibility:

- `model.rs` defines `Difficulty`, `Cell`, `Outcome`, `Settings`, `HintKind`, `Hint`, `GameState`, and `State`. `Difficulty` defines board dimensions, mine counts, display names, record keys, and record-array indices. Cells are stored row-major in a `Vec<Cell>`; an index is `y * width + x`. `State` implements `Deref<Target = GameState>` and `DerefMut`, so game fields are accessed directly through `State`.
- `game.rs` implements `State::reveal`, `reveal_adjacent`, `toggle_flag`, `new_game`, `set_difficulty`, `place_mines`, and `update_hint`. Its `neighbors` helper performs bounds-checked orthogonal and diagonal neighbor iteration for the active difficulty.
- Mine placement is deferred until the first reveal. A small deterministic wrapping PRNG selects mines outside the first cell's safe neighborhood; resets increment the seed. Empty-cell expansion uses an explicit stack rather than recursion.
- `poll`, `GameState::elapsed_seconds`, and `view::format_time` implement timing with Rune's logical clock, not wall-clock reads.
- `view.rs` renders one accessible, focusable button per cell through `cell_view`. `view` builds the title, status, difficulty controls, board, timer, New Game button, and record display from Rune elements and `rune::tok` theme values.
- Event handlers mutate `State`; transaction-aware reveal handlers read the clock from `Tx`. Difficulty handlers and the root layout share a fit check that resizes only when the current window width or height is too small for the selected difficulty. Larger dimensions are preserved, so no pending resize state or macOS fullscreen special case is needed.

After 15 seconds without successful progress, `update_hint` selects the first candidate in this priority order: a revealed number whose covered-neighbor count, including flags, equals its number and which has an unflagged covered neighbor (`MineCount`); a revealed number whose adjacent flag count matches its number and which has an unflagged covered neighbor (`DoubleClick`); or an unflagged, covered, non-mine cell adjacent to a revealed safe cell, ranking candidates first by whether their covered-neighbor count equals their own adjacent-mine count and then by revealed-safe-neighbor count (`Safe`). The first two rules use visible information only; only the final fallback inspects hidden mine positions. A hint remains stable until a successful reveal or either flag toggle clears it and restarts inactivity tracking; no-op input does neither. New games and finished games have no hint. `view.rs` applies a static accent tint and `rune::LoopSpec::opacity` for a one-second pulse without changing layout. Rune Motion owns animation timing and reduced-motion behavior. Accessibility labels identify mine-count, double-click, and safe-cell hints. Game tests own selection, priority, and reset semantics; view tests own opacity pulse, reduced-motion rendering, and hint labels.

`State::default` opens Rune preferences with application ID `com.madware.minesweeper`. Records use keys `best_easy`, `best_moderate`, and `best_hard`. Only a winning time that improves the current difficulty's record replaces it. Writes are best-effort by design; the in-memory record still updates if persistence fails. Starting a new game or changing difficulty preserves records.

Stable element keys are part of the testing and headless-tooling interface: `root`, `cell-{index}`, `difficulty-easy`, `difficulty-moderate`, `difficulty-hard`, `new-game`, `status`, `timer`, and `record`. Cell labels describe row, column, and state for accessibility and element-tree inspection.

## Build and run commands

Run commands from the repository root:

```sh
cargo build
cargo run
cargo build --release
```

`cargo run` opens the native application. Debug and release executables are written beneath `target/debug` and `target/release`, respectively. A clean build needs access to the pinned git dependency and Rune's native platform requirements.

Format and check formatting with rustfmt's default rules:

```sh
cargo fmt
cargo fmt --check
```

There is no project-specific Clippy configuration or custom build script.

## Testing instructions

Run all tests or one named test:

```sh
cargo test
cargo test first_click_is_safe
```

Tests live beside the code they cover: game-rule and record tests in `src/game.rs`, and headless UI tests in `src/view.rs`, under `#[cfg(test)]`. There is no separate `tests/` directory. UI tests exercise real interactions through `rune::Runtime`.

When changing the project:

- Add focused tests for rule changes using fixed seeds and direct `State` calls. Assert exact board counts, outcomes, and unchanged state where appropriate.
- Test UI changes through `Runtime`, stable element keys, pointer APIs, and rendered element data rather than only calling view code.
- Use `State::new` and `Prefs::memory()` in ordinary tests so real user preferences are never touched.
- Use a unique temporary path with `Prefs::at` only for persistence tests, and clean it up.
- Use Rune's virtual clock and `runtime.frame(...)` for time tests. Do not use wall-clock sleeps.
- Preserve first-click safety, row-edge neighbor handling, flag protection, win/loss input guards, and per-difficulty record isolation.

For manual headless inspection, Rune requires an output path for `--tree` and `--snap`. For example:

```sh
cargo run -- --scene revealed --tree /tmp/minesweeper-tree.json
cargo run -- --scene revealed --snap /tmp/minesweeper.png
cargo run -- --list-scenes
```

Inspect both the JSON element tree and the rendered PNG when making visual or accessibility changes. Scenes can be combined with snapshot and tree options. These commands are verification tools, not a separate golden-snapshot test suite.

## Code style and development conventions

- Keep the implementation small and direct. Keep data definitions in `model.rs`, UI-free gameplay in `game.rs`, Rune rendering and interaction in `view.rs`, and application wiring in `main.rs`. Keep tests beside the module whose behavior they cover.
- Do not add dependencies beyond Rune without a concrete requirement. Game randomness, flood fill, timing, and storage already use the standard library or Rune facilities.
- Follow existing Rust naming and rustfmt defaults.
- Reuse `State::reveal` for adjacent-cell reveal so flood fill, loss, and win handling do not diverge.
- Keep board calculations based on the active `Difficulty` and `cells.len()`; do not reintroduce fixed Easy-board assumptions.
- Preserve stable element keys, descriptive labels, button roles, and `focusable(true)` for interactive elements.
- Reuse `tok::theme(true)` and `tok::accent(...)` colors where possible. Every revealed cell uses `theme.control_track`, while covered cells use `theme.control_track.mix(tok::accent(0, true), 0.1)`. Mine symbols use `rune::Color { r: 1.0, g: 0.25, b: 0.2, a: 1.0 }`, flag symbols use `rune::Color::WHITE`, and numbered cells retain their accent colors.
- Preserve the documented timer lifecycle: flagging does not start it, win/loss stops it, and New Game or a real difficulty change resets it. Selecting the already-active difficulty is a no-op.
- Source and UI strings are in English.

## Security and data considerations

The application contains no server, account system, or application-level network protocol. Its persistent user data is limited to best-time preferences written through Rune. Do not point tests at the real `com.madware.minesweeper` preference store.

Dependency installation does fetch Rune and patched crates from GitHub, so dependency revision changes affect the trusted build supply chain. Review them deliberately and keep the shared revision consistent.

Bounds and state checks in `reveal`, `reveal_adjacent`, and `toggle_flag` are intentional: preserve protection against invalid indices, flagged or revealed cells, and input after a finished game. Preference-save errors are deliberately non-fatal, but other errors should not be silently discarded without a design reason.

## Deployment and automation

No CI workflow, installer, bundle configuration, release script, publishing metadata, or deployment process is present. `cargo build --release` is the available release-build command; any platform packaging or distribution steps remain external to this repository. Supported desktop platforms and native prerequisites inherit from the pinned Rune stack and are not independently configured here.
