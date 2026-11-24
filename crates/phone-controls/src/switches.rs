//! Metro presentation of FerroUI's native, draggable ToggleSwitch.
//!
//! Geometry and the 50 ms state transition follow the Windows Phone Toolkit
//! WP8 SwitchButton (4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df, Generic.xaml).
//! The template is implemented against FerroUI's own input/automation contract.
use crate::{
    CONTROL_BORDER, FONT_FAMILY, Palette, SWITCH_THUMB_HEIGHT, SWITCH_THUMB_WIDTH,
    SWITCH_TRACK_HEIGHT, SWITCH_TRACK_WIDTH, SWITCH_TRAVEL, reference_pixels as px,
};
use ferroui_base::animation::{DoubleTransition, ITransition, SignedDuration, Transitions};
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::{HorizontalAlignment as H, Layoutable, VerticalAlignment as V};
use ferroui_base::media::{Brushes, FontFamily, FontWeight, TextWrapping, TranslateTransform};
use ferroui_base::styling::Styles;
use ferroui_base::{Ref, Thickness, Visual};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{
    Border, Canvas, ColumnDefinitions, ContentControl, Control, Grid, StackPanel, TextBlock,
    ToggleSwitch,
};
use ferroui_view::style;
use std::{cell::OnceCell, rc::Rc};

ferroui_base::ferro_thread_local! {
    static TEMPLATE: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
    static COMPACT_TEMPLATE: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
}

/// Structural templates outlive palette changes. Set Template directly so a
/// single native styling pass cannot temporarily apply a different control
/// theme without ToggleSwitch's required drag parts.
pub(crate) fn install_templates(styles: &Styles) {
    styles.add(
        style("ToggleSwitch.phone-switch")
            .set(TemplatedControl::template_property(), Some(template(false)))
            .build(),
    );
    styles.add(
        style("ToggleSwitch.phone-switch.phone-switch-compact")
            .set(TemplatedControl::template_property(), Some(template(true)))
            .build(),
    );
}

pub(crate) fn install(styles: &Styles, p: &Palette) {
    styles.add(
        style("ToggleSwitch.phone-switch")
            .set(
                TemplatedControl::font_family_property(),
                FontFamily::new(FONT_FAMILY),
            )
            .set(
                TemplatedControl::font_size_property(),
                px(32.0) * p.text_scale,
            )
            .set(TemplatedControl::font_weight_property(), FontWeight::Light)
            .set(TemplatedControl::foreground_property(), p.foreground)
            .set(TemplatedControl::background_property(), p.background)
            .set(TemplatedControl::border_thickness_property(), 0.0)
            .set(TemplatedControl::padding_property(), 0.0)
            .set(Layoutable::min_width_property(), 0.0)
            .set(Layoutable::min_height_property(), 66.0)
            .set(Layoutable::horizontal_alignment_property(), H::Stretch)
            .set(
                ToggleSwitch::knob_transitions_property(),
                knob_transitions(p.motion_enabled),
            )
            .build(),
    );
    styles.add(
        style("ToggleSwitch.phone-switch.phone-switch-compact")
            .set(Layoutable::min_height_property(), 44.0)
            .build(),
    );
    styles.add(
        style("ToggleSwitch.phone-switch /template/ ContentPresenter#PART_ContentPresenter")
            .set(ContentPresenter::foreground_property(), p.secondary)
            .set(
                ContentPresenter::font_size_property(),
                px(22.0) * p.text_scale,
            )
            .set(ContentPresenter::font_weight_property(), FontWeight::Normal)
            .build(),
    );
    // The caption's default data template creates a TextBlock. Its foreground
    // must be set on that text, because the app's general TextBlock style has
    // higher priority than foreground inherited from the content presenter.
    styles.add(
        style("ToggleSwitch.phone-switch TextBlock.phone-switch-header")
            .set(TextBlock::foreground_property(), p.secondary)
            .build(),
    );
    for (state, off, on) in [("", true, false), (":checked", false, true)] {
        styles.add(style(&format!("ToggleSwitch.phone-switch{state} /template/ ContentPresenter#PART_OffContentPresenter"))
            .set(Visual::is_visible_property(), off).build());
        styles.add(style(&format!("ToggleSwitch.phone-switch{state} /template/ ContentPresenter#PART_OnContentPresenter"))
            .set(Visual::is_visible_property(), on).build());
    }
    for name in ["PhoneSwitchOutline", "PhoneSwitchThumb"] {
        styles.add(
            style(&format!(
                "ToggleSwitch.phone-switch /template/ Border#{name}"
            ))
            .set(Border::border_brush_property(), p.foreground)
            .build(),
        );
    }
    styles.add(
        style("ToggleSwitch.phone-switch /template/ Border#PhoneSwitchTrack")
            .set(Border::background_property(), p.accent.as_str())
            .build(),
    );
    styles.add(
        style("ToggleSwitch.phone-switch /template/ Border#PhoneSwitchThumb")
            .set(Border::background_property(), p.foreground)
            .build(),
    );
    for name in ["PhoneSwitchInset", "PhoneSwitchThumbCutout"] {
        styles.add(
            style(&format!(
                "ToggleSwitch.phone-switch /template/ Border#{name}"
            ))
            .set(Border::border_brush_property(), p.background)
            .build(),
        );
    }
    styles.add(
        style("ToggleSwitch.phone-switch /template/ Border#PhoneSwitchCover")
            .set(Border::background_property(), p.background)
            .build(),
    );
    styles.add(
        style("ToggleSwitch.phone-switch:disabled")
            .set(Visual::opacity_property(), 0.3)
            .build(),
    );
    styles.add(
        style("ToggleSwitch.phone-switch:disabled /template/ Border#PhoneSwitchTrack")
            .set(Border::background_property(), p.foreground)
            .build(),
    );
    styles.add(
        style("ToggleSwitch.phone-switch:focus-visible /template/ Border#PhoneSwitchOutline")
            .set(Border::border_brush_property(), p.accent.as_str())
            .build(),
    );
}

fn knob_transitions(enabled: bool) -> Option<Transitions> {
    enabled.then(|| {
        let transition: Rc<dyn ITransition> = DoubleTransition::with_property(
            Canvas::left_property().as_property(),
            SignedDuration::from_seconds(0.05),
        )
        .into();
        Transitions::from_items([transition])
    })
}

fn presenter(name: &str, property: &'static ferroui_base::FerroProperty) -> Ref<ContentPresenter> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some(name.into()));
    presenter.set_is_hit_test_visible(false);
    presenter.set_text_wrapping(TextWrapping::Wrap);
    let template_property = if name == "PART_ContentPresenter" {
        ContentControl::content_template_property().as_property()
    } else if name == "PART_OnContentPresenter" {
        ToggleSwitch::on_content_template_property().as_property()
    } else {
        ToggleSwitch::off_content_template_property().as_property()
    };
    for (target, source) in [
        (ContentPresenter::content_property().as_property(), property),
        (
            ContentPresenter::content_template_property().as_property(),
            template_property,
        ),
        (
            ContentPresenter::foreground_property().as_property(),
            TemplatedControl::foreground_property().as_property(),
        ),
        (
            ContentPresenter::font_family_property().as_property(),
            TemplatedControl::font_family_property().as_property(),
        ),
        (
            ContentPresenter::font_size_property().as_property(),
            TemplatedControl::font_size_property().as_property(),
        ),
        (
            ContentPresenter::font_weight_property().as_property(),
            TemplatedControl::font_weight_property().as_property(),
        ),
    ] {
        if name == "PART_ContentPresenter"
            && [
                ContentPresenter::foreground_property().as_property(),
                ContentPresenter::font_size_property().as_property(),
                ContentPresenter::font_weight_property().as_property(),
            ]
            .contains(&target)
        {
            // Palette styles supply the smaller, secondary-colored header;
            // a Template-priority binding would hide those style setters.
            continue;
        }
        presenter.bind_binding(target, &*TemplateBinding::new(source));
    }
    if name == "PART_ContentPresenter" {
        // Mark only text produced by the default data template. Custom content
        // and templates retain their original presentation and DataContext.
        // This presenter owns the observer, which captures no control.
        presenter.add_property_changed_handler(Rc::new(|change| {
            if change.property() != ContentPresenter::child_property().as_property() {
                return;
            }
            let Some(presenter) = change.sender().downcast_ref::<ContentPresenter>() else {
                return;
            };
            if presenter.content_template().is_some()
                || presenter
                    .content()
                    .as_ref()
                    .and_then(Control::from_boxed)
                    .is_some()
            {
                return;
            }
            if let Some(text) = change
                .get_new_value::<Option<Ref<Control>>>()
                .and_then(|child| child.cast::<TextBlock>())
            {
                text.classes().add("phone-switch-header");
            }
        }));
    }
    presenter
}

fn template(compact: bool) -> Rc<dyn IControlTemplate> {
    let construct = |slot: &OnceCell<Rc<dyn IControlTemplate>>| {
        slot.get_or_init(|| {
            FuncControlTemplate::for_type::<ToggleSwitch>(move |_, scope| {
                let root = Grid::new();
                root.set_background(Some(Brushes::transparent()));
                root.set_column_definitions(
                    ColumnDefinitions::parse(if compact { "Auto" } else { "*,Auto" })
                        .expect("fixed switch columns"),
                );
                root.set_column_spacing(if compact { 0.0 } else { px(12.0) });

                if !compact {
                    let labels = StackPanel::new();
                    labels.set_vertical_alignment(V::Center);
                    labels.children().add(
                        presenter(
                            "PART_ContentPresenter",
                            ContentControl::content_property().as_property(),
                        )
                        .register_in_name_scope(&**scope),
                    );
                    let state = Grid::new();
                    state.children().add(
                        presenter(
                            "PART_OffContentPresenter",
                            ToggleSwitch::off_content_property().as_property(),
                        )
                        .register_in_name_scope(&**scope),
                    );
                    state.children().add(
                        presenter(
                            "PART_OnContentPresenter",
                            ToggleSwitch::on_content_property().as_property(),
                        )
                        .register_in_name_scope(&**scope),
                    );
                    labels.children().add(state);
                    root.children().add(labels);
                }

                let area = Grid::new();
                // Alarm rows reserve 78 pixels. At the reference scale the
                // thumb foreground remains inside the 74.17-pixel track;
                // only its background-colored outer cutout can overhang.
                let area_width = if compact { 78.0 } else { px(136.0) };
                area.set_width(area_width);
                area.set_height(if compact { 44.0 } else { 66.0 });
                area.set_clip_to_bounds(compact);
                area.set_vertical_alignment(V::Center);
                area.set_background(Some(Brushes::transparent()));
                Grid::set_column(&area, if compact { 0 } else { 1 });

                let track = Border::new();
                track.set_name(Some("PhoneSwitchTrack".into()));
                track.set_width(SWITCH_TRACK_WIDTH);
                track.set_height(SWITCH_TRACK_HEIGHT);
                track.set_horizontal_alignment(H::Center);
                track.set_vertical_alignment(V::Center);
                let outline = Border::new();
                outline.set_name(Some("PhoneSwitchOutline".into()));
                outline.set_border_thickness(Thickness::uniform(CONTROL_BORDER));
                let inset = Border::new();
                inset.set_name(Some("PhoneSwitchInset".into()));
                inset.set_border_thickness(Thickness::uniform(px(4.0)));
                let clip = Grid::new();
                clip.set_clip_to_bounds(true);
                let cover = Border::new();
                cover.set_name(Some("PhoneSwitchCover".into()));
                cover.set_width(px(77.0));
                cover.set_horizontal_alignment(H::Left);
                // Borders and explicit sizes round independently. Fill the
                // actual inner clip so the off state cannot expose accent
                // rows above or below a shorter, fixed-height mask.
                cover.set_vertical_alignment(V::Stretch);
                let cover_translation = TranslateTransform::new();
                cover.set_render_transform(Some(cover_translation.clone().into()));
                clip.children().add(cover.register_in_name_scope(&**scope));
                inset.set_child(clip);
                outline.set_child(inset.register_in_name_scope(&**scope));
                track.set_child(outline.register_in_name_scope(&**scope));
                area.children().add(track.register_in_name_scope(&**scope));

                // The native switch measures this Canvas as the thumb's travel.
                // Its moving panel owns capture/drag, and Canvas.Left is the single
                // source of position for both thumb and the sliding track cover.
                let canvas = Canvas::new();
                canvas.set_name(Some("PART_SwitchKnob".into()));
                canvas.set_width(SWITCH_TRAVEL);
                canvas.set_height(44.0);
                canvas.set_horizontal_alignment(H::Left);
                canvas.set_vertical_alignment(V::Center);
                canvas.set_margin(Thickness::new(
                    (area_width - SWITCH_TRACK_WIDTH) / 2.0 - px(4.0),
                    0.0,
                    0.0,
                    0.0,
                ));
                let moving = Grid::new();
                moving.set_name(Some("PART_MovingKnobs".into()));
                moving.set_width(SWITCH_THUMB_WIDTH);
                moving.set_height(44.0);
                moving.set_background(Some(Brushes::transparent()));
                let cutout = Border::new();
                cutout.set_name(Some("PhoneSwitchThumbCutout".into()));
                cutout.set_width(SWITCH_THUMB_WIDTH);
                cutout.set_height(SWITCH_THUMB_HEIGHT);
                cutout.set_vertical_alignment(V::Center);
                cutout.set_border_thickness(Thickness::new(px(4.0), 0.0, px(4.0), 0.0));
                let thumb = Border::new();
                thumb.set_name(Some("PhoneSwitchThumb".into()));
                thumb.set_border_thickness(Thickness::uniform(px(2.0)));
                cutout.set_child(thumb.register_in_name_scope(&**scope));
                moving
                    .children()
                    .add(cutout.register_in_name_scope(&**scope));
                let weak_cover = cover_translation.downgrade();
                moving.property_changed(move |change| {
                    if change.property() == Canvas::left_property().as_property() {
                        if let Some(translation) = weak_cover.upgrade() {
                            let x = change.get_new_value::<f64>();
                            translation.set_x(if x.is_finite() { x } else { 0.0 });
                        }
                    }
                });
                canvas
                    .children()
                    .add(moving.register_in_name_scope(&**scope));
                area.children().add(canvas.register_in_name_scope(&**scope));
                root.children().add(area);
                root.upcast()
            }) as Rc<dyn IControlTemplate>
        })
        .clone()
    };
    if compact {
        COMPACT_TEMPLATE.with(construct)
    } else {
        TEMPLATE.with(construct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_base::media::{Color, SolidColorBrush};
    use ferroui_base::{Point, Rect, Size};
    use std::cell::RefCell;

    #[test]
    fn palette_changes_keep_the_mounted_switch_template_and_native_parts() {
        for compact in [false, true] {
            let root = StackPanel::new();
            install_templates(&root.styles());
            let colors = Styles::new();
            install(&colors, &Palette::new("#1BA1E2", false));
            root.styles().add(colors.clone());
            let switch = ToggleSwitch::new();
            switch.classes().add("phone-switch");
            if compact {
                switch.classes().add("phone-switch-compact");
            }
            switch.set_is_checked(Some(true));
            root.children().add(switch.clone());
            switch.apply_styling();
            switch.apply_styling();
            switch.apply_template();
            let template = switch.template().expect("the native phone template");
            let moving = switch
                .get_template_descendants()
                .into_iter()
                .find(|part| part.has_name("PART_MovingKnobs"))
                .expect("the required native drag panel");

            for (light, motion_enabled) in [(false, false), (true, false), (false, true)] {
                let mut palette = Palette::new("#00ABA9", light);
                palette.motion_enabled = motion_enabled;
                colors.clear();
                install(&colors, &palette);
                // Layout applies styles once before applying a template. The
                // current native template must survive that real ordering.
                switch.apply_styling();
                let current = switch
                    .template()
                    .expect("palette replacement keeps a template");
                switch.apply_template();
                assert!(Rc::ptr_eq(&template, &current));
                let current_moving = switch
                    .get_template_descendants()
                    .into_iter()
                    .find(|part| part.has_name("PART_MovingKnobs"))
                    .expect("palette replacement keeps the native drag panel");
                assert!(Ref::ptr_eq(&moving, &current_moving));
                assert_eq!(switch.is_checked(), Some(true));
                assert_eq!(switch.knob_transitions().is_some(), motion_enabled);
            }
        }
    }

    #[test]
    fn rounded_switch_mask_covers_the_off_track_and_follows_native_travel() {
        let mut palette = Palette::new("#1BA1E2", false);
        palette.motion_enabled = false;
        let root = StackPanel::new();
        install_templates(&root.styles());
        install(&root.styles(), &palette);
        let switch = ToggleSwitch::new();
        switch.classes().add("phone-switch");
        switch.classes().add("phone-switch-compact");
        switch.set_use_layout_rounding(true);
        switch.set_is_checked(Some(false));
        root.children().add(switch.clone());
        switch.apply_styling();
        switch.apply_styling();
        switch.apply_template();
        let part = |name: &str| {
            switch
                .get_template_descendants()
                .into_iter()
                .filter_map(|visual| visual.cast::<Control>())
                .find(|control| control.has_name(name))
                .unwrap_or_else(|| panic!("native switch part {name}"))
        };
        let cover = part("PhoneSwitchCover");
        let moving = part("PART_MovingKnobs");
        let canvas = part("PART_SwitchKnob");
        let clip = cover.visual_parent().expect("the track's inner clip");
        let layout = Rect::new(0.0, 0.0, 78.0, 44.0);
        switch.measure(Size::new(layout.width, layout.height));
        switch.arrange(layout);

        // At DPR 1 the 29-pixel track and independently rounded borders leave
        // a 19-pixel opening. The former fixed-height mask rounded to 17,
        // exposing one accent-colored row along each edge in the off state.
        assert_eq!(clip.bounds().height, 19.0);
        assert_eq!(cover.bounds().y, 0.0);
        assert_eq!(cover.bounds().height, clip.bounds().height);
        assert!(cover.bounds().width >= clip.bounds().width);
        assert_eq!(cover.width(), px(77.0), "reference mask width is preserved");
        assert_eq!(canvas.width(), SWITCH_TRAVEL);
        assert_eq!(moving.width(), SWITCH_THUMB_WIDTH);

        for checked in [false, true, false] {
            switch.set_is_checked(Some(checked));
            switch.arrange(layout);
            // This isolated tree has no window layout manager to service the
            // Canvas invalidation caused by its child's new attached Left.
            canvas.arrange(
                canvas
                    .previous_arrange()
                    .expect("arranged native travel surface"),
            );
            let expected = if checked { canvas.bounds().width } else { 0.0 };
            assert_eq!(Canvas::get_left(&moving), expected);
            assert_eq!(moving.bounds().x, expected);
            let mask_origin = cover.translate_point(Point::new(0.0, 0.0), &clip).unwrap();
            assert_eq!(
                mask_origin.x, expected,
                "mask follows the native thumb in both directions"
            );
            assert_eq!(mask_origin.y, 0.0);
            assert_eq!(cover.bounds().height, clip.bounds().height);
        }
    }

    #[test]
    fn caption_template_keeps_secondary_ink_above_the_general_text_style() {
        for light in [false, true] {
            let palette = Palette::new("#00ABA9", light);
            let root = StackPanel::new();
            install_templates(&root.styles());
            root.styles().add(
                style("TextBlock")
                    .set(TextBlock::foreground_property(), palette.foreground)
                    .build(),
            );
            install(&root.styles(), &palette);
            let switch = ToggleSwitch::new();
            switch.classes().add("phone-switch");
            switch.set_content(Some(
                Rc::new(String::from("Vibrate")) as ferroui_base::BoxedValue
            ));
            root.children().add(switch.clone());
            // Apply the class styles before building the native template.
            switch.apply_styling();
            switch.apply_styling();
            switch.apply_template();
            let caption = switch
                .get_template_descendants()
                .into_iter()
                .filter_map(|child| child.cast::<ContentPresenter>())
                .find(|presenter| presenter.has_name("PART_ContentPresenter"))
                .expect("the native caption presenter");
            caption.apply_styling();
            caption.update_child();
            let text = caption.child().unwrap().cast::<TextBlock>().unwrap();
            text.apply_styling();
            assert_eq!(
                text.foreground()
                    .unwrap()
                    .as_solid_color_brush()
                    .unwrap()
                    .color(),
                Color::parse(palette.secondary).unwrap(),
                "the generated caption overrides primary ink in both palettes"
            );

            let custom = TextBlock::new();
            let explicit = Color::parse("#FFE000").unwrap();
            custom.set_foreground(Some(SolidColorBrush::with_color(explicit).into()));
            switch.set_content(Some(Control::boxed(custom.clone())));
            caption.update_child();
            custom.apply_styling();
            assert_eq!(caption.child(), Some(custom.clone().upcast()));
            assert_eq!(
                custom
                    .foreground()
                    .unwrap()
                    .as_solid_color_brush()
                    .unwrap()
                    .color(),
                explicit,
                "control content keeps its explicit foreground"
            );
        }
    }

    #[test]
    fn shared_switch_observes_native_state_changes_from_more_than_clicks() {
        let values = Rc::new(RefCell::new(Vec::new()));
        let recorded = values.clone();
        let (control, owner) = crate::toggle("flight mode", true, move |value| {
            recorded.borrow_mut().push(value)
        })
        .mount_root();
        let switch = control
            .cast::<ToggleSwitch>()
            .expect("native drag-capable switch");
        assert_eq!(switch.is_checked(), Some(true));
        assert!(values.borrow().is_empty());
        // The native drag and UI Automation paths update IsChecked without a
        // Click event. The old illustrated ToggleButton dropped those updates.
        switch.set_is_checked(Some(false));
        switch.set_is_checked(Some(true));
        assert_eq!(*values.borrow(), [false, true]);
        owner.dispose();
    }

    #[test]
    fn reduced_motion_removes_switch_transition_without_changing_geometry() {
        assert!(knob_transitions(false).is_none());
        assert!(knob_transitions(true).is_some());
        assert!((SWITCH_TRACK_WIDTH - SWITCH_TRAVEL - crate::reference_pixels(20.0)).abs() < 1e-10);
    }

    #[test]
    fn native_keyboard_toggle_is_single_and_disabled_switch_does_not_publish() {
        let values = Rc::new(RefCell::new(Vec::new()));
        let recorded = values.clone();
        let (control, owner) = crate::toggle("flight mode", false, move |value| {
            recorded.borrow_mut().push(value)
        })
        .mount_root();
        let switch = control.cast::<ToggleSwitch>().unwrap();
        let key = || {
            let mut event = KeyEventArgs::new();
            event.key = Key::Enter;
            event.set_routed_event(Some(InputElement::key_down_event()));
            event
        };
        switch.raise_event(&key());
        assert_eq!(switch.is_checked(), Some(true));
        assert_eq!(*values.borrow(), [true]);
        switch.set_is_enabled(false);
        switch.raise_event(&key());
        assert_eq!(switch.is_checked(), Some(true));
        assert_eq!(*values.borrow(), [true]);
        switch.set_is_enabled(true);
        switch.raise_event(&key());
        assert_eq!(switch.is_checked(), Some(false));
        assert_eq!(*values.borrow(), [true, false]);
        owner.dispose();
    }

    #[test]
    fn unlabeled_alarm_switch_uses_compact_native_presentation() {
        let (control, owner) = crate::toggle("", false, |_| {}).mount_root();
        let switch = control.cast::<ToggleSwitch>().unwrap();
        assert!(switch.classes().contains("phone-switch-compact"));
        assert_eq!(switch.min_height(), 44.0);
        owner.dispose();
    }
}
