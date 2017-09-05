
//! A data structure that keeps hold of _checked_ values, each returning a _handle_ that can later be _claimed_.
//! Because handles hold no direct reference to the value or the coatcheck, they can be shared (cloned) safely.
//!
//! Checked values cannot be removed explicitly from the coatcheck.
//! A checked value is removed from the structure only when :
//! - All copies of its handle are dropped.
//! - Enough new value are checked to force detection, and then reuse, of the free handle.
//!
//! A coatcheck is similar to a [flyweight](https://en.wikipedia.org/wiki/Flyweight_pattern),
//! but values are not hashed or compared in any way, which results in two main differences.
//! - Identical values may be stored multiple times to obtain multiple handles.
//! - Values may be mutated without ill effects on the data structure
//! If the handle is shared, care must still be taken to secure concurrent access to a mutable value!
//!
//! In this implementation, values are stored in an unbounded `Vec`, with their handles holding a refcounted index to their stored position.
//! Claiming a value simply returns the Vec element at the index held by the handle.
//! To minimize growth of the backing `Vec` allocation, handles are pooled for reuse in a free list.
//! When checking a new value, it is allocated a handle. The handle allocation strategy :
//! - Try to reuse a handle from the free list (fastest).
//! - Try to allocate a new handle without causing the backing Vec to grow (fast).
//! - Scan all existing handles, adding any free handle to the free list (slow).
//! - Try to reuse a handle from the free list (again).
//! - Allocate a new handle, causing the backing Vec to grow.
//! 
//! While reusing handles from the free list is fast, scanning for them can take time when the coatcheck has many entries. 
//! Checking many values means that some unlucky callers will pay for the others to come.

// TODO watermark handles with coatcheck ID to prevent accidental cross-claiming 
// TODO try amortizing / limiting freelist scans costs when entry count is large  

use std::sync::{Arc};
use std::collections::VecDeque;

type Handle = Arc<usize>;

struct CoatcheckEntry<T> {
    inner_handle: Arc<usize>,
    payload: T,
}

impl <T> CoatcheckEntry<T> {
    fn create(index: usize, payload: T) -> CoatcheckEntry<T> {
        CoatcheckEntry {
            inner_handle: Arc::new(index),
            payload,
        }
    }

    fn handle(&self) -> Handle {
        self.inner_handle.clone()
    }

    fn is_free(&self) -> bool {
        Arc::strong_count(&self.inner_handle) == 1
    }

    fn reload(&mut self, payload: T) -> Handle {
        // TODO assert free
        self.payload = payload;
        self.handle()
    }
}

struct Coatcheck<T> {
    items: Vec<CoatcheckEntry<T>>,
    free_list: VecDeque<usize>,
}

impl <T> Coatcheck<T> {
    pub fn new() -> Coatcheck<T> {
        Coatcheck {
            items: Vec::new(),
            free_list: VecDeque::new(),
        }
    }

    pub fn with_capacity(size: usize) -> Coatcheck<T> {
        Coatcheck {
            items: Vec::with_capacity(size),
            free_list: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn free_len(&self) -> usize {
        self.free_list.len()
    }

    pub fn check(&mut self, payload: T) -> Handle {
        if let Some(free) = self.free_list.pop_back() {
            // reuse try entry that was marked empty
            self.reuse_handle(free, payload)
        } else {
            if self.items.len() < self.items.capacity() {
                // create a new entry without growing the vec
                self.new_handle(payload)
            } else {
                // look for abandoned entries to mark as free
                self.update_free_list();
                // try reusing again
                if let Some(free) = self.free_list.pop_back() {
                    self.reuse_handle(free, payload)
                } else {
                    // tough luck, growing the vec is our only option
                    self.new_handle(payload)
                }
            }
        }
    }

    /// reuse entry that was marked free
    fn reuse_handle(&mut self, free: usize, payload: T) -> Handle {
        self.items.get_mut(free)
            .expect("An element that was marked as free is actually out of bound.")
            .reload(payload)
    }

    fn new_handle(&mut self, payload: T) -> Handle {
        let entry = CoatcheckEntry::create(self.items.len(), payload);
        let handle = entry.handle();
        self.items.push(entry);
        handle
    }

    pub fn update_free_list(&mut self) {
        for (idx, entry) in self.items.iter().enumerate() {
            if entry.is_free() {
                // handle has been abandoned
                self.free_list.push_front(idx)
            }
        }
    }

    pub fn claim_mut(&mut self, handle: Handle) -> &mut T {
        &mut self.items.get_mut(*handle).expect(&format!("Invalid index {}", handle)).payload
    }


    pub fn claim(&self, handle: Handle) -> &T {
        &self.items.get(*handle).expect(&format!("Invalid index {}", handle)).payload
    }
}

mod test {

    use super::*;
    use std::ops::Index;
    use std::ops::Deref;

    #[test]
    fn linear() {
        let mut coatcheck = Coatcheck::new();
        let mut handles: Vec<Handle> = Vec::new();
        for i in 0..4 {
            handles.push(coatcheck.check("A".to_string()))
        }
        assert_eq!(coatcheck.len(), 4);
        assert_eq!(coatcheck.free_len(), 0);
        assert_eq!(handles.len(), 4);
        assert_eq!(**handles.index(3), 3);
    }

    #[test]
    fn recycle_all() {
        let mut coatcheck = Coatcheck::new();
        for i in 0..4 {
            coatcheck.check("A".to_string());
            coatcheck.update_free_list();
        }
        assert_eq!(coatcheck.len(), 1);
        assert_eq!(coatcheck.free_len(), 1);
    }

    #[test]
    fn recycle_one() {
        let mut coatcheck = Coatcheck::with_capacity(1);
        for i in 0..4 {
            coatcheck.check("A".to_string());
        }
        assert_eq!(coatcheck.len(), 1);
        assert_eq!(coatcheck.free_len(), 0);
    }

    #[test]
    fn alloc_and_free() {
        let mut coatcheck = Coatcheck::with_capacity(4);
        let mut handles: Vec<Handle> = Vec::new();
        let mut handles2: Vec<Handle> = Vec::new();
        for i in 0..4 {
            handles.push(coatcheck.check("A".to_string()))
        }
        assert_eq!(coatcheck.len(), 4);
        assert_eq!(coatcheck.free_len(), 0);

        // none free, alloc
        for i in 0..4 {
            handles2.push(coatcheck.check("A".to_string()))
        }
        handles.clear();
        assert_eq!(coatcheck.len(), 8);
        assert_eq!(coatcheck.free_len(), 0);

        // consume freed
        for i in 0..4 {
            handles.push(coatcheck.check("A".to_string()))
        }
        assert_eq!(coatcheck.len(), 8);
        assert_eq!(coatcheck.free_len(), 0);
    }

    #[test]
    fn get_handle() {
        let mut coatcheck: Coatcheck<String> = Coatcheck::with_capacity(4);
        let handle = coatcheck.check("A".to_string());
        assert_eq!(coatcheck.claim(handle), &"A".to_string());
    }

}

/// Run benchmarks with `cargo +nightly bench --features bench`
#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test::Bencher;

    #[bench]
    fn allocate_many_handles(b: &mut Bencher) {
        let mut coatcheck = Coatcheck::with_capacity(100000);
        let mut handles = Vec::with_capacity(100000);
        b.iter(|| handles.push(coatcheck.check(42)));
    }

    #[bench]
    fn recycle_four_handles(b: &mut Bencher) {
        let mut coatcheck = Coatcheck::with_capacity(64);
        let mut handles: Vec<Handle> = Vec::with_capacity(64);
        b.iter(|| {
            handles.push(coatcheck.check(42));
            handles.push(coatcheck.check(42));
            handles.push(coatcheck.check(42));
            handles.push(coatcheck.check(42));
            handles.clear()
        });
    }

    #[bench]
    fn recycle_two_handles(b: &mut Bencher) {
        let mut coatcheck = Coatcheck::with_capacity(64);
        let mut handles: Vec<Handle> = Vec::with_capacity(64);
        b.iter(|| {
            handles.push(coatcheck.check(42));
            handles.clear()
        });
    }

    #[bench]
    fn recycle_one_handle(b: &mut Bencher) {
        let mut coatcheck = Coatcheck::with_capacity(64);
        b.iter(|| {
            coatcheck.check(42);
        });
    }

    #[bench]
    fn get_handle(b: &mut Bencher) {
        let mut coatcheck: Coatcheck<u32> = Coatcheck::with_capacity(4);
        let handle = coatcheck.check(42);
        b.iter(|| coatcheck.claim(handle.clone()));
    }

}