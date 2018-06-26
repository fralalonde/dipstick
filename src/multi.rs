//! Dispatch metrics to multiple sinks.

use core::{Output, Scope, Name, AddPrefix, OutputDyn, Kind, Metric, WithAttributes, Attributes, Flush};
use error;
use std::sync::Arc;

/// Opens multiple scopes at a time from just as many outputs.
#[derive(Clone)]
pub struct MultiOutput {
    attributes: Attributes,
    outputs: Vec<Arc<OutputDyn + Send + Sync>>,
}

impl Output for MultiOutput {
    type SCOPE = Multi;

    fn open_scope(&self) -> Self::SCOPE {
        let scopes = self.outputs.iter().map(|out| out.open_scope_dyn()).collect();
        Multi {
            attributes: self.attributes.clone(),
            scopes,
        }
    }
}

impl MultiOutput {
    /// Create a new multi dispatcher with no outputs configured.
    pub fn new() -> Self {
        MultiOutput {
            attributes: Attributes::default(),
            outputs: vec![],
        }
    }

    /// Returns a clone of the dispatch with the new output added to the list.
    pub fn add_target<OUT: OutputDyn + Send + Sync + 'static>(&self, out: OUT) -> Self {
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
#[derive(Clone)]
pub struct Multi {
    attributes: Attributes,
    scopes: Vec<Arc<Scope + Send + Sync>>,
}

impl Multi {
    /// Create a new multi scope dispatcher with no scopes.
    pub fn new() -> Self {
        Multi {
            attributes: Attributes::default(),
            scopes: vec![],
        }
    }

    /// Create a new multi-output.
    pub fn output() -> MultiOutput {
        MultiOutput::new()
    }

    /// Returns a clone of the dispatch with the new output added to the list.
    pub fn add_target<IN: Scope + Send + Sync + 'static>(&self, scope: IN) -> Self {
        let mut cloned = self.clone();
        cloned.scopes.push(Arc::new(scope));
        cloned
    }
}

impl Scope for Multi {
    fn new_metric(&self, name: Name, kind: Kind) -> Metric {
        let ref name = self.qualified_name(name);
        let metrics: Vec<Metric> = self.scopes.iter()
            .map(move |scope| scope.new_metric(name.clone(), kind))
            .collect();
        Metric::new(move |value| for metric in &metrics {
            metric.write(value)
        })
    }
}

impl Flush for Multi {
    fn flush(&self) -> error::Result<()> {
        for w in &self.scopes {
            w.flush()?;
        }
        Ok(())
    }
}

impl WithAttributes for Multi {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}
