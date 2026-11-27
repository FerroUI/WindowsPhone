//! A layered phone Panorama over FerroUI's native cyclic Carousel input.
//!
//! The public Windows Phone Panorama architecture specifies independent title,
//! background and item layers, finger-tracking sections and a next-section
//! preview. This implementation reuses native selection, gesture arbitration,
//! recycling and keyboard navigation. The parallax ratios and font placement
//! are explicit adaptations pending matched original-device measurements.
use ferroui_base::animation::{IPageTransition, PageSlide, SignedDuration, SlideAxis};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::styling::Styles;
use ferroui_base::{AttachedProperty, FerroProperty, ferro_property};
use ferroui_controls::Carousel;
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControl};
use ferroui_view::{prelude::*, style};
use std::rc::Rc;

// Configuration belongs to a private attached property. The returned native
// builder's Tag remains available for application-owned state.
struct Configuration;
ferroui_base::ferro_static_type!(Configuration);
ferroui_base::ferro_properties! { impl Configuration {
    ferro_property!(
        fn title_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<Configuration, Carousel, _>("Title", None)
        }
    );
} }

mod admission;
mod layout;
mod policy;
mod presentation;
mod template;
mod wrap_preview;

#[cfg(test)]
mod policy_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wrap_tests;

pub(crate) use policy::refresh_motion;

/// One independently retained content section. Put a vertical ScrollViewer in
/// `content` when necessary. Horizontal editors/maps belong on detail routes.
#[derive(Clone)]
pub struct PanoramaPage {
    pub header: String,
    pub content: Element,
}

impl PanoramaPage {
    pub fn new(header: impl Into<String>, content: Element) -> Self {
        Self {
            header: header.into(),
            content,
        }
    }
}

/// Build a native phone Panorama. The returned Carousel builder permits normal
/// styling (including an image/gradient `background`), naming and automation.
/// The title, background and page content are separate retained template parts.
/// At reduced motion, the host's registered section policy can handle touch;
/// keyboard selection remains native and changes immediately.
pub fn panorama_pages(
    name: &str,
    title: &str,
    selection: Signal<usize>,
    pages: Vec<PanoramaPage>,
) -> El<Carousel> {
    let count = pages.len();
    let transition: Option<Rc<dyn IPageTransition>> = Some({
        Rc::new(PageSlide::with_duration(
            SignedDuration::from_milliseconds(350.0),
            SlideAxis::Horizontal,
        )) as Rc<dyn IPageTransition>
    });
    let mut host = carousel()
        .name(name)
        .class("phone-panorama")
        .set(Configuration::title_property(), Some(title.to_owned()))
        .automation_properties_name(title)
        .background(crate::palette().background)
        .clip_to_bounds(true)
        .focusable(true)
        .wrap_selection(true)
        .auto_scroll_to_selected_item(false)
        .is_swipe_enabled(true)
        // A finite initial fraction ensures the native panel is created in
        // fractional mode. The measured template replaces it before input.
        .viewport_fraction(if count > 1 { 0.8 } else { 1.0 })
        .page_transition(transition)
        .selected_index(move || {
            if count == 0 {
                -1
            } else {
                selection.get().min(count - 1) as i32
            }
        })
        .on_selection_changed(move |event| {
            let Some(host) = event.source().and_then(|source| source.cast::<Carousel>()) else {
                return;
            };
            let index = host.selected_index();
            if index >= 0 && (index as usize) < count && selection.get_untracked() != index as usize
            {
                selection.set(index as usize);
            }
        });
    let title_height = layout::TITLE_HEIGHT * crate::text_scale();
    for (index, page) in pages.into_iter().enumerate() {
        let name = format!("{name}-section-{index}");
        let mut section = grid()
            .key(name.clone())
            .name(&name)
            .class("phone-panorama-section")
            .background("Transparent")
            .clip_to_bounds(true)
            .row_definitions(&format!("{title_height},Auto,*"));
        if !page.header.is_empty() {
            section = section.child(rsx! {TextBlock {
                name:format!("{name}-header"), Grid.row:1,
                text:page.header, font_size:crate::FONT_EXTRA_LARGE*crate::text_scale(),
                font_weight:FontWeight::Light, text_wrapping:TextWrapping::NoWrap,
                margin:(crate::PAGE_MARGIN,0.0,0.0,10.0)
            }});
        }
        section = section.child(rsx! {Border {Grid.row:2,clip_to_bounds:true,{page.content}}});
        host = host.child(section);
    }
    host
}

pub(crate) fn install_template(styles: &Styles) {
    styles.add(
        style("Carousel.phone-panorama")
            .set(
                SelectingItemsControl::auto_scroll_to_selected_item_property(),
                false,
            )
            .set(
                TemplatedControl::template_property(),
                Some(template::template()),
            )
            .build(),
    );
}
