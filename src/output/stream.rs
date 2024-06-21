//! Standard stateless metric Inputs.

// TODO parameterize templates

use crate::attributes::{Attributes, Buffered, MetricId, OnFlush, Prefixed, WithAttributes};
use crate::input::InputKind;
use crate::name::MetricName;
use crate::Flush;
use crate::{CachedInput, QueuedInput};

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::RwLock;

#[cfg(feature = "parking_lot")]
use parking_lot::RwLock;

use crate::{Formatting, Input, InputMetric, InputScope, LineFormat, SimpleFormat};

/// Buffered metrics text Input.
pub struct Stream<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    format: Arc<dyn LineFormat + Send + Sync>,
    inner: Arc<RwLock<W>>,
}

impl<W: Write + Send + Sync + 'static> QueuedInput for Stream<W> {}
impl<W: Write + Send + Sync + 'static> CachedInput for Stream<W> {}

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
    #[deprecated(since = "0.8.0", note = "Use write_to_file()")]
    #[allow(clippy::wrong_self_convention)]
    pub fn to_file<P: AsRef<Path>>(file: P) -> io::Result<Stream<File>> {
        Self::write_to_file(file)
    }

    /// Write metric values to a file.
    pub fn write_to_file<P: AsRef<Path>>(file: P) -> io::Result<Stream<File>> {
        let file = OpenOptions::new().create(true).append(true).open(file)?;
        Ok(Stream::write_to(file))
    }

    /// Write metrics to a new file.
    ///
    /// Creates a new file to dump data into. If `clobber` is set to true, it allows overwriting
    /// existing file, if false, the attempt will result in an error.
    #[deprecated(since = "0.8.0", note = "Use write_to_new_file()")]
    #[allow(clippy::wrong_self_convention)]
    pub fn to_new_file<P: AsRef<Path>>(file: P, clobber: bool) -> io::Result<Stream<File>> {
        Self::write_to_new_file(file, clobber)
    }

    /// Write metrics to a new file.
    ///
    /// Creates a new file to dump data into. If `clobber` is set to true, it allows overwriting
    /// existing file, if false, the attempt will result in an error.
    pub fn write_to_new_file<P: AsRef<Path>>(file: P, clobber: bool) -> io::Result<Stream<File>> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(!clobber)
            .open(file)?;
        Ok(Stream::write_to(file))
    }
}

impl Stream<io::Stderr> {
    /// Write metric values to stderr.
    #[deprecated(since = "0.8.0", note = "Use write_to_stderr()")]
    pub fn to_stderr() -> Stream<io::Stderr> {
        Stream::write_to(io::stderr())
    }

    /// Write metric values to stderr.
    pub fn write_to_stderr() -> Stream<io::Stderr> {
        Stream::write_to(io::stderr())
    }
}

impl Stream<io::Stdout> {
    /// Write metric values to stdout.
    #[deprecated(since = "0.8.0", note = "Use write_to_stdout()")]
    pub fn to_stdout() -> Stream<io::Stdout> {
        Stream::write_to(io::stdout())
    }

    /// Write metric values to stdout.
    pub fn write_to_stdout() -> Stream<io::Stdout> {
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

impl<W: Write + Send + Sync + 'static> Input for Stream<W> {
    type SCOPE = TextScope<W>;

    fn metrics(&self) -> Self::SCOPE {
        TextScope {
            attributes: self.attributes.clone(),
            entries: Arc::new(RwLock::new(Vec::new())),
            input: self.clone(),
        }
    }
}

/// A scope for text metrics.
pub struct TextScope<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    input: Stream<W>,
}

impl<W: Write + Send + Sync + 'static> Clone for TextScope<W> {
    fn clone(&self) -> Self {
        TextScope {
            attributes: self.attributes.clone(),
            entries: self.entries.clone(),
            input: self.input.clone(),
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

impl<W: Write + Send + Sync + 'static> InputScope for TextScope<W> {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let name = self.prefix_append(name);
        let template = self.input.format.template(&name, kind);

        let entries = self.entries.clone();
        let metric_id = MetricId::forge("stream", name);

        if self.is_buffered() {
            InputMetric::new(metric_id, move |value, labels| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value, |key| labels.lookup(key)) {
                    Ok(()) => {
                        let mut entries = write_lock!(entries);
                        entries.push(buffer)
                    }
                    Err(err) => debug!("{}", err),
                }
            })
        } else {
            // unbuffered
            let input = self.input.clone();
            InputMetric::new(metric_id, move |value, labels| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value, |key| labels.lookup(key)) {
                    Ok(()) => {
                        let mut input = write_lock!(input.inner);
                        if let Err(e) = input.write_all(&buffer).and_then(|_| input.flush()) {
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
    fn flush(&self) -> io::Result<()> {
        self.notify_flush_listeners();
        let mut entries = write_lock!(self.entries);
        if !entries.is_empty() {
            let mut input = write_lock!(self.input.inner);
            for entry in entries.drain(..) {
                input.write_all(&entry)?
            }
            input.flush()?;
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
    use crate::input::InputKind;
    use std::io;

    #[test]
    fn sink_print() {
        let c = Stream::write_to(io::stdout()).metrics();
        let m = c.new_metric("test".into(), InputKind::Marker);
        m.write(33, labels![]);
    }
}
