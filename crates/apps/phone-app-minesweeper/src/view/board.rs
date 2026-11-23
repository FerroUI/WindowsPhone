use crate::{commands, model::Game};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use phone_sdk::{common::*, AppContext};

pub fn view(context: &AppContext, game: &Game, flag_mode: bool) -> Element {
    let (width, height, _) = game.difficulty.dimensions();
    let size = (phone_size(&context.vm).0 - 40.0).clamp(240.0, 360.0);
    let columns = vec!["*"; width].join(",");
    let rows = vec!["*"; height].join(",");
    let mut board = grid()
        .width(size)
        .height(size * height as f64 / width as f64)
        .column_definitions(&columns)
        .row_definitions(&rows)
        .column_spacing(2.0)
        .row_spacing(2.0);
    let dark = context.vm.data().settings.dark;
    for (index, cell) in game.cells.iter().enumerate() {
        let hit = cell.mine && cell.revealed;
        let ended_mine = cell.mine && game.complete();
        let foreground = if hit {
            "White"
        } else if cell.flagged {
            "#FFBD38"
        } else if cell.mine {
            if dark {
                "White"
            } else {
                "Black"
            }
        } else {
            match cell.adjacent {
                1 => "#369BFF",
                2 => "#48B863",
                3 => "#F05C55",
                4 => "#AA83EC",
                _ => {
                    if dark {
                        "White"
                    } else {
                        "Black"
                    }
                }
            }
        };
        let glyph = if cell.flagged && game.status != crate::model::Status::Lost {
            "⚑".into()
        } else if ended_mine {
            "●".into()
        } else if cell.flagged {
            "⚑".into()
        } else if cell.revealed && cell.adjacent != 0 {
            cell.adjacent.to_string()
        } else {
            String::new()
        };
        let label = if hit {
            "mine, exploded".into()
        } else if ended_mine {
            "mine".into()
        } else if cell.flagged {
            "flagged".into()
        } else if cell.revealed {
            format!("{} adjacent mines", cell.adjacent)
        } else {
            "hidden".into()
        };
        let play = context.clone();
        let enabled = !game.complete() && (!cell.revealed || (!flag_mode && cell.adjacent > 0));
        board = board.child(rsx! {Button {
            class:"phone-flat", name:format!("MineCell{index}"), Grid.column:(index % width) as i32, Grid.row:(index / width) as i32,
            min_width:0.0, min_height:0.0, padding:0.0,
            horizontal_content_alignment:H::Center, vertical_content_alignment:V::Center,
            background:if hit {"#C42B1C"} else if cell.revealed {if dark {"#161616"} else {"#F1F1F1"}} else {if dark {"#383838"} else {"#CCCCCC"}},
            automation_properties_name:format!("Row {}, column {}, {label}", index / width + 1, index % width + 1),
            automation_properties_help_text:if flag_mode {"Toggle flag"} else {"Reveal square"},
            is_enabled:enabled,
            on_click:move |_| commands::select(&play, index),
            TextBlock {text:glyph, font_size:(size / width as f64 * 0.62).min(28.0), font_weight:FontWeight::SemiBold, foreground:foreground}
        }});
    }
    board.into()
}
