use std::sync::Arc;
use std::ops;
use std::collections::HashMap;

/// The actual distribution (random, fixed-cycled, etc) depends on selected sampling method.
#[derive(Debug, Clone, Copy)]
pub enum Sampling {
    /// Do not sample, use all data.
    Full,

    /// Floating point sampling rate
    /// - 1.0+ records everything
    /// - 0.5 records one of two values
    /// - 0.0 records nothing
    Random(f64)
}

impl Default for Sampling {
    fn default() -> Self {
        Sampling::Full
    }
}

/// A metrics buffering strategy.
/// All strategies other than `Unbuffered` are applied as a best-effort, meaning that the buffer
/// may be flushed at any moment before reaching the limit, for any or no reason in particular.
#[derive(Debug, Clone, Copy)]
pub enum Buffering {
    /// No buffering is performed (default).
    Unbuffered,

    /// A buffer of maximum specified size is used.
    BufferSize(usize),

    /// Buffer as much as possible.
    Unlimited,
}

impl Default for Buffering {
    fn default() -> Self {
        Buffering::Unbuffered
    }
}

/// One struct to rule them all.
/// Possible attributes of metric outputs and scopes.
/// Private trait used by impls of specific With* traits.
/// Not all attributes are used by all structs!
/// This is a design choice to centralize code at the expense of slight waste of memory.
/// Fields have also not been made `pub` to make it easy to change this mechanism.
/// Field access must go through `is_` and `get_` methods declared in sub-traits.
#[derive(Debug, Clone, Default)]
pub struct Attributes {
    namespace: Name,
    sampling: Sampling,
    buffering: Buffering,
}

/// The only trait that requires concrete impl by metric components.
/// Default impl of actual attributes use this to clone & mutate the original component.
/// This trait is _not_ exposed by the lib.
pub trait WithAttributes: Clone {
    /// Return attributes for evaluation.
    // TODO replace with fields-in-traits if ever stabilized (https://github.com/nikomatsakis/fields-in-traits-rfc)
    fn get_attributes(&self) -> &Attributes;

    /// Return attributes of component to be mutated after cloning.
    // TODO replace with fields-in-traits if ever stabilized (https://github.com/nikomatsakis/fields-in-traits-rfc)
    fn mut_attributes(&mut self) -> &mut Attributes;

    /// Clone this component and its attributes before returning it.
    /// This means one of the attributes will be cloned only to be replaced immediately.
    /// But the benefits of a generic solution means we can live with that for a while.
    fn with_attributes<F: Fn(&mut Attributes)>(&self, edit: F) -> Self {
        let mut cloned = self.clone();
        (edit)(cloned.mut_attributes());
        cloned
    }
}

/// Name operations support.
pub trait AddPrefix {
    /// Return the namespace of the component.
    fn get_namespace(&self) -> &Name;

    /// Join namespace and prepend in newly defined metrics.
    fn add_prefix(&self, name: &str) -> Self;

    /// Append the specified name to the local namespace and return the concatenated result.
    fn qualified_name(&self, metric_name: Name) -> Name;
}

/// Name operations support.
pub trait Label {
    /// Return the namespace of the component.
    fn get_label(&self) -> &Arc<HashMap<String, String>>;

    /// Join namespace and prepend in newly defined metrics.
    fn label(&self, name: &str) -> Self;

}

impl<T: WithAttributes> AddPrefix for T {
    fn get_namespace(&self) -> &Name {
        &self.get_attributes().namespace
    }

    /// Join namespace and prepend in newly defined metrics.
    fn add_prefix(&self, name: &str) -> Self {
        self.with_attributes(|new_attr| new_attr.namespace = new_attr.namespace.concat(name))
    }

    /// Append the specified name to the local namespace and return the concatenated result.
    fn qualified_name(&self, name: Name) -> Name {
        // FIXME (perf) store name in reverse to prepend with an actual push() to the vec
        self.get_attributes().namespace.concat(name)
    }
}

/// Apply statistical sampling to collected metrics data.
pub trait Sampled: WithAttributes {
    /// Perform random sampling of values according to the specified rate.
    fn sampled(&self, sampling: Sampling) -> Self {
        self.with_attributes(|new_attr| new_attr.sampling = sampling)
    }

    /// Get the sampling strategy for this component, if any.
    fn get_sampling(&self) -> Sampling {
        self.get_attributes().sampling
    }
}

/// Determine scope buffering strategy, if supported by output.
/// Changing this only affects scopes opened afterwards.
/// Buffering is done on best effort, meaning flush will occur if buffer capacity is exceeded.
pub trait Buffered: WithAttributes {
    /// Return a clone with the specified buffering set.
    fn buffered(&self, buffering: Buffering) -> Self {
        self.with_attributes(|new_attr| new_attr.buffering = buffering)
    }

    /// Is this component using buffering?
    fn is_buffered(&self) -> bool {
        match self.get_buffering() {
            Buffering::Unbuffered => false,
            _ => true
        }
    }

    /// Return the buffering.
    fn get_buffering(&self) -> Buffering {
        self.get_attributes().buffering
    }
}

/// A namespace for metrics.
/// Does _not_ include the metric's "short" name itself.
/// Can be empty.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct Name {
    inner: Vec<String>,
}

impl Name {

    /// Concatenate with another namespace into a new one.
    pub fn concat(&self, name: impl Into<Name>) -> Self {
        let mut cloned = self.clone();
        cloned.inner.extend_from_slice(&name.into().inner);
        cloned
    }

    /// Returns true if the specified namespace is a subset or is equal to this namespace.
    pub fn starts_with(&self, name: &Name) -> bool {
        (self.inner.len() >= name.inner.len()) && (name.inner[..] == self.inner[..name.inner.len()])
    }

    /// Combine name parts into a string.
    pub fn join(&self, separator: &str) -> String {

        let mut buf = String::with_capacity(64);
        let mut i = self.inner.iter();
        if let Some(n) = i.next() {
            buf.push_str(n.as_ref());
        } else {
            return "".into()
        }
        for n in i {
            buf.push_str(separator);
            buf.push_str(n.as_ref());
        }
        buf
    }
}

impl ops::Deref for Name {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ops::DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<S: Into<String>> From<S> for Name {
    fn from(name: S) -> Name {
        let name: String = name.into();
        if name.is_empty() {
            Name::default()
        } else {
            Name { inner: vec![name] }
        }
    }
}

