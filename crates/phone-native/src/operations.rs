//! Ownership for native picker/codec workflows across host and document changes.
use crate::asset_store::AssetStore;
use ferroui_base::{platform::storage::IStorageFile, threading::Dispatcher};
use phone_runtime::PhoneRuntime;
use phone_state::PhoneViewModel;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};

thread_local! {
    static CURRENT: RefCell<Option<Installed>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct Installed {
    runtime: Weak<PhoneRuntime>,
    path: PathBuf,
}

pub fn install(runtime: &Rc<PhoneRuntime>) {
    CURRENT.with(|slot| {
        *slot.borrow_mut() = Some(Installed {
            runtime: Rc::downgrade(runtime),
            path: crate::storage::active_data_path(),
        })
    });
}

/// Holds no runtime alive. Storage is captured on the UI thread, never resolved
/// in a worker or after a different phone host has become active.
pub struct Lease {
    runtime: Weak<PhoneRuntime>,
    epoch: u64,
    path: PathBuf,
}

impl Lease {
    pub fn capture(vm: &Rc<PhoneViewModel>) -> Result<Self, String> {
        let lease = Self::current()?;
        if !lease
            .runtime
            .upgrade()
            .is_some_and(|runtime| Rc::ptr_eq(&runtime.vm(), vm))
        {
            return Err("This operation belongs to a detached phone host.".into());
        }
        Ok(lease)
    }

    pub fn current() -> Result<Self, String> {
        let installed = CURRENT
            .with(|slot| slot.borrow().clone())
            .ok_or("The phone host is closed.")?;
        let runtime = installed
            .runtime
            .upgrade()
            .filter(|runtime| !runtime.is_closed())
            .ok_or("The phone host is closed.")?;
        Ok(Self {
            epoch: runtime.vm().document_epoch(),
            runtime: Rc::downgrade(&runtime),
            path: installed.path,
        })
    }

    pub fn is_active(&self) -> bool {
        CURRENT.with(|slot| {
            slot.borrow()
                .as_ref()
                .is_some_and(|installed| installed.runtime.ptr_eq(&self.runtime))
        }) && self.runtime.upgrade().is_some_and(|runtime| {
            !runtime.is_closed() && runtime.vm().document_epoch() == self.epoch
        })
    }

    pub fn check(&self) -> Result<(), String> {
        if self.is_active() {
            Ok(())
        } else {
            Err("The phone session changed; this operation was cancelled.".into())
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn store(&self) -> AssetStore {
        AssetStore::new(self.path.with_file_name("assets"))
    }

    pub fn report(&self, error: String) {
        if self.is_active() {
            if let Some(runtime) = self.runtime.upgrade() {
                runtime.vm().set_status(error);
            }
        }
    }

    pub async fn read(&self, file: &dyn IStorageFile, limit: usize) -> Result<Vec<u8>, String> {
        self.check()?;
        let bytes = crate::picker_io::read_while(file, limit, || self.is_active()).await?;
        self.check()?;
        Ok(bytes)
    }

    pub async fn write(&self, file: &dyn IStorageFile, bytes: &[u8]) -> Result<(), String> {
        use std::io::Write;
        self.check()?;
        let mut stream = file
            .open_write_async()
            .await
            .map_err(|error| error.to_string())?;
        self.check()?;
        for chunk in bytes.chunks(256 * 1024) {
            self.check()?;
            stream.write_all(chunk).map_err(|error| error.to_string())?;
            Dispatcher::yield_now().await;
        }
        self.check()?;
        stream.flush().map_err(|error| error.to_string())
    }
}

/// Newly written immutable assets are removed on error/cancellation until the
/// corresponding model update has succeeded. Existing assets are never added.
pub struct StagedAssets {
    store: AssetStore,
    keys: Vec<String>,
    committed: bool,
}

impl StagedAssets {
    pub fn new(store: AssetStore) -> Self {
        Self {
            store,
            keys: Vec::new(),
            committed: false,
        }
    }
    pub fn add(&mut self, key: String) {
        self.keys.push(key);
    }
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for StagedAssets {
    fn drop(&mut self) {
        if !self.committed {
            for key in &self.keys {
                let _ = self.store.remove(key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phone_host::{platform::*, HostPlatform};

    struct Platform;
    impl PlatformPlugin for Platform {
        fn id(&self) -> &'static str {
            "test.native.operations"
        }
        fn state(&self) -> PlatformState {
            PlatformState {
                platform: HostPlatform::Ios,
                mode: HostMode::Embedded,
                launcher: HostRole::Unsupported,
                shell: HostRole::Unsupported,
                features: Vec::new(),
                detail: String::new(),
            }
        }
        fn dispatch(&self, _: PlatformRequest) -> PlatformTask {
            ready(Err("unused".into()))
        }
    }
    fn runtime() -> Rc<PhoneRuntime> {
        PhoneRuntime::attach(
            PhoneViewModel::new(phone_domain::PhoneData::demo(), Rc::new(|_| {})),
            Rc::new(Platform),
            ServiceRegistry::default(),
        )
        .unwrap()
    }

    #[test]
    fn leases_do_not_follow_restored_documents_replacement_hosts_or_closed_hosts() {
        let original = runtime();
        let original_path = std::env::temp_dir().join("phone-original/phone.json");
        crate::storage::set_active_data_path(original_path.clone());
        install(&original);
        let lease = Lease::capture(&original.vm()).unwrap();
        assert!(lease.is_active());
        crate::storage::set_active_data_path(std::env::temp_dir().join("phone-other/phone.json"));
        assert_eq!(lease.path(), original_path);
        assert_eq!(Lease::current().unwrap().path(), original_path);
        original
            .vm()
            .replace_data_after_save(phone_domain::PhoneData::demo(), |_| Ok(()))
            .unwrap();
        assert!(lease.check().is_err());
        let after_restore = Lease::capture(&original.vm()).unwrap();
        let replacement = runtime();
        install(&replacement);
        assert!(after_restore.check().is_err());
        assert!(Lease::capture(&original.vm()).is_err());
        let current = Lease::capture(&replacement.vm()).unwrap();
        replacement.lifecycle(HostLifecycle::Closing);
        assert!(current.check().is_err());
        assert!(Lease::current().is_err());
    }

    #[test]
    fn cancelled_asset_staging_preserves_existing_and_committed_files() {
        let directory = std::env::temp_dir().join(format!(
            "phone-staging-{}",
            crate::asset_store::unique_key().replace(':', "-")
        ));
        let store = AssetStore::new(directory.clone());
        let existing = store
            .put(
                "existing.png",
                "image/png",
                crate::asset_store::Kind::Image,
                b"old",
            )
            .unwrap();
        let abandoned = store
            .put(
                "abandoned.png",
                "image/png",
                crate::asset_store::Kind::Image,
                b"new",
            )
            .unwrap();
        {
            let mut stage = StagedAssets::new(store.clone());
            stage.add(abandoned.clone());
        }
        assert!(store.get(&abandoned).is_err());
        assert_eq!(store.get(&existing).unwrap().bytes, b"old");
        let committed = store
            .put(
                "committed.png",
                "image/png",
                crate::asset_store::Kind::Image,
                b"keep",
            )
            .unwrap();
        let mut stage = StagedAssets::new(store.clone());
        stage.add(committed.clone());
        stage.commit();
        assert_eq!(store.get(&committed).unwrap().bytes, b"keep");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
