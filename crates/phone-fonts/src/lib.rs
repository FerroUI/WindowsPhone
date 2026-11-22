//! Microsoft's open Selawik font family, registered once for all phone hosts.
mod assets;
mod collection;

pub use collection::PhoneFontCollection;
use ferroui_controls::AppBuilder;
use std::rc::Rc;

pub const FAMILY: &str = "fonts:Phone#Selawik";
pub const ASSEMBLY: &str = "WindowsPhone.Fonts";

pub trait PhoneFontBuilderExt {
    fn with_phone_font(&self) -> AppBuilder;
}
impl PhoneFontBuilderExt for AppBuilder {
    fn with_phone_font(&self) -> AppBuilder {
        self.configure_fonts(|manager| {
            manager.add_font_collection(Rc::new(PhoneFontCollection::new()));
        })
    }
}
