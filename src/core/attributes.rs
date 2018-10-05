use std::sync::Arc;
use std::collections::{HashMap};

use core::name::{NameParts, Name};

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

/// Attributes common to metric components.
/// Not all attributes used by all components.
#[derive(Debug, Clone, Default)]
pub struct Attributes {
    naming: NameParts,
    sampling: Option<Sampling>,
    buffering: Option<Buffering>,
}

/// This trait should not be exposed outside the crate.
pub trait WithAttributes: Clone {
    /// Return attributes of component.
    fn get_attributes(&self) -> &Attributes;

    /// Return attributes of component for mutation.
    // TODO replace with fields-in-traits if ever stabilized (https://github.com/nikomatsakis/fields-in-traits-rfc)
    fn mut_attributes(&mut self) -> &mut Attributes;

    /// Utility method. Clone a component and mutate its attributes at once.
    fn with_attributes<F: Fn(&mut Attributes)>(&self, edit: F) -> Self {
        let mut cloned = self.clone();
        (edit)(cloned.mut_attributes());
        cloned
    }
}

/// Name operations support.
pub trait Naming {
    /// Returns namespace of component.
    fn get_naming(&self) -> &NameParts;

    /// Join namespace and prepend in newly defined metrics.
    fn add_naming<S: Into<String>>(&self, name: S) -> Self;

    /// Append any name parts to the name's namespace.
    fn naming_append<S: Into<Name>>(&self, name: S) -> Name {
        name.into().append(self.get_naming().clone())
    }

    /// Prepend any name parts to the name's namespace.
    fn naming_prepend<S: Into<Name>>(&self, name: S) -> Name {
        name.into().prepend(self.get_naming().clone())
    }
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
    fn get_naming(&self) -> &NameParts {
        &self.get_attributes().naming
    }

    /// Adds a name part to any existing naming.
    /// Return a clone of the component with the updated naming.
    fn add_naming<S: Into<String>>(&self, name: S) -> Self {
        let name = name.into();
        self.with_attributes(|new_attr| new_attr.naming.push_back(name.clone()))
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
