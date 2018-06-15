//! Standard stateless metric outputs.

// TODO parameterize templates
// TODO define backing structs that can flush() on Drop
use core::{Name, WithName, Value, WriteFn, Kind, Output, Input, Flush, WithAttributes, Attributes};
use error;
use std::sync::{RwLock, Arc};
use std::io::{Write, BufWriter, self};

/// Unbuffered metrics text output.
// FIXME #[derive(Clone)] - manual impl - see below
pub struct TextOutput<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    inner: Arc<RwLock<W>>,
    format_fn: Arc<Fn(&Name, Kind) -> Vec<String> + Send + Sync>,
    print_fn: Arc<Fn(&mut W, &[String], Value) -> error::Result<()> + Send + Sync>,
}

// FIXME manual Clone impl required because auto-derive is borked (https://github.com/rust-lang/rust/issues/26925)
impl<W: Write + Send + Sync + 'static> Clone for TextOutput<W> {
    fn clone(&self) -> Self {
        TextOutput {
            attributes: self.attributes.clone(),
            inner: self.inner.clone(),
            format_fn: self.format_fn.clone(),
            print_fn: self.print_fn.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> WithAttributes for TextOutput<W> {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl<W: Write + Send + Sync + 'static> Output for TextOutput<W> {
    type Input = TextOutput<W>;
    fn new_input(&self) -> Self::Input {
        self.clone()
    }
}

impl<W: Write + Send + Sync + 'static> Input for TextOutput<W> {
    fn new_metric(&self, name: Name, kind: Kind) -> WriteFn {
        let name = self.qualified_name(name);
        let template = (self.format_fn)(&name, kind);
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

impl<W: Write + Send + Sync + 'static> Flush for TextOutput<W> {
    fn flush(&self) -> error::Result<()> {
        let mut inner = self.inner.write().expect("TextOutput");
        Ok(inner.flush()?)
    }
}

/// Buffered metrics text output.
pub struct BufferedTextOutput<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    inner: Arc<RwLock<W>>,
    format_fn: Arc<Fn(&Name, Kind) -> Vec<String> + Send + Sync>,
    buffer_print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
//    flush_print_fn: Arc<Fn(&mut W, &mut [String]) -> error::Result<()> + Send + Sync>,
}

// FIXME manual Clone impl required because auto-derive is borked (https://github.com/rust-lang/rust/issues/26925)
impl<W: Write + Send + Sync + 'static> Clone for BufferedTextOutput<W> {
    fn clone(&self) -> Self {
        BufferedTextOutput {
            attributes: self.attributes.clone(),
            inner: self.inner.clone(),
            format_fn: self.format_fn.clone(),
            buffer_print_fn: self.buffer_print_fn.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> WithAttributes for BufferedTextOutput<W> {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl<W: Write + Send + Sync + 'static> Output for BufferedTextOutput<W> {

    type Input = BufferedTextInput<W>;

    fn new_input(&self) -> Self::Input {
        BufferedTextInput {
            attributes: self.attributes.clone(),
            entries: Arc::new(RwLock::new(Vec::new())),
            output: self.clone(),
        }
    }
}

/// The scope-local input for buffered text metrics output.
pub struct BufferedTextInput<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    output: BufferedTextOutput<W>,
}

impl<W: Write + Send + Sync + 'static> Clone for BufferedTextInput<W> {
    fn clone(&self) -> Self {
        BufferedTextInput {
            attributes: self.attributes.clone(),
            entries: self.entries.clone(),
            output: self.output.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> WithAttributes for BufferedTextInput<W> {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl<W: Write + Send + Sync + 'static> Input for BufferedTextInput<W> {
    fn new_metric(&self, name: Name, kind: Kind) -> WriteFn {
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
                Err(err) => debug!("{}", err),
            }
        })
    }
}

impl<W: Write + Send + Sync + 'static> Flush for BufferedTextInput<W> {
    fn flush(&self) -> error::Result<()> {
        let mut output = self.output.inner.write().expect("TextOutput");
        let mut entries = self.entries.write().expect("Metrics TextBuffer");
        for entry in entries.drain(..) {
            output.write_all(&entry)?
        }
        Ok(())
    }
}


/// Write metric values to stdout using `println!`.
pub fn to_stdout() -> TextOutput<io::Stdout> {
    TextOutput {
        attributes: Attributes::default(),
        inner: Arc::new(RwLock::new(io::stdout())),
        format_fn: Arc::new(format_name),
        print_fn: Arc::new(print_name_value_line),
    }
}

pub fn format_name(name: &Name, _kind: Kind) -> Vec<String> {
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
        attributes: Attributes::default(),
        inner: Arc::new(RwLock::new(BufWriter::new(io::stdout()))),
        format_fn: Arc::new(format_name),
        buffer_print_fn: Arc::new(print_name_value_line),
//        flush_print_fn: Arc::new(flush_buffer_raw),
    }
}



/// Discard metrics output.
#[derive(Clone)]
pub struct Void {}

impl Output for Void {
    type Input = Void;

    fn new_input(&self) -> Void {
        self.clone()
    }
}

impl Input for Void {
    fn new_metric(&self, _name: Name, _kind: Kind) -> WriteFn {
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
        let c = super::to_stdout().new_input_dyn();
        let m = c.new_metric("test".into(), Kind::Marker);
        (m)(33);
    }

    #[test]
    fn test_to_void() {
        let c = super::to_void().new_input_dyn();
        let m = c.new_metric("test".into(), Kind::Marker);
        (m)(33);
    }

}
