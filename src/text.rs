//! Standard stateless metric outputs.

// TODO parameterize templates
use core::{Name, WithName, Value, Metric, Kind, Output, Input, Flush, WithAttributes, Attributes, WithBuffering};
use error;
use std::sync::{RwLock, Arc};
use std::io::{Write,  self};

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

/// Buffered metrics text output.
pub struct TextOutput<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    inner: Arc<RwLock<W>>,
    format_fn: Arc<Fn(&Name, Kind) -> Vec<String> + Send + Sync>,
    print_fn: Arc<Fn(&mut Vec<u8>, &[String], Value) -> error::Result<()> + Send + Sync>,
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

impl<W: Write + Send + Sync + 'static> WithBuffering for TextOutput<W> {}

impl<W: Write + Send + Sync + 'static> Output for TextOutput<W> {

    type INPUT = TextInput<W>;

    fn new_input(&self) -> Self::INPUT {
        TextInput {
            attributes: self.attributes.clone(),
            entries: Arc::new(RwLock::new(Vec::new())),
            output: self.clone(),
        }
    }
}

/// The scope-local input for buffered text metrics output.
pub struct TextInput<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    output: TextOutput<W>,
}

impl<W: Write + Send + Sync + 'static> Clone for TextInput<W> {
    fn clone(&self) -> Self {
        TextInput {
            attributes: self.attributes.clone(),
            entries: self.entries.clone(),
            output: self.output.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> WithAttributes for TextInput<W> {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl<W: Write + Send + Sync + 'static> WithBuffering for TextInput<W> {}

impl<W: Write + Send + Sync + 'static> Input for TextInput<W> {
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
                    Err(err) => debug!("{}", err),
                }
            })
        } else {
            let output = self.output.clone();
            Metric::new(move |value| {
                let mut buffer = Vec::with_capacity(32);
                match (print_fn)(&mut buffer, &template, value) {
                    Ok(()) => {
                        let mut output = output.inner.write().expect("TextOutput");
                        if let Err(e) = output.write_all(&buffer) {
                            debug!("Could not write text metrics: {}", e)
                        }
                    },
                    Err(err) => debug!("{}", err),
                }
            })
        }
    }
}

impl<W: Write + Send + Sync + 'static> Flush for TextInput<W> {
    fn flush(&self) -> error::Result<()> {
        let mut entries = self.entries.write().expect("Metrics TextBuffer");
        if !entries.is_empty() {
            let mut output = self.output.inner.write().expect("TextOutput");
            for entry in entries.drain(..) {
                output.write_all(&entry)?
            }
            output.flush()?;
        }
        Ok(())
    }
}

impl<W: Write + Send + Sync + 'static> Drop for TextInput<W> {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            warn!("Could not flush text metrics on Drop. {}", e)
        }
    }
}

/// Discard metrics output.
#[derive(Clone)]
pub struct Void {}

impl Output for Void {
    type INPUT = Void;

    fn new_input(&self) -> Void {
        self.clone()
    }
}

impl Input for Void {
    fn new_metric(&self, _name: Name, _kind: Kind) -> Metric {
        Metric::new(|_value| {})
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
