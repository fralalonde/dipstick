use crate::name::MetricName;
use crate::Flush;

use crate::attributes::MetricId;
use crate::{Input, InputDyn, InputKind, InputMetric, InputScope};
use std::error::Error;
use std::sync::Arc;

lazy_static! {
    /// The reference instance identifying an uninitialized metric config.
    pub static ref VOID_INPUT: Arc<dyn InputDyn + Send + Sync> = Arc::new(Void::new());

    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<dyn InputScope + Send + Sync> = VOID_INPUT.input_dyn();
}

/// Discard metrics Input.
#[derive(Clone, Default)]
pub struct Void {}

/// Discard metrics Input.
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

impl Input for Void {
    type SCOPE = VoidInput;

    fn metrics(&self) -> Self::SCOPE {
        VoidInput {}
    }
}

impl InputScope for VoidInput {
    fn new_metric(&self, name: MetricName, _kind: InputKind) -> InputMetric {
        InputMetric::new(MetricId::forge("void", name), |_value, _labels| {})
    }
}

impl Flush for VoidInput {
    fn flush(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_to_void() {
        let c = Void::new().metrics();
        let m = c.new_metric("test".into(), InputKind::Marker);
        m.write(33, labels![]);
    }
}
