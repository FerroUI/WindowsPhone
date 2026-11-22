use crate::PhoneViewModel;
use ferroui_dynamic_data::SourceCache;
use phone_domain::Id;
use std::{collections::HashSet, rc::Rc};

impl PhoneViewModel {
    pub(crate) fn synchronize(&self) {
        let (contacts, tiles, notifications) = {
            let data = self.data();
            (
                data.contacts.clone(),
                data.all_tiles().cloned().collect(),
                data.notifications.clone(),
            )
        };
        reconcile(&self.contacts, contacts, |value| value.id);
        reconcile(&self.tiles, tiles, |value| value.id);
        reconcile(&self.notifications, notifications, |value| value.id);
    }
}
fn reconcile<T: Clone + PartialEq + 'static>(
    cache: &SourceCache<Rc<T>, Id>,
    items: Vec<T>,
    key: impl Fn(&T) -> Id,
) {
    let keys: HashSet<_> = items.iter().map(&key).collect();
    cache.edit(|update| {
        for id in update.keys() {
            if !keys.contains(&id) {
                update.remove_key(&id);
            }
        }
        for item in items {
            let id = key(&item);
            if update.lookup(&id).is_none_or(|old| *old != item) {
                update.add_or_update(Rc::new(item));
            }
        }
    });
}
