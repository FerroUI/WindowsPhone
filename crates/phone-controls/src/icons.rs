//! Resolution-independent monochrome icon geometry, cached on the UI thread.
use ferroui_base::Ref;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::Geometry;
use ferroui_view::prelude::*;
use std::{cell::RefCell, collections::HashMap};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Icon {
    Phone,
    Message,
    People,
    Mail,
    Calendar,
    Photos,
    Camera,
    Explorer,
    Music,
    Video,
    Games,
    Store,
    Settings,
    Alarm,
    Calculator,
    Note,
    Maps,
    Cortana,
    Search,
    Back,
    Forward,
    Windows,
    Plus,
    More,
    Pin,
    Unpin,
    Resize,
    Check,
    Delete,
    Send,
    Edit,
    Folder,
    Wifi,
    Bluetooth,
    Airplane,
    Rotation,
    Battery,
    Signal,
    Play,
    Pause,
    Next,
    Previous,
    Refresh,
    Close,
    Save,
    Share,
    Download,
    Upload,
    Globe,
    Lock,
    Unlock,
    Volume,
    Mute,
    Sun,
    Cloud,
    Weather,
    Location,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    ChevronUp,
    Office,
    Files,
    Heart,
    Star,
    Info,
    List,
    Headphones,
    Power,
    Keyboard,
    Timer,
    Undo,
}

ferroui_base::ferro_thread_local! {
    static GEOMETRY: RefCell<HashMap<Icon, Ref<Geometry>>> = RefCell::new(HashMap::new());
}

pub fn icon(kind: Icon, size: f64, color: &str) -> Element {
    let data = geometry(kind);
    rsx! { PathIcon {
        data: Some(data), width:size, height:size, foreground:color,
        vertical_alignment:V::Center, horizontal_alignment:H::Center,
        is_hit_test_visible:false, automation_properties_is_control_element_override:Some(false)
    } }
}

/// Palette/state ink is supplied by the command theme, not a local value that
/// would mask pressed and disabled styles. Geometry shares the same cache.
pub(crate) fn action_glyph(kind: Icon, size: f64, label: &str) -> Element {
    rsx! { PathIcon {
        name:format!("AppBarGlyph-{label}"),class:"phone-action-glyph",
        data:Some(geometry(kind)),width:size,height:size,
        vertical_alignment:V::Center,horizontal_alignment:H::Center,
        is_hit_test_visible:false,automation_properties_is_control_element_override:Some(false)
    } }
}

fn geometry(kind: Icon) -> Ref<Geometry> {
    GEOMETRY.with(|cache| {
        cache
            .borrow_mut()
            .entry(kind)
            .or_insert_with(|| {
                Geometry::parse(crate::paths::path(kind)).expect("embedded phone icon")
            })
            .clone()
    })
}
