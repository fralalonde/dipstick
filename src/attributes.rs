use std::collections::HashMap;
use std::default::Default;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::name::{MetricName, NameParts};
use crate::scheduler::{Cancel, SCHEDULER};
use crate::{CancelHandle, Flush, InputMetric, InputScope, MetricValue};
use std::fmt;
use std::time::{Duration, Instant};

#[cfg(not(feature = "parking_lot"))]
use std::sync::RwLock;

use crate::Labels;
#[cfg(feature = "parking_lot")]
use parking_lot::RwLock;
use std::ops::Deref;

/// The actual distribution (random, fixed-cycled, etc.) depends on selected sampling method.
#[derive(Debug, Clone, Copy, Default)]
pub enum Sampling {
    /// Record every collected value.
    /// Effectively disable sampling.
    #[default]
    Full,

    /// Floating point sampling rate
    /// - 1.0+ records everything
    /// - 0.5 records one of two values
    /// - 0.0 records nothing
    Random(f64),
}

/// A metrics buffering strategy.
/// All strategies other than `Unbuffered` are applied as a best-effort, meaning that the buffer
/// may be flushed at any moment before reaching the limit, for any or no reason in particular.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum Buffering {
    /// Do not buffer output.
    #[default]
    Unbuffered,

    /// A buffer of maximum specified size is used.
    BufferSize(usize),

    /// Buffer as much as possible.
    Unlimited,
}

/// A metrics identifier
#[derive(Clone, Debug, Hash, Eq, PartialOrd, PartialEq)]
pub struct MetricId(String);

impl MetricId {
    /// Return a MetricId based on output type and metric name
    pub fn forge(out_type: &str, name: MetricName) -> Self {
        let id: String = name.join("/");
        MetricId(format!("{}:{}", out_type, id))
    }
}

pub type Shared<T> = Arc<RwLock<T>>;

pub struct Listener {
    listener_id: usize,
    listener_fn: Arc<dyn Fn(Instant) + Send + Sync + 'static>,
}

/// Attributes common to metric components.
/// Not all attributes used by all components.
#[derive(Clone, Default)]
pub struct Attributes {
    naming: NameParts,
    sampling: Sampling,
    buffering: Buffering,
    flush_listeners: Shared<HashMap<MetricId, Listener>>,
    tasks: Shared<Vec<CancelHandle>>,
}

impl fmt::Debug for Attributes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "naming: {:?}", self.naming)?;
        write!(f, "sampling: {:?}", self.sampling)?;
        write!(f, "buffering: {:?}", self.buffering)
    }
}

/// This trait should not be exposed outside the crate.
pub trait WithAttributes: Clone {
    /// Return attributes of component.
    fn get_attributes(&self) -> &Attributes;

    /// Return attributes of component for mutation.
    // TODO replace with fields-in-traits if ever stabilized (https://github.com/nikomatsakis/fields-in-traits-rfc)
    fn mut_attributes(&mut self) -> &mut Attributes;

    /// Clone the component and mutate its attributes at once.
    fn with_attributes<F: Fn(&mut Attributes)>(&self, edit: F) -> Self {
        let mut cloned = self.clone();
        (edit)(cloned.mut_attributes());
        cloned
    }
}

/// Register and notify scope-flush listeners
pub trait OnFlush {
    /// Notify registered listeners of an impending flush.
    fn notify_flush_listeners(&self);
}

impl<T> OnFlush for T
where
    T: Flush + WithAttributes,
{
    fn notify_flush_listeners(&self) {
        let now = Instant::now();
        for listener in read_lock!(self.get_attributes().flush_listeners).values() {
            (listener.listener_fn)(now)
        }
    }
}

/// When to observe a recurring task.
pub struct ObserveWhen<'a, T, F> {
    target: &'a T,
    metric: InputMetric,
    operation: Arc<F>,
}

static ID_GENERATOR: AtomicUsize = AtomicUsize::new(0);

/// A handle to cancel a flush observer.
pub struct OnFlushCancel(Arc<dyn Fn() + Send + Sync>);

impl Cancel for OnFlushCancel {
    fn cancel(&self) {
        (self.0)()
    }
}

impl<'a, T, F> ObserveWhen<'a, T, F>
where
    F: Fn(Instant) -> MetricValue + Send + Sync + 'static,
    T: InputScope + WithAttributes + Send + Sync,
{
    /// Observe the metric's value upon flushing the scope.
    pub fn on_flush(self) -> OnFlushCancel {
        let gauge = self.metric;
        let metric_id = gauge.metric_id().clone();
        let op = self.operation;
        let listener_id = ID_GENERATOR.fetch_add(1, Ordering::Relaxed);

        write_lock!(self.target.get_attributes().flush_listeners).insert(
            metric_id.clone(),
            Listener {
                listener_id,
                listener_fn: Arc::new(move |now| gauge.write(op(now), Labels::default())),
            },
        );

        let flush_listeners = self.target.get_attributes().flush_listeners.clone();
        OnFlushCancel(Arc::new(move || {
            let mut listeners = write_lock!(flush_listeners);
            let installed_listener_id = listeners.get(&metric_id).map(|v| v.listener_id);
            if let Some(id) = installed_listener_id {
                if id == listener_id {
                    listeners.remove(&metric_id);
                }
            }
        }))
    }

    /// Observe the metric's value periodically.
    pub fn every(self, period: Duration) -> CancelHandle {
        let gauge = self.metric;
        let op = self.operation;
        let handle = SCHEDULER.schedule(period, move |now| gauge.write(op(now), Labels::default()));
        write_lock!(self.target.get_attributes().tasks).push(handle.clone());
        handle
    }
}

/// Schedule a recurring task
pub trait Observe {
    /// The inner type for the [`ObserveWhen`].
    ///
    /// The observe can be delegated to a different type then `Self`, however the latter is more
    /// common.
    type Inner;
    /// Provide a source for a metric's values.
    #[must_use = "must specify when to observe"]
    fn observe<F>(
        &self,
        metric: impl Deref<Target = InputMetric>,
        operation: F,
    ) -> ObserveWhen<Self::Inner, F>
    where
        F: Fn(Instant) -> MetricValue + Send + Sync + 'static,
        Self: Sized;
}

impl<T: InputScope + WithAttributes> Observe for T {
    type Inner = Self;
    fn observe<F>(
        &self,
        metric: impl Deref<Target = InputMetric>,
        operation: F,
    ) -> ObserveWhen<Self, F>
    where
        F: Fn(Instant) -> MetricValue + Send + Sync + 'static,
        Self: Sized,
    {
        ObserveWhen {
            target: self,
            metric: (*metric).clone(),
            operation: Arc::new(operation),
        }
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        let mut tasks = write_lock!(self.tasks);
        for task in tasks.drain(..) {
            task.cancel()
        }
    }
}

/// Name operations support.
pub trait Prefixed {
    /// Returns namespace of component.
    fn get_prefixes(&self) -> &NameParts;

    /// Append a name to the existing names.
    /// Return a clone of the component with the updated names.
    #[deprecated(since = "0.7.2", note = "Use named() or add_name()")]
    fn add_prefix<S: Into<String>>(&self, name: S) -> Self;

    /// Append a name to the existing names.
    /// Return a clone of the component with the updated names.
    fn add_name<S: Into<String>>(&self, name: S) -> Self;

    /// Replace any existing names with a single name.
    /// Return a clone of the component with the new name.
    /// If multiple names are required, `add_name` may also be used.
    fn named<S: Into<String>>(&self, name: S) -> Self;

    /// Append any name parts to the name's namespace.
    fn prefix_append<S: Into<MetricName>>(&self, name: S) -> MetricName {
        name.into().append(self.get_prefixes().clone())
    }

    /// Prepend any name parts to the name's namespace.
    fn prefix_prepend<S: Into<MetricName>>(&self, name: S) -> MetricName {
        name.into().prepend(self.get_prefixes().clone())
    }
}

impl<T: WithAttributes> Prefixed for T {
    /// Returns namespace of component.
    fn get_prefixes(&self) -> &NameParts {
        &self.get_attributes().naming
    }

    /// Append a name to the existing names.
    /// Return a clone of the component with the updated names.
    fn add_prefix<S: Into<String>>(&self, name: S) -> Self {
        self.add_name(name)
    }

    /// Append a name to the existing names.
    /// Return a clone of the component with the updated names.
    fn add_name<S: Into<String>>(&self, name: S) -> Self {
        let name = name.into();
        self.with_attributes(|new_attr| new_attr.naming.push_back(name.clone()))
    }

    /// Replace any existing names with a single name.
    /// Return a clone of the component with the new name.
    /// If multiple names are required, `add_name` may also be used.
    fn named<S: Into<String>>(&self, name: S) -> Self {
        let parts = NameParts::from(name);
        self.with_attributes(|new_attr| new_attr.naming = parts.clone())
    }
}

/// Apply statistical sampling to collected metrics data.
pub trait Sampled: WithAttributes {
    /// Perform random sampling of values according to the specified rate.
    fn sampled(&self, sampling: Sampling) -> Self {
        self.with_attributes(|new_attr| new_attr.sampling = sampling)
    }

    /// Get the sampling strategy for this component, if any.
    fn get_sampling(&self) -> Sampling {
        self.get_attributes().sampling
    }
}

/// Determine scope buffering strategy, if supported by output.
/// Changing this only affects scopes opened afterwards.
/// Buffering is done on best effort, meaning flush will occur if buffer capacity is exceeded.
pub trait Buffered: WithAttributes {
    /// Return a clone with the specified buffering set.
    fn buffered(&self, buffering: Buffering) -> Self {
        self.with_attributes(|new_attr| new_attr.buffering = buffering)
    }

    /// Return the current buffering strategy.
    fn get_buffering(&self) -> Buffering {
        self.get_attributes().buffering
    }

    /// Returns false if the current buffering strategy is `Buffering::Unbuffered`.
    /// Returns true otherwise.
    fn is_buffered(&self) -> bool {
        !(self.get_attributes().buffering == Buffering::Unbuffered)
    }
}

#[cfg(test)]
mod test {
    use crate::attributes::*;
    use crate::input::Input;
    use crate::input::*;
    use crate::output::map::StatsMap;
    use crate::Flush;
    use crate::StatsMapScope;

    #[test]
    fn on_flush() {
        let metrics: StatsMapScope = StatsMap::default().metrics();
        let gauge = metrics.gauge("my_gauge");
        metrics.observe(gauge, |_| 4).on_flush();
        metrics.flush().unwrap();
        assert_eq!(Some(&4), metrics.into_map().get("my_gauge"))
    }
}
