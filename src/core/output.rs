use core::{Flush, Value};
use core::input::Kind;
use core::component::Name;
use core::void::Void;

use std::rc::Rc;

/// Define metrics, write values and flush them.
pub trait OutputScope: Flush {

    /// Define a raw metric of the specified type.
    fn new_metric(&self, name: Name, kind: Kind) -> OutputMetric;

}

impl OutputMetric {
    /// Utility constructor
    pub fn new<F: Fn(Value) + 'static>(metric: F) -> OutputMetric {
        OutputMetric { inner: Rc::new(metric) }
    }

    /// Some may prefer the `metric.write(value)` form to the `(metric)(value)` form.
    /// This shouldn't matter as metrics should be of type Counter, Marker, etc.
    #[inline]
    pub fn write(&self, value: Value) {
        (self.inner)(value)
    }
}


/// A function trait that opens a new metric capture scope.
pub trait Output: Send + Sync + 'static + OutputDyn {
    /// The type of Scope returned byt this output.
    type SCOPE: OutputScope;

    /// Open a new scope from this output.
    fn output(&self) -> Self::SCOPE;
}

/// A function trait that opens a new metric capture scope.
pub trait OutputDyn {
    /// Open a new scope from this output.
    fn output_dyn(&self) -> Rc<OutputScope + 'static>;
}

/// Blanket impl of dyn output trait
impl<T: Output + Send + Sync + 'static> OutputDyn for T {
    fn output_dyn(&self) -> Rc<OutputScope + 'static> {
        Rc::new(self.output())
    }
}

/// Output metrics are not thread safe.
#[derive(Clone)]
pub struct OutputMetric {
    inner: Rc<Fn(Value)>
}

/// Discard all metric values sent to it.
pub fn output_none() -> Void {
    Void {}
}