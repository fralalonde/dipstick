use core::{MetricType, Value, SinkWriter, MetricSink, MetricDispatch, EventMetric, ValueMetric, TimerMetric, MetricScope};
use std::sync::Arc;
use std::cell::RefCell;
use thread_local_object::ThreadLocal;

////////////

pub struct DirectEvent<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    dispatch_scope: Arc<DirectScope<C>>
}

pub struct DirectValue<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    dispatch_scope: Arc<DirectScope<C>>
}

pub struct DirectTimer<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    dispatch_scope: Arc<DirectScope<C>>
}

pub struct ScopeWriter<C: MetricSink> {
    writer: C::Writer
    // properties: hashmap
}

impl <C: MetricSink> MetricScope for ScopeWriter<C> {
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self {
        self
    }
}

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
    fn event(&self) {
        self.dispatch_scope.value(&self.metric, 1);
    }
}

impl <C: MetricSink> ValueMetric for DirectValue<C> {
    fn value(&self, value: Value) {
        self.dispatch_scope.value(&self.metric, value);
    }
}

impl <C: MetricSink> ValueMetric for DirectTimer<C> {
    fn value(&self, value: Value) {
        self.dispatch_scope.value(&self.metric,value);
    }
}

impl <C: MetricSink> TimerMetric for DirectTimer<C> {
}

impl <C: MetricSink> MetricScope for DirectScope<C> {
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
        DirectEvent { metric, dispatch_scope: self.dispatch_scope.clone() }
    }

    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Value {
        let metric = self.target.define(MetricType::Count, name, 1.0);
        DirectValue { metric, dispatch_scope: self.dispatch_scope.clone() }
    }

    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Timer {
        let metric = self.target.define(MetricType::Time, name, 1.0);
        DirectTimer { metric, dispatch_scope: self.dispatch_scope.clone() }
    }

    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Value {
        let metric = self.target.define(MetricType::Gauge, name, 1.0);
        DirectValue { metric, dispatch_scope: self.dispatch_scope.clone() }
    }

    fn scope<F>(&mut self, operations: F) where F: Fn(/*&Self::Scope*/) {
        let new_writer = self.target.new_writer();
        let mut scope = ScopeWriter{ writer: new_writer};
        self.dispatch_scope.thread_scope.set(scope);
        operations();
        self.dispatch_scope.thread_scope.remove();
    }
}

/// Run benchmarks with `cargo +nightly bench --features bench`
#[cfg(feature="bench")]
mod bench {

    use aggregate::sink::AggregateChannel;
    use core::{MetricType, MetricSink, SinkWriter, MetricDispatch, EventMetric};
    use test::Bencher;

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut Bencher) {
        let aggregate = AggregateChannel::new();
        let dispatch = super::DirectDispatch::new(aggregate);
        let metric = dispatch.new_event("aaa");
        b.iter(|| metric.event());
    }



}
