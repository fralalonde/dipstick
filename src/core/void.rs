use core::input::{InputDyn, InputKind, InputScope};
use core::name::MetricName;
use core::output::{Output, OutputMetric, OutputScope};
use core::Flush;

use std::error::Error;
use std::sync::Arc;

lazy_static! {
    /// The reference instance identifying an uninitialized metric config.
    pub static ref VOID_INPUT: Arc<InputDyn + Send + Sync> = Arc::new(Void::new());

    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<InputScope + Send + Sync> = VOID_INPUT.input_dyn();
}

/// Discard metrics output.
#[derive(Clone, Default)]
pub struct Void {}

/// Discard metrics output.
#[derive(Clone)]
pub struct VoidOutput {}

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

impl Output for Void {
    type SCOPE = VoidOutput;

    fn new_scope(&self) -> Self::SCOPE {
        VoidOutput {}
    }
}

impl OutputScope for VoidOutput {
    fn new_metric(&self, _name: MetricName, _kind: InputKind) -> OutputMetric {
        OutputMetric::new(|_value, _labels| {})
    }
}

impl Flush for VoidOutput {
    fn flush(&self) -> Result<(), Box<Error + Send + Sync>> {
        Ok(())
    }
}
