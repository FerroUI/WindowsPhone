mod logic;

use crate::{common::*, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, icon, pivot, Icon};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if ui.selected.get().is_some() {
        return board(vm, ui);
    }
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (i, label) in ["collection", "player"].iter().enumerate() {
        let state = ui.clone();
        tabs = tabs.child(pivot(label, ui.pivot.get() == i, move || {
            state.pivot.set(i)
        }));
    }
    let mut body = stack_panel().spacing(18.0).child(tabs);
    if ui.pivot.get() == 0 {
        let play = ui.clone();
        body=body.child(rsx!{Button{class:"phone-flat",padding:20.0,margin:20.0,background:"#108A15",min_height:220.0,
            on_click:move |_|play.selected.set(Some(1)),
            StackPanel{spacing:20.0,
                Border{horizontal_alignment:H::Left,{icon(Icon::Games,60.0,"White")}}
                TextBlock{text:"tic tac toe",font_size:36.0,font_weight:FontWeight::Light,foreground:"White"}
                TextBlock{text:"your turn to play",font_size:20.0,foreground:"White"}
            }
        }});
    } else {
        body =
            body.child(border().margin(20.0).child(score(&ui)))
                .child(border().margin(20.0).child(muted(
                    "Your score for this session. Play a perfect game to earn a draw.",
                    20.0,
                )));
    }
    page("games", scroll(body.into()), Element::Empty)
}

fn board(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let cells = ui.game_board.get();
    let outcome = logic::outcome(&cells);
    let active_color = accent(&vm);
    let foreground = fg(&vm);
    let size = (vm.viewport().0.min(400.0) - 40.0).max(180.0);
    let mut squares = grid()
        .width(size)
        .height(size)
        .row_definitions("*,*,*")
        .column_definitions("*,*,*")
        .row_spacing(5.0)
        .column_spacing(5.0);
    for (index, value) in cells.iter().enumerate() {
        let state = ui.clone();
        squares=squares.child(rsx!{Button{class:"phone-flat",Grid.row:(index/3)as i32,Grid.column:(index%3)as i32,
            background:if vm.data().settings.dark{"#202020"}else{"#E6E6E6"},
            horizontal_content_alignment:H::Center,vertical_content_alignment:V::Center,
            automation_properties_name:format!("Row {}, column {}, {}",index/3+1,index%3+1,match value{1=>"X",2=>"O",_=>"empty"}),
            is_enabled:*value==0&&outcome==0,on_click:move |_|play(&state,index),
            TextBlock{text:match value{1=>"×",2=>"○",_=>""},font_size:72.0,font_weight:FontWeight::Light,
                foreground:if *value==1{active_color.as_str()}else{foreground.as_str()}}
        }});
    }
    let again = ui.clone();
    let body = rsx! {StackPanel{margin:20.0,spacing:24.0,
        {text("tic tac toe",40.0)}
        {text(match outcome{1=>"you win",2=>"phone wins",3=>"it's a draw",_=>"your turn"},24.0)}
        {squares}{score(&ui)}
    }};
    page(
        "games",
        scroll(body),
        app_bar(
            vec![action(Icon::Refresh, "new game", move || {
                again.game_board.set([0; 9])
            })],
            false,
            vec![],
        ),
    )
}

fn score(ui: &Session) -> Element {
    let points = ui.game_score.get();
    rsx! {Grid{column_definitions:"*,*,*",column_spacing:14.0,
        StackPanel{spacing:5.0,{text(&points[0].to_string(),32.0)}{muted("you",15.0)}}
        StackPanel{Grid.column:1,spacing:5.0,{text(&points[1].to_string(),32.0)}{muted("phone",15.0)}}
        StackPanel{Grid.column:2,spacing:5.0,{text(&points[2].to_string(),32.0)}{muted("draws",15.0)}}
    }}
}

fn play(ui: &Session, index: usize) {
    let mut board = ui.game_board.get_untracked();
    if board[index] != 0 || logic::outcome(&board) != 0 {
        return;
    }
    board[index] = 1;
    if logic::outcome(&board) == 0 {
        if let Some(index) = logic::best_move(&board) {
            board[index] = 2;
        }
    }
    let outcome = logic::outcome(&board);
    ui.game_board.set(board);
    if outcome != 0 {
        ui.game_score
            .update(|score| score[(outcome - 1) as usize] += 1);
    }
}
