//! Phone presentation over the native Slider/Track/Thumb input contract.
use crate::Palette;
use ferroui_base::data::{BindingMode, TemplateBinding};
use ferroui_base::layout::{
    HorizontalAlignment as H, Layoutable, Orientation, VerticalAlignment as V,
};
use ferroui_base::media::Brushes;
use ferroui_base::styling::{ControlTheme, Setter, Styles};
use ferroui_base::{Ref, StyledElement, Visual};
use ferroui_controls::primitives::{RangeBase, TemplatedControl, Thumb, Track};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{Border, Button, Grid, RepeatButton, Slider};
use ferroui_view::style;
use std::{cell::OnceCell, rc::Rc};

ferroui_base::ferro_thread_local! {
    static HORIZONTAL: OnceCell<Ref<ControlTheme>> = const { OnceCell::new() };
    static VERTICAL: OnceCell<Ref<ControlTheme>> = const { OnceCell::new() };
}

pub(crate) fn install(styles: &Styles, palette: &Palette) {
    for (pseudo, vertical) in [("horizontal", false), ("vertical", true)] {
        let theme = if vertical {
            VERTICAL.with(|slot| theme(slot, true))
        } else {
            HORIZONTAL.with(|slot| theme(slot, false))
        };
        styles.add(
            style(&format!("Slider.phone-slider:{pseudo}"))
                .set(StyledElement::theme_property(), Some(theme))
                .set(TemplatedControl::background_property(), palette.subtle)
                .set(
                    TemplatedControl::foreground_property(),
                    palette.accent.as_str(),
                )
                .set(TemplatedControl::corner_radius_property(), 0.0)
                .set(
                    if vertical {
                        Layoutable::min_width_property()
                    } else {
                        Layoutable::min_height_property()
                    },
                    44.0,
                )
                .build(),
        );
    }
    styles.add(
        style("Slider.phone-slider /template/ Thumb#PhoneSliderThumb")
            .set(TemplatedControl::background_property(), palette.foreground)
            .build(),
    );
    styles.add(
        style("Slider.phone-slider /template/ RepeatButton#PART_DecreaseButton")
            .set(
                TemplatedControl::background_property(),
                palette.accent.as_str(),
            )
            .build(),
    );
    // The hardware flyout retains its dark surface even in the light phone theme.
    styles.add(
        style("Border#PhoneVolumePanel Slider.phone-slider /template/ Thumb#PhoneSliderThumb")
            .set(TemplatedControl::background_property(), "White")
            .build(),
    );
    styles.add(
        style("Slider.phone-slider:disabled")
            .set(Visual::opacity_property(), 0.4)
            .build(),
    );
}

fn theme(slot: &OnceCell<Ref<ControlTheme>>, vertical: bool) -> Ref<ControlTheme> {
    slot.get_or_init(|| {
        let theme = ControlTheme::for_type::<Slider>();
        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(move |_, scope| {
            let root = Grid::new();
            root.set_background(Some(Brushes::transparent()));
            let rail = Border::new();
            if vertical {
                rail.set_width(8.0);
                rail.set_horizontal_alignment(H::Center);
            } else {
                rail.set_height(8.0);
                rail.set_vertical_alignment(V::Center);
            }
            rail.bind_binding(
                Border::background_property().as_property(),
                &*TemplateBinding::new(TemplatedControl::background_property().as_property()),
            );
            root.children().add(rail);
            let track = Track::new();
            track.set_name(Some("PART_Track".into()));
            track.set_orientation(if vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            });
            for (target, source) in [
                (
                    Track::minimum_property().as_property(),
                    RangeBase::minimum_property().as_property(),
                ),
                (
                    Track::maximum_property().as_property(),
                    RangeBase::maximum_property().as_property(),
                ),
                (
                    Track::is_direction_reversed_property().as_property(),
                    Slider::is_direction_reversed_property().as_property(),
                ),
            ] {
                track.bind_binding(target, &*TemplateBinding::new(source));
            }
            track.bind_binding(
                Track::value_property().as_property(),
                &*TemplateBinding::new(RangeBase::value_property().as_property())
                    .with_mode(BindingMode::TwoWay),
            );
            for (name, decrease) in [
                ("PART_DecreaseButton", true),
                ("PART_IncreaseButton", false),
            ] {
                let button = RepeatButton::new();
                button.set_name(Some(name.into()));
                button.set_focusable(false);
                // WinUI's generic RepeatButton aligns left/center. Track parts
                // must fill their allocated range, including the hit surface.
                button.set_horizontal_alignment(H::Stretch);
                button.set_vertical_alignment(V::Stretch);
                button.set_min_width(0.0);
                button.set_min_height(0.0);
                button.set_template(Some(part_template(vertical, false)));
                if !decrease {
                    button.set_background(Some(Brushes::transparent()));
                }
                let button = button.register_in_name_scope(&**scope);
                if decrease {
                    track.set_decrease_button(button.upcast::<Button>());
                } else {
                    track.set_increase_button(button.upcast::<Button>());
                }
            }
            let thumb = Thumb::new();
            thumb.set_name(Some("PhoneSliderThumb".into()));
            thumb.set_width(if vertical { 44.0 } else { 24.0 });
            thumb.set_height(if vertical { 24.0 } else { 44.0 });
            thumb.set_template(Some(part_template(vertical, true)));
            let thumb = thumb.register_in_name_scope(&**scope);
            track.set_thumb(thumb);
            root.children().add(track.register_in_name_scope(&**scope));
            root.upcast()
        });
        theme.add_setter(Setter::new(
            TemplatedControl::template_property(),
            Some(template),
        ));
        theme
    })
    .clone()
}

fn part_template(vertical: bool, thumb: bool) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(move |_, _| {
        let hit = Grid::new();
        hit.set_background(Some(Brushes::transparent()));
        let fill = Border::new();
        if thumb {
            fill.set_width(if vertical { 28.0 } else { 8.0 });
            fill.set_height(if vertical { 8.0 } else { 28.0 });
            fill.set_horizontal_alignment(H::Center);
            fill.set_vertical_alignment(V::Center);
        } else if vertical {
            fill.set_width(8.0);
            fill.set_horizontal_alignment(H::Center);
        } else {
            fill.set_height(8.0);
            fill.set_vertical_alignment(V::Center);
        }
        fill.bind_binding(
            Border::background_property().as_property(),
            &*TemplateBinding::new(TemplatedControl::background_property().as_property()),
        );
        hit.children().add(fill);
        hit.upcast()
    })
}
