use super::*;
use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, NavigationMethod, TextInputEventArgs,
};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::{Rect, Ref, Size};
use ferroui_controls::{
    Control, ScrollViewer, StackPanel, Window, presenters::TextPresenter,
    primitives::ScrollBarVisibility,
};
use std::rc::Rc;

fn native_test(action: impl FnOnce() + Send + 'static) {
    use ferroui_base::threading::CancellationToken;
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            move || {
                crate::set_motion_enabled(false);
                crate::install_theme(&Application::current().unwrap());
                action();
            },
            CancellationToken::none(),
        )
        .wait_timeout(std::time::Duration::from_secs(20))
        .expect("native editor regression completes");
    session.dispose();
}

fn mounted(palette: &Palette) -> (Ref<Window>, Ref<TextBox>, Ref<Styles>) {
    use ferroui_base::threading::Dispatcher;
    use ferroui_headless::FerroHeadlessPlatform;
    let root = StackPanel::new();
    let colors = Styles::new();
    install(&colors, palette);
    root.styles().add(colors.clone());
    let input = TextBox::new();
    input.classes().add("phone-input");
    input.set_text(Some("Metro"));
    root.children().add(input.clone());
    let window = Window::new();
    window.set_width(320.0);
    window.set_height(200.0);
    window.set_content(Some(Control::boxed(root)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
    (window, input, colors)
}

fn refresh(input: &TextBox) {
    input.apply_styling();
    input.apply_template();
    for element in input.get_template_descendants() {
        element.apply_styling();
    }
}

fn part<T: ferroui_base::ObjectType>(input: &TextBox, name: &str) -> Ref<T> {
    input
        .get_template_descendants()
        .into_iter()
        .find(|element| element.has_name(name))
        .and_then(|element| element.cast::<T>())
        .unwrap_or_else(|| panic!("native text box part {name}"))
}

fn color(brush: Option<Rc<dyn IBrush>>) -> Color {
    brush
        .expect("a state brush")
        .as_solid_color_brush()
        .expect("solid state ink")
        .color()
}

fn type_text(input: &TextBox, text: &str) {
    let mut event = TextInputEventArgs::new();
    event.set_routed_event(Some(InputElement::text_input_event()));
    event.text = Some(text.to_owned());
    input.raise_event(&event);
}

#[test]
fn palette_replacement_preserves_native_text_parts_selection_and_pending_composition() {
    native_test(|| {
        let (window, input, colors) = mounted(&Palette::new("#1BA1E2", false));
        input.set_selection_start(2);
        input.set_selection_end(2);
        input.set_value(
            ScrollViewer::vertical_scroll_bar_visibility_property(),
            ScrollBarVisibility::Auto,
        );
        input.set_value(
            ScrollViewer::horizontal_scroll_bar_visibility_property(),
            ScrollBarVisibility::Visible,
        );
        let template = input.template().expect("native WinUI text template");
        let presenter = part::<TextPresenter>(&input, "PART_TextPresenter");
        let scroll = part::<ScrollViewer>(&input, "PART_ScrollViewer");
        let request = TextInputMethodClientRequestedEventArgs::new();
        request.set_routed_event(Some(
            InputElement::text_input_method_client_requested_event(),
        ));
        input.raise_event(&request);
        let client = request.client().expect("native composition client");
        client.set_preedit_text_with_cursor(Some("漢"), Some(1));
        assert_eq!(presenter.preedit_text().as_deref(), Some("漢"));

        for light in [true, false, true] {
            let palette = Palette::new("#00ABA9", light);
            colors.clear();
            install(&colors, &palette);
            refresh(&input);
            assert!(Rc::ptr_eq(
                &template,
                &input.template().expect("template survives palette change")
            ));
            assert!(Ref::ptr_eq(
                &presenter,
                &part::<TextPresenter>(&input, "PART_TextPresenter")
            ));
            assert!(Ref::ptr_eq(
                &scroll,
                &part::<ScrollViewer>(&input, "PART_ScrollViewer")
            ));
            assert_eq!(input.text().as_deref(), Some("Metro"));
            assert_eq!((input.selection_start(), input.selection_end()), (2, 2));
            assert_eq!(presenter.preedit_text().as_deref(), Some("漢"));
            assert_eq!(presenter.preedit_text_cursor_position(), Some(1));
            assert_eq!(
                ScrollViewer::get_vertical_scroll_bar_visibility(&scroll),
                ScrollBarVisibility::Auto
            );
            assert_eq!(
                ScrollViewer::get_horizontal_scroll_bar_visibility(&scroll),
                ScrollBarVisibility::Visible
            );
            assert_eq!(
                color(input.selection_brush()),
                Color::parse(&palette.accent).unwrap()
            );
        }
        client.set_preedit_text(None);
        assert_eq!(input.text().as_deref(), Some("Metro"));
        window.close();
    });
}

#[test]
fn focused_read_only_and_disabled_states_preserve_native_editing_contract_and_bounds() {
    native_test(|| {
        for light in [false, true] {
            let palette = Palette::new("#00ABA9", light);
            let (window, input, _colors) = mounted(&palette);
            input.set_use_layout_rounding(false);
            let presenter = part::<TextPresenter>(&input, "PART_TextPresenter");
            let scroll = part::<ScrollViewer>(&input, "PART_ScrollViewer");
            let border = part::<Border>(&input, "BorderElement");
            let bounds = Rect::new(0.0, 0.0, 300.0, 44.0);
            input.measure(Size::new(bounds.width, bounds.height));
            input.arrange(bounds);
            let resting_border = border.bounds();
            assert_eq!(input.font_size(), FONT_MEDIUM_LARGE);
            assert_eq!(
                border.border_thickness(),
                ferroui_base::Thickness::uniform(CONTROL_BORDER)
            );
            assert_eq!(color(border.background()), Color::parse("#D6D6D6").unwrap());

            let mut focus = FocusChangedEventArgs::new(&InputElement::got_focus_event());
            focus.navigation_method = NavigationMethod::Tab;
            input.raise_event(&focus);
            refresh(&input);
            assert!(input.is_focused());
            assert_eq!(
                (input.selection_start(), input.selection_end()),
                (0, 5),
                "native keyboard focus still selects the field"
            );
            assert_eq!(color(border.background()), Color::parse("#FFFFFF").unwrap());
            assert_eq!(
                color(border.border_brush()),
                Color::parse(&palette.accent).unwrap()
            );
            type_text(&input, "phone");
            assert_eq!(input.text().as_deref(), Some("phone"));
            input.undo();
            assert_eq!(
                input.text().as_deref(),
                Some("Metro"),
                "native undo retains the edit"
            );

            input.set_is_read_only(true);
            input.set_selection_start(1);
            input.set_selection_end(4);
            refresh(&input);
            type_text(&input, "ignored");
            assert_eq!(input.text().as_deref(), Some("Metro"));
            assert_eq!((input.selection_start(), input.selection_end()), (1, 4));
            assert!(input.can_copy() && !input.can_cut() && !input.can_paste());
            assert_eq!(color(input.caret_brush()).a, 0);
            assert_eq!(
                color(scroll.foreground()),
                Color::parse(palette.foreground).unwrap()
            );

            input.set_is_enabled(false);
            input.set_is_read_only(false);
            refresh(&input);
            assert!(!input.is_effectively_enabled());
            assert_eq!(input.text().as_deref(), Some("Metro"));
            assert_eq!(color(border.background()).a, 0);
            assert_eq!(color(border.border_brush()).a, 0);
            assert_eq!(
                color(scroll.foreground()),
                Color::parse(palette.secondary).unwrap()
            );
            assert_eq!(
                input.opacity(),
                1.0,
                "disabled ink is not compounded with a subtree fade"
            );
            assert!(Ref::ptr_eq(
                &presenter,
                &part::<TextPresenter>(&input, "PART_TextPresenter")
            ));
            input.measure(Size::new(bounds.width, bounds.height));
            input.arrange(bounds);
            assert_eq!(
                border.bounds(),
                resting_border,
                "visual state changes keep the field geometry stable"
            );
            window.close();
        }
    });
}

#[test]
fn native_keyboard_admission_rejects_disabled_text_without_replacing_the_editor() {
    use ferroui_base::input::KeyModifiers;
    use ferroui_base::threading::{CancellationToken, Dispatcher};
    use ferroui_controls::{AppBuilder, Application, Control, Window};
    use ferroui_headless::{
        FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
        HeadlessUnitTestSession, HeadlessWindowExtensions,
    };
    use std::time::Duration;

    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                crate::set_motion_enabled(false);
                crate::install_theme(&Application::current().unwrap());
                let input = TextBox::new();
                input.classes().add("phone-input");
                input.set_text(Some("Metro"));
                let window = Window::new();
                window.set_width(320.0);
                window.set_height(200.0);
                window.set_content(Some(Control::boxed(input.clone())));
                window.show();
                let frame = || {
                    Dispatcher::ui_thread().run_jobs(None);
                    FerroHeadlessPlatform::force_render_timer_tick(1);
                    Dispatcher::ui_thread().run_jobs(None);
                };
                frame();
                let presenter = part::<TextPresenter>(&input, "PART_TextPresenter");
                assert!(input.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
                window.key_text_input("phone");
                frame();
                assert_eq!(input.text().as_deref(), Some("phone"));
                input.set_is_read_only(true);
                window.key_text_input("read-only input");
                assert_eq!(input.text().as_deref(), Some("phone"));
                input.set_is_enabled(false);
                input.set_is_read_only(false);
                // Raw host input follows the keyboard device's admission path. A
                // manually raised routed event would intentionally bypass that path.
                window.key_text_input("disabled input");
                frame();
                assert_eq!(input.text().as_deref(), Some("phone"));
                assert!(!input.focus());
                assert!(Ref::ptr_eq(
                    &presenter,
                    &part::<TextPresenter>(&input, "PART_TextPresenter")
                ));
                input.set_is_enabled(true);
                assert!(input.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
                window.key_text_input("restored");
                frame();
                assert_eq!(input.text().as_deref(), Some("restored"));
                window.close();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(20))
        .expect("native text admission regression completes");
    session.dispose();
}
