use core::input::InputKind;
use core::label::Labels;
use core::name::MetricName;
use core::void::Void;
use core::{Flush, MetricValue};

use std::rc::Rc;

/// Define metrics, write values and flush them.
pub trait OutputScope: Flush {
    /// Define a raw metric of the specified type.
    fn new_metric(&self, name: MetricName, kind: InputKind) -> OutputMetric;
}

/// Output metrics are not thread safe.
#[derive(Clone)]
pub struct OutputMetric {
    inner: Rc<Fn(MetricValue, Labels)>,
}

impl OutputMetric {
    /// Utility constructor
    pub fn new<F: Fn(MetricValue, Labels) + 'static>(metric: F) -> OutputMetric {
        OutputMetric {
            inner: Rc::new(metric),
        }
    }

    /// Some may prefer the `metric.write(value)` form to the `(metric)(value)` form.
    /// This shouldn't matter as metrics should be of type Counter, Marker, etc.
    #[inline]
    pub fn write(&self, value: MetricValue, labels: Labels) {
        (self.inner)(value, labels)
    }
}

/// A function trait that opens a new metric capture scope.
pub trait Output: Send + Sync + 'static + OutputDyn {
    /// The type of Scope returned byt this output.
    type SCOPE: OutputScope;

    /// Open a new scope for this output.
    fn new_scope(&self) -> Self::SCOPE;

    /// Open a new scope for this output.
    #[deprecated(since = "0.7.2", note = "Use new_scope()")]
    fn output(&self) -> Self::SCOPE {
        self.new_scope()
    }
}

/// A function trait that opens a new metric capture scope.
pub trait OutputDyn {
    /// Open a new scope from this output.
    fn output_dyn(&self) -> Rc<OutputScope + 'static>;
}

/// Blanket impl of dyn output trait
impl<T: Output + Send + Sync + 'static> OutputDyn for T {
    fn output_dyn(&self) -> Rc<OutputScope + 'static> {
        Rc::new(self.new_scope())
    }
}

/// Discard all metric values sent to it.
pub fn output_none() -> Void {
    Void {}
}
