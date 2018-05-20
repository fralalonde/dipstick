//! Reduce the amount of data to process or transfer by statistically dropping some of it.

use core::*;
use output::*;

use std::sync::Arc;

/// Apply statistical sampling to collected metrics data.
pub trait WithSamplingRate
where
    Self: Sized,
{
    /// Perform random sampling of values according to the specified rate.
    fn with_sampling_rate(&self, sampling_rate: Sampling) -> Self;
}

impl<M: Send + Sync + 'static + Clone> WithSamplingRate for MetricOutput<M> {
    fn with_sampling_rate(&self, new_rate: Sampling) -> Self {
        let int_sampling_rate = pcg32::to_int_rate(new_rate);

        self.wrap_all(|metric_fn, scope_fn| {
            (
                Arc::new(move |name, kind, prev_rate| {
                    // TODO override only if FULL_SAMPLING else warn!()
                    if prev_rate != FULL_SAMPLING_RATE {
                        info!(
                            "Metric {:?} will be sampled twice {}, {}", name, prev_rate, new_rate);
                    }

                    let new_rate = new_rate * prev_rate;
                    metric_fn(name, kind, new_rate)
                }),
                Arc::new(move || {
                    let next_scope = scope_fn();
                    command_fn(move |cmd| match cmd {
                        Command::Write(metric, value) => {
                            if pcg32::accept_sample(int_sampling_rate) {
                                next_scope.write(metric, value)
                            }
                        }
                        Command::Flush => next_scope.flush(),
                    })
                }),
            )
        })
    }
}

/// Perform random sampling of values according to the specified rate.
#[deprecated(since = "0.5.0", note = "Use `with_sampling_rate` instead.")]
pub fn sample<M, IC>(sampling_rate: Sampling, chain: IC) -> MetricOutput<M>
where
    M: Clone + Send + Sync + 'static,
    IC: Into<MetricOutput<M>>,
{
    let chain = chain.into();
    chain.with_sampling_rate(sampling_rate)
}

mod pcg32 {
    //! PCG32 random number generation for fast sampling

    // TODO use https://github.com/codahale/pcg instead?
    use std::cell::RefCell;
    use time;

    fn seed() -> u64 {
        let seed = 5573589319906701683_u64;
        let seed = seed.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
            .wrapping_add(time::precise_time_ns());
        seed.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
    }

    /// quickly return a random int
    fn pcg32_random() -> u32 {
        thread_local! {
            static PCG32_STATE: RefCell<u64> = RefCell::new(seed());
        }

        PCG32_STATE.with(|state| {
            let oldstate: u64 = *state.borrow();
            // XXX could generate the increment from the thread ID
            *state.borrow_mut() = oldstate
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((((oldstate >> 18) ^ oldstate) >> 27) as u32).rotate_right((oldstate >> 59) as u32)
        })
    }

    /// Convert a floating point sampling rate to an integer so that a fast integer RNG can be used
    /// Float rate range is between 1.0 (send 100% of the samples) and 0.0 (_no_ samples taken)
    /// .    | float rate | int rate | percentage
    /// ---- | ---------- | -------- | ----
    /// all  | 1.0        | 0x0      | 100%
    /// none | 0.0        | 0xFFFFFFFF | 0%
    pub fn to_int_rate(float_rate: f64) -> u32 {
        assert!(float_rate <= 1.0 && float_rate >= 0.0);
        ((1.0 - float_rate) * ::std::u32::MAX as f64) as u32
    }

    /// randomly select samples based on an int rate
    pub fn accept_sample(int_rate: u32) -> bool {
        pcg32_random() > int_rate
    }

}
