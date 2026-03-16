// KLIK stdlib - Collections module

use std::collections::HashMap;

/// A dynamically-sized list
#[derive(Debug, Clone)]
pub struct List<T> {
    inner: Vec<T>,
}

impl<T> List<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),
        }
    }

    pub fn push(&mut self, value: T) {
        self.inner.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.inner.get_mut(index)
    }

    pub fn insert(&mut self, index: usize, value: T) {
        self.inner.insert(index, value);
    }

    pub fn remove(&mut self, index: usize) -> T {
        self.inner.remove(index)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.inner.iter_mut()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    pub fn into_vec(self) -> Vec<T> {
        self.inner
    }

    pub fn first(&self) -> Option<&T> {
        self.inner.first()
    }

    pub fn last(&self) -> Option<&T> {
        self.inner.last()
    }

    pub fn contains(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.inner.contains(value)
    }

    pub fn reverse(&mut self) {
        self.inner.reverse();
    }

    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.inner.sort();
    }

    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.inner.sort_by(compare);
    }

    pub fn dedup(&mut self)
    where
        T: PartialEq,
    {
        self.inner.dedup();
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.retain(f);
    }

    pub fn extend_from_slice(&mut self, other: &[T])
    where
        T: Clone,
    {
        self.inner.extend_from_slice(other);
    }

    pub fn truncate(&mut self, len: usize) {
        self.inner.truncate(len);
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<Vec<T>> for List<T> {
    fn from(v: Vec<T>) -> Self {
        Self { inner: v }
    }
}

impl<T> IntoIterator for List<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// A hash map
#[derive(Debug, Clone)]
pub struct Map<K, V> {
    inner: HashMap<K, V>,
}

impl<K, V> Map<K, V>
where
    K: std::hash::Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(cap),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.inner.get_mut(key)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.inner.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter()
    }

    pub fn entry(&mut self, key: K) -> std::collections::hash_map::Entry<'_, K, V> {
        self.inner.entry(key)
    }

    pub fn into_hashmap(self) -> HashMap<K, V> {
        self.inner
    }
}

impl<K, V> Default for Map<K, V>
where
    K: std::hash::Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A set backed by a HashMap
#[derive(Debug, Clone)]
pub struct Set<T> {
    inner: std::collections::HashSet<T>,
}

impl<T> Set<T>
where
    T: std::hash::Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            inner: std::collections::HashSet::new(),
        }
    }

    pub fn insert(&mut self, value: T) -> bool {
        self.inner.insert(value)
    }

    pub fn remove(&mut self, value: &T) -> bool {
        self.inner.remove(value)
    }

    pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.iter()
    }

    pub fn union<'a>(&'a self, other: &'a Set<T>) -> Set<T>
    where
        T: Clone,
    {
        Set {
            inner: self.inner.union(&other.inner).cloned().collect(),
        }
    }

    pub fn intersection<'a>(&'a self, other: &'a Set<T>) -> Set<T>
    where
        T: Clone,
    {
        Set {
            inner: self.inner.intersection(&other.inner).cloned().collect(),
        }
    }

    pub fn difference<'a>(&'a self, other: &'a Set<T>) -> Set<T>
    where
        T: Clone,
    {
        Set {
            inner: self.inner.difference(&other.inner).cloned().collect(),
        }
    }

    pub fn is_subset(&self, other: &Set<T>) -> bool {
        self.inner.is_subset(&other.inner)
    }

    pub fn is_superset(&self, other: &Set<T>) -> bool {
        self.inner.is_superset(&other.inner)
    }
}

impl<T> Default for Set<T>
where
    T: std::hash::Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A double-ended queue
#[derive(Debug, Clone)]
pub struct Deque<T> {
    inner: std::collections::VecDeque<T>,
}

impl<T> Deque<T> {
    pub fn new() -> Self {
        Self {
            inner: std::collections::VecDeque::new(),
        }
    }

    pub fn push_front(&mut self, value: T) {
        self.inner.push_front(value);
    }

    pub fn push_back(&mut self, value: T) {
        self.inner.push_back(value);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.inner.pop_back()
    }

    pub fn front(&self) -> Option<&T> {
        self.inner.front()
    }

    pub fn back(&self) -> Option<&T> {
        self.inner.back()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.iter()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }
}

impl<T> Default for Deque<T> {
    fn default() -> Self {
        Self::new()
    }
}
