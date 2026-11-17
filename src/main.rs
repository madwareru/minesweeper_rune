mod game;
mod model;
mod view;

use model::{Difficulty, State};
use rune::AppSpec;
use view::{view, window_size};

fn poll(state: &mut State, now: f64, tx: &mut rune::Tx) -> bool {
    state.clock = now;
    if state.started && state.outcome == model::Outcome::Playing {
        tx.wake_after(std::time::Duration::from_secs(1));
        true
    } else {
        false
    }
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
                for (i, cell) in state.cells.iter().enumerate() {
                    if cell.mine {
                        state.reveal(i);
                        return;
                    }
                }
            }),
            ("won", |state| {
                state.reveal(FIRST_TO_REVEAL);
                for (i, cell) in state.cells.iter().enumerate() {
                    if !cell.mine {
                        state.reveal(i);
                        return;
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
