use super::*;
use ferroui_base::Ref;
use ferroui_base::input::{Key, KeyModifiers, NavigationMethod, PhysicalKey, RawInputModifiers};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{
    AppBuilder, Application, Button, Control, ListBox, PathIcon, Slider, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use ferroui_view::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};

fn color(value: Option<Rc<dyn IBrush>>) -> Color {
    value
        .expect("a brush")
        .as_solid_color_brush()
        .expect("a solid brush")
        .color()
}

#[test]
fn native_commands_keep_focus_geometry_and_template_parts_across_palette_changes() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                let app = Application::current().unwrap();
                crate::set_motion_enabled(false);
                crate::install_theme(&app);
                let clicks = Rc::new(Cell::new(0));
                let calls = clicks.clone();
                let content: Element = stack_panel()
                    .spacing(14.0)
                    .margin(20.0)
                    .child(
                        crate::command_button("save", move || calls.set(calls.get() + 1))
                            .name("CommandUnderTest"),
                    )
                    .child(crate::app_bar(
                        vec![crate::action(crate::Icon::Plus, "add", || {})],
                        false,
                        vec![],
                    ))
                    .child(
                        slider()
                            .name("SliderUnderTest")
                            .class("phone-slider")
                            .value(25.0),
                    )
                    .child(rsx!{ToggleButton {class:"phone-toggle",content:"retained toggle",is_checked:Some(true)}})
                    .child(rsx!{ListBox {class:"phone-list",height:80.0,
                        with:|list| {
                            list.items().add(Some(Rc::new(String::from("retained item")) as ferroui_base::BoxedValue));
                            list.set_selected_index(0);
                        }
                    }})
                    .into();
                let (root, owner) = content.mount_root();
                let window = Window::new();
                window.set_width(320.0);
                window.set_height(520.0);
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                let frame = || {
                    Dispatcher::ui_thread().run_jobs(None);
                    FerroHeadlessPlatform::force_render_timer_tick(1);
                    Dispatcher::ui_thread().run_jobs(None);
                };
                frame();
                let command = root
                    .find_descendant_of_type_where::<Button>(false, |c| {
                        c.has_name("CommandUnderTest")
                    })
                    .unwrap();
                let presenter = command
                    .find_descendant_of_type::<ContentPresenter>(false)
                    .unwrap();
                let text = command.find_descendant_of_type::<TextBlock>(false).unwrap();
                let focus = command
                    .find_descendant_of_type_where::<Border>(false, |c| {
                        c.has_name("PhoneControlFocus")
                    })
                    .unwrap();
                let action = root
                    .find_descendant_of_type_where::<Button>(false, |c| {
                        c.has_name("AppBarAction-add")
                    })
                    .unwrap();
                let circle = action
                    .find_descendant_of_type_where::<Border>(false, |c| {
                        c.has_name("AppBarCircle-add")
                    })
                    .unwrap();
                let glyph = action.find_descendant_of_type::<PathIcon>(false).unwrap();
                let slider = root.find_descendant_of_type::<Slider>(false).unwrap();
                let track = slider
                    .get_template_descendants()
                    .into_iter()
                    .find(|c| c.has_name("PART_Track"))
                    .unwrap();
                let toggle = root.find_descendant_of_type::<ToggleButton>(false).unwrap();
                let toggle_presenter = toggle.find_descendant_of_type::<ContentPresenter>(false).unwrap();
                let list = root.find_descendant_of_type::<ListBox>(false).unwrap();
                let item = list.container_from_index(0).expect("realized native list item");
                let item_presenter = item.find_descendant_of_type::<ContentPresenter>(false).unwrap();
                let geometry = (command.bounds(), presenter.bounds(), text.bounds());
                let command_template = command.template().expect("native command template");
                let slider_template = slider.template().expect("native slider template");
                for (index, light) in [false, true, false].into_iter().enumerate() {
                    crate::set_palette(&app, "#00ABA9", light);
                    // The native layout path styles once before ApplyTemplate.
                    // Draining the dispatcher first could hide an intermediate
                    // Theme/Template removal and a recreated native part.
                    command.apply_styling();
                    command.apply_template();
                    slider.apply_styling();
                    slider.apply_template();
                    toggle.apply_styling();
                    toggle.apply_template();
                    list.apply_styling();
                    item.apply_styling();
                    item.apply_template();
                    assert!(Rc::ptr_eq(
                        &command_template,
                        &command
                            .template()
                            .expect("command template after one styling pass")
                    ));
                    assert!(Rc::ptr_eq(
                        &slider_template,
                        &slider
                            .template()
                            .expect("slider template after one styling pass")
                    ));
                    assert!(Ref::ptr_eq(
                        &presenter,
                        &command
                            .find_descendant_of_type::<ContentPresenter>(false)
                            .expect("retained presenter before dispatcher settling")
                    ));
                    assert!(Ref::ptr_eq(
                        &track,
                        &slider
                            .get_template_descendants()
                            .into_iter()
                            .find(|c| c.has_name("PART_Track"))
                            .expect("retained track before dispatcher settling")
                    ));
                    frame();
                    assert!(Ref::ptr_eq(&toggle_presenter, &toggle.find_descendant_of_type::<ContentPresenter>(false).unwrap()));
                    assert_eq!(toggle.is_checked(), Some(true));
                    assert!(Ref::ptr_eq(&item, &list.container_from_index(0).expect("same realized item after palette change")));
                    assert!(Ref::ptr_eq(&item_presenter, &item.find_descendant_of_type::<ContentPresenter>(false).unwrap()));
                    assert_eq!(list.selected_index(),0);
                    let p = crate::palette();
                    assert!(command.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
                    frame();
                    assert_eq!(
                        focus.opacity(),
                        1.0,
                        "keyboard focus has a visible independent overlay"
                    );
                    assert_eq!(
                        (command.bounds(), presenter.bounds(), text.bounds()),
                        geometry,
                        "focus and palette must not shift content"
                    );
                    assert!(Ref::ptr_eq(
                        &presenter,
                        &command
                            .find_descendant_of_type::<ContentPresenter>(false)
                            .unwrap()
                    ));
                    assert!(Ref::ptr_eq(
                        &track,
                        &slider
                            .get_template_descendants()
                            .into_iter()
                            .find(|c| c.has_name("PART_Track"))
                            .unwrap()
                    ));
                    assert_eq!(slider.value(), 25.0);

                    window.key_press(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        Some(" "),
                    );
                    frame();
                    assert!(command.is_pressed());
                    assert!(
                        command.render_transform().is_none(),
                        "command feedback does not transform its input rectangle"
                    );
                    assert_eq!(
                        color(command.background()),
                        Color::parse(p.foreground).unwrap()
                    );
                    assert_eq!(
                        color(text.foreground()),
                        Color::parse(p.background).unwrap()
                    );
                    assert_eq!(clicks.get(), index, "native Space commits on release");
                    window.key_release(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        Some(" "),
                    );
                    frame();
                    assert_eq!(clicks.get(), index + 1);
                    assert_eq!(
                        color(text.foreground()),
                        Color::parse(p.foreground).unwrap()
                    );

                    assert!(action.focus());
                    window.key_press(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        Some(" "),
                    );
                    frame();
                    assert_eq!(
                        color(circle.background()),
                        Color::parse(p.foreground).unwrap()
                    );
                    assert_eq!(
                        color(glyph.foreground()),
                        Color::parse(p.background).unwrap()
                    );
                    window.key_release(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        Some(" "),
                    );
                    frame();
                }
                command.set_is_enabled(false);
                frame();
                assert_eq!(
                    color(text.foreground()),
                    Color::parse(crate::palette().secondary).unwrap()
                );
                assert_eq!(
                    command.opacity(),
                    1.0,
                    "disabled chrome uses ink, not a faded subtree"
                );
                window.close();
                owner.dispose();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(20))
        .expect("native command regression completes");
    session.dispose();
}
