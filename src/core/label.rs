use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::cell::{RefCell, Ref};

/// Label values are immutable but can move around a lot.
pub type LabelValue = Arc<String>;

/// A reference table of key / value string pairs that may be used on output for additional metric context.
#[derive(Default, Debug, Clone)]
pub struct Labels {
    pairs: Arc<HashMap<String, LabelValue>>
}

impl Labels {
    fn set(&self, key: String, value: LabelValue) -> Self {
        let mut copy = self.pairs.as_ref().clone();
        copy.insert(key, value);
        Labels { pairs: Arc::new(copy) }
    }

    fn unset(&self, key: &str) -> Self {
        let mut copy = self.pairs.as_ref().clone();
        if copy.remove(key).is_some() {
            Labels { pairs: Arc::new(copy) }
        } else {
            // key wasn't set, labels unchanged
            self.clone()
        }
    }

    fn get(&self, key: &str) -> Option<LabelValue> {
        self.pairs.get(key).cloned()
    }
}

lazy_static!(
    ///
    pub static ref GLOBAL_LABELS: RwLock<Labels> = RwLock::new(Labels::default());
);

thread_local! {
    pub static THREAD_LABELS: RefCell<Labels> = RefCell::new(Labels::default());
}

/// Scopes to which metric labels can be attached.
pub enum LabelScope {
    /// Handle metric labels for the whole application (globals).
    APP,
    /// Handle metric labels for the current thread.
    THREAD,
//    #[cfg(feature="tokio")]
//    TASK,
}

impl LabelScope {

    /// Set a new value for the scope.
    /// Replaces any previous value for the key.
    pub fn set(&self, key: String, value: String) {
        match *self {
            LabelScope::APP => {
                let b = GLOBAL_LABELS.read().expect("Global Labels");
                *GLOBAL_LABELS.write().expect("Global Labels") = b.set(key, Arc::new(value));
            },
            LabelScope::THREAD => {
                THREAD_LABELS.with(|map| {
                    let b: Ref<Labels> = map.borrow();
                    *map.borrow_mut() = b.set(key, Arc::new(value));
                })
            },
        }
    }

    /// Unset a value for the scope.
    /// Has no effect if key was not set.
    pub fn unset(&self, key: &str) {
        match *self {
            LabelScope::APP => {
                let b = GLOBAL_LABELS.read().expect("Global Labels");
                *GLOBAL_LABELS.write().expect("Global Labels") = b.unset(key);
            },
            LabelScope::THREAD => {
                THREAD_LABELS.with(|map| {
                    let b: Ref<Labels> = map.borrow();
                    *map.borrow_mut() = b.unset(key);
                })
            },
        }
    }

    /// Retrieve a value for the scope.
    pub fn get(&self, key: &str) -> Option<Arc<String>> {
        match *self {
            LabelScope::APP => {
                let b = GLOBAL_LABELS.read().expect("Global Labels");
                b.get(key)
            },
            LabelScope::THREAD => {
                THREAD_LABELS.with(|map| {
                    let b: Ref<Labels> = map.borrow();
                    b.get(key)
                })
            },
        }
    }

    /// Generic label lookup function.
    /// Searches provided labels, provided scopes or default scopes.
    pub fn lookup(key: &str, labels: &Vec<Labels>) -> Option<LabelValue> {
        LabelScope::THREAD.get(key).or_else(|| LabelScope::APP.get(key))
    }

}
