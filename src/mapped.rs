use crate::*;
use std::{borrow::Borrow, collections::hash_map::HashMap, hash::Hash};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KeyIdx<Q> {
    Key(Q),
    Index(usize),
    Both { key: Q, index: usize },
}

impl<Q> KeyIdx<Q> {
    pub fn new(key: Option<Q>, index: Option<usize>) -> Option<KeyIdx<Q>> {
        match (key, index) {
            (Some(key), Some(index)) => Some(KeyIdx::Both { key, index }),
            (Some(key), None) => Some(KeyIdx::Key(key)),
            (None, Some(index)) => Some(KeyIdx::Index(index)),
            (None, None) => None
        }
    }

    pub fn has_key(&self) -> bool {
        match self {
            KeyIdx::Key(_) | KeyIdx::Both { .. } => true,
            _ => false
        }
    }

    pub fn has_index(&self) -> bool {
        match self {
            KeyIdx::Index(_) | KeyIdx::Both { .. } => true,
            _ => false
        }
    } 

    pub fn key(&self) -> Option<&Q> {
        match self {
            KeyIdx::Both { key, .. } => Some(key),
            KeyIdx::Key(key) => Some(key),
            _ => None,
        }
    }

    pub fn index(&self) -> Option<usize> {
        match self {
            KeyIdx::Both { index, .. } => Some(*index),
            KeyIdx::Index(index) => Some(*index),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Occupied<'a, K: 'a, T: 'a> {
    key: &'a K,
    index: usize,
    value: &'a mut T,
}

#[derive(Debug)]
pub struct VacantEntry<'a, K: 'a, T: 'a>
where
    K: Hash + Clone + Eq,
{
    key: K,
    storage: &'a mut MappedNovec<K, T>,
}

#[derive(Debug)]
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
                (index, storage.get_mut_by_index(index).unwrap())
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
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            keys: Vec::new(),
            values: NoVec::new(),
        }
    }

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

    pub fn fill_key_idx<Q>(&self, key_index: &mut KeyIdx<Q>) -> bool
    where
        K: Borrow<Q> + Into<Q>,
        Q: Hash + Eq,
    {
        let mut result = false;

        take_mut::take(key_index, |key_index| match key_index {
            KeyIdx::Key(key) => match self.get_index(&key) {
                Some(index) => KeyIdx::Both { key, index },
                None => {
                    result = true;
                    KeyIdx::Key(key)
                }
            },
            KeyIdx::Index(index) => match self.get_key(index) {
                Some(key) => KeyIdx::Both { key: key.clone().into(), index },
                None => {
                    result = true;
                    KeyIdx::Index(index)
                }
            },
            KeyIdx::Both { key, index } => {
                result = true;
                KeyIdx::Both { key, index }
            }
        });

        result
    }

    pub fn get<Q>(&self, map_index: &KeyIdx<Q>) -> Option<&T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match map_index {
            KeyIdx::Key(key) => self.get_by_key(key),
            KeyIdx::Index(index) => self.get_by_index(*index),
            KeyIdx::Both { index, .. } => self.get_by_index(*index)
        }
    }

    pub fn get_mut<Q>(&mut self, map_index: &KeyIdx<Q>) -> Option<&mut T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match map_index {
            KeyIdx::Key(key) => self.get_mut_by_key(key),
            KeyIdx::Index(index) => self.get_mut_by_index(*index),
            KeyIdx::Both { index, .. } => self.get_mut_by_index(*index)
        }
    }

    pub fn get_index<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.get(key).map(|index| *index)
    }

    pub fn get_by_index(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    pub fn get_mut_by_index(&mut self, index: usize) -> Option<&mut T> {
        self.values.get_mut(index)
    }

    pub fn get_key(&self, index: usize) -> Option<&K> {
        self.keys
            .get(index)
            .map(|value| value.as_ref())
            .unwrap_or(None)
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
