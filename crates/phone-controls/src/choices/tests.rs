use super::*;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, Key, KeyEventArgs, NavigationMethod,
};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, Point, Rect, Size};
use ferroui_controls::primitives::ToggleButton;
use std::cell::{Cell, RefCell};

fn refresh(control: &TemplatedControl) {
    control.apply_styling();
    control.apply_template();
    for element in control.get_template_descendants() {
        element.apply_styling();
        if let Some(presenter) = element.cast::<ContentPresenter>() {
            presenter.update_child();
            if let Some(child) = presenter.child() {
                child.apply_styling();
            }
        }
    }
}

fn part<T: ferroui_base::ObjectType>(control: &TemplatedControl, name: &str) -> Ref<T> {
    control
        .get_template_descendants()
        .into_iter()
        .find(|element| element.has_name(name))
        .and_then(|element| element.cast::<T>())
        .unwrap_or_else(|| panic!("mounted phone choice part {name}"))
}

fn color(brush: Option<Rc<dyn IBrush>>) -> Color {
    brush
        .expect("a state brush")
        .as_solid_color_brush()
        .expect("solid state ink")
        .color()
}

fn tab_focus(control: &InputElement) {
    let mut event = FocusChangedEventArgs::new(&InputElement::got_focus_event());
    event.navigation_method = NavigationMethod::Tab;
    control.raise_event(&event);
}

fn space(control: &InputElement, release: bool) {
    let mut event = KeyEventArgs::new();
    event.key = Key::Space;
    event.set_routed_event(Some(if release {
        InputElement::key_up_event()
    } else {
        InputElement::key_down_event()
    }));
    control.raise_event(&event);
}

#[test]
fn palette_replacement_keeps_pressed_choice_parts_and_native_focus() {
    let _scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(ferroui_headless::testing::headless_platform_render_interface());
    for is_radio in [false, true] {
        let root = StackPanel::new();
        install_templates(&root.styles());
        let colors = Styles::new();
        install(&colors, &Palette::new("#1BA1E2", false));
        root.styles().add(colors.clone());
        let control: Ref<ToggleButton> = if is_radio {
            RadioButton::new().upcast()
        } else {
            CheckBox::new().upcast()
        };
        control.classes().add(if is_radio {
            "phone-radio"
        } else {
            "phone-check"
        });
        control.set_is_checked(Some(true));
        root.children().add(control.clone());
        control.apply_styling();
        refresh(&control);
        let template = control.template().expect("persistent choice template");
        let outline = part::<Border>(&control, "PhoneChoiceOutline");
        let content = part::<ContentPresenter>(&control, "PART_ContentPresenter");
        tab_focus(&control);
        space(&control, false);
        assert!(control.is_focused() && control.is_pressed());

        for light in [true, false, true] {
            let palette = Palette::new("#00ABA9", light);
            colors.clear();
            install(&colors, &palette);
            // Exactly one styling pass before template application is the
            // layout ordering that used to lose palette-owned templates.
            refresh(&control);
            assert!(Rc::ptr_eq(
                &template,
                &control
                    .template()
                    .expect("template survives palette change")
            ));
            assert!(Ref::ptr_eq(
                &outline,
                &part::<Border>(&control, "PhoneChoiceOutline")
            ));
            assert!(Ref::ptr_eq(
                &content,
                &part::<ContentPresenter>(&control, "PART_ContentPresenter")
            ));
            assert!(control.is_focused() && control.is_pressed());
            assert_eq!(control.is_checked(), Some(true));
            assert_eq!(
                color(outline.background()),
                Color::parse(palette.foreground).unwrap()
            );
        }
        space(&control, true);
        assert!(!control.is_pressed());
        assert_eq!(
            control.is_checked(),
            Some(is_radio),
            "native release toggles checks while keeping selected radios exclusive"
        );
    }
}

#[test]
fn native_choice_states_keep_geometry_and_use_disabled_ink_without_subtree_fade() {
    let _scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(ferroui_headless::testing::headless_platform_render_interface());
    for light in [false, true] {
        let palette = Palette::new("#00ABA9", light);
        let root = StackPanel::new();
        install_templates(&root.styles());
        // Reproduce the application's general text rule as well: generated
        // labels must still receive disabled ink from the choice state.
        root.styles().add(
            style("TextBlock")
                .set(TextBlock::foreground_property(), palette.foreground)
                .build(),
        );
        install(&root.styles(), &palette);
        let checkbox = CheckBox::new();
        checkbox.classes().add("phone-check");
        checkbox.set_use_layout_rounding(false);
        checkbox.set_is_three_state(true);
        checkbox.set_is_checked(Some(true));
        root.children().add(checkbox.clone());
        checkbox.apply_styling();
        refresh(&checkbox);
        let outline = part::<Border>(&checkbox, "PhoneChoiceOutline");
        let check = part::<Path>(&checkbox, "PhoneChoiceCheck");
        let mixed = part::<Border>(&checkbox, "PhoneChoiceIndeterminate");
        let focus = part::<Border>(&checkbox, "PhoneChoiceFocus");
        let bounds = Rect::new(0.0, 0.0, 360.0, px(72.0));
        checkbox.measure(Size::new(bounds.width, bounds.height));
        checkbox.arrange(bounds);
        let position = outline
            .translate_point(Point::new(0.0, 0.0), &checkbox)
            .unwrap();
        assert!((outline.bounds().width - px(32.0)).abs() < 0.001);
        assert!((outline.bounds().height - px(32.0)).abs() < 0.001);
        assert!(check.is_visible() && !mixed.is_visible());
        assert_eq!(check.width(), px(23.0));
        assert_eq!(check.height(), px(21.0));
        assert_eq!((mixed.width(), mixed.height()), (px(16.0), px(16.0)));

        tab_focus(&checkbox);
        refresh(&checkbox);
        checkbox.measure(Size::new(bounds.width, bounds.height));
        checkbox.arrange(bounds);
        assert_eq!(
            outline.translate_point(Point::new(0.0, 0.0), &checkbox),
            Some(position),
            "focus is an overlay, not a layout-changing border"
        );
        assert_eq!(
            color(focus.border_brush()),
            Color::parse(palette.foreground).unwrap()
        );
        assert!(!focus.is_hit_test_visible());
        space(&checkbox, false);
        refresh(&checkbox);
        assert_eq!(
            color(outline.background()),
            Color::parse(palette.foreground).unwrap()
        );
        assert_eq!(
            color(check.fill()),
            Color::parse(palette.background).unwrap()
        );
        space(&checkbox, true);

        checkbox.set_is_checked(None);
        refresh(&checkbox);
        assert!(
            mixed.is_visible() && !check.is_visible(),
            "native indeterminate is a square, separate from the check mark"
        );
        checkbox.set_content(Some(
            Rc::new(String::from("sync mail")) as ferroui_base::BoxedValue
        ));
        checkbox.set_is_enabled(false);
        refresh(&checkbox);
        let content = part::<ContentPresenter>(&checkbox, "PART_ContentPresenter");
        let text = content
            .child()
            .expect("generated label")
            .cast::<TextBlock>()
            .expect("string content is a TextBlock");
        assert_eq!(
            color(outline.border_brush()),
            Color::parse(palette.secondary).unwrap()
        );
        assert_eq!(
            color(mixed.background()),
            Color::parse(palette.secondary).unwrap()
        );
        assert_eq!(
            color(text.foreground()),
            Color::parse(palette.secondary).unwrap()
        );
        assert_eq!(checkbox.opacity(), 1.0);
        assert_eq!(color(focus.border_brush()).a, 0);
        space(&checkbox, false);
        space(&checkbox, true);
        assert_eq!(
            checkbox.is_checked(),
            None,
            "disabled native choices reject keyboard activation"
        );
    }
}

#[test]
fn checkbox_publishes_native_changes_without_initial_or_duplicate_notifications() {
    let values = Rc::new(RefCell::new(Vec::new()));
    let recorded = values.clone();
    let (control, owner) = check("sync mail", true, move |value| {
        recorded.borrow_mut().push(value)
    })
    .mount_root();
    let checkbox = control.cast::<CheckBox>().expect("a native checkbox");
    assert_eq!(checkbox.is_checked(), Some(true));
    assert!(
        values.borrow().is_empty(),
        "mounting a saved value must not write it again"
    );
    checkbox.set_is_checked(Some(false));
    checkbox.set_is_checked(Some(false));
    checkbox.set_is_checked(Some(true));
    assert_eq!(*values.borrow(), [false, true]);
    owner.dispose();
}

#[test]
fn radio_selection_keeps_native_exclusive_group_and_notifies_only_selected_option() {
    let first_count = Rc::new(Cell::new(0));
    let second_count = Rc::new(Cell::new(0));
    let first_recorded = first_count.clone();
    let second_recorded = second_count.clone();
    let (control, owner) = stack_panel()
        .child(radio("dark", "phone-background-test", true, move || {
            first_recorded.set(first_recorded.get() + 1)
        }))
        .child(radio("light", "phone-background-test", false, move || {
            second_recorded.set(second_recorded.get() + 1)
        }))
        .into_element()
        .mount_root();
    let panel = control.cast::<StackPanel>().unwrap();
    let first = panel.children().get(0).cast::<RadioButton>().unwrap();
    let second = panel.children().get(1).cast::<RadioButton>().unwrap();
    assert_eq!((first_count.get(), second_count.get()), (0, 0));
    second.set_is_checked(Some(true));
    assert_eq!(first.is_checked(), Some(false));
    assert_eq!(second.is_checked(), Some(true));
    assert_eq!((first_count.get(), second_count.get()), (0, 1));
    first.set_is_checked(Some(true));
    assert_eq!(second.is_checked(), Some(false));
    assert_eq!((first_count.get(), second_count.get()), (1, 1));
    owner.dispose();
}
