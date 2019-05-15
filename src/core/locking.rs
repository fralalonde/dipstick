//! Default locking strategy for shared concurrent output.
//! This makes all outputs also immediately usable as inputs.
//! The alternatives are queuing or thread local.

use core::attributes::{Attributes, OnFlush, Prefixed, WithAttributes, MetricId};
use core::error;
use core::input::{Input, InputKind, InputMetric, InputScope};
use core::name::MetricName;
use core::output::{Output, OutputScope};
use core::Flush;
use std::rc::Rc;

use std::ops;
use std::sync::{Arc, Mutex};

/// Synchronous thread-safety for metric output using basic locking.
#[derive(Clone)]
pub struct LockingOutput {
    attributes: Attributes,
    inner: Arc<Mutex<LockedOutputScope>>,
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

impl<T: Output + Send + Sync + 'static> Input for T {
    type SCOPE = LockingOutput;

    fn metrics(&self) -> Self::SCOPE {
        LockingOutput {
            attributes: Attributes::default(),
            inner: Arc::new(Mutex::new(LockedOutputScope(self.output_dyn()))),
        }
    }
}

/// Wrap an OutputScope to make it Send + Sync, allowing it to travel the world of threads.
/// Obviously, it should only still be used from a single thread at a time or dragons may occur.
#[derive(Clone)]
struct LockedOutputScope(Rc<OutputScope + 'static>);

impl ops::Deref for LockedOutputScope {
    type Target = OutputScope + 'static;
    fn deref(&self) -> &Self::Target {
        Rc::as_ref(&self.0)
    }
}

unsafe impl Send for LockedOutputScope {}
unsafe impl Sync for LockedOutputScope {}
