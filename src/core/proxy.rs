//! Decouple metric definition from configuration with trait objects.

use core::attributes::{Attributes, WithAttributes, Naming};
use core::name::{Name, NameParts};
use core::Flush;
use core::input::{Kind, InputMetric, InputScope};
use core::void::VOID_INPUT;
use core::error;

use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, RwLock, Weak};
use std::fmt;

use atomic_refcell::*;

lazy_static! {
    /// Root of the default metrics proxy, usable by all libraries and apps.
    /// Libraries should create their metrics into sub subspaces of this.
    /// Applications should configure on startup where the proxied metrics should go.
    /// Exceptionally, one can create its own ProxyInput root, separate from this one.
    static ref ROOT_PROXY: Proxy = Proxy::new();
}

/// A dynamically proxyed metric.
#[derive(Debug)]
struct ProxyMetric {
    // basic info for this metric, needed to recreate new corresponding trait object if target changes
    name: NameParts,
    kind: Kind,

    // the metric trait object to proxy metric values to
    // the second part can be up to namespace.len() + 1 if this metric was individually targeted
    // 0 if no target assigned
    target: (AtomicRefCell<(InputMetric, usize)>),

    // a reference to the the parent proxyer to remove the metric from when it is dropped
    proxy: Arc<RwLock<InnerProxy>>,
}

/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for ProxyMetric {
    fn drop(&mut self) {
        self.proxy.write().expect("Dispatch Lock").drop_metric(&self.name)
    }
}

/// A dynamic proxy point for app and lib metrics.
/// Decouples metrics definition from backend configuration.
/// Allows defining metrics before a concrete type has been selected.
/// Allows replacing metrics backend on the fly at runtime.
#[derive(Clone, Debug)]
pub struct Proxy {
    attributes: Attributes,
    inner: Arc<RwLock<InnerProxy>>,
}

impl Default for Proxy {
    /// Return the default root metric proxy.
    fn default() -> Self {
        ROOT_PROXY.clone()
    }
}


struct InnerProxy {
    // namespaces can target one, many or no metrics
    targets: HashMap<NameParts, Arc<InputScope + Send + Sync>>,
    // last part of the namespace is the metric's name
    metrics: BTreeMap<NameParts, Weak<ProxyMetric>>,
}

impl fmt::Debug for InnerProxy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "metrics: {:?}", self.metrics.keys())?;
        write!(f, "targets: {:?}", self.targets.keys())
    }
}

impl InnerProxy {

    fn new() -> Self {
        Self {
            targets: HashMap::new(),
            metrics: BTreeMap::new(),
        }
    }

    fn set_target(&mut self, namespace: &NameParts, target_scope: Arc<InputScope + Send + Sync>) {
        self.targets.insert(namespace.clone(), target_scope.clone());

        for (metric_name, metric) in self.metrics.range_mut(namespace.clone()..) {
            if let Some(metric) = metric.upgrade() {
                // check for range end
                if !metric_name.is_within(namespace) { break }

                // check if metric targeted by _lower_ namespace
                if metric.target.borrow().1 > namespace.len() { continue }

                let target_metric = target_scope.new_metric(metric.name.short(), metric.kind);
                *metric.target.borrow_mut() = (target_metric, namespace.len());
            }
        }
    }

    fn get_effective_target(&self, namespace: &NameParts) -> Option<(Arc<InputScope + Send + Sync>, usize)> {
        if let Some(target) = self.targets.get(namespace) {
            return Some((target.clone(), namespace.len()));
        }

        // no 1:1 match, scan upper namespaces
        let mut name = namespace.clone();
        while let Some(_popped) = name.pop_back() {
            if let Some(target) = self.targets.get(&name) {
                return Some((target.clone(), name.len()))
            }
        }
        None
    }

    fn unset_target(&mut self, namespace: &NameParts) {
        if self.targets.remove(namespace).is_none() {
            // nothing to do
            return
        }

        let (up_target, up_nslen) = self.get_effective_target(namespace)
            .unwrap_or_else(|| (VOID_INPUT.input_dyn(), 0));

        // update all affected metrics to next upper targeted namespace
        for (name, metric) in self.metrics.range_mut(namespace.clone()..) {
            // check for range end
            if !name.is_within(namespace) { break }

            if let Some(mut metric) = metric.upgrade() {
                // check if metric targeted by _lower_ namespace
                if metric.target.borrow().1 > namespace.len() { continue }

                let new_metric = up_target.new_metric(name.short(), metric.kind);
                *metric.target.borrow_mut() = (new_metric, up_nslen);
            }
        }
    }

    fn drop_metric(&mut self, name: &NameParts) {
        if self.metrics.remove(name).is_none() {
            panic!("Could not remove DelegatingMetric weak ref from delegation point")
        }
    }

    fn flush(&self, namespace: &NameParts) -> error::Result<()> {
        if let Some((target, _nslen)) = self.get_effective_target(namespace) {
            target.flush()
        } else {
            Ok(())
        }
    }

}

impl Proxy {

    /// Create a new "private" metric proxy root. This is usually not what you want.
    /// Since this proxy will not be part of the standard proxy tree,
    /// it will need to be configured independently and since downstream code may not know about
    /// its existence this may never happen and metrics will not be proxyed anywhere.
    /// If you want to use the standard proxy tree, use #metric_proxy() instead.
    pub fn new() -> Self {
        Proxy {
            attributes: Attributes::default(),
            inner: Arc::new(RwLock::new(InnerProxy::new())),
        }
    }

    /// Replace target for this proxy and it's children.
    pub fn set_target<T: InputScope + Send + Sync + 'static>(&self, target: T) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.set_target(self.get_naming(), Arc::new(target));
    }

    /// Replace target for this proxy and it's children.
    pub fn unset_target(&self) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.unset_target(self.get_naming());
    }

    /// Replace target for this proxy and it's children.
    pub fn set_default_target<T: InputScope + Send + Sync + 'static>(target: T) {
        ROOT_PROXY.set_target(target)
    }

    /// Replace target for this proxy and it's children.
    pub fn unset_default_target(&self) {
        ROOT_PROXY.unset_target()
    }

}

impl<S: AsRef<str>> From<S> for Proxy {
    fn from(name: S) -> Proxy {
        Proxy::new().add_naming(name.as_ref())
    }
}

impl InputScope for Proxy {
    /// Lookup or create a proxy stub for the requested metric.
    fn new_metric(&self, name: Name, kind: Kind) -> InputMetric {
        let name: Name = self.naming_append(name);
        let mut inner = self.inner.write().expect("Dispatch Lock");
        let proxy = inner
            .metrics
            .get(&name)
            // TODO validate that Kind matches existing
            .and_then(|proxy_ref| Weak::upgrade(proxy_ref))
            .unwrap_or_else(|| {
                let namespace = &*name;
                {
                    // not found, define new
                    let (target, target_namespace_length) = inner.get_effective_target(namespace)
                        .unwrap_or_else(|| (VOID_INPUT.input_dyn(), 0));
                    let metric_object = target.new_metric(namespace.short(), kind);
                    let proxy = Arc::new(ProxyMetric {
                        name: namespace.clone(),
                        kind,
                        target: AtomicRefCell::new((metric_object, target_namespace_length)),
                        proxy: self.inner.clone(),
                    });
                    inner.metrics.insert(namespace.clone(), Arc::downgrade(&proxy));
                    proxy
                }
            });
        InputMetric::new(move |value, labels| proxy.target.borrow().0.write(value, labels))
    }
}

impl Flush for Proxy {

    fn flush(&self) -> error::Result<()> {
        self.inner.write().expect("Dispatch Lock").flush(self.get_naming())
    }
}

impl WithAttributes for Proxy {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;
    use aggregate::bucket::Bucket;

    #[bench]
    fn proxy_marker_to_aggregate(b: &mut test::Bencher) {
        ROOT_PROXY.set_target(Bucket::new());
        let metric = ROOT_PROXY.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn proxy_marker_to_void(b: &mut test::Bencher) {
        let metric = ROOT_PROXY.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
