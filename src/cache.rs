//! Cache metric definitions.

use core::*;
use std::sync::{Arc, RwLock};

struct MetricCache<OUT> {
    inner: RwLock<lru::LRUCache<Name, M>>
}

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub trait WithCache
where
    Self: Sized,
{
    /// Cache metrics to prevent them from being re-defined on every use.
    fn with_cache(&self, cache_size: usize) -> Self;
}

// TODO add selfmetrics cache stats

/// Add a caching decorator to a metric definition function.
pub fn add_cache<M>(cache_size: usize, next: DefineMetricFn<M>) -> DefineMetricFn<M>
where
    M: Clone + Send + Sync + 'static,
{
    let cache: RwLock<lru::LRUCache<Name, M>> =
        RwLock::new(lru::LRUCache::with_capacity(cache_size));
    Arc::new(move |name, kind, rate| {
        let mut cache = cache.write().expect("Metric Cache");

        // FIXME lookup should use straight &str
        if let Some(value) = cache.get(name) {
            return value.clone();
        }

        let new_metric: M = (next)(name, kind, rate);
        cache.insert(name.clone(), new_metric.clone());
        new_metric
    })
}

mod lru {
    // The MIT License (MIT)
    //
    // Copyright (c) 2016 Christian W. Briones
    //
    // Permission is hereby granted, free of charge, to any person obtaining a copy
    // of this software and associated documentation files (the "Software"), to deal
    // in the Software without restriction, including without limitation the rights
    // to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    // copies of the Software, and to permit persons to whom the Software is
    // furnished to do so, subject to the following conditions:
    //
    // The above copyright notice and this permission notice shall be included in all
    // copies or substantial portions of the Software.
    //
    // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    // IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    // FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    // AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    // LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    // OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    // SOFTWARE.

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
