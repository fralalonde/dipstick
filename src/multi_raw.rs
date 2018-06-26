//! Dispatch metrics to multiple sinks.

use core::{RawOutput, RawScope, Name, AddPrefix, RawOutputDyn, Kind, RawMetric, WithAttributes, Attributes, Flush};
use error;
use std::rc::Rc;
use std::sync::Arc;

/// Opens multiple scopes at a time from just as many outputs.
#[derive(Clone)]
pub struct MultiRawOutput {
    attributes: Attributes,
    outputs: Vec<Arc<RawOutputDyn + Send + Sync + 'static>>,
}

impl RawOutput for MultiRawOutput {
    type SCOPE = MultiRaw;

    fn open_scope_raw(&self) -> Self::SCOPE {
        let scopes = self.outputs.iter().map(|out| out.open_scope_raw_dyn()).collect();
        MultiRaw {
            attributes: self.attributes.clone(),
            scopes,
        }
    }
}

impl MultiRawOutput {

    /// Returns a clone of the dispatch with the new output added to the list.
    pub fn add_raw_target<OUT: RawOutputDyn + Send + Sync + 'static>(&self, out: OUT) -> Self {
        let mut cloned = self.clone();
        cloned.outputs.push(Arc::new(out));
        cloned
    }
}

impl WithAttributes for MultiRawOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// Dispatch metric values to a list of scopes.
#[derive(Clone)]
pub struct MultiRaw {
    attributes: Attributes,
    scopes: Vec<Rc<RawScope>>,
}

impl MultiRaw {
    /// Create a new multi scope dispatcher with no scopes.
    pub fn new() -> Self {
        MultiRaw {
            attributes: Attributes::default(),
            scopes: vec![],
        }
    }

    /// Create a new multi-output.
    pub fn output() -> MultiRawOutput {
        MultiRawOutput {
            attributes: Attributes::default(),
            outputs: vec![],
        }
    }

    /// Returns a clone of the dispatch with the new output added to the list.
    pub fn add_raw_target<IN: RawScope + 'static>(&self, scope: IN) -> Self {
        let mut cloned = self.clone();
        cloned.scopes.push(Rc::new(scope));
        cloned
    }
}

impl RawScope for MultiRaw {
    fn new_metric_raw(&self, name: Name, kind: Kind) -> RawMetric {
        let ref name = self.qualified_name(name);
        let metrics: Vec<RawMetric> = self.scopes.iter()
            .map(move |scope| scope.new_metric_raw(name.clone(), kind))
            .collect();
        RawMetric::new(move |value| for metric in &metrics {
            metric.write(value)
        })
    }
}

impl Flush for MultiRaw {
    fn flush(&self) -> error::Result<()> {
        for w in &self.scopes {
            w.flush()?;
        }
        Ok(())
    }
}

impl WithAttributes for MultiRaw {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}
