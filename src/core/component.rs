use std::sync::Arc;
use std::collections::{HashMap};

use core::name::{Namespace, Name};

/// The actual distribution (random, fixed-cycled, etc) depends on selected sampling method.
#[derive(Debug, Clone, Copy)]
pub enum Sampling {
    /// Floating point sampling rate
    /// - 1.0+ records everything
    /// - 0.5 records one of two values
    /// - 0.0 records nothing
    Random(f64)
}

/// A metrics buffering strategy.
/// All strategies other than `Unbuffered` are applied as a best-effort, meaning that the buffer
/// may be flushed at any moment before reaching the limit, for any or no reason in particular.
#[derive(Debug, Clone, Copy)]
pub enum Buffering {
    /// A buffer of maximum specified size is used.
    BufferSize(usize),

    /// Buffer as much as possible.
    Unlimited,
}

/// One struct to rule them all.
/// Possible attributes of metric outputs and scopes.
/// Private trait used by impls of specific With* traits.
/// Not all attributes are used by all structs!
#[derive(Debug, Clone, Default)]
pub struct Attributes {
    namespace: Namespace,
    sampling: Option<Sampling>,
    buffering: Option<Buffering>,
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
pub trait Naming {
    /// Returns namespace of component.
    fn get_namespace(&self) -> &Namespace;

    /// Join namespace and prepend in newly defined metrics.
    fn namespace<S: Into<String>>(&self, name: S) -> Self;

    /// Append the specified name to the local namespace and return the concatenated result.
    fn qualify<S: Into<Name>>(&self, name: S) -> Name;
}

/// Name operations support.
pub trait Label {
    /// Return the namespace of the component.
    fn get_label(&self) -> &Arc<HashMap<String, String>>;

    /// Join namespace and prepend in newly defined metrics.
    fn label(&self, name: &str) -> Self;

}

impl<T: WithAttributes> Naming for T {

    /// Returns namespace of component.
    fn get_namespace(&self) -> &Namespace {
        &self.get_attributes().namespace
    }

    /// Join namespace and prepend in newly defined metrics.
    fn namespace<S: Into<String>>(&self, name: S) -> Self {
        let name = name.into();
        self.with_attributes(|new_attr| new_attr.namespace.push_back(name.clone()))
    }

    /// Append the specified name to the local namespace and return the concatenated result.
    fn qualify<S: Into<Name>>(&self, name: S) -> Name {
        name.into().append(self.get_attributes().namespace.clone())
    }
}

/// Apply statistical sampling to collected metrics data.
pub trait Sampled: WithAttributes {
    /// Perform random sampling of values according to the specified rate.
    fn sampled(&self, sampling: Sampling) -> Self {
        self.with_attributes(|new_attr| new_attr.sampling = Some(sampling))
    }

    /// Get the sampling strategy for this component, if any.
    fn get_sampling(&self) -> Option<Sampling> {
        self.get_attributes().sampling
    }
}

/// Determine scope buffering strategy, if supported by output.
/// Changing this only affects scopes opened afterwards.
/// Buffering is done on best effort, meaning flush will occur if buffer capacity is exceeded.
pub trait Buffered: WithAttributes {
    /// Return a clone with the specified buffering set.
    fn buffered(&self, buffering: Buffering) -> Self {
        self.with_attributes(|new_attr| new_attr.buffering = Some(buffering))
    }

    /// Return the buffering.
    fn get_buffering(&self) -> Option<Buffering> {
        self.get_attributes().buffering
    }
}
