//! Independently registered FerroUI phone application.
use phone_sdk::prelude::*;
use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "example.sdk-counter";
const TITLE: &str = "SDK counter";

#[derive(Serialize, Deserialize)]
struct SavedState {
    schema: u32,
    count: u64,
}
impl Default for SavedState {
    fn default() -> Self {
        Self {
            schema: 1,
            count: 0,
        }
    }
}

pub fn registration() -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(APP_ID, TITLE, Icon::Note).category(AppCategory::Utility),
        view,
    )
}

fn view(cx: AppContext) -> Element {
    let storage = cx.storage();
    let saved = match storage.read::<SavedState>() {
        Ok(saved) if saved.schema == 1 => saved,
        Ok(_) => return empty(TITLE, "This saved data version is not supported."),
        Err(error) => return empty(TITLE, &error),
    };
    let model = cx.vm.clone();
    let body = rsx! { StackPanel { margin:20.0, spacing:18.0,
        {title(TITLE)}
        {text(&saved.count.to_string(),72.0)}
        {muted("This value belongs to this application's persistent namespace.",18.0)}
        {flat("increment",move || {
            let result = (|| {
                let mut saved = storage.read::<SavedState>()?;
                if saved.schema != 1 { return Err("Unsupported saved data version.".into()); }
                saved.count = saved.count.checked_add(1).ok_or("The counter is at its maximum.")?;
                storage.write(&saved)
            })();
            if let Err(error) = result { model.set_status(error); }
        })}
    }};
    page(TITLE, scroll(body), Element::Empty)
}
