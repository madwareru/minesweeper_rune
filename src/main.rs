use rune::prefs::Prefs;
use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, col, el, row, text};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Difficulty {
    Easy,
    Moderate,
    Hard,
}

impl Difficulty {
    const ALL: [Self; 3] = [Self::Easy, Self::Moderate, Self::Hard];

    fn width(self) -> usize {
        match self {
            Self::Easy => 9,
            Self::Moderate => 16,
            Self::Hard => 30,
        }
    }

    fn height(self) -> usize {
        match self {
            Self::Easy => 9,
            Self::Moderate | Self::Hard => 16,
        }
    }

    fn mines(self) -> usize {
        match self {
            Self::Easy => 10,
            Self::Moderate => 40,
            Self::Hard => 99,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Moderate => "Moderate",
            Self::Hard => "Hard",
        }
    }

    fn record_key(self) -> &'static str {
        match self {
            Self::Easy => "best_easy",
            Self::Moderate => "best_moderate",
            Self::Hard => "best_hard",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Cell {
    mine: bool,
    revealed: bool,
    flagged: bool,
    adjacent: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Playing,
    Won,
    Lost,
}

struct State {
    difficulty: Difficulty,
    cells: Vec<Cell>,
    started: bool,
    outcome: Outcome,
    seed: u64,
    clock: f64,
    started_at: Option<f64>,
    finished_at: Option<f64>,
    records: [Option<u64>; 3],
    prefs: Prefs,
    window_resize_pending: bool,
    /// used to avoid heap allocations for temporary vectors
    tmp_buffer: Vec<usize>,
}

impl State {
    #[cfg(test)]
    fn new(seed: u64) -> Self {
        Self::with_prefs(seed, Prefs::memory())
    }

    fn with_prefs(seed: u64, prefs: Prefs) -> Self {
        let records = Difficulty::ALL.map(|difficulty| prefs.get(difficulty.record_key()));
        Self {
            difficulty: Difficulty::Easy,
            cells: vec![Cell::default(); Difficulty::Easy.width() * Difficulty::Easy.height()],
            started: false,
            outcome: Outcome::Playing,
            seed,
            clock: 0.0,
            started_at: None,
            finished_at: None,
            records,
            prefs,
            window_resize_pending: false,
            tmp_buffer: vec![],
        }
    }

    fn elapsed_seconds(&self) -> u64 {
        self.started_at
            .map(|started_at| (self.finished_at.unwrap_or(self.clock) - started_at).max(0.0) as u64)
            .unwrap_or(0)
    }

    fn reveal(&mut self, index: usize) {
        if self.outcome != Outcome::Playing {
            return;
        }
        if index >= self.cells.len() {
            return;
        }
        let self1 = &self.cells[index];
        if self1.revealed || self1.flagged {
            return;
        }

        if !self.started {
            self.started_at = Some(self.clock);
            self.place_mines(index);
            self.started = true;
        }

        if self.check_loss(index) {
            return;
        }

        self.reveal_area_at(index);

        self.check_for_win();
    }

    fn check_for_win(&mut self) {
        let cells_revealed = self
            .cells
            .iter()
            .filter(|cell| cell.revealed && !cell.mine)
            .count();
        let cells_to_reveal_for_win = self.cells.len() - self.difficulty.mines();

        if cells_revealed == cells_to_reveal_for_win {
            self.outcome = Outcome::Won;
            self.finished_at = Some(self.clock);
            let seconds = self.elapsed_seconds();
            let index = self.difficulty.index();
            if self.records[index].is_none_or(|best| seconds < best) {
                self.records[index] = Some(seconds);
                let _ = self.prefs.set(self.difficulty.record_key(), &seconds);
            }
        }
    }

    fn reveal_area_at(&mut self, index: usize) {
        self.tmp_buffer.clear();
        self.tmp_buffer.push(index);

        while let Some(current) = self.tmp_buffer.pop() {
            let cell = &mut self.cells[current];
            if cell.revealed || cell.flagged || cell.mine {
                continue;
            }

            cell.revealed = true;
            if cell.adjacent == 0 {
                self.tmp_buffer.extend(neighbors(self.difficulty, current));
            }
        }
    }

    fn check_loss(&mut self, index: usize) -> bool {
        if !self.cells[index].mine {
            return false;
        }

        for mine_cell in self.cells.iter_mut().filter(|cell| cell.mine) {
            mine_cell.revealed = true;
        }

        self.outcome = Outcome::Lost;
        self.finished_at = Some(self.clock);
        true
    }

    fn reveal_adjacent(&mut self, index: usize) {
        if index >= self.cells.len() || self.outcome != Outcome::Playing {
            return;
        }

        let cell = self.cells[index];
        if !cell.revealed || cell.mine || cell.adjacent == 0 {
            return;
        }

        let flags = neighbors(self.difficulty, index)
            .filter(|&neighbor| self.cells[neighbor].flagged)
            .count() as u8;

        if flags != cell.adjacent {
            return;
        }

        for neighbor in neighbors(self.difficulty, index) {
            self.reveal(neighbor);
        }
    }

    fn toggle_flag(&mut self, index: usize) {
        if index < self.cells.len()
            && self.outcome == Outcome::Playing
            && !self.cells[index].revealed
        {
            self.cells[index].flagged = !self.cells[index].flagged;
        }
    }

    fn new_game(&mut self) {
        self.cells = vec![Cell::default(); self.difficulty.width() * self.difficulty.height()];
        self.started = false;
        self.outcome = Outcome::Playing;
        self.seed = self.seed.wrapping_add(1);
        self.started_at = None;
        self.finished_at = None;
    }

    fn set_difficulty(&mut self, difficulty: Difficulty) {
        if self.difficulty == difficulty {
            return;
        }
        self.difficulty = difficulty;
        self.new_game();
    }

    fn place_mines(&mut self, first: usize) {
        self.tmp_buffer.clear();
        for i in 0..self.cells.len() {
            if i == first || neighbors(self.difficulty, first).any(|neighbor| i == neighbor) {
                continue;
            }

            self.tmp_buffer.push(i);
        }

        let mut random = self.seed;
        for index in 0..self.difficulty.mines() {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let other = index + random as usize % (self.tmp_buffer.len() - index);
            self.tmp_buffer.swap(index, other);
            self.cells[self.tmp_buffer[index]].mine = true;
        }

        for index in 0..self.cells.len() {
            let adjacent = neighbors(self.difficulty, index)
                .filter(|&neighbor| self.cells[neighbor].mine)
                .count() as u8;
            self.cells[index].adjacent = adjacent;
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::with_prefs(0, Prefs::open("com.madware.minesweeper"))
    }
}

fn neighbors(difficulty: Difficulty, index: usize) -> impl Iterator<Item = usize> {
    let width = difficulty.width();
    let height = difficulty.height();
    let row = (index / width) as isize;
    let col = (index % width) as isize;
    (-1..=1).flat_map(move |dy| {
        (-1..=1).filter_map(move |dx| {
            if dx == 0 && dy == 0 {
                return None;
            }
            let y = row + dy;
            let x = col + dx;
            (y >= 0 && y < height as isize && x >= 0 && x < width as isize)
                .then(|| y as usize * width + x as usize)
        })
    })
}

fn format_time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn poll(state: &mut State, now: f64, tx: &mut rune::Tx) -> bool {
    state.clock = now;
    if state.started && state.outcome == Outcome::Playing {
        tx.wake_after(std::time::Duration::from_secs(1));
        true
    } else {
        false
    }
}

fn window_size(difficulty: Difficulty) -> (f32, f32) {
    let board_width = difficulty.width() as f32 * 40.0 - 4.0;
    let board_height = difficulty.height() as f32 * 40.0 - 4.0;
    ((board_width + 32.0).max(420.0), board_height + 248.0)
}

fn should_resize_window(titlebar_inset: f32) -> bool {
    !cfg!(target_os = "macos") || titlebar_inset > 0.0
}

fn cell_view(state: &State, index: usize, theme: tok::Theme) -> El {
    let cell = state.cells[index];
    let row_number = index / state.difficulty.width() + 1;
    let column = index % state.difficulty.width() + 1;
    let description = if cell.revealed && cell.mine {
        "mine".to_owned()
    } else if cell.revealed && cell.adjacent > 0 {
        format!("{} adjacent mines", cell.adjacent)
    } else if cell.revealed {
        "clear".to_owned()
    } else if cell.flagged {
        "flagged".to_owned()
    } else {
        "covered".to_owned()
    };
    let content = if cell.revealed && cell.mine {
        "●".to_owned()
    } else if cell.revealed && cell.adjacent > 0 {
        cell.adjacent.to_string()
    } else if cell.flagged && !cell.revealed {
        "⚑".to_owned()
    } else {
        String::new()
    };
    let color = if cell.revealed && cell.mine {
        rune::Color {
            r: 1.0,
            g: 0.25,
            b: 0.2,
            a: 1.0,
        }
    } else if cell.flagged {
        rune::Color::WHITE
    } else if cell.revealed && cell.adjacent > 0 {
        tok::accent((cell.adjacent as usize - 1) % 6, true)
    } else {
        theme.text
    };
    el().key(format!("cell-{index}"))
        .role("button")
        .label(format!("Row {row_number}, column {column}, {description}"))
        .focusable(true)
        .size(36.0, 36.0)
        .center()
        .radius(6.0)
        .fill(if cell.revealed {
            theme.control_track
        } else {
            theme.control_track.mix(tok::accent(0, true), 0.1)
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

fn view(state: &State, cx: &mut Cx<State>) -> El {
    let theme = tok::theme(true);
    let resize_window = should_resize_window(cx.titlebar_inset());
    let flags = state.cells.iter().filter(|cell| cell.flagged).count();
    let remaining = state.difficulty.mines() as isize - flags as isize;
    let status = match state.outcome {
        Outcome::Playing if !state.started => format!("{remaining} mines · Left-click to start"),
        Outcome::Playing => format!("{remaining} mines left"),
        Outcome::Won => "You won! · All safe cells revealed".to_owned(),
        Outcome::Lost => "Boom! · Try a new game".to_owned(),
    };
    let difficulties = row().gap(8.0).children(Difficulty::ALL.map(|difficulty| {
        el().key(format!("difficulty-{}", difficulty.name().to_lowercase()))
            .role("button")
            .label(difficulty.name())
            .focusable(true)
            .size(100.0, 36.0)
            .center()
            .radius(6.0)
            .fill(if state.difficulty == difficulty {
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
    let difficulty = state.difficulty;
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
    let record = state.records[state.difficulty.index()];
    let record = text(match record {
        Some(seconds) => format!("Best {}", format_time(seconds)),
        None => "Best —".to_owned(),
    })
    .key("record")
    .font(type_::LABEL.mono())
    .color(theme.text_secondary);
    let new_game = el()
        .key("new-game")
        .role("button")
        .label("New Game")
        .focusable(true)
        .size(120.0, 36.0)
        .center()
        .radius(6.0)
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
                tx.resize_window(window_size(state.difficulty));
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

fn main() {
    const FIRST_TO_REVEAL: usize = 40;
    rune::run(AppSpec::<State> {
        poll: Some(poll),
        scenes: &[
            ("revealed", |state| state.reveal(FIRST_TO_REVEAL)),
            ("flagged", |state| state.toggle_flag(0)),
            ("lost", |state| {
                state.reveal(FIRST_TO_REVEAL);
                for i in 0..state.cells.len() {
                    if state.cells[i].mine {
                        state.reveal(i);
                        return;
                    }
                }
            }),
            ("won", |state| {
                state.reveal(FIRST_TO_REVEAL);
                for index in 0..state.cells.len() {
                    if !state.cells[index].mine {
                        state.reveal(index);
                    }
                }
            }),
        ],
        ..AppSpec::new(
            "Minesweeper",
            window_size(Difficulty::Easy),
            State::default,
            view,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    fn runtime(state: State) -> Runtime<State> {
        let size = window_size(state.difficulty);
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

    fn win_at(state: &mut State, seconds: u64) {
        let mines = state.difficulty.mines();
        for (index, cell) in state.cells.iter_mut().enumerate() {
            *cell = Cell {
                mine: index < mines,
                revealed: index >= mines,
                ..Cell::default()
            };
        }
        let target = state.cells.len() - 1;
        state.cells[target].revealed = false;
        state.started = true;
        state.started_at = Some(0.0);
        state.finished_at = None;
        state.clock = seconds as f64;
        state.reveal(target);
        assert_eq!(state.outcome, Outcome::Won);
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
        assert_eq!(runtime.state.seed, 2);
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
    fn reveal_adjacent_requires_a_matching_flag_count() {
        let mut state = State::new(1);
        state.started = true;
        state.cells[30].mine = true;
        state.cells[40].revealed = true;
        state.cells[40].adjacent = 1;
        let before = state.cells.clone();
        state.reveal_adjacent(40);
        assert_eq!(state.cells, before);
        state.cells[30].flagged = true;
        state.reveal_adjacent(40);
        assert!(state.cells[31].revealed);
        assert!(state.cells[30].flagged);
        assert!(!state.cells[30].revealed);
    }

    #[test]
    fn reveal_adjacent_validates_the_target() {
        let mut state = State::new(1);
        state.started = true;
        state.cells[30].flagged = true;
        state.cells[40].adjacent = 1;
        let before = state.cells.clone();
        state.reveal_adjacent(40);
        assert_eq!(state.cells, before);

        state.cells[40].revealed = true;
        state.cells[40].mine = true;
        let before = state.cells.clone();
        state.reveal_adjacent(40);
        assert_eq!(state.cells, before);

        state.cells[40] = Cell {
            revealed: true,
            ..Cell::default()
        };
        state.cells[30].flagged = false;
        let before = state.cells.clone();
        state.reveal_adjacent(40);
        assert_eq!(state.cells, before);

        state.cells[40].adjacent = 1;
        state.cells[30].flagged = true;
        state.outcome = Outcome::Won;
        let before = state.cells.clone();
        state.reveal_adjacent(40);
        assert_eq!(state.cells, before);
        state.reveal_adjacent(state.cells.len());
        assert_eq!(state.cells, before);
    }

    #[test]
    fn reveal_adjacent_trusts_flag_count_not_flag_correctness() {
        let mut state = State::new(1);
        state.started = true;
        state.cells[30].mine = true;
        state.cells[31].flagged = true;
        state.cells[40].revealed = true;
        state.cells[40].adjacent = 1;
        state.reveal_adjacent(40);
        assert!(state.cells[30].revealed);
        assert_eq!(state.outcome, Outcome::Lost);
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
        let covered = tok::theme(true)
            .control_track
            .mix(tok::accent(0, true), 0.1);
        assert_eq!(fill("cell-2"), covered);
        assert_eq!(fill("cell-3"), covered);
        for key in ["cell-0", "cell-1", "cell-4"] {
            assert_eq!(fill(key), tok::theme(true).control_track);
        }
        assert_eq!(
            text_color("cell-4"),
            Some(rune::Color {
                r: 1.0,
                g: 0.25,
                b: 0.2,
                a: 1.0,
            })
        );
        assert_eq!(text_color("cell-3"), Some(rune::Color::WHITE));
    }

    #[test]
    fn records_are_per_mode_and_only_improve() {
        let mut state = State::new(1);
        win_at(&mut state, 12);
        assert_eq!(state.records[Difficulty::Easy.index()], Some(12));
        state.new_game();
        win_at(&mut state, 20);
        assert_eq!(state.records[Difficulty::Easy.index()], Some(12));

        state.set_difficulty(Difficulty::Moderate);
        assert_eq!(state.records[Difficulty::Moderate.index()], None);
        win_at(&mut state, 30);
        assert_eq!(state.records[Difficulty::Moderate.index()], Some(30));
        assert_eq!(state.records[Difficulty::Easy.index()], Some(12));

        state.new_game();
        state.started = true;
        state.started_at = Some(0.0);
        state.clock = 40.0;
        state.cells[0].mine = true;
        state.reveal(0);
        assert_eq!(state.outcome, Outcome::Lost);
        assert_eq!(state.records[Difficulty::Moderate.index()], Some(30));
    }

    #[test]
    fn records_reload_from_preferences() {
        let dir =
            std::env::temp_dir().join(format!("minesweeper-records-test-{}", std::process::id()));
        let path = dir.join("prefs.json");
        let mut state = State::with_prefs(1, Prefs::at(&path));
        state.set_difficulty(Difficulty::Hard);
        win_at(&mut state, 42);
        let state = State::with_prefs(1, Prefs::at(&path));
        assert_eq!(state.records[Difficulty::Hard.index()], Some(42));
        std::fs::remove_dir_all(dir).unwrap();
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
    fn difficulty_resize_only_skips_macos_fullscreen() {
        assert_eq!(should_resize_window(0.0), !cfg!(target_os = "macos"));
        assert!(should_resize_window(30.0));
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
        assert_eq!(runtime.state.difficulty, Difficulty::Moderate);
        assert_eq!(runtime.state.cells.len(), 256);
        assert_eq!(runtime.state.seed, 2);
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
        assert_eq!(runtime.state.seed, 2);
        click(&mut runtime, "difficulty-hard");
        assert_eq!(runtime.state.difficulty, Difficulty::Hard);
        assert_eq!(runtime.state.cells.len(), 480);
        assert_eq!(runtime.state.seed, 3);
    }

    #[test]
    fn difficulty_presets_drive_board_rules() {
        for (difficulty, width, height, mines) in [
            (Difficulty::Easy, 9, 9, 10),
            (Difficulty::Moderate, 16, 16, 40),
            (Difficulty::Hard, 30, 16, 99),
        ] {
            assert_eq!(difficulty.width(), width);
            assert_eq!(difficulty.height(), height);
            assert_eq!(difficulty.mines(), mines);
            let mut state = State::new(7);
            state.difficulty = difficulty;
            state.cells = vec![Cell::default(); width * height];
            let first = (height / 2) * width + width / 2;
            state.reveal(first);
            assert!(state.cells[first].revealed);
            assert!(!state.cells[first].mine);
            assert_eq!(state.cells[first].adjacent, 0);
            assert!(neighbors(difficulty, first).all(|index| !state.cells[index].mine));
            assert_eq!(state.cells.iter().filter(|cell| cell.mine).count(), mines);
            for index in 0..state.cells.len() {
                let expected = neighbors(difficulty, index)
                    .filter(|&neighbor| state.cells[neighbor].mine)
                    .count() as u8;
                assert_eq!(state.cells[index].adjacent, expected);
            }
        }
        assert_eq!(
            neighbors(Difficulty::Hard, 0).collect::<Vec<_>>(),
            [1, 30, 31]
        );
        assert_eq!(
            neighbors(Difficulty::Hard, 29).collect::<Vec<_>>(),
            [28, 58, 59]
        );
        assert_eq!(
            neighbors(Difficulty::Hard, 450).collect::<Vec<_>>(),
            [420, 421, 451]
        );
    }

    #[test]
    fn first_click_is_safe() {
        for first in [0, 4, 40, 80] {
            let mut state = State::new(7);
            state.reveal(first);
            assert!(state.cells[first].revealed);
            assert!(!state.cells[first].mine);
            assert_eq!(state.cells[first].adjacent, 0);
            assert!(neighbors(state.difficulty, first).all(|index| !state.cells[index].mine));
        }
    }

    #[test]
    fn mine_and_adjacency_counts_are_correct() {
        let mut state = State::new(11);
        state.reveal(40);
        assert_eq!(
            state.cells.iter().filter(|cell| cell.mine).count(),
            state.difficulty.mines()
        );
        for index in 0..state.cells.len() {
            let expected = neighbors(state.difficulty, index)
                .filter(|&neighbor| state.cells[neighbor].mine)
                .count() as u8;
            assert_eq!(state.cells[index].adjacent, expected);
        }
    }

    #[test]
    fn flagging_guards_reveal() {
        let mut state = State::new(1);
        state.reveal(40);
        let index = (0..state.cells.len())
            .find(|&index| !state.cells[index].mine && !state.cells[index].revealed)
            .unwrap();
        state.toggle_flag(index);
        assert!(state.cells[index].flagged);
        state.reveal(index);
        assert!(!state.cells[index].revealed);
        state.toggle_flag(index);
        assert!(!state.cells[index].flagged);
    }

    #[test]
    fn empty_reveal_floods_and_respects_flags() {
        let mut state = State::new(1);
        state.started = true;
        state.cells[8].mine = true;
        for index in 0..state.cells.len() {
            state.cells[index].adjacent = neighbors(state.difficulty, index)
                .filter(|&neighbor| state.cells[neighbor].mine)
                .count() as u8;
        }
        state.toggle_flag(40);
        state.reveal(0);
        assert!(state.cells[7].revealed);
        assert_eq!(state.cells[7].adjacent, 1);
        assert!(!state.cells[8].revealed);
        assert!(state.cells[40].flagged);
        assert!(!state.cells[40].revealed);
        assert_eq!(state.cells.iter().filter(|cell| cell.revealed).count(), 79);
    }

    #[test]
    fn revealing_all_safe_cells_wins() {
        for difficulty in Difficulty::ALL {
            let mut state = State::new(3);
            state.difficulty = difficulty;
            state.cells = vec![Cell::default(); difficulty.width() * difficulty.height()];
            let first = (difficulty.height() / 2) * difficulty.width() + difficulty.width() / 2;
            state.reveal(first);
            for index in 0..state.cells.len() {
                if !state.cells[index].mine {
                    state.reveal(index);
                }
            }
            assert_eq!(state.outcome, Outcome::Won);
            assert_eq!(
                state.cells.iter().filter(|cell| cell.revealed).count(),
                state.cells.len() - difficulty.mines()
            );
        }
    }

    #[test]
    fn finished_game_ignores_cell_input() {
        for outcome in [Outcome::Won, Outcome::Lost] {
            let mut state = State::new(5);
            state.outcome = outcome;
            let before = state.cells.clone();
            state.reveal(40);
            state.toggle_flag(40);
            assert_eq!(state.cells, before);
        }
    }

    #[test]
    fn new_game_resets_the_board() {
        let mut state = State::new(9);
        state.reveal(40);
        let mine = (0..state.cells.len())
            .find(|&index| state.cells[index].mine)
            .unwrap();
        state.reveal(mine);
        state.toggle_flag(0);
        state.new_game();
        assert_eq!(state.seed, 10);
        assert!(!state.started);
        assert!(matches!(state.outcome, Outcome::Playing));
        assert!(state.cells.iter().all(|cell| *cell == Cell::default()));
    }
}
