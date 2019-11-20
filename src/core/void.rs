use std::error::Error;
use std::sync::Arc;

use crate::core::attributes::MetricId;
use crate::{Input, InputMetric};
use crate::core::input::{InputDyn, InputKind, InputScope};
use crate::core::name::MetricName;
use crate::core::output::{Output, OutputMetric, OutputScope};
use crate::core::{Flush, error};

lazy_static! {
    /// The reference instance identifying an uninitialized metric config.
    pub static ref VOID_INPUT: Arc<dyn InputDyn + Send + Sync> = Arc::new(Void::new());

    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<dyn InputScope + Send + Sync> = VOID_INPUT.input_dyn();
}

/// Discard metrics output.
#[derive(Clone, Default)]
pub struct Void {}

/// Discard metrics output.
#[derive(Clone)]
pub struct VoidOutput {}

/// Discard metrics output.
#[derive(Clone)]
pub struct VoidInput {}

impl Void {
    /// Void metrics builder.
    #[deprecated(since = "0.7.2", note = "Use new()")]
    pub fn metrics() -> Self {
        Self::new()
    }

    /// Void metrics builder.
    pub fn new() -> Self {
        Void {}
    }
}

impl Flush for VoidInput{
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }
}

impl Input for Void {
    type SCOPE = VoidInput;

    fn metrics(&self) -> Self::SCOPE {
        VoidInput{}
    }
}

impl InputScope for VoidInput {
    fn new_metric(&self, name: MetricName, _kind: InputKind) -> InputMetric {
       InputMetric::new(MetricId::forge("void", name), |_, _| {})
    }
}

impl Output for Void {
    type SCOPE = VoidOutput;

    fn new_scope(&self) -> Self::SCOPE {
        VoidOutput {}
    }
}

impl OutputScope for VoidOutput {
    fn new_metric(&self, name: MetricName, _kind: InputKind) -> OutputMetric {
        OutputMetric::new(MetricId::forge("void", name), |_value, _labels| {})
    }
}

impl Flush for VoidOutput {
    fn flush(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}
