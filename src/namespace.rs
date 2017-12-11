//! Metrics name manipulation functions.
//!
use core::*;
use std::sync::Arc;

/// Insert prefix in newly defined metrics.
///
pub fn prefix<M, IC>(prefix: &str, chain: IC) -> Chain<M>
where
    M: Clone + Send + Sync + 'static,
    IC: Into<Chain<M>>,
{
    let chain = chain.into();
    chain.mod_metric(|next| {
        let prefix = prefix.to_string();
        Arc::new(move |kind, name, rate| {
            let name = [&prefix, name].concat();
            (next)(kind, name.as_ref(), rate)
        })
    })
}

