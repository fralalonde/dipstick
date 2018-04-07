//! Metrics name manipulation functions.
use core::*;

use std::sync::Arc;

const DEFAULT_SEPARATOR: &'static str = ".";

/// A list of parts of a metric's name.
#[derive(Debug, Clone)]
pub struct Namespace(Vec<String>);

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
    fn with_prefix(&self, prefix: &str) -> Self {
        self.with_namespace(&[prefix])
    }

    /// Join namespace and prepend in newly defined metrics.
    //    #[deprecated(since = "0.6.3", note = "Use `with_name` instead.")]
    fn with_namespace(&self, names: &[&str]) -> Self {
        self.with_name(names)
    }

    /// Join namespace and prepend in newly defined metrics.
    fn with_name<IN: Into<Namespace>>(&self, names: IN) -> Self;
}

/// Add a namespace decorator to a metric definition function.
pub fn add_namespace<M: 'static>(names: &Namespace, next: DefineMetricFn<M>) -> DefineMetricFn<M> {
    let nspace = names.join(DEFAULT_SEPARATOR);
    Arc::new(move |kind, name, rate| {
        let name = [nspace.as_ref(), name].join(DEFAULT_SEPARATOR);
        (next)(kind, name.as_ref(), rate)
    })
}
