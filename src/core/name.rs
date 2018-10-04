use std::ops::{Deref,DerefMut};
use std::collections::{VecDeque};

/// Primitive struct for Namespace and Name
/// A double-ended vec of strings constituting a metric name or a future part thereof.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct Namespace {
    /// Nodes are stored in order or their final appearance in the names
    /// If there is no namespace, the deque is empty.
    nodes: VecDeque<String>,
}

impl Namespace {

    /// Build a new StringDeque
    /// This is a private shortcut constructor,
    /// no one outside this module should need to do that, only use Name or Namespace.
    fn new() -> Self {
        Namespace { nodes: VecDeque::new() }
    }

    /// Returns true if this instance is equal to or a subset (more specific) of the target instance.
    /// e.g. `a.b.c` is within `a.b`
    /// e.g. `a.d.c` is not within `a.b`
    pub fn is_within(&self, other: &Namespace) -> bool {
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
    pub fn qualify<S: Into<String>>(&self, leaf: S) -> Name {
        let mut nodes = self.clone();
        nodes.push_back(leaf.into());
        Name { nodes }
    }

    /// Create a new Name using only the last part (leaf)
    pub fn leaf(&self) -> Name {
        self.back().expect("Short metric name").clone().into()
    }
}

/// Turn any string into a StringDeque
impl<S: Into<String>> From<S> for Namespace {
    fn from(name_part: S) -> Self {
        let name: String = name_part.into();
        // can we do better than asserting? empty names should not exist, ever...
        debug_assert!(!name.is_empty());
        let mut nodes = Namespace::new();
        nodes.push_front(name);
        nodes
    }
}

/// Enable use of VecDeque methods such as len(), push_*, insert()...
impl Deref for Namespace {
    type Target = VecDeque<String>;
    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

/// Enable use of VecDeque methods such as len(), push_*, insert()...
impl DerefMut for Namespace {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}

/// The name of a metric, including the concatenated possible namespaces in which it was defined.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Name {
    nodes: Namespace,
}

impl Name {

    /// Prepend to the existing namespace.
    pub fn prepend<S: Into<Namespace>>(mut self, namespace: S) -> Self {
        let parts: Namespace =  namespace.into();
        parts.iter().rev().for_each(|node|
            self.nodes.push_front(node.clone())
        );
        self
    }

    /// Append to the existing namespace.
    pub fn append<S: Into<Namespace>>(mut self, namespace: S) -> Self {
        let offset = self.nodes.len() - 1;
        let parts: Namespace =  namespace.into();
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

impl<S: Into<String>> From<S> for Name {
    fn from(name: S) -> Self {
        Name { nodes: Namespace::from(name) }
    }
}

impl Deref for Name {
    type Target = Namespace;
    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}


#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn string_deque_within_same() {
        let mut sd1: Namespace = "c".into();
        sd1.push_front("b".into());

        assert_eq!(true, sd1.is_within(&sd1));
    }

    #[test]
    fn string_deque_within_other() {
        let mut sd1: Namespace = "b".into();
        sd1.push_front("a".into());

        let mut sd2: Namespace = "c".into();
        sd2.push_front("b".into());
        sd2.push_front("a".into());

        assert_eq!(true, sd2.is_within(&sd1));
        assert_eq!(false, sd1.is_within(&sd2));
    }

}