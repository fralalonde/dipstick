use core::{Namespace, WithPrefix, Value, WriteFn, Kind, MetricOutput, MetricInput, Flush, WithAttributes, Attributes};
use error;
use std::sync::{RwLock, Arc};
use text;
use std::io::{Write, BufWriter, self};

use log;

/// Unbuffered metrics log output.
#[derive(Clone)]
pub struct LogOutput {
    attributes: Attributes,
    format_fn: Arc<Fn(&Namespace, Kind) -> Vec<String> + Send + Sync>,
    print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
}

impl MetricOutput for LogOutput {
    type Input = LogOutput;

    fn open(&self) -> Self::Input {
        self.clone()
    }
}

impl WithAttributes for LogOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl MetricInput for LogOutput {
    fn define_metric(&self, name: &Namespace, kind: Kind) -> WriteFn {
        let name = self.qualified_name(name);
        let template = (self.format_fn)(&name, kind);

        let print_fn = self.print_fn.clone();
        WriteFn::new(move |value| {
            let mut buf: Vec<u8> = Vec::with_capacity(32);
            match (print_fn)(&mut buf, &template, value) {
                Ok(()) => log!(log::Level::Debug, "{:?}", &buf),
                Err(err) => debug!("{}", err),
            }

        })
    }
}

impl Flush for LogOutput {}

/// Write metric values to the standard log using `info!`.
// TODO parameterize log level
pub fn to_log() -> LogOutput {
    LogOutput {
        attributes: Attributes::default(),
        format_fn: Arc::new(text::format_name),
        print_fn: Arc::new(text::print_name_value_line),
    }
}


/// Buffered metrics log output.
#[derive(Clone)]
pub struct BufferedLogOutput {
    attributes: Attributes,
    format_fn: Arc<Fn(&Namespace, Kind) -> Vec<String> + Send + Sync>,
    buffer_print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
}

impl MetricOutput for BufferedLogOutput {
    type Input = BufferedLogInput;

    fn open(&self) -> Self::Input {
        BufferedLogInput {
            attributes: self.attributes.clone(),
            entries: Arc::new(RwLock::new(Vec::new())),
            output: self.clone(),
        }
    }
}

impl WithAttributes for BufferedLogOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// The scope-local input for buffered log metrics output.
#[derive(Clone)]
pub struct BufferedLogInput {
    attributes: Attributes,
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    output: BufferedLogOutput,
}

impl WithAttributes for BufferedLogInput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl MetricInput for BufferedLogInput {
    fn define_metric(&self, name: &Namespace, kind: Kind) -> WriteFn {
        let name = self.qualified_name(name);
        let template = (self.output.format_fn)(&name, kind);

        let print_fn = self.output.buffer_print_fn.clone();
        let entries = self.entries.clone();

        WriteFn::new(move |value| {
            let mut buffer = Vec::with_capacity(32);
            match (print_fn)(&mut buffer, &template, value) {
                Ok(()) => {
                    let mut entries = entries.write().expect("TextOutput");
                    entries.push(buffer.into())
                },
                Err(err) => debug!("Could not format buffered log metric: {}", err),
            }

        })
    }
}

impl Flush for BufferedLogInput {
    fn flush(&self) -> error::Result<()> {
        let mut entries = self.entries.write().expect("Metrics TextBuffer");
        let mut buf: Vec<u8> = Vec::with_capacity(32 * entries.len());
        for entry in entries.drain(..) {
            writeln!(&mut buf, "{:?}", &entry)?;
        }
        log!(log::Level::Debug, "{:?}", &buf);
        Ok(())
    }
}

/// Record metric values to the standard log using `info!`.
/// Values are buffered until #flush is called
/// Buffered operation requires locking.
/// If thread latency is a concern you may wish to also use #with_async_queue.
// TODO parameterize log level
pub fn to_buffered_log() -> BufferedLogOutput {
    BufferedLogOutput {
        attributes: Attributes::default(),
        format_fn: Arc::new(text::format_name),
        buffer_print_fn: Arc::new(text::print_name_value_line),
    }
}

#[cfg(test)]
mod test {
    use core::*;

    #[test]
    fn test_to_log() {
        let c = super::to_log().open_scope();
        let m = c.define_metric(&"test".into(), Kind::Marker);
        (m)(33);
    }

}
