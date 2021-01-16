pub mod file_mapper;
pub mod manager;
pub mod promised;

use crate::{
    generation::GenerationStorage, idvec::IdVec, map::MappedStorage, novec::NoVec,
    ExpandableStorage, KeyIdx, UnorderedStorage,
};
use cbc::*;
use std::{
    any::{Any, TypeId},
    error::Error,
    fmt::{self, Display, Formatter},
    hash::Hash,
};

pub use promised::*;

pub type GenericSender<K> = Sender<(K, PromiseSender<GenericResult, TypeId>)>;
pub type GenericReceiver<K> = Receiver<(K, PromiseSender<GenericResult, TypeId>)>;
pub type GenericPromise<T> = Promise<T, GenericResult>;

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
}

pub trait Loader {
    type Key;
    type Item;
    type Meta;

    fn load(&self, key: Self::Key, into: PromiseSender<Self::Item, Self::Meta>) -> bool;
}

#[derive(Debug)]
pub enum GenericError {
    InvalidType,
    Error(Box<dyn Error>),
}

impl Display for GenericError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            GenericError::InvalidType => write!(f, "Invalid generic item conversion"),
            GenericError::Error(e) => write!(f, "Load error: {}", e),
        }
    }
}

impl Error for GenericError {}

pub enum GenericResult {
    Ok(Box<dyn Any + Send + Sync>),
    Err(Box<dyn Error + Send + Sync>),
}

impl GenericResult {
    pub fn new<T: 'static + Send + Sync>(item: T) -> Self {
        Self::Ok(Box::new(item) as Box<dyn Any + Send + Sync>)
    }

    pub fn new_error<T: 'static + Error + Send + Sync>(error: T) -> Self {
        Self::Err(Box::new(error) as Box<dyn Error + Send + Sync>)
    }
}

impl<T: 'static> Convert<T> for GenericResult {
    type Error = GenericError;

    fn convert(self) -> Result<T, Self::Error> {
        match self {
            GenericResult::Ok(value) => match (value as Box<dyn Any>).downcast::<T>() {
                Ok(value) => Ok(*value),
                Err(_) => Err(GenericError::InvalidType),
            },
            GenericResult::Err(e) => Err(GenericError::Error(e)),
        }
    }
}

impl<K> Loader for GenericSender<K> {
    type Key = K;
    type Item = GenericResult;
    type Meta = TypeId;

    fn load(&self, key: K, into: PromiseSender<GenericResult, TypeId>) -> bool {
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
    L::Item: Convert<T>,
{
    pub storage: MappedStorage<K, S>,
    pending_load: Vec<S::Index>,
    load_errors: Vec<(
        K::Item,
        S::Index,
        PromiseError<<L::Item as Convert<T>>::Error>,
    )>,
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
    L::Item: Convert<T>,
{
    pub fn new() -> Self
    where
        S: Default,
        K: Default,
        L: Default,
    {
        Self {
            storage: MappedStorage::new(),
            pending_load: Vec::new(),
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
            pending_load: Vec::new(),
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

    pub fn set_idx_is_loaded(&self, ki: &mut KeyIdx<K::Item, S::Index>) -> bool {
        if self.storage.set_idx(ki) {
            return self.get_status(ki) == Some(LoadStatus::Loaded);
        }

        false
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
            _ => (),
        }

        let (promise, lock) = Promise::new_waiting(TypeId::of::<T>());
        self.storage.insert_replace_idx(ki, promise);
        self.loader.load(ki.key.clone(), lock);
        self.pending_load.push(ki.index.unwrap());

        LoadStatus::Loading
    }

    pub fn update_loaded(&mut self)
    where
        L::Item: Convert<T>,
    {
        let pending = &mut self.pending_load;
        let storage = &mut self.storage;
        let errors = &mut self.load_errors;

        pending.retain(|idx| {
            let value = match storage.get_by_index_mut(idx) {
                Some(value) => value,
                None => return false,
            };

            match value.update() {
                Ok(status) => status == UpdateStatus::Waiting,
                Err(e) => {
                    errors.push((storage.get_key(idx).unwrap().clone(), *idx, e));
                    false
                }
            }
        });
    }

    pub fn update_loaded_blocking(&mut self)
    where
        L::Item: Convert<T>,
    {
        let pending = &mut self.pending_load;
        let storage = &mut self.storage;
        let errors = &mut self.load_errors;

        pending.retain(|idx| {
            let value = match storage.get_by_index_mut(idx) {
                Some(value) => value,
                None => return false,
            };

            match value.update_blocking() {
                Ok(status) => status == UpdateStatus::Waiting,
                Err(e) => {
                    errors.push((storage.get_key(idx).unwrap().clone(), *idx, e));
                    false
                }
            }
        });
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
    ) -> impl Iterator<
        Item = (
            K::Item,
            S::Index,
            PromiseError<<L::Item as Convert<T>>::Error>,
        ),
    > + 'a {
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
