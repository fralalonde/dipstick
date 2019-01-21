use core::output::{Output, OutputScope, OutputMetric};
use core::name::MetricName;
use core::input::{InputKind, InputDyn, InputScope};
use core::Flush;

use std::sync::Arc;

lazy_static! {
    /// The reference instance identifying an uninitialized metric config.
    pub static ref VOID_INPUT: Arc<InputDyn + Send + Sync> = Arc::new(Void::metrics());

    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<InputScope + Send + Sync> = VOID_INPUT.input_dyn();
}

/// Discard metrics output.
#[derive(Clone)]
pub struct Void {}

/// Discard metrics output.
#[derive(Clone)]
pub struct VoidInput {}

/// Discard metrics output.
#[derive(Clone)]
pub struct VoidOutput {}

impl Void {
    /// Void metrics builder.
    pub fn metrics() -> Self {
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
}
