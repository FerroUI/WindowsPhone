use super::presentation::Presentation;
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{Brushes, FontFamily, FontWeight, TextWrapping};
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{Border, Canvas, Carousel, Control, Grid, ItemsControl, TextBlock};
use std::{cell::OnceCell, rc::Rc};

ferroui_base::ferro_thread_local! {
    static TEMPLATE: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
}

pub(super) fn template() -> Rc<dyn IControlTemplate> {
    TEMPLATE.with(|slot| {
        slot.get_or_init(|| {
            FuncControlTemplate::for_type::<Carousel>(|owner, scope| {
                let root = Grid::new();
                root.set_name(Some("PanoramaRoot".into()));
                root.set_background(Some(Brushes::transparent()));
                root.set_clip_to_bounds(true);

                let backgrounds = (0..2)
                    .map(|index| {
                        let part = Border::new();
                        part.set_name(Some(format!("PanoramaBackground{index}")));
                        part.set_horizontal_alignment(H::Left);
                        part.set_vertical_alignment(V::Stretch);
                        part.set_is_hit_test_visible(false);
                        part.bind_binding(
                            Border::background_property().as_property(),
                            &*TemplateBinding::new(
                                TemplatedControl::background_property().as_property(),
                            ),
                        );
                        root.children()
                            .add(part.clone().register_in_name_scope(&**scope));
                        part
                    })
                    .collect::<Vec<_>>();

                let items = ItemsPresenter::new();
                items.set_name(Some("PART_ItemsPresenter".into()));
                let panel_property = ItemsControl::items_panel_property().as_property();
                items.bind_binding(panel_property, &*TemplateBinding::new(panel_property));

                // Fractional Carousel owns its logical cyclic offset. A
                // bounded ScrollViewer would coerce transient wrapped offsets
                // and feed them back into the native panel, causing edge snaps
                // and selection drift during viewport changes. Its optional
                // PART_ScrollViewer is unnecessary for this fractional host.
                let viewport = Border::new();
                viewport.set_name(Some("PanoramaViewport".into()));
                viewport.set_background(Some(Brushes::transparent()));
                viewport.set_clip_to_bounds(true);
                viewport.set_child(items.register_in_name_scope(&**scope).upcast::<Control>());
                root.children()
                    .add(viewport.clone().register_in_name_scope(&**scope));

                // Only two-section cycles need a second render instance of a
                // section at the seam. Keep it outside the live item subtree,
                // clipped by the viewport and excluded from input/automation.
                let wrap_preview = Canvas::new();
                wrap_preview.set_name(Some("PanoramaWrapPreview".into()));
                wrap_preview.set_is_hit_test_visible(false);
                wrap_preview.set_clip_to_bounds(true);
                root.children()
                    .add(wrap_preview.clone().register_in_name_scope(&**scope));

                // Canvas measures the single title at its natural width. The
                // viewport clips the large text while the title itself keeps its
                // untruncated glyph run during parallax and cyclic reentry.
                let title_layer = Canvas::new();
                title_layer.set_name(Some("PanoramaTitleLayer".into()));
                title_layer.set_is_hit_test_visible(false);
                let title = TextBlock::new();
                title.set_name(Some("PanoramaTitle".into()));
                title.set_font_family(FontFamily::new(crate::FONT_FAMILY));
                title.set_font_weight(FontWeight::Light);
                title.set_text_wrapping(TextWrapping::NoWrap);
                title.set_is_hit_test_visible(false);
                Canvas::set_left(&title, crate::PAGE_MARGIN);
                Canvas::set_top(&title, super::layout::TITLE_TOP);
                title_layer
                    .children()
                    .add(title.clone().register_in_name_scope(&**scope));
                root.children()
                    .add(title_layer.register_in_name_scope(&**scope));

                let state = Presentation::attach(
                    owner,
                    &root,
                    &viewport,
                    &title,
                    &backgrounds,
                    &wrap_preview,
                );
                // The template, rather than an application callback, owns the
                // subscriptions. Every callback points back weakly.
                root.set_tag(Some(state));
                root.register_in_name_scope(&**scope).upcast()
            }) as Rc<dyn IControlTemplate>
        })
        .clone()
    })
}
