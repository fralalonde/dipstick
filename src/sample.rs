//! Reduce the amount of data to process or transfer by statistically dropping some of it.

use core::*;
use pcg32;

use std::sync::Arc;

/// Apply statistical sampling to collected metrics data.
pub trait WithSamplingRate
where
    Self: Sized,
{
    /// Perform random sampling of values according to the specified rate.
    fn with_sampling_rate(&self, sampling_rate: Rate) -> Self;
}

impl<M: Send + Sync + 'static + Clone> WithSamplingRate for Chain<M> {
    fn with_sampling_rate(&self, sampling_rate: Rate) -> Self {
        let int_sampling_rate = pcg32::to_int_rate(sampling_rate);

        self.mod_both(|metric_fn, scope_fn| {
            (
                Arc::new(move |kind, name, rate| {
                    // TODO override only if FULL_SAMPLING else warn!()
                    if rate != FULL_SAMPLING_RATE {
                        info!(
                            "Metric {} will be downsampled again {}, {}",
                            name, rate, sampling_rate
                        );
                    }

                    let new_rate = sampling_rate * rate;
                    metric_fn(kind, name, new_rate)
                }),
                Arc::new(move |buffered| {
                    let next_scope = scope_fn(buffered);
                    ControlScopeFn::new(move |cmd| {
                        match cmd {
                            ScopeCmd::Write(metric, value) => {
                                if pcg32::accept_sample(int_sampling_rate) {
                                    next_scope.write(metric, value)
                                }
                            },
                            ScopeCmd::Flush => next_scope.flush()
                        }
                    })
                }),
            )
        })
    }
}

/// Perform random sampling of values according to the specified rate.
#[deprecated(since = "0.5.0", note = "Use `with_sampling_rate` instead.")]
pub fn sample<M, IC>(sampling_rate: Rate, chain: IC) -> Chain<M>
where
    M: Clone + Send + Sync + 'static,
    IC: Into<Chain<M>>,
{
    let chain = chain.into();
    chain.with_sampling_rate(sampling_rate)
}
