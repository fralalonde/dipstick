use std::cell::RefCell;
use std::collections::HashMap;

use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::RwLock;

#[cfg(feature = "parking_lot")]
use parking_lot::RwLock;

/// Label values are immutable but can move around a lot.
type LabelValue = Arc<String>;

/// A reference table of key / value string pairs that may be used on output for additional metric context.
///
/// For concurrency reasons, labels are immutable.
/// All write operations return a mutated clone of the original.
#[derive(Debug, Clone, Default)]
struct LabelScope {
    pairs: Option<Arc<HashMap<String, LabelValue>>>,
}

impl LabelScope {
    /// Sets the value on a new copy of the map, then returns that copy.
    fn set(&self, key: String, value: LabelValue) -> Self {
        let mut new_pairs = match self.pairs {
            None => HashMap::new(),
            Some(ref old_pairs) => old_pairs.as_ref().clone(),
        };

        new_pairs.insert(key, value);
        LabelScope {
            pairs: Some(Arc::new(new_pairs)),
        }
    }

    fn unset(&self, key: &str) -> Self {
        match self.pairs {
            None => self.clone(),
            Some(ref old_pairs) => {
                let mut new_pairs = old_pairs.as_ref().clone();
                if new_pairs.remove(key).is_some() {
                    if new_pairs.is_empty() {
                        LabelScope { pairs: None }
                    } else {
                        LabelScope {
                            pairs: Some(Arc::new(new_pairs)),
                        }
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
            Some(pairs) => pairs.get(key).cloned(),
        }
    }

    fn collect(&self, map: &mut HashMap<String, LabelValue>) {
        if let Some(pairs) = &self.pairs {
            map.extend(pairs.as_ref().clone())
        }
    }
}

lazy_static! {
    static ref APP_LABELS: RwLock<LabelScope> = RwLock::new(LabelScope::default());
}

thread_local! {
    static THREAD_LABELS: RefCell<LabelScope> = RefCell::new(LabelScope::default());
}

/// Handle metric labels for the current thread.
/// App scope labels have the lowest lookup priority and serve as a fallback to other scopes.
pub struct ThreadLabel;

impl ThreadLabel {
    /// Retrieve a value from the thread scope.
    pub fn get(key: &str) -> Option<Arc<String>> {
        THREAD_LABELS.with(|map| map.borrow().get(key))
    }

    /// Set a new value for the thread scope.
    /// Replaces any previous value for the key.
    pub fn set<S: Into<String>>(key: S, value: S) {
        THREAD_LABELS.with(|map| {
            let new = { map.borrow().set(key.into(), Arc::new(value.into())) };
            *map.borrow_mut() = new;
        });
    }

    /// Unset a value for the app scope.
    /// Has no effect if key was not set.
    pub fn unset(key: &str) {
        THREAD_LABELS.with(|map| {
            let new = { map.borrow().unset(key) };
            *map.borrow_mut() = new;
        });
    }

    fn collect(map: &mut HashMap<String, LabelValue>) {
        THREAD_LABELS.with(|mop| mop.borrow().collect(map));
    }
}

/// Handle metric labels for the whole application (globals).
/// App scope labels have the lowest lookup priority and serve as a fallback to other scopes.
pub struct AppLabel;

impl AppLabel {
    /// Retrieve a value from the app scope.
    pub fn get(key: &str) -> Option<Arc<String>> {
        read_lock!(APP_LABELS).get(key)
    }

    /// Set a new value for the app scope.
    /// Replaces any previous value for the key.
    pub fn set<S: Into<String>>(key: S, value: S) {
        let b = { read_lock!(APP_LABELS).set(key.into(), Arc::new(value.into())) };
        *write_lock!(APP_LABELS) = b;
    }

    /// Unset a value for the app scope.
    /// Has no effect if key was not set.
    pub fn unset(key: &str) {
        let b = { read_lock!(APP_LABELS).unset(key) };
        *write_lock!(APP_LABELS) = b;
    }

    fn collect(map: &mut HashMap<String, LabelValue>) {
        read_lock!(APP_LABELS).collect(map)
    }
}

/// Base structure to carry metric labels from the application to the metric backend(s).
/// Can carry both one-off labels and exported context labels (if async metrics are enabled).
/// Used in applications through the labels!() macro.
#[derive(Debug, Clone)]
pub struct Labels {
    scopes: Vec<LabelScope>,
}

impl From<HashMap<String, LabelValue>> for Labels {
    fn from(map: HashMap<String, LabelValue>) -> Self {
        Labels {
            scopes: vec![LabelScope {
                pairs: Some(Arc::new(map)),
            }],
        }
    }
}

impl Default for Labels {
    /// Create empty labels.
    /// Only Thread and App labels will be used for lookups.
    #[inline]
    fn default() -> Self {
        Labels { scopes: vec![] }
    }
}

impl Labels {
    /// Used to save metric context before enqueuing value for async output.
    pub fn save_context(&mut self) {
        self.scopes
            .push(THREAD_LABELS.with(|map| map.borrow().clone()));
        self.scopes.push(read_lock!(APP_LABELS).clone());
    }

    /// Generic label lookup function.
    /// Searches provided labels, provided scopes or default scopes.
    // TODO needs less magic, add checks?
    pub fn lookup(&self, key: &str) -> Option<LabelValue> {
        fn lookup_current_context(key: &str) -> Option<LabelValue> {
            ThreadLabel::get(key).or_else(|| AppLabel::get(key))
        }

        match self.scopes.len() {
            // no value labels, no saved context labels
            // just lookup implicit context
            0 => lookup_current_context(key),

            // some value labels, no saved context labels
            // lookup value label, then lookup implicit context
            1 => self.scopes[0]
                .get(key)
                .or_else(|| lookup_current_context(key)),

            // value + saved context labels
            // lookup explicit context in turn
            _ => {
                for src in &self.scopes {
                    if let Some(label_value) = src.get(key) {
                        return Some(label_value);
                    }
                }
                None
            }
        }
    }

    /// Export current state of labels to a map.
    /// Note: An iterator would still need to allocate to check for uniqueness of keys.
    ///
    pub fn into_map(mut self) -> HashMap<String, LabelValue> {
        let mut map = HashMap::new();
        match self.scopes.len() {
            // no value labels, no saved context labels
            // just lookup implicit context
            0 => {
                AppLabel::collect(&mut map);
                ThreadLabel::collect(&mut map);
            }

            // some value labels, no saved context labels
            // lookup value label, then lookup implicit context
            1 => {
                AppLabel::collect(&mut map);
                ThreadLabel::collect(&mut map);
                self.scopes[0].collect(&mut map);
            }

            // value + saved context labels
            // lookup explicit context in turn
            _ => {
                self.scopes.reverse();
                for src in self.scopes {
                    src.collect(&mut map)
                }
            }
        }
        map
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    use std::sync::Mutex;

    lazy_static! {
        /// Label tests use the globally shared AppLabels which may make them interfere as tests are run concurrently.
        /// We do not want to mandate usage of `RUST_TEST_THREADS=1` which would penalize the whole test suite.
        /// Instead we use a local mutex to make sure the label tests run in sequence.
        static ref TEST_SEQUENCE: Mutex<()> = Mutex::new(());
    }

    #[test]
    fn context_labels() {
        let _lock = TEST_SEQUENCE.lock().expect("Test Sequence");

        AppLabel::set("abc", "456");
        ThreadLabel::set("abc", "123");

        assert_eq!(
            Arc::new("123".into()),
            labels!().lookup("abc").expect("ThreadLabel Value")
        );
        ThreadLabel::unset("abc");

        assert_eq!(
            Arc::new("456".into()),
            labels!().lookup("abc").expect("AppLabel Value")
        );
        AppLabel::unset("abc");

        assert_eq!(true, labels!().lookup("abc").is_none());
    }

    #[test]
    fn labels_macro() {
        let _lock = TEST_SEQUENCE.lock().expect("Test Sequence");

        let labels = labels! {
            "abc" => "789",
            "xyz" => "123"
        };
        assert_eq!(
            Arc::new("789".into()),
            labels.lookup("abc").expect("Label Value")
        );
        assert_eq!(
            Arc::new("123".into()),
            labels.lookup("xyz").expect("Label Value")
        );
    }

    #[test]
    fn value_labels() {
        let _lock = TEST_SEQUENCE.lock().expect("Test Sequence");

        let labels = labels! { "abc" => "789" };
        assert_eq!(
            Arc::new("789".into()),
            labels.lookup("abc").expect("Label Value")
        );

        AppLabel::set("abc", "456");
        assert_eq!(
            Arc::new("789".into()),
            labels.lookup("abc").expect("Label Value")
        );

        ThreadLabel::set("abc", "123");
        assert_eq!(
            Arc::new("789".into()),
            labels.lookup("abc").expect("Label Value")
        );
    }
}
