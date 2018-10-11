//! Dispatch metrics to multiple sinks.

use core::Flush;
use core::attributes::{Attributes, WithAttributes, Naming};
use core::name::Name;
use core::input::Kind;
use core::output::{Output, OutputMetric, OutputScope, OutputDyn};
use core::error;

use std::rc::Rc;
use std::sync::Arc;

/// Opens multiple scopes at a time from just as many outputs.
#[derive(Clone)]
pub struct MultiOutput {
    attributes: Attributes,
    outputs: Vec<Arc<OutputDyn + Send + Sync + 'static>>,
}

impl Output for MultiOutput {
    type SCOPE = MultiOutputScope;

    fn output(&self) -> Self::SCOPE {
        let scopes = self.outputs.iter().map(|out| out.output_dyn()).collect();
        MultiOutputScope {
            attributes: self.attributes.clone(),
            scopes,
        }
    }
}

impl MultiOutput {
    /// Create a new multi-output.
    pub fn output() -> MultiOutput {
        MultiOutput {
            attributes: Attributes::default(),
            outputs: vec![],
        }
    }

    /// Returns a clone of the dispatch with the new output added to the list.
    pub fn add_target<OUT: Output + Send + Sync + 'static>(&self, out: OUT) -> Self {
        let mut cloned = self.clone();
        cloned.outputs.push(Arc::new(out));
        cloned
    }
}

impl WithAttributes for MultiOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// Dispatch metric values to a list of scopes.
#[derive(Clone, Default)]
pub struct MultiOutputScope {
    attributes: Attributes,
    scopes: Vec<Rc<OutputScope>>,
}

impl MultiOutputScope {
    /// Create a new multi scope dispatcher with no scopes.
    pub fn new() -> Self {
        MultiOutputScope {
            attributes: Attributes::default(),
            scopes: vec![],
        }
    }

    /// Returns a clone of the dispatch with the new output added to the list.
    pub fn add_target<IN: OutputScope + 'static>(&self, scope: IN) -> Self {
        let mut cloned = self.clone();
        cloned.scopes.push(Rc::new(scope));
        cloned
    }
}

impl OutputScope for MultiOutputScope {
    fn new_metric(&self, name: Name, kind: Kind) -> OutputMetric {
        let name = &self.naming_append(name);
        let metrics: Vec<OutputMetric> = self.scopes.iter()
            .map(move |scope| scope.new_metric(name.clone(), kind))
            .collect();
        OutputMetric::new(move |value, labels| for metric in &metrics {
            metric.write(value, labels.clone())
        })
    }
}

impl Flush for MultiOutputScope {
    fn flush(&self) -> error::Result<()> {
        for w in &self.scopes {
            w.flush()?;
        }
        Ok(())
    }
}

impl WithAttributes for MultiOutputScope {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}
