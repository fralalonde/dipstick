//! Default locking strategy for shared concurrent output.
//! This makes all outputs also immediately usable as inputs.
//! The alternatives are queuing or thread local.

use crate::core::attributes::{Attributes, MetricId, OnFlush, Prefixed, WithAttributes};
use crate::core::error;
use crate::core::input::{InputKind, InputMetric, InputScope};
use crate::core::name::MetricName;
use crate::core::output::{OutputScope};
use crate::core::Flush;
use std::rc::Rc;

use std::ops;
use std::sync::{Arc, Mutex};

/// Allow turning this single-thread output into a threadsafe Input.
/// Mutex locking will be used to serialize access to the output.
/// This used to automatically implemented for all Output implementers,
/// which made impossible have the opposite (a Threadsafe Input implementing Output - e.g. Log)
pub trait Locking  {

    /// Wrap single threaded output with autolocking, turning it into a threadsafe metric Input
    #[deprecated(since = "0.8.0", note = "Use locking()")]
    fn metrics(&self) -> LockingOutput{
        self.locking()
    }

    /// Wrap single threaded output with autolocking, turning it into a threadsafe metric Input
    fn locking(&self) -> LockingOutput;
}

/// Synchronous thread-safety for metric output using basic locking.
#[derive(Clone)]
pub struct LockingOutput {
    attributes: Attributes,
    inner: Arc<Mutex<LockedOutputScope>>,
}

impl LockingOutput {
    /// Wrap a single-threaded OutputScope into a mutex-locking InputScope
    pub fn new(attributes: &Attributes, scope: Rc<dyn OutputScope>) -> Self {
        LockingOutput {
            attributes: attributes.clone(),
            inner: Arc::new(Mutex::new(LockedOutputScope(scope)))
        }
    }
}

impl WithAttributes for LockingOutput {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl InputScope for LockingOutput {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let name = self.prefix_append(name);
        // lock when creating metrics
        let raw_metric = self
            .inner
            .lock()
            .expect("LockingOutput")
            .new_metric(name.clone(), kind);
        let mutex = self.inner.clone();
        InputMetric::new(MetricId::forge("locking", name), move |value, labels| {
            // lock when collecting values
            let _guard = mutex.lock().expect("LockingOutput");
            raw_metric.write(value, labels)
        })
    }
}

impl Flush for LockingOutput {
    fn flush(&self) -> error::Result<()> {
        self.notify_flush_listeners();
        self.inner.lock().expect("LockingOutput").flush()
    }
}

/// Wrap an OutputScope to make it Send + Sync, allowing it to travel the world of threads.
/// Obviously, it should only still be used from a single thread at a time or dragons may occur.
#[derive(Clone)]
struct LockedOutputScope(Rc<dyn OutputScope + 'static>);

impl ops::Deref for LockedOutputScope {
    type Target = dyn OutputScope + 'static;
    fn deref(&self) -> &Self::Target {
        Rc::as_ref(&self.0)
    }
}

unsafe impl Send for LockedOutputScope {}
unsafe impl Sync for LockedOutputScope {}
