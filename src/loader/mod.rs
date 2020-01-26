// #[cfg(feature = "json")]
// pub mod json_file;
pub mod promised;

use crate::{
    generation::GenerationStorage,
    idvec::IdVec,
    map::{Entry, MappedStorage},
    novec::NoVec,
    ExpandableStorage, KeyIdx, UnorderedStorage,
};
use std::{borrow::Borrow, error::Error, hash::Hash};
use cbc::*;

pub use promised::*;

pub type GenerationSystem<K, L, T> =
    StorageSystem<IdVec<K>, GenerationStorage<PromisedValue<T>>, L, T>;
pub type NoVecSystem<K, L, T> = StorageSystem<IdVec<K>, NoVec<PromisedValue<T>>, L, T>;
pub type GenerationLoader<K, T> = GenerationSystem<K, Sender<(K, OneTimeLock<T>)>, T>;
pub type NoVecLoader<K, T> = NoVecSystem<K, Sender<(K, OneTimeLock<T>)>, T>;

pub trait Loader {
    type Key;
    type Item;

    fn load(&self, key: Self::Key, into: OneTimeLock<Self::Item>) -> bool;
}

impl<K, T> Loader for Sender<(K, OneTimeLock<T>)> {
    type Key = K;
    type Item = T;

    fn load(&self, key: K, into: OneTimeLock<T>) -> bool {
        match self.send((key, into)) {
            Ok(()) => true,
            Err(e) => {
                dbg!(e);
                false
            }
        }
    }
}

pub struct StorageSystem<K, S, L, T>
where
    S: ExpandableStorage<Item = PromisedValue<T>>,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
    S::Index: Into<K::Index> + Copy,
    K::Index: Copy,
    L: Loader<Key = K::Item, Item = T>,
{
    storage: MappedStorage<K, S>,
    loader: L,
}

impl<K, S, L, T> StorageSystem<K, S, L, T>
where
    S: ExpandableStorage<Item = PromisedValue<T>>,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
    S::Index: Into<K::Index> + Copy,
    K::Index: Copy,
    L: Loader<Key = K::Item, Item = T>,
{
    pub fn new() -> Self
    where
        S: Default,
        K: Default,
        L: Default,
    {
        Self {
            storage: MappedStorage::new(),
            loader: L::default(),
        }
    }

    pub fn new_with_loader(loader: L) -> Self 
    where
        S: Default,
        K: Default,
    {
        Self {
            storage: MappedStorage::new(),
            loader,
        }
    }

    pub fn get<Q, I>(&self, ki: &KeyIdx<Q, I>) -> Option<&T>
    where
        K::Item: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq + Into<K::Item>,
    {
        match self.storage.get(ki) {
            Some(value) => value.get(),
            _ => None,
        }
    }

    // Returns if loaded / loading
    pub fn load<Q, I>(&mut self, ki: &mut KeyIdx<Q, I>) -> bool
    where
        K::Item: Clone + Borrow<Q>,
        S::Index: Borrow<I> + Into<I> + Clone,
        Q: Hash + Eq + Into<K::Item> + Clone,
        I: Borrow<S::Index>,
    {
        let to_return = self.storage.contains(ki);
        let (promise, lock) = PromisedValue::new_loading();

        self.storage.insert_replace_idx(ki, promise);
        self.loader.load(ki.key().unwrap().clone().into(), lock);

        to_return
    }

    pub fn update_loaded(&mut self) {
        for value in self.storage.values_mut() {
            value.update_get();
        }
    }
}
