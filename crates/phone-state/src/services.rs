use phone_domain::PhoneData;
pub use phone_host::{HostEnvironment, HostPermission, HostPlatform, HostRequest};
use std::rc::Rc;

pub type SaveCallback = Rc<dyn Fn(&PhoneData)>;

#[derive(Clone)]
pub struct HostServices {
    pub request: Rc<dyn Fn(HostRequest)>,
}
impl Default for HostServices {
    fn default() -> Self {
        Self {
            request: Rc::new(|_| {}),
        }
    }
}
