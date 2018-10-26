//! Standard stateless metric outputs.

// TODO parameterize templates

use core::{Flush};
use core::input::InputKind;
use core::attributes::{Attributes, WithAttributes, Buffered, Prefixed};
use core::name::MetricName;
use core::output::{Output, OutputMetric, OutputScope};
use core::error;

use cache::cache_out;
use queue::queue_out;
use output::format::{LineFormat, SimpleFormat, Formatting};

use std::sync::{RwLock, Arc};
use std::io::{Write, self};
use std::rc::Rc;
use std::cell::RefCell;

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

impl<W: Write + Send + Sync + 'static>  Stream<W> {
    /// Write metric values to provided Write target.
    pub fn write_to(write: W) -> Stream<W> {
        Stream {
            attributes: Attributes::default(),
            format: Arc::new(SimpleFormat::default()),
            inner: Arc::new(RwLock::new(write)),
        }
    }
}

impl Stream<io::Stderr> {
    /// Write metric values to stdout.
    pub fn stderr() -> Stream<io::Stderr> {
        Stream::write_to(io::stderr())
    }
}

impl Stream<io::Stdout> {
    /// Write metric values to stdout.
    pub fn stdout() -> Stream<io::Stdout> {
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
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl<W: Write + Send + Sync + 'static> Buffered for Stream<W> {}

impl<W: Write + Send + Sync + 'static> Output for Stream<W> {
    type SCOPE = TextScope<W>;

    fn output(&self) -> Self::SCOPE {
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
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl<W: Write + Send + Sync + 'static> Buffered for TextScope<W> {}

impl<W: Write + Send + Sync + 'static> OutputScope for TextScope<W> {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> OutputMetric {
        let name = self.prefix_append(name);
        let template = self.output.format.template(&name, kind);

        let entries = self.entries.clone();

        if let Some(_buffering) = self.get_buffering() {
            OutputMetric::new(move |value, labels| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value, |key| labels.lookup(key)) {
                    Ok(()) => {
                        let mut entries = entries.borrow_mut();
                        entries.push(buffer)
                    },
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
                        let mut output = output.inner.write().expect("Metrics Text Output");
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

impl<W: Write + Send + Sync + 'static> Flush for TextScope<W> {

    fn flush(&self) -> error::Result<()> {
        let mut entries = self.entries.borrow_mut();
        if !entries.is_empty() {
            let mut output = self.output.inner.write().expect("Metrics Text Output");
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
        let c = super::Stream::write_to(io::stdout()).output();
        let m = c.new_metric("test".into(), InputKind::Marker);
        m.write(33, labels![]);
    }
}
