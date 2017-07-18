use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, MetricChannel, TimeType};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::rc::Rc;
use std::cell::RefCell;

pub enum Score {
    Event { start_time: TimeType, hit_count: AtomicUsize },
    Value { start_time: TimeType, hit_count: AtomicUsize, value_sum: AtomicUsize, max: AtomicUsize, min: AtomicUsize },
}

pub struct AggregateMetric {
    pub m_type: MetricType,
    pub name: String,
    pub score: Score,
}

impl AggregateMetric {

    pub fn write(&self, value: usize) -> () {
        match &self.score {
            &Score::Event {ref hit_count, ..} => {
                hit_count.fetch_add(1, Ordering::Acquire);
            },
            &Score::Value {ref hit_count, ref value_sum, ref max, ref min, ..} => {
                let mut try_max = max.load(Ordering::Acquire);
                while value > try_max {
                    if max.compare_and_swap(try_max, value, Ordering::Release) == try_max {
                        break
                    } else {
                        try_max = max.load(Ordering::Acquire);
                    }
                }

                let mut try_min = min.load(Ordering::Acquire);
                while value < try_min {
                    if min.compare_and_swap(try_min, value, Ordering::Release) == try_min {
                        break
                    } else {
                        try_min = min.load(Ordering::Acquire);
                    }
                }
                value_sum.fetch_add(value, Ordering::Acquire);
                // TODO report concurrent updates / resets
                hit_count.fetch_add(1, Ordering::Acquire);
            }
        }
    }

    pub fn reset(&mut self) {
        match &mut self.score {
            &mut Score::Event {ref mut start_time, ref hit_count} => {
                *start_time = TimeType::now();
                hit_count.store(0, Ordering::Release)
            },
            &mut Score::Value {ref mut start_time, ref hit_count, ref value_sum,ref max, ref min} => {
                *start_time = TimeType::now();
                hit_count.store(0, Ordering::Release);
                value_sum.store(0, Ordering::Release);
                max.store(0, Ordering::Release);
                min.store(0, Ordering::Release);
            }
        }
    }
}

impl DefinedMetric for Rc<AggregateMetric> {
}

pub struct AggregateWrite {
}

impl MetricWrite<Rc<AggregateMetric>> for AggregateWrite {
    fn write(&self, metric: &Rc<AggregateMetric>, value: Value) {
        println!("Aggregate Metric");
        metric.write(value as usize);
    }
}

pub struct ScoreIterator {
    metrics: Rc<RefCell<Vec<Rc<AggregateMetric>>>>,
}

impl ScoreIterator {
    pub fn for_each<F>(&self, ops: F) where F: Fn(&AggregateMetric) {
        for m in self.metrics.borrow().iter() {
            ops(m)
        }
    }

//    fn write<F>(&self, operations: F) where F: Fn(&Self::Write);
}

pub struct AggregateChannel {
    write: AggregateWrite,
    metrics: Rc<RefCell<Vec<Rc<AggregateMetric>>>>,
}

impl AggregateChannel {

    pub fn new() -> AggregateChannel {
        AggregateChannel { write: AggregateWrite {}, metrics: Rc::new(RefCell::new(Vec::new())) }
    }

    pub fn scores(&self) -> ScoreIterator {
        ScoreIterator { metrics : self.metrics.clone() }
    }

}

impl MetricChannel for AggregateChannel {
    type Metric = Rc<AggregateMetric>;
    type Write = AggregateWrite;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> Rc<AggregateMetric> {
        let name = name.as_ref().to_string();
        let metric = Rc::new(AggregateMetric {
            m_type, name, score: match m_type {
                MetricType::Event => Score::Event {
                        start_time: TimeType::now(),
                        hit_count: AtomicUsize::new(0) },
                _ => Score::Value {
                        start_time: TimeType::now(),
                        hit_count: AtomicUsize::new(0),
                        value_sum: AtomicUsize::new(0),
                        max: AtomicUsize::new(0),
                        min: AtomicUsize::new(0) },
            }
        });

        self.metrics.borrow_mut().push(metric.clone());
        metric
    }

    fn write<F>(&self, operations: F ) where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}

