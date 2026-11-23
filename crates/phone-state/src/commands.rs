use crate::PhoneViewModel;
use phone_domain::Action;

impl PhoneViewModel {
    pub(crate) fn apply(&self, action: Option<Action>) {
        if let Some(action) = action {
            self.pending.borrow_mut().push_back(action);
        }
        if self.applying.replace(true) {
            return;
        }
        loop {
            let action = self.pending.borrow_mut().pop_front();
            let Some(action) = action else {
                break;
            };
            match action {
                Action::Navigate(route) => {
                    self.navigate(route);
                    continue;
                }
                Action::Back => {
                    self.back();
                    continue;
                }
                Action::Home => {
                    self.home();
                    continue;
                }
                _ => {}
            }
            let schedule_changed = matches!(
                &action,
                Action::Calendar(_)
                    | Action::SaveEvent(_)
                    | Action::DeleteEvent(_)
                    | Action::SaveAlarm(_)
                    | Action::DeleteAlarm(_)
                    | Action::ToggleAlarm(_)
            );
            let result =
                crate::mutation::mutate(&mut self.document.borrow_mut(), action, self.now());
            match result {
                Ok(true) => {
                    if schedule_changed {
                        self.schedule_revision
                            .set(self.schedule_revision.get().wrapping_add(1));
                    }
                    self.document.borrow_mut().clean_tile_targets();
                    self.repair_navigation();
                    self.synchronize();
                    self.persist();
                    if !self.valid_route(&self.route()) {
                        self.back();
                    }
                    self.notify();
                }
                Ok(false) => {}
                Err(error) => self.set_status(error),
            }
        }
        self.applying.set(false);
    }
}
