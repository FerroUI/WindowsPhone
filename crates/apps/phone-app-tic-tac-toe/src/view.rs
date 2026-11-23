use crate::{
    commands, logic,
    model::{Mode, Saved},
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
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
                "tic tac toe",
                empty("saved game unavailable", &error),
                app_bar(
                    vec![action(Icon::Refresh, "new game", move || {
                        commands::new_game(&reset, Mode::Phone)
                    })],
                    false,
                    vec![],
                ),
            );
        }
    };
    let board = save.game.board().unwrap_or([0; 9]);
    let outcome = logic::outcome(&board);
    let mode = save.game.mode;
    let active_color = accent(&context.vm);
    let foreground = fg(&context.vm);
    let size = (phone_size(&context.vm).0 - 40.0).clamp(180.0, 360.0);
    let mut squares = grid()
        .width(size)
        .height(size)
        .row_definitions("*,*,*")
        .column_definitions("*,*,*")
        .row_spacing(5.0)
        .column_spacing(5.0);
    for (index, value) in board.iter().enumerate() {
        let play = context.clone();
        squares = squares.child(rsx! {Button {
            class:"phone-flat", name:format!("TicTacToeCell{index}"),
            Grid.row:(index / 3) as i32, Grid.column:(index % 3) as i32,
            background:if context.vm.data().settings.dark {"#202020"} else {"#E6E6E6"},
            horizontal_content_alignment:H::Center, vertical_content_alignment:V::Center,
            automation_properties_name:format!("Row {}, column {}, {}", index / 3 + 1, index % 3 + 1,
                match value {1=>"X",2=>"O",_=>"empty"}),
            is_enabled:*value == 0 && outcome == 0,
            on_click:move |_| commands::change(&play, |save| save.play(index)),
            TextBlock {text:match value {1=>"×",2=>"○",_=>""}, font_size:size / 5.0,
                font_weight:FontWeight::Light,
                foreground:if *value == 1 {active_color.as_str()} else {foreground.as_str()}}
        }});
    }
    let again = context.clone();
    let undo = context.clone();
    let more = context.session.clone();
    let local = context.clone();
    let phone = context.clone();
    let pin = context.clone();
    let enabled = save.game.can_undo();
    let menu = context.session.menu.get();
    let scores = if mode == Mode::Phone {
        save.phone_scores
    } else {
        save.local_scores
    };
    let status = match (outcome, mode) {
        (1, Mode::Phone) => "you win",
        (2, Mode::Phone) => "phone wins",
        (1, _) => "X wins",
        (2, _) => "O wins",
        (3, _) => "it's a draw",
        (_, Mode::Phone) => "your turn",
        _ if save.game.moves.len() % 2 == 0 => "X to play",
        _ => "O to play",
    };
    let body = rsx! {StackPanel {margin:20.0, spacing:18.0,
        {text("tic tac toe", 40.0)} {muted(mode.label(), 18.0)} {text(status, 26.0)}
        {squares}
        Grid {column_definitions:"*,*,*", column_spacing:12.0,
            StackPanel {{text(&scores[0].to_string(), 30.0)} {muted(if mode == Mode::Phone {"you"} else {"X"}, 16.0)}}
            StackPanel {Grid.column:1, {text(&scores[1].to_string(), 30.0)} {muted(if mode == Mode::Phone {"phone"} else {"O"}, 16.0)}}
            StackPanel {Grid.column:2, {text(&scores[2].to_string(), 30.0)} {muted("draws", 16.0)}}
        }
        {muted("Your board and scores are saved on this device. Undo takes back an unfinished turn.", 16.0)}
    }};
    page(
        "games",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Refresh, "new game", move || {
                    commands::new_game(&again, mode)
                }),
                rsx! {Border {is_enabled:enabled, {action(Icon::Undo, "undo", move || commands::change(&undo, |save| Ok(save.game.undo())))}}},
                action(Icon::More, "more", move || {
                    more.menu.update(|value| *value = !*value)
                }),
            ],
            menu,
            vec![
                flat("against phone", move || {
                    phone.session.menu.set(false);
                    commands::new_game(&phone, Mode::Phone)
                }),
                flat("two players", move || {
                    local.session.menu.set(false);
                    commands::new_game(&local, Mode::TwoPlayers)
                }),
                flat("pin to start", move || {
                    pin.session.menu.set(false);
                    match pin.pin("", "Tic tac toe") {
                        Ok(()) => pin.vm.home(),
                        Err(error) => pin.vm.set_status(error),
                    }
                }),
            ],
        ),
    )
}
