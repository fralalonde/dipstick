//! Dispatch metrics to multiple sinks.

use core::{Output, Input, Name, WithName, OutputDyn, Kind, Metric, WithAttributes, Attributes};
use error;
use std::sync::Arc;

/// Opens multiple input scopes at a time from just as many outputs.
#[derive(Clone)]
pub struct MultiOutput {
    attributes: Attributes,
    outputs: Vec<Arc<OutputDyn + Send + Sync>>,
}

/// Create a new multi-output.
pub fn output_multi() -> MultiOutput {
    MultiOutput::new()
}

impl Output for MultiOutput {
    type INPUT = MultiInput;

    fn new_input(&self) -> Self::INPUT {
        let inputs = self.outputs.iter().map(|out| out.new_input_dyn()).collect();
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
    pub fn with_output<OUT: OutputDyn + Send + Sync + 'static>(&self, out: OUT) -> Self {
        let mut cloned = self.clone();
        cloned.outputs.push(Arc::new(out));
        cloned
    }
}

impl WithAttributes for MultiOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// Create a new multi-output.
pub fn input_multi() -> MultiInput {
    MultiInput::new()
}

/// Dispatch metric values to a list of inputs.
#[derive(Clone)]
pub struct MultiInput {
    attributes: Attributes,
    inputs: Vec<Arc<Input + Send + Sync>>,
}

impl MultiInput {
    /// Create a new multi input dispatcher with no inputs configured.
    pub fn new() -> Self {
        MultiInput {
            attributes: Attributes::default(),
            inputs: vec![],
        }
    }

    /// Returns a clone of the dispatch with the new output added to the list.
    pub fn with_input<IN: Input + Send + Sync + 'static>(&self, input: IN) -> Self {
        let mut cloned = self.clone();
        cloned.inputs.push(Arc::new(input));
        cloned
    }
}

impl Input for MultiInput {
    fn new_metric(&self, name: Name, kind: Kind) -> Metric {
        let ref name = self.qualified_name(name);
        let metrics: Vec<Metric> = self.inputs.iter()
            .map(move |input| input.new_metric(name.clone(), kind))
            .collect();
        Metric::new(move |value| for metric in &metrics {
            metric.write(value)
        })
    }

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
