use std::time::Instant;
use core::Value;

#[derive(Debug, Copy, Clone)]
/// A handle to the start time of a counter.
/// Wrapped so it may be changed safely later.
pub struct TimeHandle(Instant);

impl TimeHandle {
    /// Get a handle on current time.
    /// Used by the TimerMetric start_time() method.
    pub fn now() -> TimeHandle {
        TimeHandle(self::inner::now())
    }

    /// Get the elapsed time in microseconds since TimeHanduule was obtained.
    pub fn elapsed_us(self) -> Value {
        let duration = self::inner::now() - self.0;
        duration.as_secs() * 1000000 + (duration.subsec_nanos() / 1000) as Value
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_ms(self) -> Value {
        self.elapsed_us() / 1000
    }
}

#[cfg(not(any(mock_clock, test)))]
mod inner {
    use std::time::Instant;

    pub fn now() -> Instant {
        Instant::now()
    }
}

#[cfg(any(mock_clock, test))]
pub mod inner {
    use std::ops::Add;
    use std::time::{Duration, Instant};
    use std::sync::RwLock;

    lazy_static!{
        static ref MOCK_CLOCK: RwLock<Instant> = RwLock::new(Instant::now());
    }


    /// Metrics mock_clock enabled!
    /// thread::sleep will have no effect on metrics.
    /// Use advance_time() to simulate passing time.
    pub fn now() -> Instant {
        MOCK_CLOCK.read().unwrap().clone()
    }

    /// Advance the mock clock by a certain amount of time.
    /// Enables writing reproducible metrics tests.
    pub fn advance_time(period: Duration) {
        let mut now = MOCK_CLOCK.write().unwrap();
        let new_now = now.add(period);
        *now = new_now;
    }
}

#[cfg(test)]
mod test {
    use Value;
    use clock::inner;

    #[test]
    fn aggregate_all_stats() {
        use std::time::Duration;
        use std::collections::BTreeMap;
        use aggregate::{MetricAggregator, all_stats};
        use scope::MetricInput;
        use local::StatsMap;

        let metrics = MetricAggregator::new().with_suffix("test");

        let counter = metrics.counter("counter_a");
        let timer = metrics.timer("timer_a");
        let gauge = metrics.gauge("gauge_a");
        let marker = metrics.marker("marker_a");

        marker.mark();
        marker.mark();
        marker.mark();

        counter.count(10);
        counter.count(20);

        timer.interval_us(10_000_000);
        timer.interval_us(20_000_000);

        gauge.value(10);
        gauge.value(20);

        inner::advance_time(Duration::from_secs(3));

        // TODO expose & use flush_to()
        let stats = StatsMap::new();
        metrics.flush_to(&stats, &all_stats);
        let map: BTreeMap<String, Value> = stats.into();

        assert_eq!(map["test.counter_a.count"], 2);
        assert_eq!(map["test.counter_a.sum"], 30);
        assert_eq!(map["test.counter_a.mean"], 15);
        assert_eq!(map["test.counter_a.rate"], 10);

        assert_eq!(map["test.timer_a.count"], 2);
        assert_eq!(map["test.timer_a.sum"], 30_000_000);
        assert_eq!(map["test.timer_a.min"], 10_000_000);
        assert_eq!(map["test.timer_a.max"], 20_000_000);
        assert_eq!(map["test.timer_a.mean"], 15_000_000);
        assert_eq!(map["test.timer_a.rate"], 1);

        assert_eq!(map["test.gauge_a.mean"], 15);
        assert_eq!(map["test.gauge_a.min"], 10);
        assert_eq!(map["test.gauge_a.max"], 20);

        assert_eq!(map["test.marker_a.count"], 3);
        assert_eq!(map["test.marker_a.rate"], 1);
    }
}
