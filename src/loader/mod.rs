pub mod file_mapper;
pub mod manager;
pub mod promised;

use crate::{
    generation::GenerationStorage, idvec::IdVec, map::MappedStorage, novec::NoVec,
    ExpandableStorage, KeyIdx, UnorderedStorage,
};
use cbc::*;
use derive_deref::*;
use std::{
    any::{Any, TypeId},
    error::Error,
    fmt::{self, Display, Formatter},
    hash::Hash,
};

pub use promised::*;

pub type GenericSender<K> = Sender<(K, PromiseSender<GenericItem, TypeId>)>;
pub type GenericReceiver<K> = Receiver<(K, PromiseSender<GenericItem, TypeId>)>;
pub type GenericPromise<T> = Promise<T, GenericItem>;

pub type NoVecSystem<K, L, T> = StorageSystem<IdVec<K>, NoVec<GenericPromise<T>>, L, T>;
pub type NoVecLoader<K, T> = NoVecSystem<K, GenericSender<K>, T>;

pub type GenSystem<K, L, T> = StorageSystem<IdVec<K>, GenerationStorage<GenericPromise<T>>, L, T>;
pub type GenLoader<K, T> = GenSystem<K, GenericSender<K>, T>;

pub trait Convert<T> {
    type Error;
    fn convert(self) -> Result<T, Self::Error>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LoadStatus {
    Loaded,
    Loading,
    StartedLoading,
    InvalidKeyIdx,
}

pub trait Loader {
    type Key;
    type Item;
    type Meta;

    fn load(&self, key: Self::Key, into: PromiseSender<Self::Item, Self::Meta>) -> bool;
}

#[derive(Debug, Copy, Clone)]
pub struct InvalidType;

impl Display for InvalidType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Invalid generic item conversion")
    }
}

impl Error for InvalidType {}

#[derive(Deref, DerefMut)]
pub struct GenericItem(pub Box<dyn Any + Send + Sync>);

impl GenericItem {
    pub fn new<T: 'static + Send + Sync>(item: T) -> Self {
        Self(Box::new(item) as Box<dyn Any + Send + Sync>)
    }
}

impl<T: 'static> Convert<T> for GenericItem {
    type Error = InvalidType;

    fn convert(self) -> Result<T, Self::Error> {
        let value = self.0 as Box<dyn Any>;
        match value.downcast::<T>() {
            Ok(value) => Ok(*value),
            Err(_) => Err(InvalidType),
        }
    }
}

impl<K> Loader for GenericSender<K> {
    type Key = K;
    type Item = GenericItem;
    type Meta = TypeId;

    fn load(&self, key: K, into: PromiseSender<GenericItem, TypeId>) -> bool {
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
    S: ExpandableStorage<Item = Promise<T, L::Item>>,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
    S::Index: Into<K::Index> + Copy,
    K::Index: Copy,
    L: Loader<Key = K::Item>,
{
    pub storage: MappedStorage<K, S>,
    load_errors: Vec<(K::Item, S::Index, PromiseError)>,
    loader: L,
}

impl<K, S, L, T> StorageSystem<K, S, L, T>
where
    T: 'static,
    S: ExpandableStorage<Item = Promise<T, L::Item>>,
    K: UnorderedStorage,
    K::Item: Hash + Eq + Clone,
    S::Index: Into<K::Index> + Copy,
    K::Index: Copy,
    L: Loader<Key = K::Item, Meta = TypeId>,
{
    pub fn new() -> Self
    where
        S: Default,
        K: Default,
        L: Default,
    {
        Self {
            storage: MappedStorage::new(),
            load_errors: vec![],
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
            load_errors: vec![],
            loader,
        }
    }

    pub fn get(&self, ki: &KeyIdx<K::Item, S::Index>) -> Option<&T> {
        match self.storage.get(ki) {
            Some(value) => value.get(),
            _ => None,
        }
    }

    pub fn get_by_index(&self, idx: &S::Index) -> Option<&T> {
        match self.storage.get_by_index(idx) {
            Some(value) => value.get(),
            _ => None,
        }
    }

    pub fn set_idx(&self, ki: &mut KeyIdx<K::Item, S::Index>) -> bool {
        self.storage.set_idx(ki)
    }

    pub fn set_idx_get_status(&self, ki: &mut KeyIdx<K::Item, S::Index>) -> Option<LoadStatus> {
        if !self.storage.set_idx(ki) {
            return None;
        }

        match self.storage.get(ki).unwrap() {
            Promise::Owned(_) => Some(LoadStatus::Loaded),
            Promise::Waiting(_) => Some(LoadStatus::Loading),
        }
    }

    pub fn get_status(&self, ki: &KeyIdx<K::Item, S::Index>) -> Option<LoadStatus> {
        self.storage.get(ki).map(|value| match value {
            Promise::Owned(_) => LoadStatus::Loaded,
            Promise::Waiting(_) => LoadStatus::Loading,
        })
    }

    pub fn load(&mut self, ki: &mut KeyIdx<K::Item, S::Index>) -> LoadStatus {
        match self.storage.set_idx_get(ki) {
            Some(Promise::Owned(_)) => return LoadStatus::Loaded,
            Some(Promise::Waiting(_)) => return LoadStatus::Loading,
            _ => ()
        }

        let (promise, lock) = Promise::new_waiting(TypeId::of::<T>());
        self.storage.insert_replace_idx(ki, promise);
        self.loader.load(ki.key.clone(), lock);

        LoadStatus::Loading
    }

    pub fn update_loaded(&mut self)
    where
        L::Item: Convert<T>,
    {
        for (key, idx, value) in self.storage.iter_mut() {
            if let Err(e) = value.update() {
                self.load_errors.push((key.clone(), *idx, e))
            }
        }
    }

    pub fn update_loaded_blocking(&mut self)
    where
        L::Item: Convert<T>,
    {
        for (key, idx, value) in self.storage.iter_mut() {
            if let Err(e) = value.update_blocking() {
                self.load_errors.push((key.clone(), *idx, e))
            }
        }
    }

    // Calls f with each item that is successfully loaded
    pub fn on_update_loaded(&mut self, mut f: impl FnMut(&K::Item, &S::Index, &T))
    where
        L::Item: Convert<T>,
    {
        for (key, idx, value) in self.storage.iter_mut() {
            match value.update() {
                Ok(UpdateStatus::Updated) => f(key, idx, value.get().unwrap()),
                Err(e) => self.load_errors.push((key.clone(), *idx, e)),
                _ => (),
            }
        }
    }

    pub fn on_update_loaded_blocking(&mut self, mut f: impl FnMut(&K::Item, &S::Index, &T))
    where
        L::Item: Convert<T>,
    {
        for (key, idx, value) in self.storage.iter_mut() {
            match value.update_blocking() {
                Ok(UpdateStatus::Updated) => f(key, idx, value.get().unwrap()),
                Err(e) => self.load_errors.push((key.clone(), *idx, e)),
                _ => (),
            }
        }
    }

    pub fn were_errors(&self) -> bool {
        !self.load_errors.is_empty()
    }

    pub fn remove_failed<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = (K::Item, S::Index, PromiseError)> + 'a {
        for (_, idx, _) in self.load_errors.iter() {
            self.storage.remove_with_index(idx);
        }

        self.load_errors.drain(..)
    }

    pub fn values(&self) -> impl Iterator<Item = &'_ T> + '_ {
        self.storage
            .iter()
            .filter(|(_, _, promise)| promise.is_owned())
            .map(|(_, _, promise)| promise.unwrap_ref())
    }
}
