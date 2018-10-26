use std::ops::{Deref,DerefMut};
use std::collections::{VecDeque};

/// A double-ended vec of strings constituting a metric name or a future part thereof.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct NameParts {
    /// Nodes are stored in order or their final appearance in the names
    /// If there is no namespace, the deque is empty.
    nodes: VecDeque<String>,
}

impl NameParts {

    /// Returns true if this instance is equal to or a subset (more specific) of the target instance.
    /// e.g. `a.b.c` is within `a.b`
    /// e.g. `a.d.c` is not within `a.b`
    pub fn is_within(&self, other: &NameParts) -> bool {
        // quick check: if this name has less parts it cannot be equal or more specific
        if self.len() < other.nodes.len() {
            return false
        }
        for (i, part) in other.nodes.iter().enumerate() {
            if part != &self.nodes[i] {
                return false
            }
        }
        true
    }

    /// Make a name in this namespace
    pub fn make_name<S: Into<String>>(&self, leaf: S) -> MetricName {
        let mut nodes = self.clone();
        nodes.push_back(leaf.into());
        MetricName { nodes }
    }

    /// Extract a copy of the last name part
    /// Panics if empty
    pub fn short(&self) -> MetricName {
        self.back().expect("Short metric name").clone().into()
    }
}

/// Turn any string into a StringDeque
impl<S: Into<String>> From<S> for NameParts {
    fn from(name_part: S) -> Self {
        let name: String = name_part.into();
        // can we do better than asserting? empty names should not exist, ever...
        debug_assert!(!name.is_empty());
        let mut nodes = NameParts::default();
        nodes.push_front(name);
        nodes
    }
}

/// Enable use of VecDeque methods such as len(), push_*, insert()...
impl Deref for NameParts {
    type Target = VecDeque<String>;
    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

/// Enable use of VecDeque methods such as len(), push_*, insert()...
impl DerefMut for NameParts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}

/// The name of a metric, including the concatenated possible namespaces in which it was defined.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct MetricName {
    nodes: NameParts,
}

impl MetricName {

    /// Prepend to the existing namespace.
    pub fn prepend<S: Into<NameParts>>(mut self, namespace: S) -> Self {
        let parts: NameParts =  namespace.into();
        parts.iter().rev().for_each(|node|
            self.nodes.push_front(node.clone())
        );
        self
    }

    /// Append to the existing namespace.
    pub fn append<S: Into<NameParts>>(mut self, namespace: S) -> Self {
        let offset = self.nodes.len() - 1;
        let parts: NameParts =  namespace.into();
        for (i, part) in parts.iter().enumerate() {
            self.nodes.insert(i + offset, part.clone())
        }
        self
    }

    /// Combine name parts into a string.
    pub fn join(&self, separator: &str) -> String {
        self.nodes.iter().map(|s| &**s).collect::<Vec<&str>>().join(separator)
    }
}

impl<S: Into<String>> From<S> for MetricName {
    fn from(name: S) -> Self {
        MetricName { nodes: NameParts::from(name) }
    }
}

impl Deref for MetricName {
    type Target = NameParts;
    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

impl DerefMut for MetricName {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}


#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn string_deque_within_same() {
        let mut sd1: NameParts = "c".into();
        sd1.push_front("b".into());

        assert_eq!(true, sd1.is_within(&sd1));
    }

    #[test]
    fn string_deque_within_other() {
        let mut sd1: NameParts = "b".into();
        sd1.push_front("a".into());

        let mut sd2: NameParts = "c".into();
        sd2.push_front("b".into());
        sd2.push_front("a".into());

        assert_eq!(true, sd2.is_within(&sd1));
        assert_eq!(false, sd1.is_within(&sd2));
    }

}