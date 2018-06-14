//! Dispatch metrics to multiple sinks.

use core::{MetricOutput, MetricInput, Namespace, WithPrefix, OpenScope, Kind, WriteFn, Flush, WithAttributes, Attributes};
use error;
use std::sync::Arc;

/// Opens multiple input scopes at a time from just as many outputs.
#[derive(Clone)]
pub struct MultiOutput {
    attributes: Attributes,
    outputs: Vec<Arc<OpenScope + Send + Sync>>,
}

impl MetricOutput for MultiOutput {
    type Input = MultiInput;

    fn open(&self) -> Self::Input {
        let inputs = self.outputs.iter().map(|out| out.open_scope()).collect();
        MultiInput {
            attributes: self.attributes.clone(),
            inputs,
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
    pub fn with_output<O: OpenScope + Send + Sync + 'static>(&self, out: O) -> Self {
        let mut cloned = self.clone();
        cloned.outputs.push(Arc::new(out));
        cloned
    }
}

impl WithAttributes for MultiOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// Dispatch metric values to a list of inputs.
#[derive(Clone)]
pub struct MultiInput {
    attributes: Attributes,
    inputs: Vec<Arc<MetricInput + Send + Sync>>,
}

impl MetricInput for MultiInput {
    fn define_metric(&self, name: &Namespace, kind: Kind) -> WriteFn {
        let name = self.qualified_name(name);
        let write_fns: Vec<WriteFn> = self.inputs.iter().map(|input| input.define_metric(&name, kind)).collect();
        WriteFn::new(move |value| for w in &write_fns {
            (w)(value)
        })
    }
}

impl Flush for MultiInput {
    fn flush(&self) -> error::Result<()> {
        for w in &self.inputs {
            w.flush()?;
        }
        Ok(())
    }
}

impl WithAttributes for MultiInput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}
