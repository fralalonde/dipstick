//! Standard stateless metric outputs.

// TODO parameterize templates

use core::attributes::{Attributes, Buffered, Prefixed, WithAttributes};
use core::error;
use core::input::InputKind;
use core::name::MetricName;
use core::output::{Output, OutputMetric, OutputScope};
use core::Flush;

use cache::cache_out;
use output::format::{Formatting, LineFormat, SimpleFormat};
use queue::queue_out;

use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::rc::Rc;

use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::RwLock;

#[cfg(feature = "parking_lot")]
use parking_lot::RwLock;

/// Buffered metrics text output.
pub struct Stream<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    format: Arc<LineFormat + Send + Sync>,
    inner: Arc<RwLock<W>>,
}

impl<W: Write + Send + Sync + 'static> queue_out::QueuedOutput for Stream<W> {}
impl<W: Write + Send + Sync + 'static> cache_out::CachedOutput for Stream<W> {}

impl<W: Write + Send + Sync + 'static> Formatting for Stream<W> {
    fn formatting(&self, format: impl LineFormat + 'static) -> Self {
        let mut cloned = self.clone();
        cloned.format = Arc::new(format);
        cloned
    }
}

impl<W: Write + Send + Sync + 'static> Stream<W> {
    /// Write metric values to provided Write target.
    pub fn write_to(write: W) -> Stream<W> {
        Stream {
            attributes: Attributes::default(),
            format: Arc::new(SimpleFormat::default()),
            inner: Arc::new(RwLock::new(write)),
        }
    }
}

impl Stream<File> {
    /// Write metric values to a file.
    pub fn to_file<P: AsRef<Path>>(file: P) -> error::Result<Stream<File>> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(file)?;
        Ok(Stream::write_to(file))
    }

    /// Write to a new file.
    ///
    /// Creates a new file to dump data into. If `clobber` is set to true, it allows overwriting
    /// existing file, if false, the attempt will result in an error.
    pub fn to_new_file<P: AsRef<Path>>(file: P, clobber: bool) -> error::Result<Stream<File>> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .create_new(!clobber)
            .open(file)?;
        Ok(Stream::write_to(file))
    }
}

impl Stream<io::Stderr> {
    /// Write metric values to stdout.
    pub fn to_stderr() -> Stream<io::Stderr> {
        Stream::write_to(io::stderr())
    }
}

impl Stream<io::Stdout> {
    /// Write metric values to stdout.
    pub fn to_stdout() -> Stream<io::Stdout> {
        Stream::write_to(io::stdout())
    }
}

// FIXME manual Clone impl required because auto-derive is borked (https://github.com/rust-lang/rust/issues/26925)
impl<W: Write + Send + Sync + 'static> Clone for Stream<W> {
    fn clone(&self) -> Self {
        Stream {
            attributes: self.attributes.clone(),
            format: self.format.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> WithAttributes for Stream<W> {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl<W: Write + Send + Sync + 'static> Buffered for Stream<W> {}

impl<W: Write + Send + Sync + 'static> Output for Stream<W> {
    type SCOPE = TextScope<W>;

    fn new_scope(&self) -> Self::SCOPE {
        TextScope {
            attributes: self.attributes.clone(),
            entries: Rc::new(RefCell::new(Vec::new())),
            output: self.clone(),
        }
    }
}

/// A scope for text metrics.
pub struct TextScope<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    entries: Rc<RefCell<Vec<Vec<u8>>>>,
    output: Stream<W>,
}

impl<W: Write + Send + Sync + 'static> Clone for TextScope<W> {
    fn clone(&self) -> Self {
        TextScope {
            attributes: self.attributes.clone(),
            entries: self.entries.clone(),
            output: self.output.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> WithAttributes for TextScope<W> {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl<W: Write + Send + Sync + 'static> Buffered for TextScope<W> {}

impl<W: Write + Send + Sync + 'static> OutputScope for TextScope<W> {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> OutputMetric {
        let name = self.prefix_append(name);
        let template = self.output.format.template(&name, kind);

        let entries = self.entries.clone();

        if self.is_buffered() {
            OutputMetric::new(move |value, labels| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value, |key| labels.lookup(key)) {
                    Ok(()) => {
                        let mut entries = entries.borrow_mut();
                        entries.push(buffer)
                    }
                    Err(err) => debug!("{}", err),
                }
            })
        } else {
            // unbuffered
            let output = self.output.clone();
            OutputMetric::new(move |value, labels| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value, |key| labels.lookup(key)) {
                    Ok(()) => {
                        let mut output = write_lock!(output.inner);
                        if let Err(e) = output.write_all(&buffer).and_then(|_| output.flush()) {
                            debug!("Could not write text metrics: {}", e)
                        }
                    }
                    Err(err) => debug!("{}", err),
                }
            })
        }
    }
}

impl<W: Write + Send + Sync + 'static> Flush for TextScope<W> {
    fn flush(&self) -> error::Result<()> {
        let mut entries = self.entries.borrow_mut();
        if !entries.is_empty() {
            let mut output = write_lock!(self.output.inner);
            for entry in entries.drain(..) {
                output.write_all(&entry)?
            }
            output.flush()?;
        }
        Ok(())
    }
}

impl<W: Write + Send + Sync + 'static> Drop for TextScope<W> {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            warn!("Could not flush text metrics on Drop. {}", e)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::input::InputKind;
    use std::io;

    #[test]
    fn sink_print() {
        let c = super::Stream::write_to(io::stdout()).new_scope();
        let m = c.new_metric("test".into(), InputKind::Marker);
        m.write(33, labels![]);
    }
}
