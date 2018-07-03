//! Cache metric definitions.

use core::{Output, OutputDyn, WithAttributes, Attributes, OutputScope, OutputMetric, Name, Flush, Kind, AddPrefix};
use error;
use std::sync::{Arc, RwLock};
use std::rc::Rc;

/// Wrap an output with a metric definition cache.
/// This is useless if all metrics are statically declared but can provide performance
/// benefits if some metrics are dynamically defined at runtime.
pub trait WithOutputCache: Output + Send + Sync + 'static + Sized {
    /// Wrap this output with an asynchronous dispatch queue of specified length.
    fn with_cache(self, max_size: usize) -> OutputCache {
        OutputCache::wrap(self, max_size)
    }
}

/// Output wrapper caching frequently defined metrics
#[derive(Clone)]
pub struct OutputCache {
    attributes: Attributes,
    target: Arc<OutputDyn + Send + Sync + 'static>,
    cache: Arc<RwLock<lru::LRUCache<Name, OutputMetric>>>,
}

impl OutputCache {
    /// Wrap scopes with an asynchronous metric write & flush dispatcher.
    pub fn wrap<OUT: Output + Send + Sync + 'static>(target: OUT, max_size: usize) -> OutputCache {
        OutputCache {
            attributes: Attributes::default(),
            target: Arc::new(target),
            cache: Arc::new(RwLock::new(lru::LRUCache::with_capacity(max_size)))
        }
    }
}

impl WithAttributes for OutputCache {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Output for OutputCache {
    type SCOPE = OutputScopeCache;

    fn output(&self) -> Self::SCOPE {
        let target = self.target.output_dyn();
        OutputScopeCache {
            attributes: self.attributes.clone(),
            target,
            cache: self.cache.clone(),
        }
    }
}

/// Output wrapper caching frequently defined metrics
#[derive(Clone)]
pub struct OutputScopeCache {
    attributes: Attributes,
    target: Rc<OutputScope + 'static>,
    cache: Arc<RwLock<lru::LRUCache<Name, OutputMetric>>>,
}

impl WithAttributes for OutputScopeCache {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl OutputScope for OutputScopeCache {
    fn new_metric(&self, name: Name, kind: Kind) -> OutputMetric {
        let name = self.qualified_name(name);
        let lookup = {
            let mut cache = self.cache.write().expect("Cache Lock");
            cache.get(&name).map(|found| found.clone())
        };
        lookup.unwrap_or_else(|| {
            let new_metric: OutputMetric = self.target.new_metric(name.clone(), kind);
            // FIXME (perf) having to take another write lock for a cache miss
            let mut cache_miss = self.cache.write().expect("Cache Lock");
            cache_miss.insert(name, new_metric.clone());
            new_metric
        })
    }
}

impl Flush for OutputScopeCache {

    fn flush(&self) -> error::Result<()> {
        self.target.flush()
    }
}

mod lru {
    //! A fixed-size cache with LRU expiration criteria.
    //! Stored values will be held onto as long as there is space.
    //! When space runs out, the oldest unused value will get evicted to make room for a new value.

    use std::hash::Hash;
    use std::collections::HashMap;

    struct CacheEntry<K, V> {
        key: K,
        value: Option<V>,
        next: Option<usize>,
        prev: Option<usize>,
    }

    /// A fixed-size cache.
    pub struct LRUCache<K, V> {
        table: HashMap<K, usize>,
        entries: Vec<CacheEntry<K, V>>,
        first: Option<usize>,
        last: Option<usize>,
        capacity: usize,
    }

    impl<K: Clone + Hash + Eq, V> LRUCache<K, V> {
        /// Creates a new cache that can hold the specified number of elements.
        pub fn with_capacity(size: usize) -> Self {
            LRUCache {
                table: HashMap::with_capacity(size),
                entries: Vec::with_capacity(size),
                first: None,
                last: None,
                capacity: size,
            }
        }

        /// Inserts a key-value pair into the cache and returns the previous value, if any.
        /// If there is no room in the cache the oldest item will be removed.
        pub fn insert(&mut self, key: K, value: V) -> Option<V> {
            if self.table.contains_key(&key) {
                self.access(&key);
                let entry = &mut self.entries[self.first.unwrap()];
                let old = entry.value.take();
                entry.value = Some(value);
                old
            } else {
                self.ensure_room();
                // Update old head
                let idx = self.entries.len();
                self.first.map(|e| {
                    let prev = Some(idx);
                    self.entries[e].prev = prev;
                });
                // This is the new head
                self.entries.push(CacheEntry {
                    key: key.clone(),
                    value: Some(value),
                    next: self.first,
                    prev: None,
                });
                self.first = Some(idx);
                self.last = self.last.or(self.first);
                self.table.insert(key, idx);
                None
            }
        }

        /// Retrieves a reference to the item associated with `key` from the cache
        /// without promoting it.
        pub fn peek(&mut self, key: &K) -> Option<&V> {
            let entries = &self.entries;
            self.table
                .get(key)
                .and_then(move |i| entries[*i].value.as_ref())
        }

        /// Retrieves a reference to the item associated with `key` from the cache.
        pub fn get(&mut self, key: &K) -> Option<&V> {
            if self.contains_key(key) {
                self.access(key);
            }
            self.peek(key)
        }

        /// Returns the number of elements currently in the cache.
        pub fn len(&self) -> usize {
            self.table.len()
        }

        /// Promotes the specified key to the top of the cache.
        fn access(&mut self, key: &K) {
            let i = *self.table.get(key).unwrap();
            self.remove_from_list(i);
            self.first = Some(i);
        }

        pub fn contains_key(&mut self, key: &K) -> bool {
            self.table.contains_key(key)
        }

        /// Removes an item from the linked list.
        fn remove_from_list(&mut self, i: usize) {
            let (prev, next) = {
                let entry = &mut self.entries[i];
                (entry.prev, entry.next)
            };
            match (prev, next) {
                // Item was in the middle of the list
                (Some(j), Some(k)) => {
                    {
                        let first = &mut self.entries[j];
                        first.next = next;
                    }
                    let second = &mut self.entries[k];
                    second.prev = prev;
                }
                // Item was at the end of the list
                (Some(j), None) => {
                    let first = &mut self.entries[j];
                    first.next = None;
                    self.last = prev;
                }
                // Item was at front
                _ => (),
            }
        }

        fn ensure_room(&mut self) {
            if self.capacity == self.len() {
                self.remove_last();
            }
        }

        /// Removes the oldest item in the cache.
        fn remove_last(&mut self) {
            if let Some(idx) = self.last {
                self.remove_from_list(idx);
                let key = &self.entries[idx].key;
                self.table.remove(key);
            }
            if self.last.is_none() {
                self.first = None;
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn get_and_get_mut_promote() {
            let mut cache: LRUCache<&str, _> = LRUCache::with_capacity(2);
            cache.insert("foo", 1);
            cache.insert("bar", 2);

            cache.get(&"foo").unwrap();
            cache.insert("baz", 3);

            assert!(cache.contains_key(&"foo"));
            assert!(cache.contains_key(&"baz"));
            assert!(!cache.contains_key(&"bar"));

            cache.get(&"foo").unwrap();
            cache.insert("qux", 4);

            assert!(cache.contains_key(&"foo"));
            assert!(cache.contains_key(&"qux"));
            assert!(!cache.contains_key(&"baz"));
        }
    }

}
