use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::cell::{RefCell, Ref};

/// Label values are immutable but can move around a lot.
pub type LabelValue = Arc<String>;

/// A reference table of key / value string pairs that may be used on output for additional metric context.
#[derive(Default, Debug, Clone)]
pub struct Labels {
    pairs: Option<Arc<HashMap<String, LabelValue>>>
}

impl Labels {
    fn set(&self, key: String, value: LabelValue) -> Self {
        let mut new_pairs = match self.pairs {
            None => HashMap::new(),
            Some(ref old_pairs) => old_pairs.as_ref().clone()
        };

        new_pairs.insert(key, value);
        Labels { pairs: Some(Arc::new(new_pairs)) }
    }

    fn unset(&self, key: &str) -> Self {
        match self.pairs {
            None => self.clone(),
            Some(ref old_pairs) => {
                let mut new_pairs = old_pairs.as_ref().clone();
                if new_pairs.remove(key).is_some() {
                    if new_pairs.is_empty() {
                        Labels { pairs: None }
                    } else {
                        Labels { pairs: Some(Arc::new(new_pairs)) }
                    }
                } else {
                    // key wasn't set, labels unchanged
                    self.clone()
                }
            }
        }
    }

    fn get(&self, key: &str) -> Option<LabelValue> {
        // FIXME should use .and_then(), how?
        match &self.pairs {
            None => None,
            Some(pairs) => pairs.get(key).cloned()
        }
    }
}

lazy_static!(
    static ref GLOBAL_LABELS: RwLock<Labels> = RwLock::new(Labels::default());
);

thread_local! {
    static THREAD_LABELS: RefCell<Labels> = RefCell::new(Labels::default());
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

    /// Freeze the current label values for usage at later time.
    pub fn export(&self) -> Labels {
        match *self {
            LabelScope::APP => GLOBAL_LABELS.read().expect("Global Labels").clone(),
            LabelScope::THREAD => {
                // FIXME is there a cleaner way to capture the clone out of the 'with' closure?
                let mut labels: Option<Labels> = None;
                THREAD_LABELS.with(|map| labels = Some(map.borrow().clone()));
                labels.unwrap()
            },
        }
    }

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
