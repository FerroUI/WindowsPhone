//! Local membership cards, tickets and offers registered independently through the SDK.
mod commands;
mod model;
mod views;
use phone_sdk::prelude::*;
pub const APP_ID: &str = "org.ferroui.wallet";
pub fn registration() -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(APP_ID, "Wallet", Icon::Store)
            .category(AppCategory::Utility)
            .accent("#2D89EF")
            .capabilities(&[
                Capability::Files,
                Capability::Sharing,
                Capability::Clipboard,
            ]),
        views::view,
    )
    .with_pivot_count(|_| 3)
}
