//! Standard stateless metric outputs.

// TODO parameterize templates
use core::{Name, AddPrefix, Value, Kind, OutputScope, WithAttributes, Attributes,
           WithBuffering, OutputMetric, Output, WithMetricCache, WithOutputQueue, Flush};
use error;
use std::sync::{RwLock, Arc};
use std::io::{Write, self};
use std::rc::Rc;
use std::cell::RefCell;

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

impl<W: Write + Send + Sync + 'static> WithMetricCache for TextOutput<W> {}

impl<W: Write + Send + Sync + 'static> WithOutputQueue for TextOutput<W> {}

impl<W: Write + Send + Sync + 'static> WithBuffering for TextOutput<W> {}

impl<W: Write + Send + Sync + 'static> Output for TextOutput<W> {

    type SCOPE = Text<W>;

    fn open_scope_raw(&self) -> Self::SCOPE {
        Text {
            attributes: self.attributes.clone(),
            entries: Rc::new(RefCell::new(Vec::new())),
            output: self.clone(),
        }
    }
}

/// A scope for text metrics.
pub struct Text<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    entries: Rc<RefCell<Vec<Vec<u8>>>>,
    output: TextOutput<W>,
}

impl<W: Write + Send + Sync + 'static> Text<W> {
    /// Write metric values to provided Write target.
    pub fn output(write: W) -> TextOutput<W> {
        TextOutput {
            attributes: Attributes::default(),
            inner: Arc::new(RwLock::new(write)),
            format_fn: Arc::new(format_name),
            print_fn: Arc::new(print_name_value_line),
        }
    }

    /// Write metric values to stdout.
    pub fn stdout() -> TextOutput<io::Stdout> {
        Text::output(io::stdout())
    }

    /// Write metric values to stdout.
    pub fn stderr() -> TextOutput<io::Stderr> {
        Text::output(io::stderr())
    }
}


impl<W: Write + Send + Sync + 'static> Clone for Text<W> {
    fn clone(&self) -> Self {
        Text {
            attributes: self.attributes.clone(),
            entries: self.entries.clone(),
            output: self.output.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> WithAttributes for Text<W> {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl<W: Write + Send + Sync + 'static> WithBuffering for Text<W> {}

impl<W: Write + Send + Sync + 'static> OutputScope for Text<W> {
    fn new_metric_raw(&self, name: Name, kind: Kind) -> OutputMetric {
        let name = self.qualified_name(name);
        let template = (self.output.format_fn)(&name, kind);

        let print_fn = self.output.print_fn.clone();
        let entries = self.entries.clone();

        if self.is_buffering() {
            OutputMetric::new(move |value| {
                let mut buffer = Vec::with_capacity(32);
                match (print_fn)(&mut buffer, &template, value) {
                    Ok(()) => {
                        let mut entries = entries.borrow_mut();
                        entries.push(buffer.into())
                    },
                    Err(err) => debug!("{}", err),
                }
            })
        } else {
            let output = self.output.clone();
            OutputMetric::new(move |value| {
                let mut buffer = Vec::with_capacity(32);
                match (print_fn)(&mut buffer, &template, value) {
                    Ok(()) => {
                        let mut output = output.inner.write().expect("TextOutput");
                        if let Err(e) = output.write_all(&buffer).and_then(|_| output.flush()) {
                            debug!("Could not write text metrics: {}", e)
                        }
                    },
                    Err(err) => debug!("{}", err),
                }
            })
        }
    }
}

impl<W: Write + Send + Sync + 'static> Flush for Text<W> {

    fn flush(&self) -> error::Result<()> {
        let mut entries = self.entries.borrow_mut();
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

impl<W: Write + Send + Sync + 'static> Drop for Text<W> {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            warn!("Could not flush text metrics on Drop. {}", e)
        }
    }
}

#[cfg(test)]
mod test {
    use core::*;
    use std::io;

    #[test]
    fn sink_print() {
        let c = super::Text::output(io::stdout()).open_scope();
        let m = c.new_metric("test".into(), Kind::Marker);
        m.write(33);
    }

}
