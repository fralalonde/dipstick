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
use crate::{Output, Input, OutputDyn};
use std::error::Error;

/// Allow turning this single-thread output into a threadsafe Input.
/// Mutex locking will be used to serialize access to the output.
/// This used to automatically implemented for all Output implementers,
/// which made impossible have the opposite (a Threadsafe Input implementing Output - e.g. Log)
pub trait Locking  {

    /// Wrap single threaded output with mutex locking, turning it into a thread safe Input.
    #[deprecated(since = "0.8.0", note = "Use locking() or queued(n) first, then metrics()")]
    fn metrics(&self) -> LockedOutputScope {
        self.locking().metrics()
    }

    /// Wrap single threaded output with autolocking, turning it into a threadsafe metric Input
    fn locking(&self) -> OutputSerializer;
}

/// Synchronous thread-safety for metric output using basic locking.
#[derive(Clone)]
pub struct OutputSerializer {
    attributes: Attributes,
    inner: Arc<Mutex<Box<dyn OutputDyn>>>,
}

impl OutputSerializer {
    /// Wrap a single-threaded OutputScope into a mutex-locking InputScope
    pub fn new(attributes: &Attributes, output: Box<dyn OutputDyn>) -> Self {
        OutputSerializer {
            attributes: attributes.clone(),
            inner: Arc::new(Mutex::new(output))
        }
    }
}

impl Input for OutputSerializer {
    type SCOPE = LockedOutputScope;

    fn metrics(&self) -> Self::SCOPE {
        let out_scope = self.inner.lock().expect("Metrics Output").output_dyn();
        let inner = SafeScope {
            inner: out_scope,
        };
        LockedOutputScope {
            attributes: self.attributes.clone(),
            inner: Arc::new(Mutex::new(inner))
        }
    }
}

impl WithAttributes for OutputSerializer {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Wrap an OutputScope to make it Send + Sync, allowing it to travel the world of threads.
/// Obviously, it should only still be used from a single thread at a time or dragons may occur.
#[derive(Clone)]
pub struct LockedOutputScope {
    attributes: Attributes,
    inner: Arc<Mutex<SafeScope>>
}

impl WithAttributes for LockedOutputScope {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl InputScope for LockedOutputScope {
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

impl Flush for LockedOutputScope {
    fn flush(&self) -> error::Result<()> {
        self.inner.lock().expect("SafeOutput").flush()
    }
}

struct SafeScope {
    inner: Rc<dyn OutputScope>
}

impl ops::Deref for SafeScope {
    type Target = dyn OutputScope + 'static;
    fn deref(&self) -> &Self::Target {
        Rc::as_ref(&self.inner)
    }
}

unsafe impl Send for SafeScope {}
unsafe impl Sync for SafeScope {}
