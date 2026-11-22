use ferroui_base::media::fonts::{
    EmbeddedFontCollection, FontCollectionBase, FontCollectionBaseImpl,
};
use ferroui_base::utilities::UriRef;

pub struct PhoneFontCollection {
    base: EmbeddedFontCollection,
}
impl PhoneFontCollection {
    pub fn new() -> Self {
        crate::assets::register();
        Self {
            base: EmbeddedFontCollection::new(
                UriRef::parse_absolute("fonts:Phone").expect("embedded font key"),
                UriRef::parse_absolute("ferres://WindowsPhone.Fonts/Assets")
                    .expect("embedded font asset URI"),
            ),
        }
    }
}
impl Default for PhoneFontCollection {
    fn default() -> Self {
        Self::new()
    }
}
impl FontCollectionBaseImpl for PhoneFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        EmbeddedFontCollection::base(&this.base)
    }
    fn key(this: &Self) -> url::Url {
        EmbeddedFontCollection::key(&this.base)
    }
}
