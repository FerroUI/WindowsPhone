mod board;

use crate::{
    commands,
    model::{Difficulty, Saved, Status},
    state::ViewState,
};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, Icon};
use phone_sdk::{common::*, AppContext};

pub fn view(context: AppContext) -> Element {
    let save = match context.storage().read::<Saved>().and_then(|save| {
        save.validate()?;
        Ok(save)
    }) {
        Ok(save) => save,
        Err(error) => {
            let reset = context.clone();
            return page(
                "minesweeper",
                empty("saved game unavailable", &error),
                app_bar(
                    vec![action(Icon::Refresh, "new game", move || {
                        commands::new_game(&reset, Difficulty::Beginner)
                    })],
                    false,
                    vec![],
                ),
            );
        }
    };
    let state = context.session.state::<ViewState>();
    let flag_mode = state.flag_mode.get();
    let tick = state.clock.get();
    let now = if tick == 0 { context.vm.now() } else { tick };
    let game = &save.game;
    let stats = save.stats[game.difficulty.index()];
    let outcome = match game.status {
        Status::Ready => "choose your first square",
        Status::Playing if flag_mode => "tap a square to mark a mine",
        Status::Playing => "clear the minefield",
        Status::Won => "minefield cleared",
        Status::Lost => "you found a mine",
    };
    let body = rsx! {StackPanel {margin:20.0, spacing:16.0,
        {text("minesweeper", 40.0)} {muted(game.difficulty.label(), 18.0)}
        Grid {column_definitions:"*,*,*", column_spacing:12.0,
            StackPanel {{text(&game.remaining().to_string(), 30.0)} {muted("mines left", 15.0)}}
            StackPanel {Grid.column:1, {text(&game.moves.to_string(), 30.0)} {muted("moves", 15.0)}}
            StackPanel {Grid.column:2,
                TextBlock {name:"MinesweeperSeconds", text:game.elapsed(now).to_string(), font_size:30.0,
                    font_weight:ferroui_base::media::FontWeight::Light}
                {muted("seconds", 15.0)}}
        }
        {text(outcome, 22.0)}
        {board::view(&context, game, flag_mode)}
        {muted("Your first square and its neighbours are safe. Tap a revealed number to clear its neighbours when the matching number of flags is placed.", 15.0)}
        {muted(&format!("{} won · {} lost · best {}", stats.won, stats.lost,
            stats.best_seconds.map(|seconds| format!("{seconds}s")).unwrap_or_else(|| "—".into())), 16.0)}
    }};
    let again = context.clone();
    let flag = context.session.state::<ViewState>();
    let menu = context.session.menu;
    let difficulty = game.difficulty;
    let mut options = Vec::new();
    for next in Difficulty::ALL {
        let choose = context.clone();
        options.push(flat(next.label(), move || {
            choose.session.menu.set(false);
            commands::new_game(&choose, next);
        }));
    }
    let pin = context.clone();
    options.push(flat("pin to start", move || {
        pin.session.menu.set(false);
        match pin.pin("", "Minesweeper") {
            Ok(()) => pin.vm.home(),
            Err(error) => pin.vm.set_status(error),
        }
    }));
    page(
        "games",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Refresh, "new game", move || {
                    commands::new_game(&again, difficulty)
                }),
                action(
                    if flag_mode { Icon::Check } else { Icon::Pin },
                    if flag_mode { "reveal" } else { "flag" },
                    move || flag.flag_mode.update(|value| *value = !*value),
                ),
                action(Icon::More, "more", move || {
                    menu.update(|value| *value = !*value)
                }),
            ],
            context.session.menu.get(),
            options,
        ),
    )
}
