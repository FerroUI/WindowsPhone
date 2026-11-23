use ferroui_base::media::IImage;
use ferroui_view::Signal;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

pub const BUDGET: usize = 24 * 1024 * 1024;
pub type Loader = Rc<dyn Fn(String)>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub source: String,
    pub edge: u32,
}
impl Key {
    pub fn new(source: &str, edge: u32) -> Self {
        Self {
            source: source.into(),
            edge,
        }
    }
}

pub enum ImageState {
    Waiting,
    Ready(Rc<dyn IImage>),
    Failed,
}
pub type Lease = Rc<RefCell<ImageState>>;
pub struct Entry {
    pub image: Rc<dyn IImage>,
    pub bytes: usize,
    pub used: u64,
}

#[derive(Default)]
pub struct Cache {
    pub images: HashMap<Key, Entry>,
    pub leases: HashMap<Key, Vec<Weak<RefCell<ImageState>>>>,
    pub pending: HashSet<String>,
    pub failed: HashSet<String>,
    pub forgotten: HashSet<String>,
    pub loader: Option<Loader>,
    pub revision: Option<Signal<u64>>,
    pub bytes: usize,
    pub clock: u64,
}

impl Cache {
    pub fn get(&mut self, key: &Key) -> Option<Rc<dyn IImage>> {
        self.clock = self.clock.wrapping_add(1);
        let entry = self.images.get_mut(key)?;
        entry.used = self.clock;
        Some(entry.image.clone())
    }

    pub fn insert(&mut self, key: Key, image: Rc<dyn IImage>, bytes: usize) {
        if let Some(previous) = self.images.remove(&key) {
            self.bytes -= previous.bytes;
        }
        // Resolve mounted leases before eviction. Batched image completions must never evict
        // each other before the corresponding native image controls get their next render.
        if let Some(leases) = self.leases.get_mut(&key) {
            leases.retain(|weak| {
                let Some(lease) = weak.upgrade() else {
                    return false;
                };
                *lease.borrow_mut() = ImageState::Ready(image.clone());
                true
            });
        }
        self.clock = self.clock.wrapping_add(1);
        self.bytes += bytes;
        self.images.insert(
            key,
            Entry {
                image,
                bytes,
                used: self.clock,
            },
        );
    }

    pub fn trim(&mut self) {
        while self.bytes > BUDGET {
            // Native controls and mounted leases both retain their images. Only idle cache
            // entries are eligible; the target may be exceeded by the visible working set.
            let oldest = self
                .images
                .iter()
                .filter(|(_, entry)| Rc::strong_count(&entry.image) == 1)
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| key.clone());
            let Some(key) = oldest else { break };
            if let Some(entry) = self.images.remove(&key) {
                self.bytes -= entry.bytes;
            }
        }
    }

    pub fn fail(&mut self, source: &str) {
        self.pending.remove(source);
        self.failed.insert(source.into());
        for (key, leases) in &mut self.leases {
            if key.source == source {
                leases.retain(|weak| {
                    let Some(lease) = weak.upgrade() else {
                        return false;
                    };
                    let waiting = matches!(*lease.borrow(), ImageState::Waiting);
                    if waiting {
                        *lease.borrow_mut() = ImageState::Failed;
                    }
                    true
                });
            }
        }
        self.leases.retain(|_, leases| !leases.is_empty());
    }

    pub fn forget(&mut self, source: &str) {
        // A late asynchronous completion for a deleted asset must not resurrect it.
        self.forgotten.insert(source.into());
        self.fail(source);
        let mut removed_bytes = 0;
        self.images.retain(|key, entry| {
            if key.source == source {
                removed_bytes += entry.bytes;
                false
            } else {
                true
            }
        });
        self.bytes -= removed_bytes;
        for (key, leases) in &self.leases {
            if key.source == source {
                for lease in leases.iter().filter_map(Weak::upgrade) {
                    *lease.borrow_mut() = ImageState::Failed;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.images.clear();
        self.pending.clear();
        self.failed.clear();
        self.forgotten.clear();
        self.bytes = 0;
        for leases in self.leases.values_mut() {
            leases.retain(|weak| {
                let Some(lease) = weak.upgrade() else {
                    return false;
                };
                *lease.borrow_mut() = ImageState::Waiting;
                true
            });
        }
        self.leases.retain(|_, leases| !leases.is_empty());
    }

    pub fn requested_keys(&mut self, source: &str) -> Vec<Key> {
        self.leases.retain(|_, leases| {
            leases.retain(|lease| lease.strong_count() > 0);
            !leases.is_empty()
        });
        let mut keys = self
            .leases
            .keys()
            .filter(|key| key.source == source && !self.images.contains_key(*key))
            .cloned()
            .collect::<Vec<_>>();
        keys.sort_by_key(|key| key.edge);
        keys
    }
}
