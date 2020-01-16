pub mod oom;
pub mod generation;
pub mod novec;
pub mod loader;
pub mod idvec;

pub mod map;

#[cfg(test)]
mod test;

// pub use crate::novec::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KeyIdx<K, I> {
    Key(K),
    Index(I),
    Both { key: K, index: I },
}

impl<K, I> KeyIdx<K, I> {
    pub fn new(key: Option<K>, index: Option<I>) -> Option<KeyIdx<K, I>> {
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

    pub fn key(&self) -> Option<&K> {
        match self {
            KeyIdx::Both { key, .. } => Some(key),
            KeyIdx::Key(key) => Some(key),
            _ => None,
        }
    }

    pub fn index(&self) -> Option<&I> {
        match self {
            KeyIdx::Both { index, .. } => Some(index),
            KeyIdx::Index(index) => Some(index),
            _ => None,
        }
    }

    pub fn into_key(self) -> Option<K> {
        match self {
            Self::Key(key) => Some(key),
            Self::Both{ key, .. } => Some(key),
            _ => None 
        }
    }

    pub fn into_index(self) -> Option<I> {
        match self {
            Self::Index(index) => Some(index),
            Self::Both{ index, .. } => Some(index),
            _ => None 
        }
    }
}

pub trait UnorderedStorage {
    type Index;
    type Item;

    fn insert(&mut self, index: &Self::Index, value: Self::Item) -> Option<Self::Item>;
    fn remove(&mut self, index: &Self::Index) -> Option<Self::Item>;
    fn get(&self, index: &Self::Index) -> Option<&Self::Item>;
    fn get_mut(&mut self, index: &Self::Index) -> Option<&mut Self::Item>;
}

pub trait ExpandableStorage: UnorderedStorage {
    fn push(&mut self, value: Self::Item) -> Self::Index;
}