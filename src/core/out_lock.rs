use core::input::{InputScope, InputMetric, Input, Kind};
use core::output::{Output, OutputScope};
use core::component::{Attributes, WithAttributes, Naming};
use core::name::Name;
use core::Flush;
use core::error;
use std::rc::Rc;

use std::sync::{Arc, Mutex};
use std::ops;

/// Provide thread-safe locking to RawScope implementers.
#[derive(Clone)]
pub struct LockingScopeBox {
    attributes: Attributes,
    inner: Arc<Mutex<LockScope>>
}

impl WithAttributes for LockingScopeBox {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl InputScope for LockingScopeBox {

    fn new_metric(&self, name: Name, kind: Kind) -> InputMetric {
        let name = self.qualify(name);
        let raw_metric = self.inner.lock().expect("RawScope Lock").new_metric(name, kind);
        let mutex = self.inner.clone();
        InputMetric::new(move |value| {
            let _guard = mutex.lock().expect("OutputMetric Lock");
            raw_metric.write(value)
        } )
    }

}

impl Flush for LockingScopeBox {
    fn flush(&self) -> error::Result<()> {
        self.inner.lock().expect("OutputScope Lock").flush()
    }
}

/// Blanket impl that provides RawOutputs their dynamic flavor.
impl<T: Output + Send + Sync + 'static> Input for T {
    type SCOPE = LockingScopeBox;

    fn input(&self) -> Self::SCOPE {
        LockingScopeBox {
            attributes: Attributes::default(),
            inner: Arc::new(Mutex::new(LockScope(self.output_dyn())))
        }
    }
}

/// Wrap an OutputScope to make it Send + Sync, allowing it to travel the world of threads.
/// Obviously, it should only still be used from a single thread or dragons may occur.
#[derive(Clone)]
struct LockScope(Rc<OutputScope + 'static> );

impl ops::Deref for LockScope {
    type Target = OutputScope + 'static;
    fn deref(&self) -> &Self::Target {
        Rc::as_ref(&self.0)
    }
}

unsafe impl Send for LockScope {}
unsafe impl Sync for LockScope {}

