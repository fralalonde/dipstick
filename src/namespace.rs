//! Metrics name manipulation functions.
use core::*;

use std::sync::Arc;

const DEFAULT_SEPARATOR: &'static str = ".";

/// A list of parts of a metric's name.
#[derive(Debug, Clone)]
pub struct Namespace (Vec<String>);

impl Namespace {
    /// Make this namespace a subspace of the parent.
    pub fn subspace_of(self, parent: &Namespace) -> Self {
        Namespace([parent.0.clone(), self.0].concat())
    }

    /// Combine name parts into a string.
    pub fn join(&self, separator: &str) -> String {
        self.0.join(separator)
    }
}

impl<'a> From<&'a str> for Namespace {
    fn from(name: &'a str) -> Namespace {
        Namespace(vec![name.to_string()])
    }
}

impl From<String> for Namespace {
    fn from(name: String) -> Namespace {
        Namespace(vec![name])
    }
}

impl<'a, 'b: 'a> From<&'b [&'a str]> for Namespace {
    fn from(names: &'a [&'a str]) -> Namespace {
        Namespace(names.iter().map(|n| n.to_string()).collect())
    }
}

/// Prepend metric names with custom prefix.
pub trait WithNamespace
where
    Self: Sized,
{
    /// Insert prefix in newly defined metrics.
//    #[deprecated(since = "0.6.3", note = "Use `with_name` instead.")]
    fn with_prefix<AS: AsRef<str>>(&self, prefix: AS) -> Self {
        self.with_namespace(&[prefix.as_ref()])
    }

    /// Join namespace and prepend in newly defined metrics.
//    #[deprecated(since = "0.6.3", note = "Use `with_name` instead.")]
    fn with_namespace(&self, names: &[&str]) -> Self {
        self.with_name(names)
    }

    /// Join namespace and prepend in newly defined metrics.
    fn with_name<IN: Into<Namespace>>(&self, names: IN) -> Self;

}

impl<M: Send + Sync + Clone + 'static> WithNamespace for Chain<M> {
    fn with_name<IN: Into<Namespace>>(&self, names: IN) -> Self {
        let ninto = names.into();
        self.mod_metric(|next| {
            let nspace = ninto.join(DEFAULT_SEPARATOR);
            Arc::new(move |kind, name, rate| {
                let name = [nspace.as_ref(), name].join(DEFAULT_SEPARATOR);
                (next)(kind, name.as_ref(), rate)
            })
        })
    }
}

/// deprecated, use with_prefix() omitting any previously supplied separator
#[deprecated(since = "0.5.0",
             note = "Use `with_name` instead, omitting any previously supplied separator.")]
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
