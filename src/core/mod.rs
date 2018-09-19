pub mod error;
pub mod component;
pub mod input;
pub mod output;
pub mod out_lock;
pub mod clock;
pub mod void;
pub mod proxy;
pub mod pcg32;
pub mod scheduler;
pub mod metrics;

/// Base type for recorded metric values.
pub type Value = u64;

/// Both InputScope and OutputScope share the ability to flush the recorded data.
pub trait Flush {
    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use super::input::*;

    #[test]
    fn test_to_void() {
        let c = void::Void::metrics().input();
        let m = c.new_metric("test".into(), input::Kind::Marker);
        m.write(33);
    }

}

#[cfg(feature = "bench")]
pub mod bench {

    use core::{TimeHandle, Marker, Input};
    use aggregate::bucket::Bucket;
    use super::clock::TimeHandle;
    use test;
    use aggregate::Bucket;

    #[bench]
    fn get_instant(b: &mut test::Bencher) {
        b.iter(|| test::black_box(TimeHandle::now()));
    }

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut test::Bencher) {
        let metrics = Bucket::new();
        let marker = metrics.marker("aaa");
        b.iter(|| test::black_box(marker.mark()));
    }
}
