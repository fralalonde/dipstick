use core::{MetricType, Value, MetricWriter, MetricSink, MetricDispatch, EventMetric, ValueMetric, TimerMetric, DispatchScope};
use std::sync::Arc;
use thread_local_object::ThreadLocal;

/// Base struct for all direct dispatch metrics
struct DirectMetric<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    dispatch_scope: Arc<DirectScope<C>>
}

/// An event marker that dispatches values directly to the metrics backend
pub struct DirectEvent<C: MetricSink + 'static>(DirectMetric<C>);

/// A gauge or counter that dispatches values directly to the metrics backend
pub struct DirectValue<C: MetricSink + 'static>(DirectMetric<C>);

/// An timer that dispatches values directly to the metrics backend
pub struct DirectTimer<C: MetricSink + 'static>(DirectMetric<C>);

/// A scoped writer
pub struct ScopeWriter<C: MetricSink> {
    writer: C::Writer
//    properties: Hashmap;
}

impl <C: MetricSink> DispatchScope for ScopeWriter<C> {
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self {
        self
    }
}

/// The shared scope-selector for all of a single Dispatcher metrics
pub struct DirectScope<C: MetricSink + 'static> {
    default_scope: C::Writer,
    thread_scope: ThreadLocal<ScopeWriter<C>>,
}

impl <C: MetricSink> DirectScope<C> {
    fn value(&self, metric: &C::Metric, value: Value) {
        let scope = self.thread_scope.get(|scope|
            match scope {
                Some(scoped) => scoped.writer.write(metric, value),
                None => self.default_scope.write(metric, value)
            }
        );
    }
}

impl <C: MetricSink> EventMetric for DirectEvent<C>  {
    fn mark(&self) {
        self.0.dispatch_scope.value(&self.0.metric, 1);
    }
}

impl <C: MetricSink> ValueMetric for DirectValue<C> {
    fn value(&self, value: Value) {
        self.0.dispatch_scope.value(&self.0.metric, value);
    }
}

impl <C: MetricSink> ValueMetric for DirectTimer<C> {
    fn value(&self, value: Value) {
        self.0.dispatch_scope.value(&self.0.metric,value);
    }
}

impl <C: MetricSink> TimerMetric for DirectTimer<C> {
}

impl <C: MetricSink> DispatchScope for DirectScope<C> {
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self {
        self
    }
}

pub struct DirectDispatch<C: MetricSink + 'static> {
    target: C,
    dispatch_scope: Arc<DirectScope<C>>
}

impl <C: MetricSink> DirectDispatch<C> {
    pub fn new(target: C) -> DirectDispatch<C> {
        let default_scope = target.new_writer();
        DirectDispatch { target, dispatch_scope: Arc::new(DirectScope { default_scope, thread_scope: ThreadLocal::new()}) }
    }
}

impl <C: MetricSink> MetricDispatch for DirectDispatch<C> {
    type Event = DirectEvent<C>;
    type Value = DirectValue<C>;
    type Timer = DirectTimer<C>;
    type Scope = ScopeWriter<C>;

    fn new_event<S: AsRef<str>>(&self, name: S) -> Self::Event {
        let metric = self.target.define(MetricType::Event, name, 1.0);
        DirectEvent ( DirectMetric{ metric, dispatch_scope: self.dispatch_scope.clone() })
    }

    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Value {
        let metric = self.target.define(MetricType::Count, name, 1.0);
        DirectValue ( DirectMetric { metric, dispatch_scope: self.dispatch_scope.clone() })
    }

    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Timer {
        let metric = self.target.define(MetricType::Time, name, 1.0);
        DirectTimer ( DirectMetric { metric, dispatch_scope: self.dispatch_scope.clone() })
    }

    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Value {
        let metric = self.target.define(MetricType::Gauge, name, 1.0);
        DirectValue ( DirectMetric { metric, dispatch_scope: self.dispatch_scope.clone() })
    }

    fn with_scope<F>(&mut self, operations: F) where F: Fn(&Self::Scope) {
        let new_writer = self.target.new_writer();
        let scope = ScopeWriter{ writer: new_writer};
        // TODO add ThreadLocal with(T, FnOnce) method to replace these three
        self.dispatch_scope.thread_scope.set(scope);
        self.dispatch_scope.thread_scope.get(|option_scope| {
            operations(option_scope.unwrap())
        });
        self.dispatch_scope.thread_scope.remove();
    }
}

/// Run benchmarks with `cargo +nightly bench --features bench`
#[cfg(feature="bench")]
mod bench {

    use aggregate::sink::MetricAggregator;
    use core::{MetricDispatch, EventMetric};
    use test::Bencher;

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut Bencher) {
        let aggregate = MetricAggregator::new().sink();
        let dispatch = super::DirectDispatch::new(aggregate);
        let event = dispatch.new_event("aaa");
        b.iter(|| event.mark());
    }



}
