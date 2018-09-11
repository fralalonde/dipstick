use core::{Name, AddPrefix, Value, InputMetric, Kind, Input, InputScope, WithAttributes, Attributes,
           Buffered, Flush};
use error;
use std::sync::{RwLock, Arc};
use text;
use std::io::Write;

use log;

/// Buffered metrics log output.
#[derive(Clone)]
pub struct Log {
    attributes: Attributes,
    format_fn: Arc<Fn(&Name, Kind) -> Vec<String> + Send + Sync>,
    print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
}

impl Input for Log {
    type SCOPE = LogScope;

    fn input(&self) -> Self::SCOPE {
        LogScope {
            attributes: self.attributes.clone(),
            entries: Arc::new(RwLock::new(Vec::new())),
            output: self.clone(),
        }
    }
}

impl WithAttributes for Log {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Buffered for Log {}

/// A scope for metrics log output.
#[derive(Clone)]
pub struct LogScope {
    attributes: Attributes,
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    output: Log,
}

impl Log {
    /// Write metric values to the standard log using `info!`.
    // TODO parameterize log level, logger
    pub fn log_to() -> Log {
        Log {
            attributes: Attributes::default(),
            format_fn: Arc::new(text::format_name),
            print_fn: Arc::new(text::print_name_value_line),
        }
    }
}

impl WithAttributes for LogScope {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Buffered for LogScope {}

use queue_in;
use cache_in;

impl queue_in::QueuedInput for Log {}
impl cache_in::CachedInput for Log {}

impl InputScope for LogScope {
    fn new_metric(&self, name: Name, kind: Kind) -> InputMetric {
        let name = self.qualified_name(name);
        let template = (self.output.format_fn)(&name, kind);

        let print_fn = self.output.print_fn.clone();
        let entries = self.entries.clone();

        if self.is_buffered() {
            InputMetric::new(move |value| {
                let mut buffer = Vec::with_capacity(32);
                match (print_fn)(&mut buffer, &template, value) {
                    Ok(()) => {
                        let mut entries = entries.write().expect("TextOutput");
                        entries.push(buffer.into())
                    },
                    Err(err) => debug!("Could not format buffered log metric: {}", err),
                }
            })
        } else {
            InputMetric::new(move |value| {
                let mut buffer = Vec::with_capacity(32);
                match (print_fn)(&mut buffer, &template, value) {
                    Ok(()) => log!(log::Level::Debug, "{:?}", &buffer),
                    Err(err) => debug!("Could not format buffered log metric: {}", err),
                }
            })
        }
    }
}

impl Flush for LogScope {

    fn flush(&self) -> error::Result<()> {
        let mut entries = self.entries.write().expect("Metrics TextBuffer");
        if !entries.is_empty() {
            let mut buf: Vec<u8> = Vec::with_capacity(32 * entries.len());
            for entry in entries.drain(..) {
                writeln!(&mut buf, "{:?}", &entry)?;
            }
            log!(log::Level::Debug, "{:?}", &buf);
        }
        Ok(())
    }
}

impl Drop for LogScope {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            warn!("Could not flush log metrics on Drop. {}", e)
        }
    }
}

#[cfg(test)]
mod test {
    use core::*;

    #[test]
    fn test_to_log() {
        let c = super::Log::log_to().input();
        let m = c.new_metric("test".into(), Kind::Marker);
        m.write(33);
    }

}
