// #[cfg(feature = "json")]
// pub mod json_file;

use crate::{ map::MappedStorage, KeyIdx, ExpandableStorage, UnorderedStorage };
use std::{ hash::Hash, error::Error, borrow::Borrow};

pub trait Loader {
    type Key;
    type Item;

    fn request_load(&self, key: &Self::Key) -> bool;
    fn load_now(&mut self, key: &Self::Key) -> Result<Self::Item, Box<dyn Error>>;
    fn drain_loaded<F: FnMut((Self::Key, Self::Item))>(&mut self, f: F);
}

pub struct StorageSystem<K, S, L> 
where
    S: ExpandableStorage,
    K: UnorderedStorage<Index = S::Index>,
    K::Item: Hash + Eq,
    S::Index: Borrow<K::Index>,
    K::Index: Clone,
    L: Loader<Key = K::Item>, 
    L::Item: Into<S::Item>
{
    storage: MappedStorage<K, S>,
    loader: L
}

impl<K, S, L> StorageSystem<K, S, L> 
where
    S: ExpandableStorage,
    K: UnorderedStorage<Index = S::Index>,
    K::Item: Hash + Eq,
    S::Index: Borrow<K::Index>,
    K::Index: Clone,
    L: Loader<Key = K::Item>, 
    L::Item: Into<S::Item>
{
    pub fn new() -> Self 
    where
        S: Default,
        K: Default,
        L: Default,
    {
        Self {
            storage: MappedStorage::new(),
            loader: L::default()
        }
    }
}