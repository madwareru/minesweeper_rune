use crate::model::{Difficulty, HintKind, Outcome, State};
use rune::tok::{self, type_};
use rune::{Color, Cx, El, LoopSpec, col, el, row, text};

const CELL_SIZE: f32 = 36.0;
const CELL_GAP: f32 = 4.0;
const CONTROL_HEIGHT: f32 = 36.0;
const DIFFICULTY_BUTTON_WIDTH: f32 = 100.0;
const NEW_GAME_BUTTON_WIDTH: f32 = 120.0;
const CORNER_RADIUS: f32 = 6.0;
const CONTROL_GAP: f32 = 8.0;
const SECTION_GAP: f32 = 16.0;
const ROOT_PADDING: f32 = 16.0;
const MIN_WINDOW_WIDTH: f32 = 420.0;
const NON_BOARD_HEIGHT: f32 = 248.0;
const NUMBER_ACCENT_COUNT: usize = 6;
const COVERED_CELL_ALPHA: f32 = 0.2;
const SELECTED_DIFFICULTY_ALPHA: f32 = 0.2;
const HINT_PERIOD: f64 = 1.0;
const HINT_LOW_OPACITY: f32 = 0.75;

const VIBRANT_RED: Color = Color {
    r: 1.0,
    g: 0.05,
    b: 0.02,
    a: 1.0,
};

fn format_time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub(crate) fn window_size(difficulty: Difficulty) -> (f32, f32) {
    let board_width = difficulty.width() as f32 * (CELL_SIZE + CELL_GAP) - CELL_GAP;
    let board_height = difficulty.height() as f32 * (CELL_SIZE + CELL_GAP) - CELL_GAP;
    (
        (board_width + ROOT_PADDING * 2.0).max(MIN_WINDOW_WIDTH),
        board_height + NON_BOARD_HEIGHT,
    )
}

fn fit_window_size(difficulty: Difficulty, current: (f32, f32)) -> Option<(f32, f32)> {
    let required = window_size(difficulty);
    (current.0 < required.0 || current.1 < required.1)
        .then_some((current.0.max(required.0), current.1.max(required.1)))
}

fn resize_to_fit(difficulty: Difficulty, current: (f32, f32), tx: &mut rune::Tx) {
    if let Some(size) = fit_window_size(difficulty, current) {
        tx.resize_window(size);
    }
}

fn button_base(key: impl Into<String>) -> El {
    el().role("button")
        .focusable(true)
        .center()
        .radius(CORNER_RADIUS)
        .key(key)
}

fn text_button(
    key: impl Into<String>,
    label: &'static str,
    width: f32,
    fill: Color,
    text_color: Color,
) -> El {
    button_base(key)
        .label(label)
        .size(width, CONTROL_HEIGHT)
        .fill(fill)
        .child(text(label).font(type_::LABEL).color(text_color))
}

fn cell_view(state: &State, index: usize, theme: tok::Theme) -> El {
    let cell = state.cells[index];
    let difficulty = state.settings.difficulty;
    let row_number = index / difficulty.width() + 1;
    let column = index % difficulty.width() + 1;
    let (description, content, color) = match (cell.revealed, cell.mine, cell.flagged) {
        (true, true, _) => ("mine".to_owned(), "●".to_owned(), VIBRANT_RED),
        (true, _, _) if cell.adjacent == 0 => ("clear".to_owned(), "".to_owned(), theme.text),
        (true, _, _) => (
            format!("{} adjacent mines", cell.adjacent),
            format!("{}", cell.adjacent),
            tok::accent((usize::from(cell.adjacent) - 1) % NUMBER_ACCENT_COUNT, true),
        ),
        (_, _, true) => ("flagged".to_owned(), "⚑".to_owned(), Color::WHITE),
        _ => ("covered".to_owned(), "".to_owned(), theme.text),
    };

    let hint = state.hint.as_ref().filter(|hint| hint.cell == index);
    let mut alpha = COVERED_CELL_ALPHA;
    if cell.revealed {
        alpha /= 2.0;
    }
    if hint.is_some() {
        alpha += 0.1;
    }

    let fill = tok::accent(0, true).alpha(alpha);
    let hint_label = hint.map_or("", |hint| match hint.kind {
        HintKind::MineCount => " · hint",
        HintKind::DoubleClick => " · double-click hint",
        HintKind::Safe => " · safe cell hint",
    });
    let button = button_base(format!("cell-{index}"))
        .label(format!(
            "Row {row_number}, column {column}, {description}{hint_label}"
        ))
        .size(CELL_SIZE, CELL_SIZE)
        .fill(fill)
        .on_press_tx(move |state: &mut State, tx| {
            state.clock = tx.clock();
            state.reveal(index);
        })
        .on_double_press_tx(move |state: &mut State, tx| {
            state.clock = tx.clock();
            state.reveal_adjacent(index);
        })
        .on_secondary_press(move |state: &mut State, _, _| state.toggle_flag(index))
        .child(text(content).font(type_::TITLE).color(color));
    if hint.is_some() {
        button.looping(LoopSpec::opacity(HINT_PERIOD, 0.0, HINT_LOW_OPACITY, 1.0))
    } else {
        button
    }
}

pub(crate) fn view(state: &State, cx: &mut Cx<State>) -> El {
    let theme = tok::theme(true);
    let difficulty = state.settings.difficulty;
    let current_size = cx
        .layout_size("root")
        .map(|[width, height]| (width, height));
    let flags = state.cells.iter().copied().filter(|it| it.flagged).count();
    let remaining = difficulty.mines() as isize - flags as isize;
    let status = match state.outcome {
        Outcome::Playing if !state.started => format!("{remaining} mines · Left-click to start"),
        Outcome::Playing => format!("{remaining} mines left"),
        Outcome::Won => "You won! · All safe cells revealed".to_owned(),
        Outcome::Lost => "Boom! · Try a new game".to_owned(),
    };
    let difficulties = row()
        .gap(CONTROL_GAP)
        .children(Difficulty::ALL.map(|option| {
            text_button(
                format!("difficulty-{}", option.name().to_lowercase()),
                option.name(),
                DIFFICULTY_BUTTON_WIDTH,
                if difficulty == option {
                    tok::accent(0, true).alpha(SELECTED_DIFFICULTY_ALPHA)
                } else {
                    theme.card
                },
                theme.text,
            )
            .on_press_tx(move |state: &mut State, tx| {
                state.set_difficulty(option);
                if let Some(current) = current_size {
                    resize_to_fit(option, current, tx);
                }
            })
        }));
    let board = col()
        .gap(CELL_GAP)
        .children((0..difficulty.height()).map(|y| {
            row().gap(CELL_GAP).children(
                (0..difficulty.width())
                    .map(move |x| cell_view(state, y * difficulty.width() + x, theme)),
            )
        }));
    let content = format!("Time {}", format_time(state.elapsed_seconds()));
    let timer = text(content)
        .key("timer")
        .font(type_::LABEL.mono())
        .color(theme.text_secondary);
    let record = state.records[difficulty.index()]
        .map(|seconds| format!("Best {}", format_time(seconds)))
        .unwrap_or_else(|| "Best —".to_owned());
    let record = text(record)
        .key("record")
        .font(type_::LABEL.mono())
        .color(theme.text_secondary);
    let new_game = text_button(
        "new-game",
        "New Game",
        NEW_GAME_BUTTON_WIDTH,
        theme.card,
        theme.text,
    )
    .on_press(|state: &mut State| state.new_game());

    col()
        .key("root")
        .w_full()
        .h_full()
        .center()
        .gap(SECTION_GAP)
        .padding(ROOT_PADDING)
        .fill(theme.bg)
        .on_layout_tx(move |state: &mut State, [_, _, width, height], tx| {
            resize_to_fit(state.settings.difficulty, (width, height), tx);
        })
        .child(text("MINESWEEPER").font(type_::TITLE).color(theme.text))
        .child(
            text(status)
                .key("status")
                .font(type_::BODY)
                .color(theme.text_secondary),
        )
        .child(difficulties)
        .child(board)
        .child(
            row()
                .gap(SECTION_GAP)
                .center()
                .child(timer)
                .child(new_game)
                .child(record),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Cell, Hint, HintKind};
    use crate::{game::win_at, poll};
    use rune::{Env, Runtime};

    fn runtime(state: State) -> Runtime<State> {
        let size = window_size(state.settings.difficulty);
        let env = Env {
            system_dark: true,
            scale: 2.0,
        };
        let mut runtime = Runtime::new(state, view, size, env);
        runtime.poll = Some(poll);
        runtime.frame(0.0);
        runtime
    }

    fn click(runtime: &mut Runtime<State>, key: &str) {
        let Some((x, y)) = runtime.find(key) else {
            panic!("not found: {}", key)
        };
        runtime.pointer_down(x, y);
        runtime.pointer_up(x, y);
        runtime.frame(0.0);
    }

    fn element(runtime: &Runtime<State>, key: &str) -> Option<rune::ElementInfo> {
        runtime
            .elements()
            .into_iter()
            .find(|element| element.key.as_deref() == Some(key))
    }

    fn label(runtime: &Runtime<State>, key: &str) -> String {
        element(runtime, key)
            .and_then(|element| element.label)
            .unwrap_or_else(|| "not_found".to_owned())
    }

    fn text_value(runtime: &Runtime<State>, key: &str) -> String {
        element(runtime, key)
            .and_then(|element| element.text)
            .unwrap_or_else(|| "not_found".to_owned())
    }

    fn fill(runtime: &Runtime<State>, key: &str) -> Color {
        element(runtime, key).map(|element| element.fill).unwrap()
    }

    fn opacity(runtime: &Runtime<State>, key: &str) -> f32 {
        element(runtime, key)
            .map(|element| element.opacity)
            .unwrap()
    }

    fn hinted_state() -> State {
        let mut state = State::new(1);
        state.started = true;
        state.cells[0].revealed = true;
        state.hint = Some(Hint {
            cell: 0,
            kind: HintKind::MineCount,
        });
        state
    }

    #[test]
    fn hint_pulses_with_configured_period_without_changing_other_cells() {
        let mut runtime = runtime(hinted_state());
        let initial_hint = opacity(&runtime, "cell-0");
        let initial_normal = opacity(&runtime, "cell-1");
        runtime.frame(HINT_PERIOD / 2.0);
        assert_ne!(opacity(&runtime, "cell-0"), initial_hint);
        assert_eq!(opacity(&runtime, "cell-1"), initial_normal);
        runtime.frame(HINT_PERIOD / 2.0);
        assert_eq!(opacity(&runtime, "cell-0"), initial_hint);
    }

    #[test]
    fn hint_is_exposed_in_accessibility_label() {
        let runtime = runtime(hinted_state());
        assert_eq!(label(&runtime, "cell-0"), "Row 1, column 1, clear · hint");
    }

    #[test]
    fn hint_labels_describe_double_click_and_safe_cells() {
        for (kind, revealed, description) in [
            (HintKind::DoubleClick, true, "clear · double-click hint"),
            (HintKind::Safe, false, "covered · safe cell hint"),
        ] {
            let mut state = hinted_state();
            state.cells[0].revealed = revealed;
            state.hint = Some(Hint { cell: 0, kind });
            let runtime = runtime(state);
            assert_eq!(
                label(&runtime, "cell-0"),
                format!("Row 1, column 1, {description}")
            );
        }
    }

    #[test]
    fn hint_is_a_static_tint_with_reduced_motion() {
        let mut runtime = runtime(hinted_state());
        runtime.set_system_reduced_motion(true);
        runtime.frame(2.0);
        let initial = fill(&runtime, "cell-0");
        let initial_opacity = opacity(&runtime, "cell-0");
        assert_ne!(initial, tok::theme(true).control_track);
        runtime.frame(4.0);
        assert_eq!(fill(&runtime, "cell-0"), initial);
        assert_eq!(opacity(&runtime, "cell-0"), initial_opacity);
    }

    #[test]
    fn left_click_reveals_a_cell() {
        let mut runtime = runtime(State::new(1));
        click(&mut runtime, "cell-40");
        assert!(runtime.state.cells[40].revealed);
    }

    #[test]
    fn right_click_toggles_a_flag() {
        let mut runtime = runtime(State::new(1));
        let (x, y) = runtime.find("cell-0").unwrap();
        runtime.secondary_down(x, y);
        runtime.secondary_up(x, y);
        runtime.frame(0.0);
        assert!(runtime.state.cells[0].flagged);
        assert_eq!(label(&runtime, "cell-0"), "Row 1, column 1, flagged");
        runtime.secondary_down(x, y);
        runtime.secondary_up(x, y);
        runtime.frame(0.0);
        assert!(!runtime.state.cells[0].flagged);
    }

    #[test]
    fn new_game_button_resets_the_game() {
        let mut runtime = runtime(State::new(1));
        click(&mut runtime, "cell-40");
        click(&mut runtime, "new-game");
        assert_eq!(runtime.state.settings.seed, 2);
        assert!(!runtime.state.started);
        assert!(
            runtime
                .state
                .cells
                .iter()
                .all(|cell| *cell == Cell::default())
        );
    }

    #[test]
    fn cell_labels_describe_state() {
        let mut state = State::new(1);
        state.cells[0].mine = true;
        state.cells[0].revealed = true;
        state.cells[1].revealed = true;
        state.cells[1].adjacent = 3;
        state.cells[2].flagged = true;
        state.cells[4].revealed = true;
        let runtime = runtime(state);
        assert_eq!("Row 1, column 1, mine", label(&runtime, "cell-0"));
        assert_eq!(
            label(&runtime, "cell-1"),
            "Row 1, column 2, 3 adjacent mines"
        );
        assert_eq!(label(&runtime, "cell-2"), "Row 1, column 3, flagged");
        assert_eq!(label(&runtime, "cell-3"), "Row 1, column 4, covered");
        assert_eq!(label(&runtime, "cell-4"), "Row 1, column 5, clear");
    }

    #[test]
    fn double_click_reveals_adjacent_cells() {
        let mut state = State::new(1);
        state.started = true;
        state.cells[30].mine = true;
        state.cells[30].flagged = true;
        state.cells[40].revealed = true;
        state.cells[40].adjacent = 1;
        let mut runtime = runtime(state);
        click(&mut runtime, "cell-40");
        click(&mut runtime, "cell-40");
        assert!(runtime.state.cells[31].revealed);
    }

    #[test]
    fn cell_colors_distinguish_covered_and_revealed_states() {
        let mut state = State::new(1);
        state.cells[0].revealed = true;
        state.cells[1].revealed = true;
        state.cells[1].adjacent = 1;
        state.cells[3].flagged = true;
        state.cells[4].mine = true;
        state.cells[4].revealed = true;
        let runtime = runtime(state);
        let elements = runtime.elements();
        let cell = |key: &str| {
            elements
                .iter()
                .position(|element| element.key.as_deref() == Some(key))
                .unwrap()
        };
        let fill = |key: &str| elements[cell(key)].fill;
        let text_color = |key: &str| {
            elements
                .iter()
                .find(|element| element.parent == Some(cell(key)))
                .unwrap()
                .text_color
        };
        let covered = tok::accent(0, true).alpha(COVERED_CELL_ALPHA);
        assert_eq!(fill("cell-2"), covered);
        assert_eq!(fill("cell-3"), covered);
        for key in ["cell-0", "cell-1", "cell-4"] {
            assert_eq!(fill(key), tok::accent(0, true).alpha(COVERED_CELL_ALPHA / 2.0));
        }
        assert_eq!(text_color("cell-4"), Some(VIBRANT_RED));
        assert_eq!(text_color("cell-3"), Some(Color::WHITE));
    }

    #[test]
    fn records_label_tracks_best_time() {
        let mut runtime = runtime(State::new(1));
        assert_eq!(text_value(&runtime, "record"), "Best —");
        win_at(&mut runtime.state, 12);
        runtime.update(|_, _| {});
        runtime.frame(0.0);
        assert_eq!(text_value(&runtime, "record"), "Best 0:12");
    }

    #[test]
    fn timer_starts_only_on_reveal_and_stops_at_game_end() {
        let mut runtime = runtime(State::new(1));
        let (x, y) = runtime.find("cell-0").unwrap();
        runtime.secondary_down(x, y);
        runtime.secondary_up(x, y);
        runtime.frame(0.0);
        assert_eq!(text_value(&runtime, "timer"), "Time 0:00");

        click(&mut runtime, "cell-40");
        runtime.frame(65.0);
        assert_eq!(text_value(&runtime, "timer"), "Time 1:05");
        let mine = (0..runtime.state.cells.len())
            .find(|&index| runtime.state.cells[index].mine)
            .unwrap();
        runtime.state.reveal(mine);
        runtime.frame(0.0);
        assert_eq!(text_value(&runtime, "timer"), "Time 1:05");
        runtime.frame(10.0);
        assert_eq!(text_value(&runtime, "timer"), "Time 1:05");
    }

    #[test]
    fn new_game_and_difficulty_reset_the_timer() {
        let mut runtime = runtime(State::new(1));
        click(&mut runtime, "cell-40");
        runtime.frame(12.0);
        assert_eq!(text_value(&runtime, "timer"), "Time 0:12");
        click(&mut runtime, "new-game");
        assert_eq!(text_value(&runtime, "timer"), "Time 0:00");
        click(&mut runtime, "cell-40");
        runtime.frame(5.0);
        click(&mut runtime, "difficulty-moderate");
        assert_eq!(text_value(&runtime, "timer"), "Time 0:00");
    }

    #[test]
    fn format_time_uses_minutes_and_padded_seconds() {
        assert_eq!(format_time(0), "0:00");
        assert_eq!(format_time(65), "1:05");
        assert_eq!(format_time(3600), "60:00");
    }

    #[test]
    fn window_fit_only_grows_insufficient_dimensions() {
        let easy = window_size(Difficulty::Easy);
        let hard = window_size(Difficulty::Hard);
        assert_eq!(fit_window_size(Difficulty::Hard, easy), Some(hard));
        assert_eq!(fit_window_size(Difficulty::Hard, (1440.0, 900.0)), None);
        assert_eq!(
            fit_window_size(Difficulty::Hard, (hard.0 - 1.0, 900.0)),
            Some((hard.0, 900.0))
        );
    }

    #[test]
    fn difficulty_buttons_start_a_fresh_selected_board() {
        let mut runtime = runtime(State::new(1));
        for (key, name) in [
            ("difficulty-easy", "Easy"),
            ("difficulty-moderate", "Moderate"),
            ("difficulty-hard", "Hard"),
        ] {
            assert_eq!(label(&runtime, key), name);
        }
        click(&mut runtime, "cell-40");
        runtime.state.cells[0].flagged = true;
        click(&mut runtime, "difficulty-moderate");
        assert_eq!(runtime.state.settings.difficulty, Difficulty::Moderate);
        assert_eq!(runtime.state.cells.len(), 256);
        assert_eq!(runtime.state.settings.seed, 2);
        assert!(!runtime.state.started);
        assert_eq!(runtime.state.outcome, Outcome::Playing);
        assert!(
            runtime
                .state
                .cells
                .iter()
                .all(|cell| *cell == Cell::default())
        );

        click(&mut runtime, "difficulty-moderate");
        assert_eq!(runtime.state.settings.seed, 2);
        click(&mut runtime, "difficulty-hard");
        assert_eq!(runtime.state.settings.difficulty, Difficulty::Hard);
        assert_eq!(runtime.state.cells.len(), 480);
        assert_eq!(runtime.state.settings.seed, 3);
    }
}
