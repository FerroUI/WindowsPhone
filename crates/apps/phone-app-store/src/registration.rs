use crate::state::StoreSession;
use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Store, crate::view)
        .with_back(|_, ui| {
            let state = ui.state::<StoreSession>();
            if state.selected.get_untracked().is_some() {
                state.selected.set(None);
                true
            } else {
                false
            }
        })
        .with_reset(|ui| ui.state::<StoreSession>().selected.set(None))
        .with_pivot_count(|ui| {
            if ui
                .state::<StoreSession>()
                .selected
                .get_untracked()
                .is_none()
            {
                2
            } else {
                0
            }
        })
}
