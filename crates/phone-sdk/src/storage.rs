use phone_state::PhoneViewModel;
use serde::{de::DeserializeOwned, Serialize};
use std::rc::Rc;

/// A bounded persistent namespace carried by the phone's existing save/backup
/// transaction. Each application owns its schema and version migration.
#[derive(Clone)]
pub struct AppStorage {
    vm: Rc<PhoneViewModel>,
    id: String,
}
impl AppStorage {
    pub fn new(vm: Rc<PhoneViewModel>, id: &str) -> Self {
        Self { vm, id: id.into() }
    }
    pub fn read<T: DeserializeOwned + Default>(&self) -> Result<T, String> {
        phone_domain::validate_app_key(&self.id)?;
        self.vm
            .app_data(&self.id)
            .map(serde_json::from_value)
            .transpose()
            .map(|value| value.unwrap_or_default())
            .map_err(|error| format!("Invalid app data: {error}"))
    }
    pub fn write<T: Serialize>(&self, value: &T) -> Result<(), String> {
        let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
        self.vm.set_app_data(&self.id, Some(value))
    }
    pub fn clear(&self) -> Result<(), String> {
        self.vm.set_app_data(&self.id, None)
    }
}
