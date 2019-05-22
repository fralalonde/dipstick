pub mod attributes;
pub mod clock;
pub mod error;
pub mod input;
pub mod label;
pub mod locking;
pub mod metrics;
pub mod name;
pub mod output;
pub mod pcg32;
pub mod proxy;
pub mod scheduler;
pub mod void;

/// Base type for recorded metric values.
pub type MetricValue = isize;

/// Both InputScope and OutputScope share the ability to flush the recorded data.
pub trait Flush {
    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()>;
}

#[cfg(test)]
pub mod test {
    use super::input::*;
    use super::*;

    #[test]
    fn test_to_void() {
        let c = void::Void::new().metrics();
        let m = c.new_metric("test".into(), input::InputKind::Marker);
        m.write(33, labels![]);
    }
}

#[cfg(feature = "bench")]
pub mod bench {

    use super::super::bucket::atomic::*;
    use super::clock::*;
    use super::input::*;

    #[bench]
    fn get_instant(b: &mut test::Bencher) {
        b.iter(|| test::black_box(TimeHandle::now()));
    }

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut test::Bencher) {
        let metrics = AtomicBucket::new();
        let marker = metrics.marker("aaa");
        b.iter(|| test::black_box(marker.mark()));
    }
}
