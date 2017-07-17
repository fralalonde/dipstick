use core::{MetricType, Value, MetricWrite, Channel, MetricDispatch, EventMetric, ValueMetric, TimerMetric, MetricScope};
use std::rc::Rc;

////////////

pub struct DirectEvent<C: Channel> {
    metric: <C as Channel>::Metric,
    target: Rc<C>,
}

pub struct DirectValue<C: Channel> {
    metric: <C as Channel>::Metric,
    target: Rc<C>,
}

pub struct DirectTimer<C: Channel> {
    metric: <C as Channel>::Metric,
    target: Rc<C>,
}

pub struct DirectScope {
}

impl <C: Channel> EventMetric for DirectEvent<C>  {
    fn event(&self) {
        self.target.write(|scope| scope.write(&self.metric, 1))
    }
}

impl <C: Channel> ValueMetric for DirectValue<C> {
    fn value(&self, value: Value) {
        self.target.write(|scope| scope.write(&self.metric, value))
    }
}

impl <C: Channel> ValueMetric for DirectTimer<C> {
    fn value(&self, value: Value) {
        self.target.write(|scope| scope.write(&self.metric, value))
    }
}

impl <C: Channel> TimerMetric for DirectTimer<C> {}

impl MetricScope for DirectScope {
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self {
        self
    }
}

pub struct DirectDispatch<C: Channel> {
    target: Rc<C>
}

impl <C: Channel> DirectDispatch<C> {
    pub fn new(target: C) -> DirectDispatch<C> {
        DirectDispatch { target: Rc::new(target) }
    }
}

impl <C: Channel> MetricDispatch for DirectDispatch<C> {
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

