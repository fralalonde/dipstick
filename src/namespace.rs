//! Metrics name manipulation functions.
use core::*;

use std::sync::Arc;
use std::collections::HashMap;

const DEFAULT_SEPARATOR: &'static str = ".";

/// A namespace for metrics.
/// Does _not_ include the metric's "short" name itself.
/// Can be empty.
#[derive(Debug, Clone)]
pub struct Namespace {
    inner: Vec<String>
}

impl Namespace {

    pub fn split_first(&self) -> Option<(&String, &[String])> {
        self.inner.split_first()
    }

    pub fn with_suffix(&self, names: &Namespace) -> Self {
        Namespace { inner: self.inner.clone().extend(names) }
    }

    /// Combine name parts into a string.
    pub fn join(&self, separator: &str) -> String {
        self.inner.join(separator)
    }
}

impl<'a> From<&'a str> for Namespace {
    fn from(name: &'a str) -> Namespace {
        Namespace { inner: vec![name.to_string()] }
    }
}

impl From<String> for Namespace {
    fn from(name: String) -> Namespace {
        Namespace { inner: vec![name] }
    }
}

pub trait Registry {
    fn with_prefix(&self, prefix: &str) -> Self;


//    fn parent(&self) -> Option<&Registry>;
//
//    fn namespace(&self) -> &Namespace;
//
//    fn children(&mut self) -> &mut HashMap<String, T>;
//
//    fn create_children(parent: R, name: String) -> Self;

//    fn with_names(&mut self, namespace: Namespace) -> Self {
//
//        let namespace = &names.into();
//        let (first, rest) = namespace.split_first();
//        // recursively find or create children for every namespace component
//        first.map(|f| {
//            f.with_pre
//            Self::make_new(self.children().entry(*first)
//                    .or_insert_with(|| InnerDispatch::with_parent(Some(self.inner.clone())))
//                    .clone()
//            ).with_name(rest)
//        }).unwrap_or_else(self.clone())
//
//    }
}

///// Prepend metric names with custom prefix.
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
