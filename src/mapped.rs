use crate::*;
use std::{borrow::Borrow, collections::hash_map::HashMap, hash::Hash};

// Can be indexed by either a Key or a usize, usize will be faster
// Keys stored in separate vector for faster iteration over values (probably doesnt matter)
#[derive(Clone, Debug)]
pub struct MappedNovec<K, T>
where
    K: Hash + Clone + Eq,
{
    pub map: HashMap<K, usize>,
    keys: Vec<Option<K>>,
    values: NoVec<T>,
}

impl<K, T> MappedNovec<K, T>
where
    K: Hash + Clone + Eq,
{
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

    pub fn insert(&mut self, key: K, value: T) {
        let index = self.values.push(value);

        while self.keys.len() <= index {
            self.keys.push(None);
        }

        self.keys[index] = Some(key.clone());
        self.map.insert(key, index);
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
