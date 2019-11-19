use crate::*;
use std::{borrow::Borrow, collections::hash_map::HashMap, hash::Hash};

pub struct Occupied<'a, K: 'a, T: 'a> {
    key: &'a K,
    index: usize,
    value: &'a mut T,
}

pub struct VacantEntry<'a, K: 'a, T: 'a>
where
    K: Hash + Clone + Eq,
{
    key: K,
    storage: &'a mut MappedNovec<K, T>,
}

pub enum Entry<'a, K: 'a, T: 'a>
where
    K: Hash + Clone + Eq,
{
    Occupied(Occupied<'a, K, T>),
    VacantEntry(VacantEntry<'a, K, T>),
}

impl<'a, K: 'a, T: 'a> Entry<'a, K, T>
where
    K: Hash + Clone + Eq,
{
    pub fn or_insert(self, default: T) -> (usize, &'a mut T) {
        match self {
            Entry::Occupied(Occupied { value, index, .. }) => (index, value),
            Entry::VacantEntry(VacantEntry { key, storage }) => {
                let index = storage.insert(key, default);
                (index, storage.get_mut(index).unwrap())
            }
        }
    }

    pub fn or_insert_with<F: FnOnce() -> T>(self, default: F) -> (usize, &'a mut T) {
        self.or_insert(default())
    }

    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(Occupied { key, .. }) => key,
            Entry::VacantEntry(VacantEntry { key, .. }) => &key,
        }
    }

    pub fn and_modify<F: FnOnce(&mut T)>(mut self, f: F) -> Self {
        match &mut self {
            Entry::Occupied(Occupied { value, .. }) => f(value),
            _ => {}
        }

        self
    }

    pub fn or_default(self) -> (usize, &'a mut T)
    where
        T: Default,
    {
        self.or_insert(Default::default())
    }
}

// Can be indexed by either a Key or a usize, usize will be faster
// Keys stored in separate vector for faster iteration over values (probably doesnt matter)
#[derive(Clone, Debug)]
pub struct MappedNovec<K, T>
where
    K: Hash + Clone + Eq,
{
    map: HashMap<K, usize>,
    keys: Vec<Option<K>>,
    values: NoVec<T>,
}

impl<K, T> MappedNovec<K, T>
where
    K: Hash + Clone + Eq,
{
    pub fn entry<Q>(&mut self, key: K) -> Entry<K, T> {
        match self.map.get(&key) {
            Some(&index) => Entry::Occupied(Occupied {
                key: self.keys[index].as_mut().unwrap(),
                index,
                value: self.values.get_mut(index).unwrap(),
            }),
            None => Entry::VacantEntry(VacantEntry { key, storage: self }),
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.values.get_mut(index)
    }

    pub fn get_by_key<Q>(&self, key: &Q) -> Option<&T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let index = self.map.get(key)?;
        self.values.get(*index)
    }

    pub fn get_mut_by_key<Q>(&mut self, key: &Q) -> Option<&mut T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let index = self.map.get(key)?;
        self.values.get_mut(*index)
    }

    pub fn insert(&mut self, key: K, value: T) -> usize {
        let index = self.values.push(value);

        while self.keys.len() <= index {
            self.keys.push(None);
        }

        self.keys[index] = Some(key.clone());
        self.map.insert(key, index);

        index
    }

    pub fn remove(&mut self, index: usize) -> Option<(K, T)> {
        let value = self.values.remove(index)?;
        let (key, _) = self.map.remove_entry(self.keys[index].as_ref().unwrap())?;
        self.keys[index] = None;

        Some((key, value))
    }

    pub fn remove_by_key<Q>(&mut self, key: &Q) -> Option<(K, T)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let (key, index) = self.map.remove_entry(key)?;
        let value = self.values.remove(index)?;
        self.keys[index] = None;

        Some((key, value))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, usize, &T)> {
        self.values
            .iter()
            .zip(self.keys.iter())
            .map(|((index, value), key)| (key.as_ref().unwrap(), index, value))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, usize, &mut T)> {
        self.values
            .iter_mut()
            .zip(self.keys.iter())
            .map(|((index, value), key)| (key.as_ref().unwrap(), index, value))
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.values.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.values.values_mut()
    }
}
