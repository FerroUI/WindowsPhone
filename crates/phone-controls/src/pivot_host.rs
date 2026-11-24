//! Native cyclic phone pivots. Each pane keeps its own retained content and
//! vertical scroll viewer while FerroUI owns horizontal manipulation and focus.
use crate::Palette;
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::{
    HorizontalAlignment as H, Layoutable, Orientation, VerticalAlignment as V,
};
use ferroui_base::media::{Brushes, FontFamily, FontWeight};
use ferroui_base::styling::Styles;
use ferroui_base::{BoxedValue, Visual};
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::{
    ScrollBarVisibility, SnapPointsAlignment, SnapPointsType, TemplatedControl,
};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{
    ContentControl, Control, Grid, ItemsControl, Panel, Pivot, PivotHeaderPanel, PivotPanel,
    RowDefinitions, ScrollViewer, TextBlock, ZoomMode,
};
use ferroui_view::{prelude::*, style};
use std::{cell::OnceCell, rc::Rc};

ferroui_base::ferro_thread_local! {
    static TEMPLATE: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
}

#[derive(Clone)]
pub struct PivotPage {
    pub header: String,
    pub content: Element,
}

impl PivotPage {
    pub fn new(header: impl Into<String>, content: Element) -> Self {
        Self {
            header: header.into(),
            content,
        }
    }
}

/// Create a phone pivot with retained neighboring panes and a shared selection.
/// Put a vertically scrolling view in each page; do not wrap the pivot in one.
/// For reduced motion, use the same headings and selection without compositor
/// animations. The shell's registered pivot count keeps swipe navigation active.
pub fn pivot_pages(name: &str, selection: Signal<usize>, pages: Vec<PivotPage>) -> Element {
    if pages.is_empty() {
        return Element::Empty;
    }
    let count = pages.len();
    if !crate::motion_enabled() {
        let selected = selection.get().min(count - 1);
        let mut headings = stack_panel()
            .orientation(Orientation::Horizontal)
            .margin((20.0, 0.0, 0.0, 8.0));
        for (index, page) in pages.iter().enumerate() {
            headings = headings.child(crate::pivot(&page.header, index == selected, move || {
                selection.set(index)
            }));
        }
        let header = crate::pivot_header(headings.into(), &pages[selected].header);
        let content = pages
            .into_iter()
            .nth(selected)
            .expect("bounded pivot index")
            .content;
        return rsx! {Grid {name:name,row_definitions:"Auto,*",clip_to_bounds:true,
            {header} Border {Grid.row:1,{content}}
        }};
    }
    let mut pivot = ferroui_view::prelude::pivot()
        .name(name)
        .class("phone-pivot-host")
        .class("phone-native-input")
        .background(crate::palette().background)
        .is_header_items_carousel_enabled(true)
        .selected_index(move || selection.get().min(count - 1) as i32)
        .on_selection_changed(move |event| {
            let Some(pivot) = event.source().and_then(|source| source.cast::<Pivot>()) else {
                return;
            };
            let index = pivot.selected_index();
            if index >= 0 && (index as usize) < count && selection.get_untracked() != index as usize
            {
                selection.set(index as usize);
            }
        });
    for page in pages {
        let header: BoxedValue = Rc::new(page.header.clone());
        pivot = pivot.child(
            pivot_item()
                .key(page.header.clone())
                .header(Some(header))
                .automation_properties_name(page.header)
                .margin(0.0)
                .padding(0.0)
                .child(page.content),
        );
    }
    pivot.into()
}

pub(crate) fn install(styles: &Styles, p: &Palette) {
    styles.add(
        style("Pivot.phone-pivot-host")
            .set(
                TemplatedControl::template_property(),
                Some(phone_template()),
            )
            .build(),
    );
    styles.add(
        style("PivotHeaderItem")
            .set(
                TemplatedControl::font_family_property(),
                FontFamily::new(crate::FONT_FAMILY),
            )
            .set(
                TemplatedControl::font_size_property(),
                crate::FONT_EXTRA_LARGE * p.text_scale,
            )
            .set(TemplatedControl::font_weight_property(), FontWeight::Light)
            .set(TemplatedControl::foreground_property(), p.secondary)
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::padding_property(), (0.0, 0.0, 22.0, 8.0))
            .set(Layoutable::height_property(), 64.0 * p.text_scale)
            .set(
                ContentControl::horizontal_content_alignment_property(),
                H::Left,
            )
            .build(),
    );
    // The default header data template makes a logical TextBlock child of the
    // native header. Apply its ink directly so the general app TextBlock style
    // cannot replace the header's inherited selection color. Explicit local
    // colors in a custom header template retain their higher priority.
    styles.add(
        style("PivotHeaderItem TextBlock")
            .set(TextBlock::foreground_property(), p.secondary)
            .build(),
    );
    for state in [":pointerover", ":pressed", ":pointerover:pressed"] {
        styles.add(
            style(&format!("PivotHeaderItem{state}"))
                .set(TemplatedControl::foreground_property(), p.secondary)
                .set(TemplatedControl::background_property(), "Transparent")
                .build(),
        );
        styles.add(
            style(&format!("PivotHeaderItem{state} TextBlock"))
                .set(TextBlock::foreground_property(), p.secondary)
                .build(),
        );
    }
    for state in [
        ":selected",
        ":selected:pointerover",
        ":selected:pressed",
        ":selected:pointerover:pressed",
    ] {
        styles.add(
            style(&format!("PivotHeaderItem{state}"))
                .set(TemplatedControl::foreground_property(), p.foreground)
                .set(TemplatedControl::background_property(), "Transparent")
                .build(),
        );
        styles.add(
            style(&format!("PivotHeaderItem{state} TextBlock"))
                .set(TextBlock::foreground_property(), p.foreground)
                .build(),
        );
        // The desktop theme shows its selection pipe with a state trigger;
        // a base style alone cannot override that trigger's priority.
        styles.add(
            style(&format!(
                "PivotHeaderItem{state} /template/ Rectangle#SelectedPipe"
            ))
            .set(Visual::is_visible_property(), false)
            .build(),
        );
    }
    styles.add(
        style("PivotHeaderItem /template/ Rectangle#SelectedPipe")
            .set(Visual::is_visible_property(), false)
            .build(),
    );
    styles.add(
        style("Pivot.phone-pivot-host /template/ ContentControl#HeaderClipper")
            .set(Layoutable::margin_property(), (20.0, 0.0, 0.0, 0.0))
            .build(),
    );
}

/// Use the native Pivot's dynamic-only template contract. WinUI's desktop
/// template also supplies `StaticHeader`, causing the native control to stop
/// wrapping whenever the labels fit. Omitting that optional part preserves the
/// phone's cyclic headers, page selection, parallax and snap points at any width.
/// All pointer capture, axis arbitration and selection remain in FerroUI.
fn phone_template() -> Rc<dyn IControlTemplate> {
    TEMPLATE.with(|slot| {
        slot.get_or_init(|| {
            FuncControlTemplate::for_type::<Pivot>(|_, scope| {
                let header = PivotHeaderPanel::new();
                header.set_name(Some("Header".into()));
                let header_grid = Grid::new();
                header_grid.set_background(Some(Brushes::transparent()));
                header_grid
                    .children()
                    .add(header.register_in_name_scope(&**scope));

                let focus = Rectangle::new();
                focus.set_name(Some("FocusFollower".into()));
                focus.set_fill(Some(Brushes::transparent()));
                focus.set_is_hit_test_visible(false);
                header_grid
                    .children()
                    .add(focus.register_in_name_scope(&**scope));

                let clipper = ContentControl::new();
                clipper.set_name(Some("HeaderClipper".into()));
                clipper.set_focusable(true);
                clipper.set_clip_to_bounds(true);
                clipper.set_horizontal_content_alignment(H::Stretch);
                clipper.set_content(Some(Control::boxed(header_grid)));

                let items = ItemsPresenter::new();
                items.set_name(Some("PivotItemPresenter".into()));
                let property = ItemsControl::items_panel_property().as_property();
                items.bind_binding(property, &*TemplateBinding::new(property));
                Grid::set_row(&items, 1);

                let layout = Grid::new();
                layout.set_name(Some("PivotLayoutElement".into()));
                layout.set_row_definitions(RowDefinitions::parse("Auto,*").expect("pivot rows"));
                layout
                    .children()
                    .add(clipper.register_in_name_scope(&**scope));
                layout
                    .children()
                    .add(items.register_in_name_scope(&**scope));

                let panel = PivotPanel::new();
                panel.set_name(Some("Panel".into()));
                panel.set_vertical_alignment(V::Stretch);
                panel
                    .children()
                    .add(layout.register_in_name_scope(&**scope));

                let scroll = ScrollViewer::new();
                scroll.set_name(Some("ScrollViewer".into()));
                scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
                scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
                scroll.set_horizontal_snap_points_type(SnapPointsType::MandatorySingle);
                scroll.set_horizontal_snap_points_alignment(SnapPointsAlignment::Center);
                scroll.set_vertical_snap_points_type(SnapPointsType::None);
                scroll.set_vertical_content_alignment(V::Stretch);
                scroll.set_zoom_mode(ZoomMode::Disabled);
                ScrollViewer::set_bring_into_view_on_focus_change_on(&scroll, false);
                scroll.bind_binding(
                    Layoutable::margin_property().as_property(),
                    &*TemplateBinding::new(TemplatedControl::padding_property().as_property()),
                );
                scroll.set_content(Some(Control::boxed(panel.register_in_name_scope(&**scope))));
                // Keep the installed native ScrollViewer template: its
                // ScrollGestureRecognizer supplies touch capture, inertia and
                // the viewport/extent bindings used by the Pivot state machine.
                let root = Grid::new();
                root.set_name(Some("RootElement".into()));
                root.bind_binding(
                    Panel::background_property().as_property(),
                    &*TemplateBinding::new(TemplatedControl::background_property().as_property()),
                );
                root.children().add(scroll.register_in_name_scope(&**scope));
                root.register_in_name_scope(&**scope).upcast()
            })
        })
        .clone()
    })
}
