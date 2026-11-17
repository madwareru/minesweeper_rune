use crate::model::{Difficulty, Outcome, State};
use rune::tok::{self, type_};
use rune::{Color, Cx, El, col, el, row, text};

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
    let board_width = difficulty.width() as f32 * 40.0 - 4.0;
    let board_height = difficulty.height() as f32 * 40.0 - 4.0;
    ((board_width + 32.0).max(420.0), board_height + 248.0)
}

fn button_base(key: impl Into<String>) -> El {
    el().role("button")
        .focusable(true)
        .center()
        .radius(6.0)
        .key(key)
}

fn cell_view(state: &State, index: usize, theme: tok::Theme) -> El {
    let cell = state.cells[index];
    let row_number = index / state.settings.difficulty.width() + 1;
    let column = index % state.settings.difficulty.width() + 1;
    let (description, content, color) = match (cell.revealed, cell.mine, cell.flagged) {
        (true, true, _) => ("mine".to_owned(), "●".to_owned(), VIBRANT_RED),
        (true, _, _) if cell.adjacent == 0 => ("clear".to_owned(), "".to_owned(), theme.text),
        (true, _, _) => (
            format!("{} adjacent mines", cell.adjacent),
            format!("{}", cell.adjacent),
            tok::accent((cell.adjacent as usize + 6 - 1) % 6, true),
        ),
        (_, _, true) => ("flagged".to_owned(), "⚑".to_owned(), Color::WHITE),
        _ => ("covered".to_owned(), "".to_owned(), theme.text),
    };
    button_base(format!("cell-{index}"))
        .label(format!("Row {row_number}, column {column}, {description}"))
        .size(36.0, 36.0)
        .fill(if cell.revealed {
            theme.control_track
        } else {
            theme.control_track.mix(Color::WHITE, 0.05)
        })
        .on_press_tx(move |state: &mut State, tx| {
            state.clock = tx.clock();
            state.reveal(index);
        })
        .on_double_press_tx(move |state: &mut State, tx| {
            state.clock = tx.clock();
            state.reveal_adjacent(index);
        })
        .on_secondary_press(move |state: &mut State, _, _| state.toggle_flag(index))
        .child(text(content).font(type_::TITLE).color(color))
}

pub(crate) fn view(state: &State, cx: &mut Cx<State>) -> El {
    let theme = tok::theme(true);
    let resize_window = !cfg!(target_os = "macos") || cx.titlebar_inset() > 0.0;
    let flags = state.cells.iter().copied().filter(|it| it.flagged).count();
    let remaining = state.settings.difficulty.mines() as isize - flags as isize;
    let status = match state.outcome {
        Outcome::Playing if !state.started => format!("{remaining} mines · Left-click to start"),
        Outcome::Playing => format!("{remaining} mines left"),
        Outcome::Won => "You won! · All safe cells revealed".to_owned(),
        Outcome::Lost => "Boom! · Try a new game".to_owned(),
    };
    let difficulties = row().gap(8.0).children(Difficulty::ALL.map(|difficulty| {
        button_base(format!("difficulty-{}", difficulty.name().to_lowercase()))
            .label(difficulty.name())
            .size(100.0, 36.0)
            .fill(if state.settings.difficulty == difficulty {
                tok::accent(0, true).alpha(0.2)
            } else {
                theme.card
            })
            .on_press_tx(move |state: &mut State, tx| {
                state.set_difficulty(difficulty);
                if resize_window {
                    tx.resize_window(window_size(difficulty));
                } else {
                    state.window_resize_pending = true;
                }
            })
            .child(text(difficulty.name()).font(type_::LABEL).color(theme.text))
    }));
    let difficulty = state.settings.difficulty;
    let board = col().gap(4.0).children((0..difficulty.height()).map(|y| {
        row().gap(4.0).children(
            (0..difficulty.width())
                .map(move |x| cell_view(state, y * difficulty.width() + x, theme)),
        )
    }));
    let timer = text(format!("Time {}", format_time(state.elapsed_seconds())))
        .key("timer")
        .font(type_::LABEL.mono())
        .color(theme.text_secondary);
    let record = state.records[difficulty.index()]
        .map(|seconds| format!("Best {}", format_time(seconds)))
        .unwrap_or_else(|| "Best —".to_owned());
    let record = text(record).key("record")
        .font(type_::LABEL.mono())
        .color(theme.text_secondary);
    let new_game = button_base("new-game")
        .label("New Game")
        .size(120.0, 36.0)
        .fill(theme.card)
        .on_press(|state: &mut State| state.new_game())
        .child(text("New Game").font(type_::LABEL).color(theme.text));

    col()
        .w_full()
        .h_full()
        .center()
        .gap(16.0)
        .padding(16.0)
        .fill(theme.bg)
        .on_layout_tx(move |state: &mut State, _, tx| {
            if resize_window && state.window_resize_pending {
                state.window_resize_pending = false;
                tx.resize_window(window_size(state.settings.difficulty));
            }
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
                .gap(16.0)
                .center()
                .child(timer)
                .child(new_game)
                .child(record),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Cell;
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

    fn label(runtime: &Runtime<State>, key: &str) -> String {
        runtime
            .elements()
            .into_iter()
            .find(|element| element.key.as_deref() == Some(key))
            .and_then(|element| element.label)
            .unwrap_or_else(|| "not_found".to_owned())
    }

    fn text_value(runtime: &Runtime<State>, key: &str) -> String {
        runtime
            .elements()
            .into_iter()
            .find(|element| element.key.as_deref() == Some(key))
            .and_then(|element| element.text)
            .unwrap_or_else(|| "not_found".to_owned())
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
        let covered = tok::theme(true).control_track.mix(Color::WHITE, 0.05);
        assert_eq!(fill("cell-2"), covered);
        assert_eq!(fill("cell-3"), covered);
        for key in ["cell-0", "cell-1", "cell-4"] {
            assert_eq!(fill(key), tok::theme(true).control_track);
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

    #[cfg(target_os = "macos")]
    #[test]
    fn leaving_fullscreen_applies_pending_difficulty_resize() {
        let mut runtime = runtime(State::new(1));
        runtime.set_titlebar_inset(0.0);
        runtime.set_viewport((1440.0, 900.0), 2.0);
        runtime.frame(0.0);
        click(&mut runtime, "difficulty-hard");
        assert!(runtime.state.window_resize_pending);

        runtime.set_titlebar_inset(30.0);
        runtime.set_viewport(window_size(Difficulty::Easy), 2.0);
        runtime.frame(0.0);
        runtime.frame(0.0);
        assert!(!runtime.state.window_resize_pending);
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
