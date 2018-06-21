use core::{Name, WithName, Value, Metric, Kind, Output, Input, WithAttributes, Attributes, WithBuffering};
use error;
use std::sync::{RwLock, Arc};
use text;
use std::io::Write;

use log;

/// Write metric values to the standard log using `info!`.
// TODO parameterize log level
pub fn output_log() -> LogOutput {
    LogOutput {
        attributes: Attributes::default(),
        format_fn: Arc::new(text::format_name),
        print_fn: Arc::new(text::print_name_value_line),
    }
}

/// Buffered metrics log output.
#[derive(Clone)]
pub struct LogOutput {
    attributes: Attributes,
    format_fn: Arc<Fn(&Name, Kind) -> Vec<String> + Send + Sync>,
    print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
}

impl Output for LogOutput {
    type INPUT = LogInput;

    fn new_input(&self) -> Self::INPUT {
        LogInput {
            attributes: self.attributes.clone(),
            entries: Arc::new(RwLock::new(Vec::new())),
            output: self.clone(),
        }
    }
}

impl WithAttributes for LogOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl WithBuffering for LogOutput {}

/// The scope-local input for buffered log metrics output.
#[derive(Clone)]
pub struct LogInput {
    attributes: Attributes,
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    output: LogOutput,
}

impl WithAttributes for LogInput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl WithBuffering for LogInput {}

impl Input for LogInput {
    fn new_metric(&self, name: Name, kind: Kind) -> Metric {
        let name = self.qualified_name(name);
        let template = (self.output.format_fn)(&name, kind);

        let print_fn = self.output.print_fn.clone();
        let entries = self.entries.clone();

        if self.is_buffering() {
            Metric::new(move |value| {
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
            Metric::new(move |value| {
                let mut buffer = Vec::with_capacity(32);
                match (print_fn)(&mut buffer, &template, value) {
                    Ok(()) => log!(log::Level::Debug, "{:?}", &buffer),
                    Err(err) => debug!("Could not format buffered log metric: {}", err),
                }
            })
        }
    }

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

impl Drop for LogInput {
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
        let c = super::output_log().new_input_dyn();
        let m = c.new_metric("test".into(), Kind::Marker);
        m.write(33);
    }

}
