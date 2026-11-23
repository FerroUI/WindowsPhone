use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use phone_controls::icon;
use phone_sdk::{common::*, AppManifest};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, games: &[AppManifest]) -> Element {
    if games.is_empty() {
        return empty(
            "your games",
            "Install a game from the app collection to start playing.",
        );
    }
    let mut body = stack_panel().spacing(8.0);
    for game in games {
        let launch = vm.clone();
        let route = game.route("");
        let color = if game.accent.is_empty() {
            "#108A15"
        } else {
            game.accent
        };
        body = body.child(rsx! {Button {
            class:"phone-flat", name:format!("GameLaunch-{}", game.id),
            horizontal_alignment:H::Stretch, horizontal_content_alignment:H::Stretch,
            margin:(12.0,0.0), padding:8.0, min_height:106.0,
            automation_properties_name:game.title,
            on_click:move |_| launch.navigate(route.clone()),
            Grid {column_definitions:"86,*", column_spacing:16.0,
                Border {width:86.0, height:86.0, background:color, {icon(game.icon,48.0,"White")}}
                StackPanel {Grid.column:1, vertical_alignment:V::Center, spacing:4.0,
                    TextBlock {text:game.title, font_size:26.0, font_weight:FontWeight::Light,
                        text_wrapping:ferroui_base::media::TextWrapping::Wrap}
                    {muted("play", 16.0)}
                }
            }
        }});
    }
    body.into()
}
