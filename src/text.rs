//! Standard stateless metric outputs.

// TODO parameterize templates
// TODO define backing structs that can flush() on Drop
use core::{Namespace, Value, WriteFn, Kind, MetricOutput, MetricInput, Flush};
use error;
use std::sync::{RwLock, Arc};
use std::io::{Write, BufWriter, self};

/// Unbuffered metrics text output.
pub struct TextOutput<W: Write> {
    inner: Arc<RwLock<W>>,
    format_fn: Arc<Fn(&Namespace, Kind) -> Vec<String> + Send + Sync>,
    print_fn: Arc<Fn(&mut W, &[String], Value) -> error::Result<()> + Send + Sync>,
}

// FIXME manual Clone impl required because auto-derive is borked (https://github.com/rust-lang/rust/issues/26925)
impl<W: Write> Clone for TextOutput<W> {
    fn clone(&self) -> Self {
        TextOutput {
            inner: self.inner.clone(),
            format_fn: self.format_fn.clone(),
            print_fn: self.print_fn.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> MetricOutput for TextOutput<W> {

    type Input = TextOutput<W>;

    fn open(&self) -> Self::Input {
        self.clone()
    }
}

impl<W: Write + Send + Sync + 'static> MetricInput for TextOutput<W> {
    fn define_metric(&self, name: &Namespace, kind: Kind) -> WriteFn {
        let template = (self.format_fn)(name, kind);
        let print_fn = self.print_fn.clone();
        let output = self.inner.clone();
        WriteFn::new(move |value| {
            let mut inner = output.write().expect("TextOutput");
            if let Err(err) = (print_fn)(&mut inner, &template, value) {
                debug!("{}", err)
            }
        })
    }
}

impl<W: Write> Flush for TextOutput<W> {
    fn flush(&self) -> error::Result<()> {
        let mut inner = self.inner.write().expect("TextOutput");
        Ok(inner.flush()?)
    }
}

/// Buffered metrics text output.
pub struct BufferedTextOutput<W: Write> {
    inner: Arc<RwLock<W>>,
    format_fn: Arc<Fn(&Namespace, Kind) -> Vec<String> + Send + Sync>,
    buffer_print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
//    flush_print_fn: Arc<Fn(&mut W, &mut [String]) -> error::Result<()> + Send + Sync>,
}

// FIXME manual Clone impl required because auto-derive is borked (https://github.com/rust-lang/rust/issues/26925)
impl<W: Write> Clone for BufferedTextOutput<W> {
    fn clone(&self) -> Self {
        BufferedTextOutput {
            inner: self.inner.clone(),
            format_fn: self.format_fn.clone(),
            buffer_print_fn: self.buffer_print_fn.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> MetricOutput for BufferedTextOutput<W> {

    type Input = BufferedTextInput<W>;

    fn open(&self) -> Self::Input {
        BufferedTextInput {
            entries: Arc::new(RwLock::new(Vec::new())),
            output: self.clone(),
        }
    }
}

#[derive(Clone)]
pub struct BufferedTextInput<W: Write> {
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    output: BufferedTextOutput<W>,
}

impl<W: Write + Send + Sync + 'static> MetricInput for BufferedTextInput<W> {
    fn define_metric(&self, name: &Namespace, kind: Kind) -> WriteFn {
        let template = (self.output.format_fn)(name, kind);
        let print_fn = self.output.buffer_print_fn.clone();
        let entries = self.entries.clone();

        WriteFn::new(move |value| {
            let mut buffer = Vec::with_capacity(32);
            match (print_fn)(&mut buffer, &template, value) {
                Ok(()) => {
                    let mut entries = entries.write().expect("TextOutput");
                    entries.push(buffer.into())
                },
                Err(err) => debug!("{}", err),
            }
        })
    }
}

impl<W: Write> Flush for BufferedTextInput<W> {
    fn flush(&self) -> error::Result<()> {
        let mut output = self.output.inner.write().expect("TextOutput");
        let entries = self.entries.write().expect("Metrics TextBuffer");
        for entry in entries.iter() {
            output.write_all(entry)?
        }
        Ok(())
    }
}


/// Write metric values to stdout using `println!`.
pub fn to_stdout() -> TextOutput<io::Stdout> {
    TextOutput {
        inner: Arc::new(RwLock::new(io::stdout())),
        format_fn: Arc::new(format_name),
        print_fn: Arc::new(print_name_value_line),
    }
}

pub fn format_name(name: &Namespace, _kind: Kind) -> Vec<String> {
    let mut z = name.join(".");
    z.push_str(" ");
    vec![z]
}

pub fn print_name_value_line(output: &mut impl Write, template: &[String], value: Value) -> error::Result<()> {
    write!(output, "{}", template[0])?;
    write!(output, "{}", value)?;
    writeln!(output)?;
    Ok(())
}


/// Record metric values to stdout using `println!`.
/// Values are buffered until #flush is called
/// Buffered operation requires locking.
/// If thread latency is a concern you may wish to also use #with_async_queue.
pub fn to_buffered_stdout() -> BufferedTextOutput<BufWriter<io::Stdout>> {
    BufferedTextOutput {
        inner: Arc::new(RwLock::new(BufWriter::new(io::stdout()))),
        format_fn: Arc::new(format_name),
        buffer_print_fn: Arc::new(print_name_value_line),
//        flush_print_fn: Arc::new(flush_buffer_raw),
    }
}

// log output

use log;

/// Unbuffered metrics log output.
#[derive(Clone)]
pub struct LogOutput {
    format_fn: Arc<Fn(&Namespace, Kind) -> Vec<String> + Send + Sync>,
    print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
}

impl MetricOutput for LogOutput {

    type Input = LogOutput;

    fn open(&self) -> Self::Input {
        self.clone()
    }
}

impl MetricInput for LogOutput {
    fn define_metric(&self, name: &Namespace, kind: Kind) -> WriteFn {
        let template = (self.format_fn)(name, kind);
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
        format_fn: Arc::new(format_name),
        print_fn: Arc::new(print_name_value_line),
    }
}


/// Buffered metrics log output.
#[derive(Clone)]
pub struct BufferedLogOutput {
    format_fn: Arc<Fn(&Namespace, Kind) -> Vec<String> + Send + Sync>,
    buffer_print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
}

impl MetricOutput for BufferedLogOutput {

    type Input = BufferedLogInput;

    fn open(&self) -> Self::Input {
        BufferedLogInput {
            entries: Arc::new(RwLock::new(Vec::new())),
            output: self.clone(),
        }
    }
}

#[derive(Clone)]
pub struct BufferedLogInput {
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    output: BufferedLogOutput,
}

impl MetricInput for BufferedLogInput {
    fn define_metric(&self, name: &Namespace, kind: Kind) -> WriteFn {
        let template = (self.output.format_fn)(name, kind);
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
        format_fn: Arc::new(format_name),
        buffer_print_fn: Arc::new(print_name_value_line),
    }
}

/// Discard metrics output.
#[derive(Clone)]
pub struct Void {}

impl MetricOutput for Void {

    type Input = Void;

    fn open(&self) -> Void {
        self.clone()
    }
}

impl MetricInput for Void {
    fn define_metric(&self, _name: &Namespace, _kind: Kind) -> WriteFn {
        WriteFn::new(|_value| {})
    }
}

impl Flush for Void {}

/// Discard all metric values sent to it.
pub fn to_void() -> Void {
    Void {}
}

#[cfg(test)]
mod test {
    use core::*;

    #[test]
    fn sink_print() {
        let c = super::to_stdout().open_scope();
        let m = c.define_metric(&"test".into(), Kind::Marker);
        (m)(33);
    }

    #[test]
    fn test_to_log() {
        let c = super::to_log().open_scope();
        let m = c.define_metric(&"test".into(), Kind::Marker);
        (m)(33);
    }

    #[test]
    fn test_to_void() {
        let c = super::to_void().open_scope();
        let m = c.define_metric(&"test".into(), Kind::Marker);
        (m)(33);
    }

}
