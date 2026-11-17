use rune::prefs::Prefs;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum Difficulty {
    #[default]
    Easy,
    Moderate,
    Hard,
}

impl Difficulty {
    pub(crate) const ALL: [Self; 3] = [Self::Easy, Self::Moderate, Self::Hard];

    pub(crate) fn width(self) -> usize {
        match self {
            Self::Easy => 9,
            Self::Moderate => 16,
            Self::Hard => 30,
        }
    }

    pub(crate) fn height(self) -> usize {
        match self {
            Self::Easy => 9,
            Self::Moderate | Self::Hard => 16,
        }
    }

    pub(crate) fn mines(self) -> usize {
        match self {
            Self::Easy => 10,
            Self::Moderate => 40,
            Self::Hard => 99,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Moderate => "Moderate",
            Self::Hard => "Hard",
        }
    }

    pub(crate) fn record_key(self) -> &'static str {
        match self {
            Self::Easy => "best_easy",
            Self::Moderate => "best_moderate",
            Self::Hard => "best_hard",
        }
    }

    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) mine: bool,
    pub(crate) revealed: bool,
    pub(crate) flagged: bool,
    pub(crate) adjacent: u8,
}

impl Cell {
    pub(crate) fn switch_flag(&mut self) {
        self.flagged = !self.flagged;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum Outcome {
    #[default]
    Playing,
    Won,
    Lost,
}

#[derive(Default)]
pub(crate) struct Settings {
    pub(crate) difficulty: Difficulty,
    pub(crate) seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HintKind {
    MineCount,
    DoubleClick,
    Safe,
}

pub(crate) struct Hint {
    pub(crate) cell: usize,
    pub(crate) kind: HintKind,
}

#[derive(Default)]
pub(crate) struct GameState {
    pub(crate) cells: Vec<Cell>,
    pub(crate) started: bool,
    pub(crate) outcome: Outcome,
    pub(crate) clock: f64,
    pub(crate) started_at: Option<f64>,
    pub(crate) finished_at: Option<f64>,
    pub(crate) last_progress_at: Option<f64>,
    pub(crate) hint: Option<Hint>,
    pub(crate) window_resize_pending: bool,
}

impl GameState {
    pub(crate) fn elapsed_seconds(&self) -> u64 {
        self.started_at
            .map(|started_at| (self.finished_at.unwrap_or(self.clock) - started_at).max(0.0) as u64)
            .unwrap_or(0)
    }
}

pub(crate) struct State {
    pub(crate) settings: Settings,
    pub(crate) game_state: GameState,
    pub(crate) records: [Option<u64>; 3],
    pub(crate) prefs: Prefs,
    /// used to avoid heap allocations for temporary vectors
    pub(crate) tmp_buffer: Vec<usize>,
}

impl std::ops::Deref for State {
    type Target = GameState;

    fn deref(&self) -> &Self::Target {
        &self.game_state
    }
}

impl std::ops::DerefMut for State {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.game_state
    }
}

impl State {
    #[cfg(test)]
    pub(crate) fn new(seed: u64) -> Self {
        Self::with_prefs(seed, Prefs::memory())
    }

    pub(crate) fn with_prefs(seed: u64, prefs: Prefs) -> Self {
        let records = Difficulty::ALL.map(|difficulty| prefs.get(difficulty.record_key()));
        Self {
            settings: Settings {
                seed,
                ..Default::default()
            },
            game_state: GameState {
                cells: vec![Cell::default(); Difficulty::Easy.width() * Difficulty::Easy.height()],
                ..Default::default()
            },
            records,
            prefs,
            tmp_buffer: vec![],
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::with_prefs(0, Prefs::open("com.madware.minesweeper"))
    }
}
