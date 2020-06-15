use std::{hash::Hash, collections::HashMap};

pub mod generation;
pub mod idvec;
pub mod loader;
pub mod novec;
pub mod oom;
pub mod map;

#[cfg(test)]
mod test;

// pub use crate::novec::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct KeyIdx<K, I> {
    pub key: K,
    pub index: Option<I>
}

impl<K, I> KeyIdx<K, I> {
    pub fn new(key: K) -> Self {
        Self {
            key, 
            index: None
        }
    }

    pub fn as_ref(&self) -> KeyIdx<&K, &I> {
        KeyIdx {
            key: &self.key,
            index: self.index.as_ref()
        }
    }

    pub fn key_ref(&self) -> KeyIdx<&K, I> 
    where
        I: Copy
    {
        KeyIdx {
            key: &self.key,
            index: self.index
        }
    }

    pub fn mut_index(&mut self) -> (&K, &mut Option<I>) {
        (&self.key, &mut self.index)
    }

    pub fn has_index(&self) -> bool {
        self.index.is_some()
    }

    pub fn is_only_key(&self) -> bool {
        self.index.is_none()
    }

    pub fn index_ref(&self) -> Option<&I> {
        self.index.as_ref()
    }

    pub fn into_key(self) -> K {
        self.key
    }

    pub fn into_index(self) -> Option<I> {
        self.index
    }
}

impl<'a, K: ?Sized, I> From<(&'a K, &'a mut Option<I>)> for KeyIdx<&'a K, &'a I> {
    fn from((key, idx): (&'a K, &'a mut Option<I>)) -> Self {
        Self {
            key,
            index: idx.as_ref()
        }
    }
}

impl<'a, K: ?Sized, I> From<(&'a K, &'a Option<I>)> for KeyIdx<&'a K, &'a I> {
    fn from((key, idx): (&'a K, &'a Option<I>)) -> Self {
        Self {
            key,
            index: idx.as_ref()
        }
    }
}

impl<'a, K: ?Sized, I> From<(&'a K, Option<I>)> for KeyIdx<&'a K, I> {
    fn from((key, index): (&'a K, Option<I>)) -> Self {
        Self {
            key,
            index
        }
    }
}

pub trait UnorderedStorage {
    type Index;
    type Item;

    fn insert(&mut self, index: Self::Index, value: Self::Item) -> Option<Self::Item>;
    fn remove(&mut self, index: &Self::Index) -> Option<Self::Item>;
    fn get(&self, index: &Self::Index) -> Option<&Self::Item>;
    fn get_mut<'a, 'b>(&'a mut self, index: &'b Self::Index) -> Option<&'a mut Self::Item>;
}

impl<K, T> UnorderedStorage for HashMap<K, T> 
where
    K: Hash + Eq,
{
    type Index = K;
    type Item = T;
    fn insert(&mut self, index: Self::Index, value: Self::Item) -> Option<Self::Item> {
        <HashMap<K, T>>::insert(self, index, value)
    }
    fn remove(&mut self, index: &Self::Index) -> Option<Self::Item> {
        <HashMap<K, T>>::remove(self, index)
    }
    fn get(&self, index: &Self::Index) -> Option<&Self::Item> {
        <HashMap<K, T>>::get(self, index)
    }
    fn get_mut<'a, 'b>(&'a mut self, index: &'b Self::Index) -> Option<&'a mut Self::Item> {
        <HashMap<K, T>>::get_mut(self, index)
    }
}

pub trait ExpandableStorage: UnorderedStorage {
    fn push(&mut self, value: Self::Item) -> Self::Index;
}
