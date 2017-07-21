use core::{MetricType, Value, SinkWriter, MetricSink, MetricDispatch, EventMetric, ValueMetric, TimerMetric, MetricScope};
use std::rc::Rc;
use std::cell::RefCell;
use thread_local::ThreadLocal;

////////////

pub struct DirectEvent<C: MetricSink> {
    metric: <C as MetricSink>::Metric,
    target: Rc<C>,
}

pub struct DirectValue<C: MetricSink> {
    metric: <C as MetricSink>::Metric,
    target: Rc<C>,
}

pub struct DirectTimer<C: MetricSink> {
    metric: <C as MetricSink>::Metric,
    target: Rc<C>,
}

pub struct DirectScope {
}

impl <C: MetricSink> EventMetric for DirectEvent<C>  {
    fn event(&self) {
        self.target.write(|scope| scope.write(&self.metric, 1))
    }
}

impl <C: MetricSink> ValueMetric for DirectValue<C> {
    fn value(&self, value: Value) {
        self.target.write(|scope| scope.write(&self.metric, value))
    }
}

impl <C: MetricSink> ValueMetric for DirectTimer<C> {
    fn value(&self, value: Value) {
        self.target.write(|scope| scope.write(&self.metric, value))
    }
}

impl <C: MetricSink> TimerMetric for DirectTimer<C> {}

impl MetricScope for DirectScope {
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self {
        self
    }
}

pub struct DirectDispatch<C: MetricSink> {
    target: Rc<C>
}

impl <C: MetricSink> DirectDispatch<C> {
    pub fn new(target: C) -> DirectDispatch<C> {
        DirectDispatch { target: Rc::new(target) }
    }
}

thread_local! {
    static DISPATCH_SCOPE: RefCell<DirectScope> = RefCell::new(DirectScope {});
}

impl <C: MetricSink> MetricDispatch for DirectDispatch<C> {
    type Event = DirectEvent<C>;
    type Value = DirectValue<C>;
    type Timer = DirectTimer<C>;
    type Scope = DirectScope;

    fn new_event<S: AsRef<str>>(&self, name: S) -> Self::Event {
        let metric = self.target.define(MetricType::Event, name, 1.0);
        DirectEvent { metric, target: self.target.clone() }
    }

    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Value {
        let metric = self.target.define(MetricType::Count, name, 1.0);
        DirectValue { metric, target: self.target.clone() }
    }

    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Timer {
        let metric = self.target.define(MetricType::Time, name, 1.0);
        DirectTimer { metric, target: self.target.clone() }
    }

    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Value {
        let metric = self.target.define(MetricType::Gauge, name, 1.0);
        DirectValue { metric, target: self.target.clone() }
    }

    fn scope<F>(&self, operations: F) where F: Fn(&Self::Scope) {}
}

