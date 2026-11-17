use crate::model::{Cell, Difficulty, Hint, HintKind, Outcome, State};

const HINT_DELAY: f64 = 15.0;

impl State {
    pub(crate) fn reveal(&mut self, index: usize) {
        if self.outcome != Outcome::Playing {
            return;
        }
        if index >= self.cells.len() {
            return;
        }
        let cell = &self.cells[index];
        if cell.revealed || cell.flagged {
            return;
        }

        if !self.started {
            self.started_at = Some(self.clock);
            self.place_mines(index);
            self.started = true;
        }

        self.note_progress();

        if self.check_loss(index) {
            return;
        }

        self.reveal_area_at(index);

        self.check_for_win();
    }

    fn note_progress(&mut self) {
        self.last_progress_at = Some(self.clock);
        self.hint = None;
    }

    pub(crate) fn update_hint(&mut self) {
        if !self.started || self.outcome != Outcome::Playing {
            self.hint = None;
            return;
        }
        if self.hint.is_some()
            || self
                .last_progress_at
                .is_none_or(|last| self.clock - last < HINT_DELAY)
        {
            return;
        }

        self.hint = self.hint_candidate();
    }

    fn hint_candidate(&self) -> Option<Hint> {
        let candidates: [(fn(&State) -> Option<usize>, HintKind); 3] = [
            (Self::mine_count_hint, HintKind::MineCount),
            (Self::double_click_hint, HintKind::DoubleClick),
            (Self::safe_cell_hint, HintKind::Safe),
        ];
        candidates
            .into_iter()
            .find_map(|(candidate, kind)| candidate(self).map(|cell| Hint { cell, kind }))
    }

    fn mine_count_hint(&self) -> Option<usize> {
        self.cells
            .iter()
            .enumerate()
            .find(|(index, cell)| {
                if !cell.revealed || cell.mine || cell.adjacent == 0 {
                    return false;
                }

                let mut covered = 0;
                let mut has_unflagged = false;
                for neighbor in neighbors(self.settings.difficulty, *index) {
                    let neighbor = self.cells[neighbor];
                    if !neighbor.revealed {
                        covered += 1;
                        has_unflagged |= !neighbor.flagged;
                    }
                }
                has_unflagged && covered == cell.adjacent
            })
            .map(|(index, _)| index)
    }

    fn double_click_hint(&self) -> Option<usize> {
        self.cells
            .iter()
            .enumerate()
            .find(|(index, cell)| {
                if !cell.revealed || cell.mine || cell.adjacent == 0 {
                    return false;
                }

                let mut flags = 0;
                let mut has_unflagged = false;
                for neighbor in neighbors(self.settings.difficulty, *index) {
                    let neighbor = self.cells[neighbor];
                    flags += u8::from(neighbor.flagged);
                    has_unflagged |= !neighbor.revealed && !neighbor.flagged;
                }
                flags == cell.adjacent && has_unflagged
            })
            .map(|(index, _)| index)
    }

    fn safe_cell_hint(&self) -> Option<usize> {
        self.cells
            .iter()
            .enumerate()
            .filter(|(index, cell)| {
                !cell.revealed
                    && !cell.mine
                    && !cell.flagged
                    && neighbors(self.settings.difficulty, *index).any(|neighbor| {
                        let neighbor = self.cells[neighbor];
                        neighbor.revealed && !neighbor.mine
                    })
            })
            .max_by_key(|(index, cell)| {
                let covered = neighbors(self.settings.difficulty, *index)
                    .filter(|neighbor| !self.cells[*neighbor].revealed)
                    .count();
                let revealed = neighbors(self.settings.difficulty, *index)
                    .filter(|neighbor| {
                        let neighbor = self.cells[*neighbor];
                        neighbor.revealed && !neighbor.mine
                    })
                    .count();
                (covered == usize::from(cell.adjacent), revealed)
            })
            .map(|(index, _)| index)
    }

    fn check_for_win(&mut self) {
        let cells_revealed = self
            .cells
            .iter()
            .filter(|cell| cell.revealed && !cell.mine)
            .count();
        let cells_to_reveal_for_win = self.cells.len() - self.settings.difficulty.mines();

        if cells_revealed == cells_to_reveal_for_win {
            self.outcome = Outcome::Won;
            self.finished_at = Some(self.clock);
            let seconds = self.elapsed_seconds();
            let index = self.settings.difficulty.index();
            if self.records[index].is_none_or(|best| seconds < best) {
                self.records[index] = Some(seconds);
                let _ = self
                    .prefs
                    .set(self.settings.difficulty.record_key(), &seconds);
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
                self.tmp_buffer
                    .extend(neighbors(self.settings.difficulty, current));
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

    pub(crate) fn reveal_adjacent(&mut self, index: usize) {
        if index >= self.cells.len() || self.outcome != Outcome::Playing {
            return;
        }

        let cell = self.cells[index];
        if !cell.revealed || cell.mine || cell.adjacent == 0 {
            return;
        }

        let flags = neighbors(self.settings.difficulty, index)
            .map(|neighbor| self.cells[neighbor])
            .filter(|it| it.flagged)
            .count() as u8;

        if flags != cell.adjacent {
            return;
        }

        for neighbor in neighbors(self.settings.difficulty, index) {
            self.reveal(neighbor);
        }
    }

    pub(crate) fn toggle_flag(&mut self, index: usize) {
        if index < self.cells.len()
            && self.outcome == Outcome::Playing
            && !self.cells[index].revealed
        {
            self.cells[index].switch_flag();
            self.note_progress();
        }
    }

    pub(crate) fn new_game(&mut self) {
        self.cells = vec![
            Cell::default();
            self.settings.difficulty.width() * self.settings.difficulty.height()
        ];
        self.started = false;
        self.outcome = Outcome::Playing;
        self.settings.seed = self.settings.seed.wrapping_add(1);
        self.started_at = None;
        self.finished_at = None;
        self.last_progress_at = None;
        self.hint = None;
    }

    pub(crate) fn set_difficulty(&mut self, difficulty: Difficulty) {
        if self.settings.difficulty == difficulty {
            return;
        }
        self.settings.difficulty = difficulty;
        self.new_game();
    }

    fn place_mines(&mut self, first: usize) {
        self.tmp_buffer.clear();
        for i in 0..self.cells.len() {
            if i == first
                || neighbors(self.settings.difficulty, first).any(|neighbor| i == neighbor)
            {
                continue;
            }

            self.tmp_buffer.push(i);
        }

        let mut random = self.settings.seed;
        for index in 0..self.settings.difficulty.mines() {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let other = index + random as usize % (self.tmp_buffer.len() - index);
            self.tmp_buffer.swap(index, other);
            let cell_index = self.tmp_buffer[index];
            self.cells[cell_index].mine = true;
        }

        for index in 0..self.cells.len() {
            let adjacent = neighbors(self.settings.difficulty, index)
                .filter(|&neighbor| self.cells[neighbor].mine)
                .count() as u8;
            self.cells[index].adjacent = adjacent;
        }
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

#[cfg(test)]
pub(crate) fn win_at(state: &mut State, seconds: u64) {
    let mines = state.settings.difficulty.mines();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::HintKind;
    use rune::prefs::Prefs;

    fn inactive_state() -> State {
        let mut state = State::new(1);
        state.started = true;
        state.last_progress_at = Some(0.0);
        state.clock = 15.0;
        state
    }

    fn hint_state() -> State {
        let mut state = inactive_state();
        for neighbor in neighbors(Difficulty::Easy, 40) {
            state.cells[neighbor].revealed = true;
        }
        state.cells[40].revealed = true;
        state.cells[40].adjacent = 1;
        state.cells[30].revealed = false;
        state
    }

    #[test]
    fn hint_appears_at_the_inactivity_threshold_using_visible_information() {
        let mut state = hint_state();
        state.clock = 14.999;
        state.update_hint();
        assert!(state.hint.is_none());

        state.clock = 15.0;
        state.update_hint();
        let hint = state.hint.as_ref().unwrap();
        assert_eq!(hint.cell, 40);
        assert_eq!(hint.kind, HintKind::MineCount);
        assert!(state.cells.iter().all(|cell| !cell.mine));
    }

    #[test]
    fn hint_candidates_count_flags_but_skip_fully_flagged_numbers() {
        let mut state = inactive_state();
        state.cells.fill(Cell {
            revealed: true,
            ..Cell::default()
        });
        state.cells[40].adjacent = 1;
        state.cells[41].adjacent = 1;
        state.cells[30].revealed = false;
        state.cells[30].flagged = true;
        state.cells[42].revealed = false;

        state.update_hint();
        assert_eq!(state.hint.as_ref().map(|hint| hint.cell), Some(41));
    }

    #[test]
    fn hint_falls_back_to_an_actionable_double_click() {
        let mut state = inactive_state();
        state.cells[40].revealed = true;
        state.cells[40].adjacent = 1;
        state.cells[30].flagged = true;

        state.update_hint();
        let hint = state.hint.as_ref().unwrap();
        assert_eq!(hint.cell, 40);
        assert_eq!(hint.kind, HintKind::DoubleClick);
    }

    #[test]
    fn hint_double_click_requires_an_unflagged_covered_cell() {
        let mut state = inactive_state();
        for neighbor in neighbors(Difficulty::Easy, 40) {
            state.cells[neighbor].revealed = true;
        }
        state.cells[40].revealed = true;
        state.cells[40].adjacent = 1;
        state.cells[30].revealed = false;
        state.cells[30].flagged = true;

        state.update_hint();
        assert_eq!(state.hint.as_ref().unwrap().kind, HintKind::Safe);
    }

    #[test]
    fn hint_falls_back_to_an_unflagged_safe_boundary_cell() {
        let mut state = inactive_state();
        state.cells[40].revealed = true;
        state.cells[30].mine = true;
        state.cells[31].flagged = true;

        state.update_hint();
        let hint = state.hint.as_ref().unwrap();
        assert_eq!(hint.kind, HintKind::Safe);
        assert!(!state.cells[hint.cell].mine);
        assert!(!state.cells[hint.cell].flagged);
        assert!(neighbors(Difficulty::Easy, 40).any(|index| index == hint.cell));
    }

    #[test]
    fn safe_hint_prefers_the_cell_with_most_revealed_neighbors() {
        let mut state = inactive_state();
        for index in [40, 41, 49] {
            state.cells[index].revealed = true;
        }

        state.update_hint();
        let hint = state.hint.as_ref().unwrap();
        assert_eq!(hint.cell, 50);
        assert_eq!(hint.kind, HintKind::Safe);
    }

    #[test]
    fn safe_hint_prioritizes_covered_count_matching_adjacent_mines() {
        let mut state = inactive_state();
        for index in [40, 41, 49] {
            state.cells[index].revealed = true;
        }
        state.cells[30].adjacent = 7;

        state.update_hint();
        let hint = state.hint.as_ref().unwrap();
        assert_eq!(hint.cell, 30);
        assert_eq!(hint.kind, HintKind::Safe);
    }

    #[test]
    fn hint_is_stable_and_only_real_progress_resets_it() {
        let mut state = hint_state();
        state.clock = 15.0;
        state.update_hint();
        state.clock = 30.0;
        state.update_hint();
        assert_eq!(state.hint.as_ref().unwrap().cell, 40);

        state.reveal(40);
        state.toggle_flag(40);
        state.reveal(state.cells.len());
        state.toggle_flag(state.cells.len());
        assert_eq!(state.last_progress_at, Some(0.0));

        state.toggle_flag(30);
        assert!(state.hint.is_none());
        assert_eq!(state.last_progress_at, Some(30.0));

        state.clock = 40.0;
        state.toggle_flag(30);
        assert_eq!(state.last_progress_at, Some(40.0));
        state.clock = 55.0;
        state.update_hint();
        assert_eq!(state.hint.as_ref().unwrap().cell, 40);

        state.reveal(30);
        assert!(state.hint.is_none());
        assert_eq!(state.last_progress_at, Some(55.0));
    }

    #[test]
    fn hint_clears_for_new_and_finished_games() {
        let mut state = hint_state();
        state.clock = 15.0;
        state.update_hint();
        assert!(state.hint.is_some());

        state.outcome = Outcome::Lost;
        state.update_hint();
        assert!(state.hint.is_none());

        state.outcome = Outcome::Playing;
        state.update_hint();
        assert!(state.hint.is_some());
        state.new_game();
        assert!(state.hint.is_none());
        assert_eq!(state.last_progress_at, None);
        state.clock = 100.0;
        state.update_hint();
        assert!(state.hint.is_none());
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
            state.settings.difficulty = difficulty;
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
            assert!(
                neighbors(state.settings.difficulty, first).all(|index| !state.cells[index].mine)
            );
        }
    }

    #[test]
    fn mine_and_adjacency_counts_are_correct() {
        let mut state = State::new(11);
        state.reveal(40);
        assert_eq!(
            state.cells.iter().filter(|cell| cell.mine).count(),
            state.settings.difficulty.mines()
        );
        for index in 0..state.cells.len() {
            let expected = neighbors(state.settings.difficulty, index)
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
            state.cells[index].adjacent = neighbors(state.settings.difficulty, index)
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
            state.settings.difficulty = difficulty;
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
        assert_eq!(state.settings.seed, 10);
        assert!(!state.started);
        assert!(matches!(state.outcome, Outcome::Playing));
        assert!(state.cells.iter().all(|cell| *cell == Cell::default()));
    }
}
