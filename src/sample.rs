//! Reduce the amount of data to process or transfer by statistically dropping some of it.

use core::*;
use pcg32;
use std::sync::Arc;

/// The metric sampling key also holds the sampling rate to apply to it.
#[derive(Debug, Clone)]
pub struct Sample<M> {
    target: M,
    int_sampling_rate: u32,
}

/// Perform random sampling of values according to the specified rate.
pub fn sample<M, IC>(sampling_rate: Rate, chain: IC) -> Chain<Sample<M>>
    where
        M: Clone + Send + Sync + 'static,
        IC: Into<Chain<M>>,
{
    let chain = chain.into();
    chain.mod_both(|metric_fn, scope_fn|
        (Arc::new(move |kind, name, rate| {
            // TODO override only if FULL_SAMPLING else warn!()
            if rate != FULL_SAMPLING_RATE {
                info!("Metric {} will be downsampled again {}, {}", name, rate, sampling_rate);
            }

            let new_rate = sampling_rate * rate;
            Sample {
                target: metric_fn(kind, name, new_rate),
                int_sampling_rate: pcg32::to_int_rate(new_rate),
            }
        }),
        Arc::new(move |auto_flush| {
            let next_scope = scope_fn(auto_flush);
            Arc::new(move |cmd| {
                if let ScopeCmd::Write(metric, value) = cmd {
                   if pcg32::accept_sample(metric.int_sampling_rate) {
                       next_scope(ScopeCmd::Write(&metric.target, value))
                   }
                }
                next_scope(ScopeCmd::Flush)
            })
        }))
    )
}


