//! Metrics name manipulation functions.
use core::*;

use std::sync::Arc;

/// Prepend metric names with custom prefix.
pub trait WithNamespace
where
    Self: Sized,
{
    /// Insert prefix in newly defined metrics.
    fn with_prefix(&self, prefix: &str) -> Self {
        self.with_namespace(&[prefix])
    }

    /// Join namespace and prepend in newly defined metrics.
    fn with_namespace(&self, names: &[&str]) -> Self;
}

impl<M: Send + Sync + Clone + 'static> WithNamespace for Chain<M> {
    fn with_namespace(&self, names: &[&str]) -> Self {
        self.mod_metric(|next| {
            let nspace = names.join(".");
            Arc::new(move |kind, name, rate| {
                let name = [nspace.as_ref(), name].join(".");
                (next)(kind, name.as_ref(), rate)
            })
        })
    }
}

/// deprecated, use with_prefix() omitting any previously supplied separator
#[deprecated(since = "0.5.0",
             note = "Use `with_prefix` instead, omitting any previously supplied separator.")]
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
