use std::{collections::HashMap, hash::Hash};

pub struct OneWayMap<K, T> {
    mapping: HashMap<K, usize>,
    storage: Vec<T>,
}

impl<K: Hash + Eq, T> OneWayMap<K, T> {
    pub fn new() -> Self {
        Self { mapping: HashMap::new(), storage: vec![] }
    }

    pub fn get_idx(&self, binding: &K) -> Option<usize> {
        self.mapping.get(binding).map(|value| *value)
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.storage.get(idx)
    }

    pub fn clear(&mut self) {
        self.mapping.clear();
        self.storage.clear();
    }

    pub fn push(&mut self, key: K, value: T) -> usize {
        let next = self.storage.len();
        let mapping = &mut self.mapping;

        let idx = mapping.entry(key).or_insert(next);

        match self.storage.get_mut(*idx) {
            Some(data) => *data = value,
            None => self.storage.push(value),
        }

        *idx
    }
}
