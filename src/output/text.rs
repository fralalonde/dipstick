//! Standard stateless metric outputs.

// TODO parameterize templates

use core::{Flush};
use core::input::Kind;
use core::attributes::{Attributes, WithAttributes, Buffered, Naming};
use core::name::Name;
use core::output::{Output, OutputMetric, OutputScope};
use core::error;
use ::{Format, LineFormat};

use cache::cache_out;
use queue::queue_out;

use std::sync::{RwLock, Arc};
use std::io::{Write, self};
use std::rc::Rc;
use std::cell::RefCell;

/// Buffered metrics text output.
pub struct Text<W: Write + Send + Sync + 'static> {
    attributes: Attributes,
    format: Arc<Format + Send + Sync>,
    inner: Arc<RwLock<W>>,
}

impl<W: Write + Send + Sync + 'static> queue_out::QueuedOutput for Text<W> {}
impl<W: Write + Send + Sync + 'static> cache_out::CachedOutput for Text<W> {}

impl<W: Write + Send + Sync + 'static>  Text<W> {
    /// Write metric values to provided Write target.
    pub fn write_to(write: W) -> Text<W> {
        Text {
            attributes: Attributes::default(),
            format: Arc::new(LineFormat::default()),
            inner: Arc::new(RwLock::new(write)),
        }
    }
}

impl Text<io::Stderr> {
    /// Write metric values to stdout.
    pub fn stderr() -> Text<io::Stderr> {
        Text::write_to(io::stderr())
    }
}

impl Text<io::Stdout> {
    /// Write metric values to stdout.
    pub fn stdout() -> Text<io::Stdout> {
        Text::write_to(io::stdout())
    }
}


// FIXME manual Clone impl required because auto-derive is borked (https://github.com/rust-lang/rust/issues/26925)
impl<W: Write + Send + Sync + 'static> Clone for Text<W> {
    fn clone(&self) -> Self {
        Text {
            attributes: self.attributes.clone(),
            format: self.format.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<W: Write + Send + Sync + 'static> WithAttributes for Text<W> {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl<W: Write + Send + Sync + 'static> Buffered for Text<W> {}

impl<W: Write + Send + Sync + 'static> Output for Text<W> {
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
    output: Text<W>,
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
    fn new_metric(&self, name: Name, kind: Kind) -> OutputMetric {
        let name = self.naming_append(name);
        let template = self.output.format.template(&name, kind);

        let entries = self.entries.clone();

        if let Some(_buffering) = self.get_buffering() {
            OutputMetric::new(move |value| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value) {
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
            OutputMetric::new(move |value| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value) {
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

impl<W: Write + Send + Sync + 'static> Flush for TextScope<W> {

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
    use core::input::Kind;
    use std::io;

    #[test]
    fn sink_print() {
        let c = super::Text::write_to(io::stdout()).output();
        let m = c.new_metric("test".into(), Kind::Marker);
        m.write(33);
    }
}
